//! Issue #7: the land beside the route is the song's spectrum, and the onsets that
//! stand out are things the listener passes. Synthetic signals whose answers are known
//! by construction, judged by the numbers beside a still.
mod common;

use std::path::Path;

use common::{Value, curve, inspect, json, num, scratch, silence, write_wav};

/// The numbers of a small still at each song time, in the order asked.
fn moments(song: &Path, out: &Path, times: &[&str]) -> Vec<Value> {
    let mut options: Vec<&str> = times.iter().flat_map(|time| ["--at", time]).collect();
    options.extend(["--size", "320x180"]);
    let written = inspect("still", song, out, &options);
    assert_eq!(written.len(), 2 * times.len(), "{written:?}");
    written.chunks(2).map(|pair| json(&pair[1])).collect()
}

/// The land's profile across the route at the listener's position: its height in metres
/// on the left and on the right, from beside the route outwards.
fn profile(moment: &Value) -> (Vec<f64>, Vec<f64>) {
    let land = &moment["land"];
    let (left, right) = (curve(&land["left_m"]), curve(&land["right_m"]));
    let across = curve(&land["across_m"]);
    assert!(
        across.len() >= 12 && across.len() == left.len() && across.len() == right.len(),
        "a height on each side at each distance: {land}"
    );
    assert!(
        across[0] > 0.0 && across.windows(2).all(|pair| pair[1] > pair[0]),
        "distances from the route, outwards: {land}"
    );
    (left, right)
}

fn tallest(heights: &[f64]) -> f64 {
    heights.iter().copied().fold(0.0, f64::max)
}

#[test]
fn sound_in_the_left_channel_only_raises_the_land_on_the_left_and_not_on_the_right() {
    let dir = scratch("land-left-only");
    let song = dir.join("left-only.wav");
    write_wav(&song, &common::noise(12.0, 0.3), &silence(12.0));

    for moment in moments(&song, &dir.join("out"), &["3", "6", "9"]) {
        let (left, right) = profile(&moment);
        assert!(
            tallest(&left) > 10.0 && tallest(&right) == 0.0,
            "left {left:?}, right {right:?}"
        );
    }
}

/// The tallest the land stands near the route and far out from it: over the nearest
/// third of the profile and over the farthest.
fn near_and_far(moment: &Value) -> (f64, f64) {
    let (left, right) = profile(moment);
    let third = left.len() / 3;
    let of =
        |from: usize| tallest(&left[from..from + third]).max(tallest(&right[from..from + third]));
    (of(0), of(left.len() - third))
}

#[test]
fn a_low_tone_then_a_high_tone_gives_land_taller_near_the_route_then_taller_far_out() {
    let dir = scratch("land-low-then-high");
    let song = dir.join("low-then-high.wav");
    let signal = [
        common::tone(12.0, 70.0, 0.5),
        common::tone(12.0, 8000.0, 0.5),
    ]
    .concat();
    write_wav(&song, &signal, &signal);

    let moments = moments(&song, &dir.join("out"), &["4", "20"]);
    let (low, high) = (near_and_far(&moments[0]), near_and_far(&moments[1]));
    assert!(
        low.0 > 1.0 && low.0 > low.1 && low.0 > high.0,
        "under the low tone the land stands {} m near the route and {} m far out; under the \
         high tone {} m near",
        low.0,
        low.1,
        high.0
    );
    assert!(
        high.1 > 1.0 && high.1 > high.0 && high.1 > low.1,
        "under the high tone the land stands {} m far out and {} m near the route; under the \
         low tone {} m far out",
        high.1,
        high.0,
        low.1
    );
}

