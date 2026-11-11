//! A run of consecutive moments, drawn with no display: where the route goes, and
//! comfort, measured from the pictures and the camera poses.
mod common;

use std::path::Path;

use common::{Value, f, inspect, json, rising_and_falling_song, scratch, write_wav};

fn run(song: &Path, out: &Path, options: &[&str]) -> Value {
    let written = inspect("run", song, out, options);
    let [numbers] = written.as_slice() else {
        panic!("the command names the numbers of the run: {written:?}");
    };
    json(numbers)
}

fn series(run: &Value, name: &str) -> Vec<f64> {
    let values = run["series"][name].as_array();
    values
        .unwrap_or_else(|| panic!("no series {name}"))
        .iter()
        .map(f)
        .collect()
}

#[test]
fn position_on_the_route_never_runs_backwards_and_goes_faster_where_the_song_is_intense() {
    let dir = scratch("run-route");
    let song = dir.join("song.wav");
    rising_and_falling_song(&song);

    let run = run(
        &song,
        &dir.join("out"),
        &["--rate", "20", "--size", "320x180"],
    );
    let position = series(&run, "route_position_m");
    assert_eq!(
        position.len(),
        24 * 20,
        "the whole song, 20 moments a second"
    );
    assert!(
        position.windows(2).all(|pair| pair[1] > pair[0]),
        "position on the route moves forward as song time advances"
    );
    // The song is quiet for 8 s, loud for 8 s, quiet for 8 s.
    let travelled = |from_s: usize, to_s: usize| position[to_s * 20] - position[from_s * 20];
    assert!(
        travelled(9, 15) > 1.5 * travelled(1, 7),
        "{} m in six loud seconds, {} m in six quiet ones",
        travelled(9, 15),
        travelled(1, 7)
    );
}

#[test]
fn a_run_at_display_rate_counts_flashes_and_camera_reversals_and_times_the_drawing() {
    // A hard case for comfort: loud noise with eight clicks a second, between quiet stretches.
    let dir = scratch("run-comfort");
    let song = dir.join("song.wav");
    let mut signal = common::noise(4.0, 0.02);
    let mut loud = common::noise(6.0, 0.3);
    common::add_clicks(&mut loud, &common::every(0.1, 0.125, 5.9), 0.7);
    signal.extend(loud);
    signal.extend(common::noise(4.0, 0.02));
    write_wav(&song, &signal, &signal);

    let run = run(
        &song,
        &dir.join("out"),
        &["--from", "2", "--to", "12", "--size", "640x360"],
    );
    assert_eq!(
        run["moments_per_second"], 144,
        "the display's rate unless asked"
    );
    let luminance = series(&run, "luminance");
    assert_eq!(luminance.len(), 1440, "ten seconds of consecutive moments");
    assert!(
        luminance.iter().all(|value| (0.0..=1.0).contains(value)),
        "relative luminance of each picture"
    );
    assert!(
        series(&run, "impulses").iter().any(|count| *count > 0.0),
        "the clicks are answered by impulses"
    );
    assert!(
        f(&run["flashes"]["worst_in_any_second"]) < 3.0,
        "full-screen flashes: {}",
        run["flashes"]
    );
    assert!(
        f(&run["camera"]["worst_reversals_in_any_second"]) <= 2.0,
        "the camera does not shake: {}",
        run["camera"]
    );
    let draw = &run["draw_ms"];
    assert!(
        f(&draw["median"]) > 0.0 && f(&draw["median"]) <= f(&draw["max"]),
        "the time to draw a moment: {draw}"
    );
}
