use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Stopped(PathBuf),
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
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let supported = device
            .default_input_config()
            .map_err(|e| format!("Input config error: {e}"))?;

        let config = supported.config();
        self.sample_rate = config.sample_rate;
        self.channels = config.channels;

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
                |err| eprintln!("Stream error: {err}"),
                None,
            )
            .map_err(|e| format!("Build stream error: {e}"))?;

        stream.play().map_err(|e| format!("Play error: {e}"))?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self, output_dir: &str) -> Result<PathBuf, String> {
        self.stream.take();

        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return Err("No audio data recorded".into());
        }

        let path = wav_path(output_dir);
        write_wav(&path, &samples, self.sample_rate, self.channels)?;
        Ok(path)
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
    let host = cpal::default_host();
    host.default_input_device().is_some()
}

pub fn generate_test_wav(output_dir: &str) -> Result<PathBuf, String> {
    let sample_rate = 44100_u32;
    let duration_secs = 3.0_f32;
    let frequency = 440.0_f32;
    let num_samples = (duration_secs * sample_rate as f32) as usize;

    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (t * frequency * 2.0 * std::f32::consts::PI).sin()
        })
        .collect();

    let path = PathBuf::from(output_dir).join("test_sine_440hz.wav");
    write_wav(&path, &samples, sample_rate, 1)?;
    Ok(path)
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
