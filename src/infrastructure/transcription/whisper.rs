use super::hesitations::clean_hesitations;
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
}

impl WhisperLocal {
    pub fn new(model_path: PathBuf) -> Self {
        Self { model_path }
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub async fn transcribe(
        &self,
        path: &Path,
        language: Option<&str>,
    ) -> Result<String, String> {
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
        Ok(clean_hesitations(&raw))
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
        let text = run_whisper(&model_path, &wav, None)?;
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

fn run_whisper(
    model: &Path,
    wav: &Path,
    language: Option<&str>,
) -> Result<String, String> {
    let ctx = cached_context(model)?;
    let audio = load_wav_mono_16k(wav)?;
    if audio.is_empty() {
        return Err("Empty audio".to_string());
    }
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(language.unwrap_or("auto")));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("Whisper state: {e}"))?;
    state
        .full(params, &audio)
        .map_err(|e| format!("Whisper inference: {e}"))?;
    let n = state.full_n_segments();
    let mut text = String::new();
    for i in 0..n {
        if let Some(seg) = state.get_segment(i) {
            text.push_str(&seg.to_str_lossy().unwrap_or_default());
        }
    }
    Ok(text.trim().to_string())
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
