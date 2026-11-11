//! Independent references: ffmpeg's decode and ffmpeg's EBU R128 reading.
mod common;

use std::path::Path;
use std::process::Command;

use common::{f, numbers, scratch, skip};

fn ffmpeg(args: &[&str], input: &Path, tail: &[&str]) -> Option<(String, String)> {
    let output = Command::new("ffmpeg")
        .args(args)
        .arg("-i")
        .arg(input)
        .args(tail)
        .output()
        .ok()?;
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some((
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

/// ffmpeg's checksum of the audio decoded to 24-bit PCM, or None where ffmpeg is not installed.
fn ffmpeg_md5(song: &Path) -> Option<String> {
    let (stdout, _) = ffmpeg(
        &["-v", "error"],
        song,
        &["-map", "0:a", "-c:a", "pcm_s24le", "-f", "md5", "-"],
    )?;
    Some(stdout.trim().trim_start_matches("MD5=").to_string())
}

/// Forty seconds at four levels, different in each channel. The levels stay within
/// 20 LU of each other so that no stretch sits on the edge of the R128 gates.
fn varied_song(path: &Path) {
    let (mut left, mut right) = (Vec::new(), Vec::new());
    for (noise_level, tone_level) in [(0.05, 0.1), (0.4, 0.5), (0.1, 0.2), (0.2, 0.3)] {
        left.extend(common::noise(10.0, noise_level));
        right.extend(common::tone(10.0, 330.0, tone_level));
    }
    common::write_wav(path, &left, &right);
}

#[test]
fn decoded_audio_matches_ffmpegs_decode() {
    let dir = scratch("reference-decode");
    let wav = dir.join("varied.wav");
    varied_song(&wav);
    let Some(expected) = ffmpeg_md5(&wav) else {
        return skip(
            "decoded_audio_matches_ffmpegs_decode",
            "ffmpeg is not installed",
        );
    };
    let heard = numbers(&wav, &dir);
    assert_eq!(heard["decoded_audio_md5"], expected.as_str());
    assert_eq!(heard["sample_rate"], 48_000);
    assert_eq!(heard["channels"], 2);
    assert_eq!(heard["frames"], 40 * 48_000);

    // The same audio in another container, encoded by ffmpeg, decodes to the same samples.
    let flac = dir.join("varied.flac");
    ffmpeg(&["-v", "error", "-y"], &wav, &[flac.to_str().unwrap()]).unwrap();
    assert_eq!(numbers(&flac, &dir)["decoded_audio_md5"], expected.as_str());
}

#[test]
fn measured_loudness_agrees_with_ffmpegs_ebu_r128_reading() {
    let dir = scratch("reference-loudness");
    let wav = dir.join("varied.wav");
    varied_song(&wav);
    let Some((_, report)) = ffmpeg(
        &["-nostats"],
        &wav,
        &["-filter_complex", "ebur128=peak=sample", "-f", "null", "-"],
    ) else {
        return skip(
            "measured_loudness_agrees_with_ffmpegs_ebu_r128_reading",
            "ffmpeg is not installed",
        );
    };
    let summary = &report[report.rfind("Summary:").expect("ffmpeg printed no summary")..];
    let reading = |label: &str| -> f64 {
        let rest = &summary[summary.find(label).unwrap() + label.len()..];
        rest.split_whitespace().next().unwrap().parse().unwrap()
    };
    let heard = numbers(&wav, &dir);
    let loudness = &heard["loudness"];
    assert!(
        (f(&loudness["integrated_lufs"]) - reading("I:")).abs() <= 0.1,
        "{loudness} against {summary}"
    );
    assert!(
        (f(&loudness["range_lu"]) - reading("LRA:")).abs() <= 0.5,
        "{loudness} against {summary}"
    );
    assert!(
        (f(&loudness["sample_peak_dbfs"]) - reading("Peak:")).abs() <= 0.1,
        "{loudness} against {summary}"
    );
}
