use super::gpu_gate::{self, Engine};
use super::hesitations::clean_hesitations_words;
use crate::domain::transcript::words_from_span;
use crate::domain::{Dictionary, Transcript, Word};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use tokio::sync::Semaphore;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters,
};

static WHISPER_LOCK: Semaphore = Semaphore::const_new(1);

// A chunk is the unit of progress: a pause, a crash or a kill loses at most
// the one in flight.
const CHUNK_MS: u32 = 60_000;
// A hard cut can clip the last word of a chunk; words ending this close to the
// cut are dropped and heard whole at the head of the next chunk.
const TAIL_MS: u32 = 5_000;

// Load the model once and reuse it across transcriptions. Building a WhisperContext loads the full
// (hundreds of MB) model from disk and inits the Metal backend; doing that on every call was the
// multi-second warm-up felt before each transcription. The per-inference state stays cheap and is
// created fresh each call. Keyed by model path so switching the model in Settings reloads it,
// and by engine: a GPU context cannot run on the CPU.
type ContextCache = Mutex<Option<(PathBuf, Engine, Arc<WhisperContext>)>>;
static CONTEXT_CACHE: LazyLock<ContextCache> =
    LazyLock::new(|| Mutex::new(None));

fn cached_context(
    model: &Path,
    engine: Engine,
) -> Result<Arc<WhisperContext>, String> {
    let mut cache = CONTEXT_CACHE.lock().unwrap();
    if let Some((path, cached, ctx)) = cache.as_ref() {
        if path == model && *cached == engine {
            return Ok(ctx.clone());
        }
    }
    // Two copies of a model this size would crowd a phone in the background,
    // where memory is reclaimed first: the old one goes before the new loads.
    *cache = None;
    let model_str = model
        .to_str()
        .ok_or_else(|| "non-utf8 model path".to_string())?;
    let mut params = WhisperContextParameters::default();
    if engine == Engine::Cpu {
        params.use_gpu(false);
    }
    let ctx = Arc::new(
        WhisperContext::new_with_params(model_str, params)
            .map_err(|e| format!("Whisper model load: {e}"))?,
    );
    *cache = Some((model.to_path_buf(), engine, ctx.clone()));
    Ok(ctx)
}

pub fn available_slots() -> usize {
    WHISPER_LOCK.available_permits()
}

pub struct WhisperLocal {
    model_path: PathBuf,
    dictionary: Dictionary,
}

impl WhisperLocal {
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            model_path,
            dictionary: Dictionary::default(),
        }
    }

    /// Whisper has no vocabulary API, so the dictionary only ever acts on the
    /// decoded text. `initial_prompt` biasing is deliberately not used.
    pub fn with_dictionary(mut self, dictionary: Dictionary) -> Self {
        self.dictionary = dictionary;
        self
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub async fn transcribe(
        &self,
        path: &Path,
        language: Option<&str>,
    ) -> Result<Transcript, String> {
        self.transcribe_from(path, language, Checkpoint::default(), |_| {})
            .await
    }

    /// Transcribes from `from.done_ms` on, reporting every finished chunk so
    /// the caller can persist where to resume.
    pub async fn transcribe_from(
        &self,
        path: &Path,
        language: Option<&str>,
        from: Checkpoint,
        mut on_chunk: impl FnMut(&Checkpoint) + Send + 'static,
    ) -> Result<Transcript, String> {
        let _permit = WHISPER_LOCK
            .acquire()
            .await
            .map_err(|e| format!("Whisper lock: {e}"))?;
        let model = self.model_path.clone();
        let audio_path = path.to_path_buf();
        let lang = language.map(str::to_string);
        let raw = tokio::task::spawn_blocking(move || {
            run_whisper(
                &model,
                &audio_path,
                lang.as_deref(),
                from,
                &mut on_chunk,
            )
        })
        .await
        .map_err(|e| format!("Whisper task: {e}"))??;
        let cleaned = clean_hesitations_words(raw);
        Ok(Transcript::new(self.dictionary.apply_words(cleaned)))
    }
}

/// How far a transcription got: raw words (before hesitation and dictionary
/// passes) with absolute timings, and the audio position they cover.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Checkpoint {
    pub done_ms: u32,
    pub words: Vec<Word>,
}

/// One word per segment: whisper.cpp wraps segments after decoding, so this only
/// changes where segments break, never the decoded text. Nothing moves to
/// `WhisperContextParameters`, so `CONTEXT_CACHE` keeps its key and its behaviour
/// - which is also the seam that keeps a later DTW swap cheap.
fn word_params(language: Option<&str>) -> FullParams<'_, '_> {
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(language.unwrap_or("auto")));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_token_timestamps(true);
    params.set_max_len(1);
    params.set_split_on_word(true);
    params
}

