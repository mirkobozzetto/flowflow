use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Transcribing,
    /// Dictation only: the transcript goes to whatever input field is
    /// listening. A kept recording never reaches this state - its transcript
    /// travels through the durable `TranscriptionManager` queue instead.
    Transcribed {
        transcript: crate::domain::Transcript,
    },
    Error(String),
}

#[derive(Clone, Debug)]
pub enum InterruptionEvent {
    Began,
    Ended { should_resume: bool },
}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    channels: u16,
    interruption_rx: Option<std::sync::mpsc::Receiver<InterruptionEvent>>,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            sample_rate: 44100,
            channels: 1,
            interruption_rx: None,
        }
    }

    fn build_stream(&mut self) -> Result<(), String> {
        #[cfg(target_os = "ios")]
        crate::infrastructure::platform::ios::configure_audio_session();

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
        Ok(())
    }

    pub fn setup_interruption_channel(
        &mut self,
    ) -> std::sync::mpsc::Sender<InterruptionEvent> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.interruption_rx = Some(rx);
        tx
    }

    pub fn poll_interruption(&self) -> Option<InterruptionEvent> {
        self.interruption_rx.as_ref()?.try_recv().ok()
    }

    pub fn on_interruption_began(&mut self) {
        eprintln!("[audio] interruption began");
        self.stream.take();
    }

    pub fn on_interruption_ended(
        &mut self,
        should_resume: bool,
    ) -> Result<(), String> {
        eprintln!("[audio] interruption ended, should_resume={should_resume}");
        if should_resume {
            self.build_stream()?;
        }
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), String> {
        eprintln!("[audio] start recording");
        self.samples.lock().unwrap().clear();
        self.build_stream()?;
        #[cfg(target_os = "ios")]
        {
            let tx = self.setup_interruption_channel();
            crate::infrastructure::platform::ios::set_interruption_sender(tx);
            crate::infrastructure::platform::ios::observe_interruptions();
            crate::infrastructure::platform::ios::live_activity::start();
        }
        eprintln!("[audio] recording started");
        Ok(())
    }

    pub fn pause(&mut self) {
        eprintln!("[audio] pause recording");
        self.stream.take();
        #[cfg(target_os = "ios")]
        crate::infrastructure::platform::ios::live_activity::update(true);
    }

    pub fn resume(&mut self) -> Result<(), String> {
        eprintln!("[audio] resume recording");
        self.build_stream()?;
        #[cfg(target_os = "ios")]
        crate::infrastructure::platform::ios::live_activity::update(false);
        eprintln!("[audio] recording resumed");
        Ok(())
    }

    pub fn cancel(&mut self) {
        eprintln!("[audio] cancel recording");
        self.stream.take();
        self.samples.lock().unwrap().clear();
        #[cfg(target_os = "ios")]
        crate::infrastructure::platform::ios::live_activity::end();
    }

    pub fn stop(&mut self, output_dir: &str) -> Result<PathBuf, String> {
        eprintln!("[audio] stop recording");
        self.stream.take();
        #[cfg(target_os = "ios")]
        crate::infrastructure::platform::ios::live_activity::end();

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
        for (i, level) in levels.iter_mut().enumerate().take(num_bars) {
            let from = start + i * chunk;
            let to = from + chunk;
            let rms: f32 = samples[from..to].iter().map(|s| s * s).sum::<f32>()
                / chunk as f32;
            *level = (rms.sqrt() * 8.0).min(1.0);
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

pub fn output_dir() -> String {
    #[cfg(target_os = "ios")]
    {
        let dir = crate::infrastructure::platform::ios::documents_dir()
            .join("flowflow");
        std::fs::create_dir_all(&dir).ok();
        dir.to_string_lossy().to_string()
    }
    #[cfg(not(target_os = "ios"))]
    {
        let dir = crate::infrastructure::persistence::desktop_data_dir();
        std::fs::create_dir_all(&dir).ok();
        dir.to_string_lossy().to_string()
    }
}

pub fn resolve_audio_path(filename: &str) -> String {
    let dir = output_dir();
    format!("{dir}/{filename}")
}

pub fn audio_filename(absolute_path: &str) -> String {
    std::path::Path::new(absolute_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(absolute_path)
        .to_string()
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
