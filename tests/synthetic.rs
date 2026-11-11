//! Synthetic signals whose answers are known by construction. They test mechanics
//! only and say nothing about fitness for real songs.
mod common;

use common::{Value, f, numbers, scratch, silence, tone, write_wav};

fn curve(value: &Value) -> Vec<f64> {
    value.as_array().unwrap().iter().map(f).collect()
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: expected {expected} within {tolerance}, got {actual}"
    );
}

fn song_with_clicks(path: &std::path::Path, seconds: f64, clicks: &[(&[f64], f32)]) {
    let mut signal = silence(seconds);
    for (times, amplitude) in clicks {
        common::add_clicks(&mut signal, times, *amplitude);
    }
    write_wav(path, &signal, &signal);
}

/// The trusted stretches of the pulse, as (start, end) in seconds.
fn trusted(pulse: &Value) -> Vec<(f64, f64)> {
    pulse["trusted"]
        .as_array()
        .unwrap()
        .iter()
        .map(|span| (f(&span["start_s"]), f(&span["end_s"])))
        .collect()
}

#[test]
fn a_click_sequence_gives_its_onset_times_and_its_pulse_rate() {
    let dir = scratch("synthetic-clicks");
    let song = dir.join("clicks.wav");
    // 140 clicks a minute from 0.5 s to 24 s, in 25 s of silence. The period is not a
    // whole number of analysis frames, so the clicks fall at every alignment to them.
    let clicks = common::every(0.5, 60.0 / 140.0, 24.3);
    song_with_clicks(&song, 25.0, &[(&clicks, 0.8)]);

    let heard = numbers(&song, &dir);
    let onsets = common::times(&heard["onsets"]);
    assert_eq!(
        onsets.len(),
        clicks.len(),
        "one onset per click: {onsets:?}"
    );
    for (onset, click) in onsets.iter().zip(&clicks) {
        assert_close(*onset, *click, 0.005, "onset time of a click");
    }
    assert_close(f(&heard["pulse"]["bpm"]), 140.0, 1.0, "pulse rate");
    let spans = trusted(&heard["pulse"]);
    assert!(
        spans
            .iter()
            .any(|(start, end)| *start <= 2.0 && *end >= 23.0),
        "trusted throughout: {spans:?}"
    );
}

#[test]
fn a_rhythm_at_one_and_a_half_times_the_beat_is_not_taken_for_the_pulse() {
    let dir = scratch("synthetic-one-and-a-half");
    let song = dir.join("one-and-a-half.wav");
    // The beat: 120 clicks a minute for the whole 40 s. On top of it, twice as loud,
    // 180 clicks a minute for two stretches: 60% of the song, as a song might carry
    // such a rhythm through some of its sections.
    let beat = common::every(0.25, 0.5, 39.9);
    let mut rhythm = common::every(0.25, 1.0 / 3.0, 14.0);
    rhythm.extend(common::every(22.25, 1.0 / 3.0, 32.0));
    song_with_clicks(&song, 40.0, &[(&beat, 0.4), (&rhythm, 0.8)]);

    let heard = numbers(&song, &dir);
    assert_close(f(&heard["pulse"]["bpm"]), 120.0, 120.0 * 0.02, "pulse rate");
    assert!(
        !trusted(&heard["pulse"]).is_empty(),
        "the beat is trusted somewhere"
    );
}