fn run_whisper(
    model: &Path,
    wav: &Path,
    language: Option<&str>,
    mut progress: Checkpoint,
    on_chunk: &mut dyn FnMut(&Checkpoint),
) -> Result<Vec<Word>, String> {
    let total_ms = wav_duration_ms(wav)?;
    if total_ms == 0 {
        return Err("Empty audio".to_string());
    }
    while progress.done_ms < total_ms {
        let start_ms = progress.done_ms;
        let end_ms = start_ms.saturating_add(CHUNK_MS).min(total_ms);
        let audio = load_wav_mono_16k_range(wav, start_ms, end_ms)?;
        let busy = gpu_gate::enter();
        let ctx = cached_context(model, busy.engine)?;
        let started = std::time::Instant::now();
        let Some(words) = infer(&ctx, busy.engine, &audio, language, start_ms)?
        else {
            continue;
        };
        eprintln!(
            "[whisper] chunk {start_ms}-{end_ms} ms on {:?} in {} ms",
            busy.engine,
            started.elapsed().as_millis()
        );
        drop(busy);
        let (kept, next_ms) = commit_chunk(start_ms, end_ms, total_ms, words);
        progress.words.extend(kept);
        progress.done_ms = next_ms;
        on_chunk(&progress);
    }
    Ok(progress.words)
}

/// Splits a chunk's words into the ones kept and where the next chunk starts.
/// The last chunk keeps everything. Otherwise words ending in the tail are
/// dropped and the next chunk starts right after the last kept word.
pub fn commit_chunk(
    start_ms: u32,
    end_ms: u32,
    total_ms: u32,
    words: Vec<Word>,
) -> (Vec<Word>, u32) {
    if end_ms >= total_ms {
        return (words, total_ms);
    }
    let cut_ms = end_ms.saturating_sub(TAIL_MS).max(start_ms + 1);
    let kept: Vec<Word> = words
        .into_iter()
        .take_while(|w| w.end_ms <= cut_ms)
        .collect();
    let next_ms = kept.last().map_or(cut_ms, |w| w.end_ms);
    (kept, if next_ms > start_ms { next_ms } else { cut_ms })
}

/// `None` when `engine` had to stop mid-inference: the same audio runs again
/// on whichever engine `gpu_gate` allows next.
fn infer(
    ctx: &WhisperContext,
    engine: Engine,
    audio: &[f32],
    language: Option<&str>,
    offset_ms: u32,
) -> Result<Option<Vec<Word>>, String> {
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("Whisper state: {e}"))?;
    let mut params = word_params(language);
    // whisper-rs 0.16 reads the callback back under the wrong type: only a
    // capture-free function survives it, a closure's captures come back as
    // garbage and abort at random.
    match engine {
        Engine::Gpu => params.set_abort_callback_safe(gpu_must_stop),
        Engine::Cpu => params.set_abort_callback_safe(cpu_must_stop),
    }
    if let Err(e) = state.full(params, audio) {
        if gpu_gate::must_stop(engine) {
            return Ok(None);
        }
        return Err(format!("Whisper inference: {e}"));
    }

    // Everything at or above the end-of-transcript id is a special or timestamp
    // token. Under max_len(1) a segment holds one word, so those are a large
    // fraction of every segment's token set and their probabilities would
    // dominate an unfiltered mean.
    let first_special = ctx.token_eot();
    let mut words = Vec::with_capacity(state.full_n_segments() as usize);
    for i in 0..state.full_n_segments() {
        let Some(segment) = state.get_segment(i) else {
            continue;
        };
        let text = segment.to_str_lossy().unwrap_or_default();
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let start_ms =
            offset_ms + centiseconds_to_ms(segment.start_timestamp());
        let end_ms = (offset_ms + centiseconds_to_ms(segment.end_timestamp()))
            .max(start_ms);
        words.extend(words_from_span(
            text,
            start_ms,
            end_ms,
            segment_confidence(&segment, first_special),
        ));
    }
    Ok(Some(words))
}

fn gpu_must_stop() -> bool {
    gpu_gate::must_stop(Engine::Gpu)
}

fn cpu_must_stop() -> bool {
    gpu_gate::must_stop(Engine::Cpu)
}

fn centiseconds_to_ms(centiseconds: i64) -> u32 {
    (centiseconds.max(0) as u32).saturating_mul(10)
}

fn segment_confidence(
    segment: &whisper_rs::WhisperSegment<'_>,
    first_special: whisper_rs::WhisperTokenId,
) -> f32 {
    let tokens: Vec<(whisper_rs::WhisperTokenId, f32)> = (0..segment
        .n_tokens())
        .filter_map(|t| segment.get_token(t))
        .map(|token| (token.token_id(), token.token_probability()))
        .collect();
    mean_text_probability(&tokens, first_special)
}

