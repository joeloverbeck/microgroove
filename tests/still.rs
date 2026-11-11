//! One command writes a still of any moment and the numbers behind it, with no display.
//! Stills are compared with each other and by their numbers, never with a stored picture.
mod common;

use std::path::Path;

use common::{Value, f, inspect, json, rising_and_falling_song, scratch, write_wav};

/// Makes a still of each song time in one command. Returns each still and its numbers,
/// in the order asked. Small stills are enough where only the numbers are judged.
fn stills(song: &Path, out: &Path, times: &[&str], small: bool) -> Vec<(Vec<u8>, Value)> {
    let mut options: Vec<&str> = times.iter().flat_map(|time| ["--at", time]).collect();
    if small {
        options.extend(["--size", "320x180"]);
    }
    let written = inspect("still", song, out, &options);
    assert_eq!(written.len(), 2 * times.len(), "{written:?}");
    let still = |pair: &[std::path::PathBuf]| (std::fs::read(&pair[0]).unwrap(), json(&pair[1]));
    written.chunks(2).map(still).collect()
}

#[test]
fn one_command_writes_a_still_and_the_numbers_of_its_moment_with_no_display() {
    let dir = scratch("still-outputs");
    let song = dir.join("song.wav");
    rising_and_falling_song(&song);
    let out = dir.join("out");

    let written = inspect("still", &song, &out, &["--at", "0:12.5"]);
    let [still, numbers] = written.as_slice() else {
        panic!("the command names a still and its numbers: {written:?}");
    };
    assert!(still.starts_with(&out) && numbers.starts_with(&out));

    let picture = std::fs::read(still).unwrap();
    assert_eq!(&picture[..8], b"\x89PNG\r\n\x1a\n", "the still is a PNG");
    let size = |at: usize| u32::from_be_bytes(picture[at..at + 4].try_into().unwrap());
    assert_eq!(
        (size(16), size(20)),
        (3840, 2160),
        "a still is as large as the display unless asked otherwise"
    );

    let moment: Value = json(numbers);
    assert_eq!(f(&moment["song_time_s"]), 12.5);
    assert!(f(&moment["route_position_m"]) > 0.0, "{moment}");
    for part in ["yaw_deg", "pitch_deg", "bank_deg"] {
        assert!(
            moment["camera"][part].is_number(),
            "camera {part}: {moment}"
        );
    }
    assert_eq!(moment["camera"]["position_m"].as_array().unwrap().len(), 3);
    assert!((0.0..=1.0).contains(&f(&moment["light"])), "{moment}");
    assert!(moment["impulses"].is_array(), "{moment}");
}

/// `1:-50` is not ten seconds: the command is not understood, and no file is looked for.
#[test]
fn a_song_time_with_seconds_below_zero_is_not_a_song_time() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_microgroove"))
        .args(["still", "no-such-song.wav", "--at", "1:-50"])
        .output()
        .unwrap();
    let message = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{message}");
    assert!(message.starts_with("usage:"), "{message}");
}

#[test]
fn the_same_moment_gives_the_same_still_on_every_run_and_in_whatever_order() {
    let dir = scratch("still-determinism");
    let song = dir.join("song.wav");
    rising_and_falling_song(&song);

    let first = stills(&song, &dir.join("first"), &["3", "12.5", "20"], false);
    let again = stills(&song, &dir.join("again"), &["3", "12.5", "20"], false);
    let mut reversed = stills(&song, &dir.join("reversed"), &["20", "12.5", "3"], false);
    reversed.reverse();
    for ((first, again), reversed) in first.iter().zip(&again).zip(&reversed) {
        let at = &first.1["song_time_s"];
        assert!(
            first.0 == again.0,
            "two fresh runs give different stills at {at}"
        );
        assert!(
            first.0 == reversed.0,
            "the order of asking changed the still at {at}"
        );
        assert_eq!(
            first.1, reversed.1,
            "the order of asking changed the numbers"
        );
    }
    assert!(
        first[0].0 != first[1].0 && first[1].0 != first[2].0,
        "different moments give different stills"
    );
}

/// The valid no-match outcome: nothing is heard, so the place shows nothing for it.
#[test]
fn a_steady_tone_gives_a_place_with_no_impulses_and_no_region_change() {
    let dir = scratch("still-steady-tone");
    let song = dir.join("tone.wav");
    let signal = common::tone(20.0, 220.0, 0.5);
    write_wav(&song, &signal, &signal);

    for (_, moment) in stills(&song, &dir.join("out"), &["1", "7", "13", "19"], true) {
        assert_eq!(moment["place"]["regions"], 1, "one region: {moment}");
        assert_eq!(moment["place"]["impulses"], 0, "no impulses: {moment}");
        assert_eq!(moment["place"]["pulse_shown"], false, "no pulse: {moment}");
        assert_eq!(moment["region"], 0, "{moment}");
        assert_eq!(moment["impulses"].as_array().unwrap().len(), 0, "{moment}");
    }
}

#[test]
fn the_region_changes_at_a_trusted_boundary_and_only_there() {
    let dir = scratch("still-boundary");
    let song = dir.join("silence-then-noise.wav");
    let mut signal = common::silence(9.0);
    signal.extend(common::noise(13.0, 0.3));
    write_wav(&song, &signal, &signal);

    let moments = stills(&song, &dir.join("out"), &["4", "8.5", "9.5", "20"], true);
    let regions: Vec<&Value> = moments
        .iter()
        .map(|(_, moment)| &moment["region"])
        .collect();
    assert_eq!(regions, [0, 0, 1, 1], "the noise starts at 9 s");
    assert_eq!(moments[0].1["place"]["regions"], 2);
}

#[test]
fn an_impulse_is_in_play_from_its_onset_and_dies_away_with_the_time_since() {
    let dir = scratch("still-impulse");
    let song = dir.join("clicks.wav");
    let mut signal = common::silence(8.0);
    common::add_clicks(&mut signal, &[2.0, 5.0], 0.8);
    write_wav(&song, &signal, &signal);

    let moments = stills(
        &song,
        &dir.join("out"),
        &["1.95", "2.05", "2.25", "3"],
        true,
    );
    let impulses = |index: usize| moments[index].1["impulses"].as_array().unwrap();
    assert_eq!(
        moments[0].1["place"]["impulses"], 2,
        "one impulse for each click"
    );
    assert!(
        impulses(0).is_empty(),
        "nothing answers a click before it sounds"
    );
    assert!(
        impulses(3).is_empty(),
        "an impulse dies away within a second"
    );
    assert_eq!((impulses(1).len(), impulses(2).len()), (1, 1));
    let (early, late) = (&impulses(1)[0], &impulses(2)[0]);
    for (impulse, age) in [(early, 0.05), (late, 0.25)] {
        assert!((f(&impulse["onset_s"]) - 2.0).abs() <= 0.005, "{impulse}");
        assert!((f(&impulse["age_s"]) - age).abs() <= 0.005, "{impulse}");
    }
    assert!(
        f(&late["level"]) < f(&early["level"]) && f(&late["reach_m"]) > f(&early["reach_m"]),
        "the impulse fades and runs on: {early} then {late}"
    );
}
