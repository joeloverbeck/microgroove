//! Checks against the first-journey song. They run only where `MICROGROOVE_SONG` is set
//! and the file decodes to the audio identified in issue #1; otherwise each check says
//! that it was skipped. Every expected value here comes from issue #1: the owner's
//! annotation, the independent measurements and ffmpeg's readings.
mod common;

use std::path::PathBuf;
use std::sync::OnceLock;

use common::{Value, inspect, num, numbers, numbers_bytes, run, scratch, series, skip};

/// MD5 of the first-journey song's audio decoded to 24-bit PCM, from issue #1.
const FIRST_JOURNEY_MD5: &str = "9345b4152173327879c721122c3eb2ba";

/// The analysis numbers of the first-journey song, or why they cannot be had here. Where
/// ffmpeg is installed it is ffmpeg that says which song the file is, so that a fault in
/// the program's own decode fails these checks instead of skipping them.
fn first_journey(check: &str) -> Option<&'static Value> {
    static HEARD: OnceLock<Result<Value, String>> = OnceLock::new();
    let heard = HEARD.get_or_init(|| {
        let song = std::env::var_os("MICROGROOVE_SONG").ok_or("MICROGROOVE_SONG is not set")?;
        let song = PathBuf::from(song);
        if !song.is_file() {
            return Err("MICROGROOVE_SONG does not name a file".to_string());
        }
        let other = "MICROGROOVE_SONG is not the first-journey song (its decoded audio differs)";
        let by_ffmpeg = common::ffmpeg_md5(&song);
        if by_ffmpeg.as_deref().is_some_and(|md5| md5 != FIRST_JOURNEY_MD5) {
            return Err(other.to_string());
        }
        let heard = numbers(&song, &scratch("real-song"));
        if heard["decoded_audio_md5"] != FIRST_JOURNEY_MD5 {
            let decoded = &heard["decoded_audio_md5"];
            assert!(
                by_ffmpeg.is_none(),
                "ffmpeg decodes MICROGROOVE_SONG to the first-journey song; the program decodes it to {decoded}"
            );
            return Err(other.to_string());
        }
        Ok(heard)
    });
    heard.as_ref().map_err(|reason| skip(check, reason)).ok()
}

fn clock(seconds: f64) -> String {
    format!("{}:{:04.1}", (seconds / 60.0) as u32, seconds % 60.0)
}

/// The seven section starts the owner's annotation requires, in seconds. Issue #1 listed
/// nine. On 2026-11-11 the owner listened again and reported that two of them, 0:57 and
/// 3:08, mark the lyrics more than the music, as 3:36 does, so no boundary is required
/// there (recorded in the comments on issue #1).
const REQUIRED_STARTS: [f64; 7] = [28.0, 72.0, 94.0, 152.0, 167.0, 197.0, 227.0];
/// The five measured, unconfirmed changes: a boundary near one of them is allowed.
const MEASURED_CHANGES: [f64; 5] = [124.0, 136.0, 210.0, 231.0, 243.0];
/// Two changes the owner hears inside a verse (same listening): a lower passage from about
/// 1:00, and the guitar going from single strums to continuous ones at 3:08. A boundary
/// near one of them is allowed. Whether either must be a boundary is the owner's to say.
const HEARD_INSIDE_A_VERSE: [f64; 2] = [60.0, 188.0];
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
            !near(&REQUIRED_STARTS, **boundary)
                && !near(&MEASURED_CHANGES, **boundary)
                && !near(&HEARD_INSIDE_A_VERSE, **boundary)
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
    let bpm = num(&heard["pulse"]["bpm"]);
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
    // Each stretch is summed up by its mean. The Build is taken without the dip that
    // issue #1 measures inside it at about 2:04-2:16: the issue lists that dip as a
    // change of its own, so "highest in the Build" does not speak of it.
    let mean = |from_s: f64, to_s: f64| mean_intensity(heard, from_s, to_s);
    let build = (30.0 * mean(94.0, 124.0) + 16.0 * mean(136.0, 152.0)) / 46.0;

    // Lowest in Verse 1 and at 3:18-3:30; highest at 3:30-3:47, at 3:51-4:03 and in the Build.
    let lowest = [
        ("Verse 1", mean(0.0, 28.0)),
        ("3:18-3:30", mean(198.0, 210.0)),
    ];
    let highest = [
        ("3:30-3:47", mean(210.0, 227.0)),
        ("3:51-4:03", mean(231.0, 243.0)),
        ("Build", build),
    ];
    // Every other stretch of the annotation. The fade to silence after 4:03 is not a passage.
    let others = [
        ("Verse 2", mean(28.0, 57.0)),
        ("Verse 2 continued", mean(57.0, 72.0)),
        ("Verse 3", mean(72.0, 94.0)),
        ("Bridge", mean(152.0, 167.0)),
        ("Verse 4", mean(167.0, 188.0)),
        ("Verse 5", mean(188.0, 197.0)),
        ("3:47-3:51", mean(227.0, 231.0)),
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
    let thirds = [mean(152.0, 157.0), mean(157.0, 162.0), mean(162.0, 167.0)];
    assert!(
        thirds[0] > thirds[1] && thirds[1] > thirds[2],
        "intensity across the Bridge, by thirds: {thirds:?}"
    );
}

