use flowflow::infrastructure::transcription::whisper::{
    load_wav_mono_16k_range, quiet_cut, wav_duration_ms,
};

const RATE: usize = 16_000;

fn loud_minute() -> Vec<f32> {
    (0..60 * RATE)
        .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
        .collect()
}

fn silence(audio: &mut [f32], from_ms: usize, to_ms: usize) {
    audio[from_ms * RATE / 1000..to_ms * RATE / 1000].fill(0.0);
}

#[test]
fn the_cut_lands_in_the_quiet_frame_of_the_last_ten_seconds() {
    let mut audio = loud_minute();
    silence(&mut audio, 55_000, 55_100);

    assert_eq!(quiet_cut(&audio), 55_050 * RATE / 1000);
}

#[test]
fn a_quiet_spot_before_the_last_ten_seconds_is_ignored() {
    let mut audio = loud_minute();
    silence(&mut audio, 30_000, 31_000);
    silence(&mut audio, 52_000, 52_100);

    assert_eq!(quiet_cut(&audio), 52_050 * RATE / 1000);
}

#[test]
fn a_silent_chunk_still_cuts_after_fifty_seconds() {
    let audio = vec![0.0; 60 * RATE];

    assert!(quiet_cut(&audio) >= 50 * RATE);
}

#[test]
fn a_range_decodes_only_its_own_samples() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ramp.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    for second in 0..3i16 {
        for _ in 0..16_000 {
            writer.write_sample(second * 8_192).unwrap();
        }
    }
    writer.finalize().unwrap();

    let middle = load_wav_mono_16k_range(&path, 1_000, 2_000).unwrap();

    assert_eq!(wav_duration_ms(&path).unwrap(), 3_000);
    assert_eq!(middle.len(), 16_000);
    assert!(middle.iter().all(|s| (s - 0.25).abs() < 0.001));
}
