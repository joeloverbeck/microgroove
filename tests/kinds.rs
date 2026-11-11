//! Issue #8: a region is a kind of place, and its section's own sound chooses the kind.
//! Synthetic signals whose answers are known by construction, judged by the numbers
//! beside a still.
mod common;

use common::{moments, scratch, tone, write_wav};

/// A section that returns is the same kind of place revisited.
#[test]
fn a_quiet_thin_sound_then_a_loud_dense_one_then_the_first_again_is_three_regions_of_two_kinds() {
    let dir = scratch("kinds-there-and-back");
    let song = dir.join("there-and-back.wav");
    // Thin: one quiet tone. Dense: loud noise, which sounds in every band.
    let thin = tone(12.0, 440.0, 0.02);
    let signal = [thin.clone(), common::noise(12.0, 0.3), thin].concat();
    write_wav(&song, &signal, &signal);

    let moments = moments(&song, &dir.join("out"), &["6", "18", "30"]);
    let regions: Vec<_> = moments.iter().map(|moment| &moment["region"]).collect();
    assert_eq!(regions, [0, 1, 2], "a region for each sound");
    let kinds: Vec<&str> = moments
        .iter()
        .map(|moment| moment["kind"].as_str().expect("a still names its kind"))
        .collect();
    assert!(
        kinds[0] == kinds[2] && kinds[0] != kinds[1],
        "the first and third the same kind, the second another: {kinds:?}"
    );
    assert_eq!(moments[0]["place"]["kinds"], serde_json::json!(kinds));
}

/// The song dying away is not a place of its own; an ending long enough to be one is.
#[test]
fn a_tail_of_a_few_seconds_takes_the_kind_of_the_region_before_it() {
    let dir = scratch("kinds-tail");
    let kinds_with_an_ending_of = |seconds: f64| {
        let song = dir.join(format!("ending-{seconds}.wav"));
        let signal = [common::noise(12.0, 0.3), tone(seconds, 440.0, 0.02)].concat();
        write_wav(&song, &signal, &signal);
        let out = dir.join(format!("out-{seconds}"));
        moments(&song, &out, &["6"])[0]["place"]["kinds"].clone()
    };
    let (tail, ending) = (kinds_with_an_ending_of(2.5), kinds_with_an_ending_of(8.0));
    let kinds = |kinds: &common::Value| kinds.as_array().expect("a kind for each region").clone();
    let (tail, ending) = (kinds(&tail), kinds(&ending));
    assert_eq!((tail.len(), ending.len()), (2, 2), "{tail:?}, {ending:?}");
    assert_eq!(tail[0], tail[1], "2.5 s of another sound to end with");
    assert_ne!(ending[0], ending[1], "8 s of another sound to end with");
    assert_eq!(tail[0], ending[0], "the same sound before each ending");
}

/// The valid no-match outcome: one sound throughout is one kind of place throughout.
#[test]
fn a_steady_tone_is_one_region_of_one_kind() {
    let dir = scratch("kinds-steady-tone");
    let song = dir.join("tone.wav");
    let signal = tone(24.0, 220.0, 0.5);
    write_wav(&song, &signal, &signal);

    let moments = moments(&song, &dir.join("out"), &["3", "12", "21"]);
    let kind = moments[0]["kind"].as_str().expect("a still names its kind");
    for moment in &moments {
        assert_eq!(moment["place"]["regions"], 1, "one region: {moment}");
        assert_eq!(moment["kind"], kind, "{moment}");
        assert_eq!(moment["place"]["kinds"], serde_json::json!([kind]));
    }
}