/// The average intensity between two song times.
fn mean_intensity(heard: &Value, from_s: f64, to_s: f64) -> f64 {
    let hop_s = num(&heard["intensity"]["hop_s"]);
    let values = heard["intensity"]["values"].as_array().unwrap();
    let stretch = &values[(from_s / hop_s) as usize..(to_s / hop_s) as usize];
    stretch.iter().map(num).sum::<f64>() / stretch.len() as f64
}

/// What the owner heard on listening again on 2026-11-11 (comments on issue #1): two
/// changes that stay inside a verse, which intensity must still follow.
#[test]
fn intensity_follows_what_the_owner_hears_inside_the_verses() {
    let Some(heard) = first_journey("intensity inside the verses") else {
        return;
    };
    // A lower passage from about 1:00 to the start of Verse 3, inside Verse 2.
    let verse_2 = mean_intensity(heard, 30.0, 54.0);
    let lower_passage = mean_intensity(heard, 61.0, 71.0);
    assert!(
        lower_passage < verse_2,
        "the lower passage reads {lower_passage} against {verse_2} in Verse 2 before it"
    );

    // A build-up across 2:58-3:12: its last third above its first, which is all that
    // the owner's words say of it.
    let first = mean_intensity(heard, 178.0, 182.67);
    let last = mean_intensity(heard, 187.33, 192.0);
    assert!(
        first < last,
        "intensity across 2:58-3:12: {first} in the first third, {last} in the last"
    );
}

/// The passages issue #1 names as the quietest and as the loudest, in seconds of the song.
const QUIETEST: [(usize, usize); 2] = [(0, 28), (198, 210)];
const LOUDEST: [(usize, usize); 4] = [(94, 124), (136, 152), (210, 227), (231, 243)];

/// Issue #7: the onsets that stand out, each of which the place makes a form of, come to
/// between 0.7 and 2.5 a second over the song, and to fewer in the passages issue #1
/// names as the quietest than in those it names as the loudest. All the onsets together
/// come to about 7.5 a second in every part of the song.
#[test]
fn fewer_onsets_stand_out_where_the_song_is_quiet_than_where_it_is_loud() {
    let Some(heard) = first_journey("onsets that stand out") else {
        return;
    };
    let onsets = heard["onsets"].as_array().unwrap().iter();
    let standing_out = onsets.filter(|onset| onset["stands_out"] == true);
    let times: Vec<f64> = standing_out.map(|onset| num(&onset["time_s"])).collect();
    let rate = |(from_s, to_s): (f64, f64)| {
        let within = times.iter().filter(|time| (from_s..to_s).contains(*time));
        within.count() as f64 / (to_s - from_s)
    };
    let through = |(from_s, to_s): (usize, usize)| rate((from_s as f64, to_s as f64));
    let (quiet, loud) = (QUIETEST.map(through), LOUDEST.map(through));
    let by_twenty: Vec<f64> = (0..12)
        .map(|n| 20.0 * rate((n as f64 * 20.0, n as f64 * 20.0 + 20.0)))
        .collect();
    let overall = rate((0.0, num(&heard["duration_s"])));
    assert!(
        (0.7..=2.5).contains(&overall) && quiet.iter().all(|q| loud.iter().all(|l| q < l)),
        "{overall} a second over the song; {quiet:?} in the quietest passages, {loud:?} in the \
         loudest; in each 20 s: {by_twenty:?}"
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
        let measured = num(&heard["loudness"][measure]);
        assert!(
            (measured - expected).abs() <= 0.1,
            "{measure}: expected {expected}, got {measured}"
        );
    }
    assert!(
        (num(&heard["duration_s"]) - 246.18).abs() <= 0.01,
        "246.18 seconds long"
    );
    assert_eq!(
        (heard["sample_rate"].as_u64(), heard["channels"].as_u64()),
        (Some(48_000), Some(2))
    );
}

