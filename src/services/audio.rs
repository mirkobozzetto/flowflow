use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Stopped(PathBuf),
    Transcribing,
    Transcribed(String),
    Error(String),
}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    channels: u16,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            sample_rate: 44100,
            channels: 1,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        eprintln!("[audio] start recording");
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let supported = device.default_input_config().map_err(|e| {
            eprintln!("[audio] input config error: {e}");
            format!("Input config error: {e}")
        })?;

        let config = supported.config();
        self.sample_rate = config.sample_rate;
        self.channels = config.channels;
        eprintln!(
            "[audio] config: {}Hz, {} ch",
            self.sample_rate, self.channels
        );

        let samples = self.samples.clone();
        samples.lock().unwrap().clear();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = samples.lock() {
                        buf.extend_from_slice(data);
                    }
                },
                |err| eprintln!("[audio] stream error: {err}"),
                None,
            )
            .map_err(|e| {
                eprintln!("[audio] build stream error: {e}");
                format!("No mic available (simulator?): {e}")
            })?;

        stream.play().map_err(|e| {
            eprintln!("[audio] play error: {e}");
            format!("Play error: {e}")
        })?;
        self.stream = Some(stream);
        eprintln!("[audio] recording started");
        Ok(())
    }

    pub fn stop(&mut self, output_dir: &str) -> Result<PathBuf, String> {
        eprintln!("[audio] stop recording");
        self.stream.take();

        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            eprintln!("[audio] no data recorded");
            return Err("No audio data recorded".into());
        }

        let path = wav_path(output_dir);
        eprintln!(
            "[audio] writing {} samples to {}",
            samples.len(),
            path.display()
        );
        write_wav(&path, &samples, self.sample_rate, self.channels)?;
        eprintln!("[audio] saved {}", path.display());
        Ok(path)
    }

    pub fn current_levels(&self, num_bars: usize) -> Vec<f32> {
        let samples = self.samples.lock().unwrap();
        let len = samples.len();
        if len < 200 || num_bars == 0 {
            return vec![0.0; num_bars];
        }
        let window = (num_bars * 300).min(len);
        let start = len - window;
        let chunk = window / num_bars;
        let mut levels = vec![0.0f32; num_bars];
        for i in 0..num_bars {
            let from = start + i * chunk;
            let to = from + chunk;
            let rms: f32 = samples[from..to].iter().map(|s| s * s).sum::<f32>()
                / chunk as f32;
            levels[i] = (rms.sqrt() * 8.0).min(1.0);
        }
        levels
    }

    pub fn duration_secs(&self) -> f32 {
        let count = self.samples.lock().unwrap().len();
        if self.sample_rate == 0 || self.channels == 0 {
            return 0.0;
        }
        count as f32 / (self.sample_rate as f32 * self.channels as f32)
    }
}

pub fn has_input_device() -> bool {
    cpal::default_host().default_input_device().is_some()
}

pub fn output_dir() -> String {
    #[cfg(target_os = "ios")]
    {
        let dir = crate::platform::ios::documents_dir().join("flowflow");
        std::fs::create_dir_all(&dir).ok();
        dir.to_string_lossy().to_string()
    }
    #[cfg(not(target_os = "ios"))]
    {
        let mut dir = std::env::temp_dir();
        dir.push("flowflow");
        std::fs::create_dir_all(&dir).ok();
        dir.to_string_lossy().to_string()
    }
}

fn wav_path(output_dir: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    PathBuf::from(output_dir).join(format!("recording_{ts}.wav"))
}

fn write_wav(
    path: &PathBuf,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<(), String> {
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| format!("Create WAV error: {e}"))?;
    for &s in samples {
        let amplitude = (s * i16::MAX as f32) as i16;
        writer
            .write_sample(amplitude)
            .map_err(|e| format!("Write sample error: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("Finalize WAV error: {e}"))?;
    Ok(())
}
