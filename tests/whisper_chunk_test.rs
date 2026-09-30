use flowflow::domain::Word;
use flowflow::infrastructure::transcription::whisper::{
    commit_chunk, load_wav_mono_16k_range, wav_duration_ms,
};

fn word(start_ms: u32, end_ms: u32) -> Word {
    Word::new("w", start_ms, end_ms, 1.0)
}

#[test]
fn a_middle_chunk_drops_its_tail_and_restarts_after_the_last_kept_word() {
    let words = vec![
        word(60_500, 61_000),
        word(110_000, 114_000),
        word(116_000, 119_900),
    ];

    let (kept, next_ms) = commit_chunk(60_000, 120_000, 600_000, words);

    assert_eq!(kept, vec![word(60_500, 61_000), word(110_000, 114_000)]);
    assert_eq!(next_ms, 114_000);
}

#[test]
fn a_silent_middle_chunk_advances_to_its_cut() {
    let (kept, next_ms) = commit_chunk(60_000, 120_000, 600_000, Vec::new());

    assert!(kept.is_empty());
    assert_eq!(next_ms, 115_000);
}

#[test]
fn the_last_chunk_keeps_every_word_and_finishes() {
    let words = vec![word(590_000, 599_900)];

    let (kept, next_ms) =
        commit_chunk(540_000, 600_000, 600_000, words.clone());

    assert_eq!(kept, words);
    assert_eq!(next_ms, 600_000);
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