#[test]
fn the_flac_and_the_wav_give_identical_numbers() {
    let Some((song, wav)) = first_journey_song_and_its_wav("flac and wav") else {
        return;
    };
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

/// The first-journey song's file, for the checks on the place made from it.
fn first_journey_song(check: &str) -> Option<PathBuf> {
    first_journey(check)?;
    std::env::var_os("MICROGROOVE_SONG").map(PathBuf::from)
}

/// The same, with the WAV of the same master, for the checks that set the two side by side.
fn first_journey_song_and_its_wav(check: &str) -> Option<(PathBuf, PathBuf)> {
    let song = first_journey_song(check)?;
    let wav = std::env::var_os("MICROGROOVE_SONG_WAV").map(PathBuf::from);
    let wav = wav.filter(|wav| wav.is_file());
    if wav.is_none() {
        let reason = "MICROGROOVE_SONG_WAV is not set to the WAV of the same master";
        skip(check, reason);
    }
    Some((song, wav?))
}

#[test]
fn the_flac_and_the_wav_give_identical_stills() {
    let Some((song, wav)) = first_journey_song_and_its_wav("stills from flac and wav") else {
        return;
    };
    // A quiet moment, a loud one, and one a tenth of a second after the surge at 3:31.9.
    let moments = ["--at", "0:10", "--at", "1:50", "--at", "3:32"];
    let from_song = inspect("still", &song, &scratch("real-stills-flac"), &moments);
    let from_wav = inspect("still", &wav, &scratch("real-stills-wav"), &moments);
    assert_eq!((from_song.len(), from_wav.len()), (6, 6));
    for (of_song, of_wav) in from_song.iter().zip(&from_wav) {
        assert!(
            std::fs::read(of_song).unwrap() == std::fs::read(of_wav).unwrap(),
            "{:?} differs between the FLAC and the WAV",
            of_song.file_name().unwrap()
        );
    }
}

/// Issue #8. The groundwork on that issue grouped the song's fourteen sections like with
/// like, outside the program and from more measurements than the program takes: A B B B C
/// D C C B C A C C, and the two-second tail, which takes the kind of the region before it.
/// Sections it grouped together are one kind of place, and sections it kept apart are not,
/// so the kind changes at 8 of the 13 boundaries.
#[test]
fn sections_that_sound_alike_are_the_same_kind_of_place_and_the_others_are_not() {
    let Some(song) = first_journey_song("kinds of place") else {
        return;
    };
    let grouped: Vec<char> = "ABBBCDCCBCACCC".chars().collect();
    let options = ["--at", "0:10", "--size", "320x180"];
    let written = inspect("still", &song, &scratch("real-kinds"), &options);
    let kinds = common::json(&written[1])["place"]["kinds"].clone();
    let kinds = kinds.as_array().expect("a kind for each region");
    assert_eq!(kinds.len(), grouped.len(), "{kinds:?}");
    for (one, other) in (0..kinds.len()).flat_map(|one| (0..one).map(move |other| (one, other))) {
        assert_eq!(
            kinds[one] == kinds[other],
            grouped[one] == grouped[other],
            "regions {other} and {one}, counted from 0, of {kinds:?}"
        );
    }
}

/// A run of the whole first-journey song at 50 moments a second, for the checks on its motion.
fn whole_run(check: &str) -> Option<&'static Value> {
    static RUN: OnceLock<Value> = OnceLock::new();
    let song = first_journey_song(check)?;
    let options = ["--rate", "50", "--size", "320x180"];
    Some(RUN.get_or_init(|| run(&song, &scratch("real-motion"), &options)))
}

