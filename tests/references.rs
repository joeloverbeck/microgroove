//! Independent references: ffmpeg's decode and ffmpeg's EBU R128 reading.
mod common;

use std::path::Path;
use std::process::Command;

use common::{ffmpeg, ffmpeg_md5, num, numbers, scratch, skip};

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

    // The third format read, Ogg Vorbis, does not keep the samples exactly. What must
    // come back is every frame of the song.
    let mut encoders = Command::new("ffmpeg");
    let encoders = encoders.arg("-encoders").output().unwrap().stdout;
    if !String::from_utf8_lossy(&encoders).contains(" libvorbis ") {
        return skip(
            "an Ogg Vorbis file decodes to the song's length",
            "this ffmpeg has no Vorbis encoder (libvorbis)",
        );
    }
    let ogg = dir.join("varied.ogg");
    let vorbis = ["-c:a", "libvorbis", ogg.to_str().unwrap()];
    ffmpeg(&["-v", "error", "-y"], &wav, &vorbis).unwrap();
    assert_eq!(numbers(&ogg, &dir)["frames"], 40 * 48_000);
}

#[test]
fn measured_loudness_agrees_with_ffmpegs_ebu_r128_reading() {
    let dir = scratch("reference-loudness");
    let wav = dir.join("varied.wav");
    varied_song(&wav);
    // The same song with its two channels mixed into one, by ffmpeg.
    let mono = dir.join("mono.wav");
    let mixed = ffmpeg(
        &["-v", "error", "-y"],
        &wav,
        &["-ac", "1", mono.to_str().unwrap()],
    );
    if mixed.is_none() {
        return skip(
            "measured_loudness_agrees_with_ffmpegs_ebu_r128_reading",
            "ffmpeg is not installed",
        );
    }
    for (song, channels) in [(&wav, 2), (&mono, 1)] {
        let measure = ["-filter_complex", "ebur128=peak=sample", "-f", "null", "-"];
        let (_, report) = ffmpeg(&["-nostats"], song, &measure).unwrap();
        let summary = &report[report.rfind("Summary:").expect("ffmpeg printed no summary")..];
        let reading = |label: &str| -> f64 {
            let rest = &summary[summary.find(label).unwrap() + label.len()..];
            rest.split_whitespace().next().unwrap().parse().unwrap()
        };
        let heard = numbers(song, &dir);
        assert_eq!(heard["channels"], channels);
        let loudness = &heard["loudness"];
        assert!(
            (num(&loudness["integrated_lufs"]) - reading("I:")).abs() <= 0.1,
            "{loudness} against {summary}"
        );
        assert!(
            (num(&loudness["range_lu"]) - reading("LRA:")).abs() <= 0.5,
            "{loudness} against {summary}"
        );
        assert!(
            (num(&loudness["sample_peak_dbfs"]) - reading("Peak:")).abs() <= 0.1,
            "{loudness} against {summary}"
        );
    }
}
