//! A file that cannot be read is refused plainly; nothing is substituted.
mod common;

use common::{run_timeline, scratch};

fn assert_refused(output: &std::process::Output, out: &std::path::Path, expected: &str) {
    let message = common::complaint(output);
    assert!(
        message.contains(expected),
        "expected {expected:?} in the message, got {message:?}"
    );
    assert!(
        !out.join("analysis.json").exists(),
        "nothing may be written for a refused file"
    );
    assert!(
        !out.join("timeline.png").exists(),
        "nothing may be written for a refused file"
    );
}

#[test]
fn a_missing_file_is_refused() {
    let dir = scratch("refusal-missing");
    let output = run_timeline(&dir.join("no-such-song.flac"), &dir);
    assert_refused(&output, &dir, "does not exist");
}

#[test]
fn an_unreadable_file_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("refusal-unreadable");
    let song = dir.join("locked.wav");
    common::write_wav(
        &song,
        &common::tone(0.5, 440.0, 0.5),
        &common::tone(0.5, 440.0, 0.5),
    );
    std::fs::set_permissions(&song, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(&song).is_ok() {
        return common::skip(
            "an_unreadable_file_is_refused",
            "this user can read any file",
        );
    }
    let output = run_timeline(&song, &dir);
    assert_refused(&output, &dir, "cannot be read");
}

#[test]
fn a_file_cut_off_part_way_is_refused() {
    let dir = scratch("refusal-cut-off");
    let whole = dir.join("whole.wav");
    common::write_wav(
        &whole,
        &common::tone(1.0, 440.0, 0.5),
        &common::tone(1.0, 440.0, 0.5),
    );
    let bytes = std::fs::read(&whole).unwrap();
    let song = dir.join("cut-off.wav");
    std::fs::write(&song, &bytes[..bytes.len() / 2 + 1]).unwrap();
    let output = run_timeline(&song, &dir);
    assert_refused(&output, &dir, "cannot be decoded");
}

/// AIFF is real audio that the program's decoders do not cover.
#[test]
fn an_unsupported_format_is_refused() {
    let dir = scratch("refusal-unsupported");
    let song = dir.join("song.aiff");
    let frames = 4800u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"FORM");
    bytes.extend_from_slice(&(46 + frames * 4).to_be_bytes());
    bytes.extend_from_slice(b"AIFFCOMM");
    bytes.extend_from_slice(&18u32.to_be_bytes());
    bytes.extend_from_slice(&2u16.to_be_bytes()); // channels
    bytes.extend_from_slice(&frames.to_be_bytes());
    bytes.extend_from_slice(&16u16.to_be_bytes()); // bits per sample
    bytes.extend_from_slice(&[0x40, 0x0E, 0xBB, 0x80, 0, 0, 0, 0, 0, 0]); // 48 kHz, 80-bit float
    bytes.extend_from_slice(b"SSND");
    bytes.extend_from_slice(&(8 + frames * 4).to_be_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend(std::iter::repeat_n(0u8, frames as usize * 4));
    std::fs::write(&song, bytes).unwrap();
    let output = run_timeline(&song, &dir);
    assert_refused(&output, &dir, "not a supported audio format");
}

/// Analysis cannot handle every sample rate a file can have: one too low for the bands
/// of its spectrum to fit, one too low for its loudness meter to weigh, and one beyond
/// what that meter takes.
#[test]
fn a_sample_rate_analysis_cannot_handle_is_refused() {
    let dir = scratch("refusal-sample-rate");
    for rate in [800, 2_000, 3_000_000] {
        let song = dir.join(format!("{rate}-hz.wav"));
        let tone = common::tone(0.5, 440.0, 0.5);
        common::write_wav_at(&song, rate, &tone, &tone);
        let output = run_timeline(&song, &dir);
        assert_refused(&output, &dir, &format!("its sample rate is {rate} Hz"));
    }
}

/// The ground of a place goes to the GPU whole, and the GPU holds only so much of it:
/// forty minutes of steadily loud noise run further than that.
#[test]
fn a_song_too_long_for_the_place_to_hold_is_refused() {
    let dir = scratch("refusal-too-long");
    let song = dir.join("forty-minutes.wav");
    let noise = common::noise(200.0, 0.3);
    common::write_wav_at(&song, 4_000, &noise, &noise);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_microgroove"))
        .args(["still", "--at", "1", "--out"])
        .args([&dir, &song])
        .output()
        .unwrap();
    assert_refused(&output, &dir, "cannot hold a song this long yet, only ");
    assert!(!dir.join("still-0001.000.png").exists(), "no still is made");
}

#[test]
fn a_file_that_is_not_audio_is_refused() {
    let dir = scratch("refusal-not-audio");
    let song = dir.join("notes.flac");
    std::fs::write(&song, "these are words, not sound\n".repeat(400)).unwrap();
    let output = run_timeline(&song, &dir);
    assert_refused(&output, &dir, "not a supported audio format");
}