#[test]
fn position_on_the_route_never_runs_backwards_across_the_whole_song() {
    let Some(whole) = whole_run("route") else {
        return;
    };
    let position = series(whole, "route_position_m");
    assert_eq!(position.len(), (246.18f64 * 50.0).round() as usize);
    assert!(
        position.windows(2).all(|pair| pair[1] > pair[0]),
        "position on the route moves forward as song time advances"
    );
}

/// Issue #6, over the passages issue #1 names as the quietest and the loudest: the ground
/// passes at least three times as fast in the loud ones, and the land beside no slower.
#[test]
fn the_ground_passes_three_times_as_fast_in_the_loudest_passages_as_in_the_quietest() {
    let Some(whole) = whole_run("passing") else {
        return;
    };
    let mean = |rate: &str, (from_s, to_s): (usize, usize)| {
        let stretch = &series(whole, rate)[from_s * 50..to_s * 50];
        stretch.iter().sum::<f64>() / stretch.len() as f64
    };
    for quiet in QUIETEST {
        for loud in LOUDEST {
            let ground = [quiet, loud].map(|passage| mean("over_ground_per_s", passage));
            let land = [quiet, loud].map(|passage| mean("past_land_per_s", passage));
            assert!(
                ground[1] >= 3.0 * ground[0] && land[1] >= land[0],
                "{quiet:?} then {loud:?}: over the ground {ground:?}, past the land {land:?}"
            );
        }
    }
    // Nor does the land beside pass slower through the two quiet passages and the ice at
    // 2:05-2:12 than it did at viewing 1, by the report on issue #6.
    for (passage, at_viewing_1) in [((1, 28), 1.17), ((198, 210), 1.24), ((125, 132), 0.9)] {
        let land = mean("past_land_per_s", passage);
        assert!(land >= at_viewing_1, "{passage:?}: past the land {land}");
    }
}

/// Issue #6, through each turn of the song: the lean peaks within 0.25 s of the view's
/// fastest turning, levels out at most 1.3 times as fast as it leaned in, and stays within 18°.
#[test]
fn through_every_turn_the_camera_leans_as_its_view_turns_and_levels_out_gently() {
    let Some(whole) = whole_run("turns") else {
        return;
    };
    let heard = first_journey("turns").expect("heard for the run");
    for boundary in common::times(&heard["boundaries"]) {
        common::assert_the_camera_leans_as_its_view_turns(whole, boundary);
    }
}

/// Comfort across the most intense stretches named in issue #1: the Build, 3:30-3:47 and
/// 3:51-4:03, as consecutive moments at the display's 144 a second. No full-screen flash
/// in any one second, and a camera that does not shake: no part of its pose turns back on
/// itself more than twice in any one second. And no flash across any boundary (issue #8):
/// no second of the whole song, at 50 moments a second, holds the change of 0.10 in the
/// picture's light that a flash is made of twice over.
#[test]
fn the_most_intense_stretches_stay_within_the_comfort_limits() {
    let Some(song) = first_journey_song("comfort") else {
        return;
    };
    for (from, to) in [("1:34", "2:32"), ("3:30", "3:47"), ("3:51", "4:03")] {
        let stretch = run(
            &song,
            &scratch("real-comfort"),
            &["--from", from, "--to", to, "--size", "960x540"],
        );
        assert_eq!(stretch["moments_per_second"], 144);
        let flashes = &stretch["flashes"];
        assert_eq!(flashes["worst_in_any_second"], 0, "{from}-{to}: {flashes}");
        assert!(
            num(&stretch["camera"]["worst_reversals_in_any_second"]) <= 2.0,
            "camera reversals over {from}-{to}: {}",
            stretch["camera"]
        );
    }
    let whole = &whole_run("comfort").expect("the song is here")["flashes"];
    let widest = num(&whole["widest_luminance_range_in_any_second"]);
    assert!(widest < 0.10, "the light changes by {widest} in a second");
}
