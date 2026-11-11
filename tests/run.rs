//! A run of consecutive moments, drawn with no display: where the route goes, and
//! comfort, measured from the pictures and the camera poses.
mod common;

use common::{num, rising_and_falling_song, run, scratch, series, write_wav};

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
fn a_run_reports_how_fast_the_ground_and_the_land_beside_pass_and_the_song_drives_both() {
    let dir = scratch("run-passing");
    let song = dir.join("song.wav");
    rising_and_falling_song(&song);

    let run = run(
        &song,
        &dir.join("out"),
        &["--rate", "20", "--size", "320x180"],
    );
    // Quiet for 8 s, loud for 8 s, quiet for 8 s: the mean over six seconds of each.
    let mean = |rate: &str, from_s: usize| {
        series(&run, rate)[from_s * 20..(from_s + 6) * 20]
            .iter()
            .sum::<f64>()
            / 120.0
    };
    let (quiet, loud) = (mean("over_ground_per_s", 1), mean("over_ground_per_s", 9));
    assert!(
        loud >= 3.0 * quiet,
        "the ground passes {quiet} times a second in the quiet, {loud} in the loud"
    );
    let (quiet, loud) = (mean("past_land_per_s", 1), mean("past_land_per_s", 9));
    assert!(
        loud >= quiet,
        "the land beside passes {quiet} times a second in the quiet, {loud} in the loud"
    );
    for rate in ["over_ground_per_s", "past_land_per_s"] {
        let summed = &run["passing"][rate];
        assert!(num(&summed["least"]) > 0.0, "{rate}: {summed}");
    }
}

/// One boundary, where the loudness jumps: quiet noise, then loud noise from 10 s on.
#[test]
fn the_camera_leans_as_its_view_turns_and_levels_out_as_gently_as_it_leaned_in() {
    let dir = scratch("run-turn");
    let song = dir.join("song.wav");
    let signal = [common::noise(10.0, 0.02), common::noise(12.0, 0.3)].concat();
    write_wav(&song, &signal, &signal);

    let run = run(
        &song,
        &dir.join("out"),
        &["--rate", "50", "--size", "320x180"],
    );
    let (lean_after_view_s, out_over_in, most_deg) = common::turn(&run, 10.0);
    assert!(
        lean_after_view_s.abs() <= 0.25 && out_over_in <= 1.3 && (1.0..=18.0).contains(&most_deg),
        "the lean peaks {lean_after_view_s} s after the view turns fastest, levels out \
         {out_over_in} times as fast as it leaned in, and reaches {most_deg} degrees"
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
        num(&run["flashes"]["worst_in_any_second"]) < 3.0,
        "full-screen flashes: {}",
        run["flashes"]
    );
    assert!(
        num(&run["camera"]["worst_reversals_in_any_second"]) <= 2.0,
        "the camera does not shake: {}",
        run["camera"]
    );
    let draw = &run["draw_ms"];
    assert!(
        num(&draw["median"]) > 0.0 && num(&draw["median"]) <= num(&draw["max"]),
        "the time to draw a moment: {draw}"
    );
}
