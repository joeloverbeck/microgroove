//! Checks against the first-journey song. They run only where `MICROGROOVE_SONG` is set
//! and the file decodes to the audio identified in issue #1; otherwise each check says
//! that it was skipped. Every expected value here comes from issue #1: the owner's
//! annotation, the independent measurements and ffmpeg's readings.
mod common;

use std::path::PathBuf;
use std::sync::OnceLock;

use common::{Value, f, numbers, numbers_bytes, scratch, skip};

/// MD5 of the first-journey song's audio decoded to 24-bit PCM, from issue #1.
const FIRST_JOURNEY_MD5: &str = "9345b4152173327879c721122c3eb2ba";

/// The analysis numbers of the first-journey song, or why they cannot be had here.
fn first_journey(check: &str) -> Option<&'static Value> {
    static HEARD: OnceLock<Result<Value, String>> = OnceLock::new();
    let heard = HEARD.get_or_init(|| {
        let song = std::env::var_os("MICROGROOVE_SONG").ok_or("MICROGROOVE_SONG is not set")?;
        let song = PathBuf::from(song);
        if !song.is_file() {
            return Err("MICROGROOVE_SONG does not name a file".to_string());
        }
        let heard = numbers(&song, &scratch("real-song"));
        if heard["decoded_audio_md5"] != FIRST_JOURNEY_MD5 {
            return Err(
                "MICROGROOVE_SONG is not the first-journey song (its decoded audio differs)"
                    .to_string(),
            );
        }
        Ok(heard)
    });
    heard.as_ref().map_err(|reason| skip(check, reason)).ok()
}

fn clock(seconds: f64) -> String {
    format!("{}:{:04.1}", (seconds / 60.0) as u32, seconds % 60.0)
}

/// The nine section starts the owner's annotation requires, in seconds.
const REQUIRED_STARTS: [f64; 9] = [28.0, 57.0, 72.0, 94.0, 152.0, 167.0, 188.0, 197.0, 227.0];
/// The five measured, unconfirmed changes: a boundary near one of them is allowed.
const MEASURED_CHANGES: [f64; 5] = [124.0, 136.0, 210.0, 231.0, 243.0];
const WITHIN_S: f64 = 2.0;

#[test]
fn sections_every_required_start_has_a_boundary_and_few_boundaries_are_unexplained() {
    let Some(heard) = first_journey("sections") else {
        return;
    };
    let boundaries = common::times(&heard["boundaries"]);
    let near =
        |times: &[f64], boundary: f64| times.iter().any(|time| (boundary - time).abs() <= WITHIN_S);
    let missing: Vec<String> = REQUIRED_STARTS
        .iter()
        .filter(|start| !near(&boundaries, **start))
        .map(|start| clock(*start))
        .collect();
    let unexplained: Vec<String> = boundaries
        .iter()
        .filter(|boundary| {
            !near(&REQUIRED_STARTS, **boundary) && !near(&MEASURED_CHANGES, **boundary)
        })
        .map(|boundary| clock(*boundary))
        .collect();
    let all: Vec<String> = boundaries.iter().map(|boundary| clock(*boundary)).collect();
    assert!(
        missing.is_empty() && unexplained.len() <= 3,
        "required starts with no boundary within {WITHIN_S} s: {missing:?}; unexplained boundaries: {unexplained:?}; all boundaries: {all:?}"
    );
}

#[test]
fn pulse_is_132_bpm_where_trusted() {
    let Some(heard) = first_journey("pulse") else {
        return;
    };
    let bpm = f(&heard["pulse"]["bpm"]);
    assert!(
        (bpm - 132.0).abs() <= 132.0 * 0.02,
        "expected 132 BPM within 2%, got {bpm} (198 and 66 are the traps)"
    );
    assert!(
        !heard["pulse"]["trusted"].as_array().unwrap().is_empty(),
        "the pulse is trusted somewhere"
    );
}