#[test]
fn silence_followed_by_noise_gives_exactly_one_section_boundary_where_the_noise_starts() {
    let dir = scratch("synthetic-silence-then-noise");
    let song = dir.join("silence-then-noise.wav");
    let mut signal = silence(9.0);
    signal.extend(common::noise(13.0, 0.3));
    write_wav(&song, &signal, &signal);

    let heard = numbers(&song, &dir);
    let boundaries = common::times(&heard["boundaries"]);
    assert_eq!(
        boundaries.len(),
        1,
        "exactly one boundary: {}",
        heard["boundaries"]
    );
    assert_close(boundaries[0], 9.0, 0.25, "boundary time");
    let confidence = f(&heard["boundaries"][0]["confidence"]);
    assert!(
        (0.0..=1.0).contains(&confidence),
        "confidence is a share: {confidence}"
    );
    // The sections are the stretches between the boundaries: silence, then noise.
    let starts: Vec<f64> = heard["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| f(&s["start_s"]))
        .collect();
    assert_eq!(starts, vec![0.0, boundaries[0]]);
}

/// A change is trusted when it lasts: the same step in level gives a boundary where the
/// sound stays changed, and none where it goes back five seconds later.
#[test]
fn a_change_that_stays_gives_a_boundary_and_the_same_change_soon_undone_gives_none() {
    let dir = scratch("synthetic-lasting-change");
    let (quiet, loud) = (0.2, 0.2 * 10f32.powf(3.7 / 20.0)); // 3.7 dB apart
    let stays = [common::noise(20.0, quiet), common::noise(25.0, loud)].concat();
    let undone = [
        common::noise(20.0, quiet),
        common::noise(5.0, loud),
        common::noise(20.0, quiet),
    ]
    .concat();

    let song = dir.join("stays.wav");
    write_wav(&song, &stays, &stays);
    let boundaries = common::times(&numbers(&song, &dir)["boundaries"]);
    assert_eq!(boundaries.len(), 1, "a step that stays: {boundaries:?}");
    assert_close(boundaries[0], 20.0, 0.25, "boundary time");

    let song = dir.join("undone.wav");
    write_wav(&song, &undone, &undone);
    let boundaries = common::times(&numbers(&song, &dir)["boundaries"]);
    assert!(
        boundaries.is_empty(),
        "a step undone five seconds later: {boundaries:?}"
    );
}

/// The valid no-match outcome: nothing is found, so nothing is reported.
#[test]
fn a_steady_tone_gives_no_onsets_no_boundary_and_no_pulse() {
    let dir = scratch("synthetic-steady-tone");
    for hz in [55.0, 220.0, 3000.0] {
        let song = dir.join(format!("tone-{hz}.wav"));
        let signal = tone(20.0, hz, 0.5);
        write_wav(&song, &signal, &signal);

        let heard = numbers(&song, &dir);
        assert_eq!(
            heard["onsets"].as_array().unwrap().len(),
            0,
            "onsets in a {hz} Hz tone: {}",
            heard["onsets"]
        );
        assert_eq!(
            heard["boundaries"].as_array().unwrap().len(),
            0,
            "boundaries in a {hz} Hz tone"
        );
        assert!(
            heard["pulse"]["bpm"].is_null(),
            "pulse in a {hz} Hz tone: {}",
            heard["pulse"]
        );
        assert!(
            trusted(&heard["pulse"]).is_empty(),
            "trusted pulse in a {hz} Hz tone: {}",
            heard["pulse"]
        );
        assert_eq!(
            heard["sections"].as_array().unwrap().len(),
            1,
            "the whole tone is one section"
        );
    }
}

#[test]
fn a_signal_in_one_channel_only_is_heard_on_that_side() {
    let dir = scratch("synthetic-one-channel");
    let song = dir.join("left-only.wav");
    // Left: a tone of amplitude 0.5 for four seconds, then four seconds of silence. Right: silence.
    let mut left = tone(4.0, 440.0, 0.5);
    left.extend(silence(4.0));
    write_wav(&song, &left, &silence(8.0));

    let heard = numbers(&song, &dir);
    let stereo = &heard["stereo"];
    let (left_level, right_level) = (curve(&stereo["left"]), curve(&stereo["right"]));
    let per_second = (1.0 / f(&stereo["hop_s"])).round() as usize;
    assert_eq!(left_level.len(), 8 * per_second);
    for level in &left_level[..4 * per_second] {
        assert_close(
            *level,
            0.5 / 2f64.sqrt(),
            0.005,
            "level of a sine of amplitude 0.5",
        );
    }
    assert!(
        left_level[4 * per_second..]
            .iter()
            .all(|level| *level == 0.0),
        "the left falls silent at 4 s"
    );
    assert!(
        right_level.iter().all(|level| *level == 0.0),
        "the right is silent throughout"
    );
    assert_close(
        f(&stereo["balance"]),
        -1.0,
        1e-6,
        "balance of a left-only signal",
    );
    assert_close(
        f(&stereo["width"]),
        0.5,
        1e-6,
        "width of a one-sided signal",
    );
}