/// Mean probability over the text tokens only.
///
/// Split out from the segment walk so the filter can be tested: it is the whole
/// point. Everything at or above the end-of-transcript id is special or a
/// timestamp, and under `max_len(1)` those are a large share of a segment's
/// tokens - their probabilities are not a confidence in anything.
pub fn mean_text_probability(
    tokens: &[(whisper_rs::WhisperTokenId, f32)],
    first_special: whisper_rs::WhisperTokenId,
) -> f32 {
    let kept: Vec<f32> = tokens
        .iter()
        .filter(|(id, _)| *id < first_special)
        .map(|(_, p)| *p)
        .collect();
    if kept.is_empty() {
        return 0.0;
    }
    kept.iter().sum::<f32>() / kept.len() as f32
}

pub fn load_wav_mono_16k(path: &Path) -> Result<Vec<f32>, String> {
    load_wav_mono_16k_range(path, 0, u32::MAX)
}

pub fn wav_duration_ms(path: &Path) -> Result<u32, String> {
    let reader =
        hound::WavReader::open(path).map_err(|e| format!("WAV open: {e}"))?;
    let rate = u64::from(reader.spec().sample_rate.max(1));
    Ok((u64::from(reader.duration()) * 1000 / rate) as u32)
}

/// Decodes `[start_ms, end_ms)` only: an hour-long recording is never held in
/// memory whole.
pub fn load_wav_mono_16k_range(
    path: &Path,
    start_ms: u32,
    end_ms: u32,
) -> Result<Vec<f32>, String> {
    let mut reader =
        hound::WavReader::open(path).map_err(|e| format!("WAV open: {e}"))?;
    let spec = reader.spec();
    let rate = u64::from(spec.sample_rate);
    let frames = u64::from(reader.duration());
    let first = (u64::from(start_ms) * rate / 1000).min(frames);
    let last = (u64::from(end_ms) * rate / 1000).min(frames);
    reader
        .seek(first as u32)
        .map_err(|e| format!("WAV seek: {e}"))?;
    let channels = spec.channels.max(1) as usize;
    let count = (last - first) as usize * channels;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .take(count)
            .collect::<Result<_, _>>()
            .map_err(|e| format!("WAV decode: {e}"))?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .take(count)
            .map(|s| s.map(|v| f32::from(v) / 32768.0))
            .collect::<Result<_, _>>()
            .map_err(|e| format!("WAV decode: {e}"))?,
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .take(count)
                .map(|s| s.map(|v| v as f32 / scale))
                .collect::<Result<_, _>>()
                .map_err(|e| format!("WAV decode: {e}"))?
        }
    };
    let mono: Vec<f32> = if channels == 1 {
        samples
    } else {
        samples
            .chunks(channels)
            .map(|c| c.iter().sum::<f32>() / c.len() as f32)
            .collect()
    };
    if spec.sample_rate == 16_000 {
        Ok(mono)
    } else {
        Ok(resample_linear(&mono, spec.sample_rate, 16_000))
    }
}

fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if input.is_empty() || from == 0 || to == 0 {
        return Vec::new();
    }
    let ratio = f64::from(from) / f64::from(to);
    let out_len = (input.len() as f64 / ratio) as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos as usize;
            let frac = (pos - idx as f64) as f32;
            let a = input[idx.min(input.len() - 1)];
            let b = input[(idx + 1).min(input.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_wav(
        path: &Path,
        sample_rate: u32,
        channels: u16,
        samples: &[i16],
    ) {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for s in samples {
            writer.write_sample(*s).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn semaphore_has_a_single_permit() {
        assert_eq!(available_slots(), 1);
    }

    #[test]
    fn decode_mono_16k_passthrough() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.wav");
        write_wav(&path, 16_000, 1, &[0, 16384, -16384, 0]);
        let audio = load_wav_mono_16k(&path).unwrap();
        assert_eq!(audio.len(), 4);
        assert!((audio[1] - 0.5).abs() < 0.001);
        assert!((audio[2] + 0.5).abs() < 0.001);
    }

    #[test]
    fn decode_stereo_mixes_down_to_mono() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.wav");
        write_wav(&path, 16_000, 2, &[16384, 0, 0, 16384]);
        let audio = load_wav_mono_16k(&path).unwrap();
        assert_eq!(audio.len(), 2);
        assert!((audio[0] - 0.25).abs() < 0.001);
        assert!((audio[1] - 0.25).abs() < 0.001);
    }

    #[test]
    fn decode_44k_resamples_to_16k() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.wav");
        let samples: Vec<i16> = vec![1000; 44_100];
        write_wav(&path, 44_100, 1, &samples);
        let audio = load_wav_mono_16k(&path).unwrap();
        let expected = 16_000usize;
        assert!(
            audio.len().abs_diff(expected) <= 2,
            "got {} samples, expected ~{expected}",
            audio.len()
        );
    }

    #[test]
    fn resample_empty_input_is_empty() {
        assert!(resample_linear(&[], 44_100, 16_000).is_empty());
    }
}