#[test]
fn intensity_is_lowest_and_highest_where_the_song_is_and_falls_across_the_bridge() {
    let Some(heard) = first_journey("intensity") else {
        return;
    };
    let hop_s = f(&heard["intensity"]["hop_s"]);
    let values: Vec<f64> = heard["intensity"]["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(f)
        .collect();
    let stretch =
        |from_s: f64, to_s: f64| &values[(from_s / hop_s) as usize..(to_s / hop_s) as usize];
    let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len() as f64;
    // The typical intensity of a stretch: its median, which the short dip that issue #1
    // measures inside the Build does not drag down.
    let typical = |from_s: f64, to_s: f64| {
        let mut sorted = stretch(from_s, to_s).to_vec();
        sorted.sort_by(f64::total_cmp);
        sorted[sorted.len() / 2]
    };

    // Lowest in Verse 1 and at 3:18-3:30; highest at 3:30-3:47, at 3:51-4:03 and in the Build.
    let lowest = [
        ("Verse 1", typical(0.0, 28.0)),
        ("3:18-3:30", typical(198.0, 210.0)),
    ];
    let highest = [
        ("3:30-3:47", typical(210.0, 227.0)),
        ("3:51-4:03", typical(231.0, 243.0)),
        ("Build", typical(94.0, 152.0)),
    ];
    // Every other stretch of the annotation. The fade to silence after 4:03 is not a passage.
    let others = [
        ("Verse 2", typical(28.0, 57.0)),
        ("Verse 2 continued", typical(57.0, 72.0)),
        ("Verse 3", typical(72.0, 94.0)),
        ("Bridge", typical(152.0, 167.0)),
        ("Verse 4", typical(167.0, 188.0)),
        ("Verse 5", typical(188.0, 197.0)),
        ("3:47-3:51", typical(227.0, 231.0)),
    ];
    let report = format!("lowest {lowest:?}; highest {highest:?}; others {others:?}");
    for (_, low) in lowest {
        assert!(
            others.iter().chain(&highest).all(|(_, other)| low < *other),
            "{report}"
        );
    }
    for (_, high) in highest {
        assert!(
            others.iter().chain(&lowest).all(|(_, other)| high > *other),
            "{report}"
        );
    }

    // Falling across the Bridge (2:32-2:47): each third lower than the one before.
    let thirds = [
        mean(stretch(152.0, 157.0)),
        mean(stretch(157.0, 162.0)),
        mean(stretch(162.0, 167.0)),
    ];
    assert!(
        thirds[0] > thirds[1] && thirds[1] > thirds[2],
        "intensity across the Bridge, by thirds: {thirds:?}"
    );
}

#[test]
fn loudness_agrees_with_the_ebu_r128_reading_in_the_issue() {
    let Some(heard) = first_journey("loudness") else {
        return;
    };
    // ffmpeg's reading: -14.0 LUFS integrated, loudness range 9.2 LU, peak -1.1 dBFS.
    for (measure, expected) in [
        ("integrated_lufs", -14.0),
        ("range_lu", 9.2),
        ("sample_peak_dbfs", -1.1),
    ] {
        let measured = f(&heard["loudness"][measure]);
        assert!(
            (measured - expected).abs() <= 0.1,
            "{measure}: expected {expected}, got {measured}"
        );
    }
    assert!(
        (f(&heard["duration_s"]) - 246.18).abs() <= 0.01,
        "246.18 seconds long"
    );
    assert_eq!(
        (heard["sample_rate"].as_u64(), heard["channels"].as_u64()),
        (Some(48_000), Some(2))
    );
}

#[test]
fn the_flac_and_the_wav_give_identical_numbers() {
    if first_journey("flac and wav").is_none() {
        return;
    }
    let Some(wav) = std::env::var_os("MICROGROOVE_SONG_WAV")
        .map(PathBuf::from)
        .filter(|wav| wav.is_file())
    else {
        return skip(
            "flac and wav",
            "MICROGROOVE_SONG_WAV is not set to the WAV of the same master",
        );
    };
    let song = PathBuf::from(std::env::var_os("MICROGROOVE_SONG").unwrap());
    assert_ne!(
        song.extension(),
        wav.extension(),
        "the two files are different containers"
    );
    let (from_song, from_wav) = (scratch("real-song-flac"), scratch("real-song-wav"));
    assert!(
        numbers_bytes(&song, &from_song) == numbers_bytes(&wav, &from_wav),
        "analysis numbers differ"
    );
    assert!(
        std::fs::read(from_song.join("timeline.png")).unwrap()
            == std::fs::read(from_wav.join("timeline.png")).unwrap(),
        "timelines differ"
    );
}