/// The valid no-match outcome, and a held sound as a ridge that runs beside the listener.
#[test]
fn a_steady_tone_gives_land_that_does_not_change_along_the_route_one_region_and_no_forms() {
    let dir = scratch("land-steady-tone");
    let song = dir.join("tone.wav");
    let signal = common::tone(24.0, 220.0, 0.5);
    write_wav(&song, &signal, &signal);

    let moments = moments(&song, &dir.join("out"), &["6", "9", "12.5", "15", "18"]);
    let (first, _) = profile(&moments[0]);
    assert!(tallest(&first) > 1.0, "the tone raises land: {first:?}");
    for moment in &moments {
        let (left, right) = profile(moment);
        for (here, there) in left.iter().chain(&right).zip(first.iter().chain(&first)) {
            assert!(
                (here - there).abs() <= 0.01 * tallest(&first),
                "the land at {} s is not the land at 6 s: {left:?} and {right:?} against {first:?}",
                moment["song_time_s"]
            );
        }
        assert_eq!(moment["place"]["regions"], 1, "one region: {moment}");
        assert_eq!(moment["place"]["forms"], 0, "no forms: {moment}");
        assert_eq!(moment["forms"].as_array().unwrap().len(), 0, "{moment}");
    }
}

/// Adds a sound that starts at once and dies away within a twentieth of a second.
fn strike(signal: &mut [f32], at_s: f64, hz: f64, amplitude: f32) {
    let start = (at_s * f64::from(common::RATE)).round() as usize;
    for (n, sample) in common::tone(0.05, hz, amplitude).iter().enumerate() {
        signal[start + n] += sample * (-(n as f32) / 400.0).exp();
    }
}

#[test]
fn onsets_that_stand_out_get_a_form_of_their_kind_on_their_side_where_their_time_is() {
    let dir = scratch("land-forms");
    let song = dir.join("strikes.wav");
    // Four low thumps, four middle bursts and four high clicks a second apart, with a
    // faint one of each kind among them. The middle burst at 9 s is in the left channel
    // only, with the power the others have in their two channels together.
    let kinds = [
        ("low", 80.0, 2.0),
        ("middle", 1000.0, 3.0),
        ("high", 8000.0, 4.0),
    ];
    let (mut left, mut right) = (silence(16.0), silence(16.0));
    for (_, hz, first_s) in kinds {
        for at_s in common::every(first_s, 3.0, 14.0) {
            if at_s == 9.0 {
                strike(&mut left, at_s, hz, 0.6 * std::f32::consts::SQRT_2);
                continue;
            }
            for channel in [&mut left, &mut right] {
                strike(channel, at_s, hz, 0.6);
            }
        }
        for channel in [&mut left, &mut right] {
            strike(channel, first_s + 3.5, hz, 0.02);
        }
    }
    write_wav(&song, &left, &right);

    // Every form is in view from some one of these moments.
    let out = dir.join("out");
    let mut forms: Vec<Value> = moments(&song, &out, &["1", "4.2", "7.2", "10.2"])
        .iter()
        .flat_map(|moment| moment["forms"].as_array().unwrap().clone())
        .collect();
    forms.sort_by(|a, b| num(&a["onset_s"]).total_cmp(&num(&b["onset_s"])));
    forms.dedup();
    let found: Vec<(f64, &str, &str)> = forms
        .iter()
        .map(|form| {
            let (kind, side) = (form["kind"].as_str(), form["side"].as_str());
            (num(&form["onset_s"]).round(), kind.unwrap(), side.unwrap())
        })
        .collect();
    let expected: Vec<(f64, &str, &str)> = (2..14)
        .map(|second| {
            let side = if second == 9 { "left" } else { "centre" };
            (f64::from(second), kinds[(second as usize - 2) % 3].0, side)
        })
        .collect();
    assert_eq!(
        found, expected,
        "a form for each strong strike and no other: {forms:?}"
    );

    // The listener is at a form's place on the route as its strike sounds.
    let times: Vec<String> = found
        .iter()
        .map(|(second, ..)| second.to_string())
        .collect();
    let times: Vec<&str> = times.iter().map(String::as_str).collect();
    for (moment, form) in moments(&song, &dir.join("passing"), &times)
        .iter()
        .zip(&forms)
    {
        let (listener, stands) = (num(&moment["route_position_m"]), num(&form["route_m"]));
        let within = 0.02 * num(&moment["speed_m_s"]);
        assert!(
            (listener - stands).abs() <= within,
            "the form of the strike at {} s stands at {stands} m; the listener is at \
             {listener} m then, and should be within {within} m",
            moment["song_time_s"]
        );
    }
}
