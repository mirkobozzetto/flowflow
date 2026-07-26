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

// Load the model once and reuse it across transcriptions. Building a WhisperContext loads the full
// (hundreds of MB) model from disk and inits the Metal backend; doing that on every call was the
// multi-second warm-up felt before each transcription. The per-inference state stays cheap and is
// created fresh each call. Keyed by model path so switching the model in Settings reloads it.
type ContextCache = Mutex<Option<(PathBuf, Arc<WhisperContext>)>>;
static CONTEXT_CACHE: LazyLock<ContextCache> =
    LazyLock::new(|| Mutex::new(None));

fn cached_context(model: &Path) -> Result<Arc<WhisperContext>, String> {
    let mut cache = CONTEXT_CACHE.lock().unwrap();
    if let Some((path, ctx)) = cache.as_ref() {
        if path == model {
            return Ok(ctx.clone());
        }
    }
    let model_str = model
        .to_str()
        .ok_or_else(|| "non-utf8 model path".to_string())?;
    let ctx = Arc::new(
        WhisperContext::new_with_params(
            model_str,
            WhisperContextParameters::default(),
        )
        .map_err(|e| format!("Whisper model load: {e}"))?,
    );
    *cache = Some((model.to_path_buf(), ctx.clone()));
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
        let _permit = WHISPER_LOCK
            .acquire()
            .await
            .map_err(|e| format!("Whisper lock: {e}"))?;
        let model = self.model_path.clone();
        let audio_path = path.to_path_buf();
        let lang = language.map(str::to_string);
        let raw = tokio::task::spawn_blocking(move || {
            run_whisper(&model, &audio_path, lang.as_deref())
        })
        .await
        .map_err(|e| format!("Whisper task: {e}"))??;
        let cleaned = clean_hesitations_words(raw);
        Ok(Transcript::new(self.dictionary.apply_words(cleaned)))
    }
}

pub async fn bench(
    model_path: PathBuf,
    wav: PathBuf,
) -> Result<String, String> {
    let _permit = WHISPER_LOCK
        .acquire()
        .await
        .map_err(|e| format!("Whisper lock: {e}"))?;
    tokio::task::spawn_blocking(move || {
        let audio = load_wav_mono_16k(&wav)?;
        let audio_secs = audio.len() as f32 / 16_000.0;
        let started = std::time::Instant::now();
        let text = Transcript::new(run_whisper(&model_path, &wav, None)?).text();
        let elapsed_ms = started.elapsed().as_millis();
        let rss_mb = peak_rss_mb();
        let preview: String = text.chars().take(120).collect();
        Ok(format!(
            "{audio_secs:.0}s audio -> {elapsed_ms} ms, RSS {rss_mb} MB\n{preview}"
        ))
    })
    .await
    .map_err(|e| format!("Whisper task: {e}"))?
}

fn peak_rss_mb() -> u64 {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) == 0 {
            (usage.ru_maxrss as u64) / 1_048_576
        } else {
            0
        }
    }
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
) -> Result<Vec<Word>, String> {
    let ctx = cached_context(model)?;
    let audio = load_wav_mono_16k(wav)?;
    if audio.is_empty() {
        return Err("Empty audio".to_string());
    }
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("Whisper state: {e}"))?;
    state
        .full(word_params(language), &audio)
        .map_err(|e| format!("Whisper inference: {e}"))?;

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
        let start_ms = centiseconds_to_ms(segment.start_timestamp());
        let end_ms = centiseconds_to_ms(segment.end_timestamp()).max(start_ms);
        words.extend(words_from_span(
            text,
            start_ms,
            end_ms,
            segment_confidence(&segment, first_special),
        ));
    }
    Ok(words)
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
    let reader =
        hound::WavReader::open(path).map_err(|e| format!("WAV open: {e}"))?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| format!("WAV decode: {e}"))?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .into_samples::<i16>()
            .map(|s| s.map(|v| f32::from(v) / 32768.0))
            .collect::<Result<_, _>>()
            .map_err(|e| format!("WAV decode: {e}"))?,
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|v| v as f32 / scale))
                .collect::<Result<_, _>>()
                .map_err(|e| format!("WAV decode: {e}"))?
        }
    };
    let channels = spec.channels.max(1) as usize;
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
