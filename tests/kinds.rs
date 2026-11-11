//! Issue #8: a region is a kind of place, and its section's own sound chooses the kind.
//! Synthetic signals whose answers are known by construction, judged by the numbers
//! beside a still.
mod common;

use common::{moments, scratch, tone, write_wav};

/// The kinds of place a region can be, as the numbers beside a still name them.
const KINDS: [&str; 4] = ["water", "dunes", "rock", "ice"];

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

/// The valid no-match outcome: one sound throughout is one kind of place throughout.
#[test]
fn a_steady_tone_is_one_region_of_one_kind() {
    let dir = scratch("kinds-steady-tone");
    let song = dir.join("tone.wav");
    let signal = tone(24.0, 220.0, 0.5);
    write_wav(&song, &signal, &signal);

    let moments = moments(&song, &dir.join("out"), &["3", "12", "21"]);
    let kind = moments[0]["kind"].as_str().expect("a still names its kind");
    assert!(KINDS.contains(&kind), "{kind} is no kind of place");
    for moment in &moments {
        assert_eq!(moment["place"]["regions"], 1, "one region: {moment}");
        assert_eq!(moment["kind"], kind, "{moment}");
        assert_eq!(moment["place"]["kinds"], serde_json::json!([kind]));
    }
}
