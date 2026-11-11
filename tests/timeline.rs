//! One command writes a timeline and the analysis numbers, the same every time.
mod common;

use common::{Value, curve, num, numbers, numbers_bytes, rising_and_falling_song};
use common::{run_timeline, scratch};

#[test]
fn one_command_writes_a_timeline_and_the_numbers_it_was_drawn_from() {
    let dir = scratch("timeline-outputs");
    let song = dir.join("song.wav");
    rising_and_falling_song(&song);
    let out = dir.join("out");

    let output = run_timeline(&song, &out);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let said = String::from_utf8_lossy(&output.stdout);
    for name in ["timeline.png", "analysis.json"] {
        assert!(
            said.contains(out.join(name).to_str().unwrap()),
            "the command says where {name} went: {said}"
        );
    }

    let png = std::fs::read(out.join("timeline.png")).unwrap();
    assert_eq!(
        &png[..8],
        b"\x89PNG\r\n\x1a\n",
        "the timeline is a PNG picture"
    );
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    assert!(
        width >= 1200,
        "the timeline is wide enough to read song time from: {width}"
    );

    let heard: Value =
        serde_json::from_slice(&std::fs::read(out.join("analysis.json")).unwrap()).unwrap();
    for part in [
        "intensity",
        "onsets",
        "pulse",
        "boundaries",
        "sections",
        "fingerprint",
        "stereo",
        "loudness",
    ] {
        assert!(!heard[part].is_null(), "the numbers cover {part}");
    }
}

#[test]
fn intensity_rises_and_falls_with_the_song() {
    let dir = scratch("timeline-intensity");
    let song = dir.join("song.wav");
    rising_and_falling_song(&song);
    let heard = numbers(&song, &dir);
    let intensity = curve(&heard["intensity"]["values"]);
    let per_second = (1.0 / num(&heard["intensity"]["hop_s"])).round() as usize;
    assert_eq!(
        intensity.len(),
        24 * per_second,
        "one value for each step of the song"
    );
    assert!(
        intensity.iter().all(|value| (0.0..=1.0).contains(value)),
        "intensity runs from 0 to 1"
    );
    let mean = |from: usize, to: usize| {
        intensity[from * per_second..to * per_second]
            .iter()
            .sum::<f64>()
            / ((to - from) * per_second) as f64
    };
    let (before, during, after) = (mean(1, 7), mean(9, 15), mean(17, 23));
    assert!(
        during > before + 0.5 && during > after + 0.5,
        "quiet {before}, loud {during}, quiet {after}"
    );
}

#[test]
fn two_fresh_runs_on_the_same_file_give_byte_identical_results() {
    let dir = scratch("timeline-determinism");
    let song = dir.join("song.wav");
    rising_and_falling_song(&song);
    let (first, second) = (dir.join("first"), dir.join("second"));
    assert_eq!(
        numbers_bytes(&song, &first),
        numbers_bytes(&song, &second),
        "analysis numbers"
    );
    assert_eq!(
        std::fs::read(first.join("timeline.png")).unwrap(),
        std::fs::read(second.join("timeline.png")).unwrap(),
        "timeline"
    );
}
