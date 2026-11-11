//! Shared helpers: every test drives the built program through its command line.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub use serde_json::Value;

/// A fresh, empty, git-ignored directory for one test.
pub fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn run_timeline(song: &Path, out: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_microgroove"))
        .arg("timeline")
        .arg(song)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

/// Runs `still` or `run` with no display to draw on, and returns the files it says it wrote.
pub fn inspect(command: &str, song: &Path, out: &Path, options: &[&str]) -> Vec<PathBuf> {
    let output = Command::new(env!("CARGO_BIN_EXE_microgroove"))
        .arg(command)
        .arg(song)
        .arg("--out")
        .arg(out)
        .args(options)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{command} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let said = String::from_utf8(output.stdout).unwrap();
    let files = said
        .lines()
        .filter(|line| line.ends_with(".png") || line.ends_with(".json"));
    files.map(PathBuf::from).collect()
}

/// Makes a still of each song time in one command. Returns each still and its numbers,
/// in the order asked. Small stills are enough where only the numbers are judged.
pub fn stills(song: &Path, out: &Path, times: &[&str], small: bool) -> Vec<(Vec<u8>, Value)> {
    let mut options: Vec<&str> = times.iter().flat_map(|time| ["--at", time]).collect();
    if small {
        options.extend(["--size", "320x180"]);
    }
    let written = inspect("still", song, out, &options);
    assert_eq!(written.len(), 2 * times.len(), "{written:?}");
    let still = |pair: &[PathBuf]| (std::fs::read(&pair[0]).unwrap(), json(&pair[1]));
    written.chunks(2).map(still).collect()
}

/// The numbers of a small still at each song time, in the order asked.
pub fn moments(song: &Path, out: &Path, times: &[&str]) -> Vec<Value> {
    let stills = stills(song, out, times, true);
    stills.into_iter().map(|(_, numbers)| numbers).collect()
}

/// Checks the turn a run makes at a song time: the lean peaks within 0.25 s of the view's
/// fastest turning, levels out at most 1.3 times as fast as it leaned in, and reaches
/// between 1 and 18 degrees.
pub fn assert_the_camera_leans_as_its_view_turns(run: &Value, around_s: f64) {
    let (after_s, out_over_in, most_deg) = turn(run, around_s);
    assert!(
        after_s.abs() <= 0.25 && out_over_in <= 1.3 && (1.0..=18.0).contains(&most_deg),
        "the turn at {around_s} s: the lean peaks {after_s} s after the view turns fastest, \
         levels out {out_over_in} times as fast as it leaned in, and reaches {most_deg} degrees"
    );
}

pub fn json(file: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap()
}

/// Draws a run of consecutive moments and returns its numbers.
pub fn run(song: &Path, out: &Path, options: &[&str]) -> Value {
    let written = inspect("run", song, out, options);
    let [numbers] = written.as_slice() else {
        panic!("the command names the numbers of the run: {written:?}");
    };
    json(numbers)
}

/// One value for each moment of a run.
pub fn series(run: &Value, name: &str) -> Vec<f64> {
    assert!(run["series"][name].is_array(), "no series {name}");
    curve(&run["series"][name])
}

/// The turn a run makes within three seconds of a song time: how long after the view's
/// fastest turning the lean peaks, the fastest levelling-out over the fastest leaning-in,
/// and the greatest lean in degrees.
fn turn(run: &Value, around_s: f64) -> (f64, f64, f64) {
    let per_second = num(&run["moments_per_second"]);
    let (time, yaw) = (series(run, "song_time_s"), series(run, "yaw_deg"));
    let lean: Vec<f64> = series(run, "bank_deg").iter().map(|l| l.abs()).collect();
    let within = (1..time.len() - 1).filter(|at| (time[*at] - around_s).abs() <= 3.0);
    let within: Vec<usize> = within.collect();
    let change = |of: &[f64], at: usize| (of[at + 1] - of[at - 1]) * per_second / 2.0;
    let peak = |of: &dyn Fn(usize) -> f64| {
        let greater = |a: &usize, b: &usize| of(*a).total_cmp(&of(*b));
        within.iter().copied().max_by(greater).unwrap()
    };
    let (turning, leaning) = (peak(&|at| change(&yaw, at).abs()), peak(&|at| lean[at]));
    let fastest = |way: f64| {
        let changes = within.iter().map(|at| way * change(&lean, *at));
        changes.fold(0.0, f64::max)
    };
    let after_s = time[leaning] - time[turning];
    (after_s, fastest(-1.0) / fastest(1.0), lean[leaning])
}

/// Quiet noise, then loud noise with clicks, then quiet noise again: 24 s.
pub fn rising_and_falling_song(path: &Path) {
    let mut signal = noise(8.0, 0.02);
    let mut loud = noise(8.0, 0.3);
    add_clicks(&mut loud, &every(0.25, 0.5, 7.9), 0.6);
    signal.extend(loud);
    signal.extend(noise(8.0, 0.02));
    write_wav(path, &signal, &signal);
}

/// Runs `timeline` and returns the analysis numbers as raw bytes.
pub fn numbers_bytes(song: &Path, out: &Path) -> Vec<u8> {
    let output = run_timeline(song, out);
    assert!(
        output.status.success(),
        "timeline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read(out.join("analysis.json")).unwrap()
}

pub fn numbers(song: &Path, out: &Path) -> Value {
    serde_json::from_slice(&numbers_bytes(song, out)).unwrap()
}

/// Runs ffmpeg on a file and returns what it printed and what it reported, or None
/// where ffmpeg is not installed.
pub fn ffmpeg(args: &[&str], input: &Path, tail: &[&str]) -> Option<(String, String)> {
    let output = Command::new("ffmpeg")
        .args(args)
        .arg("-i")
        .arg(input)
        .args(tail)
        .output()
        .ok()?;
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some((
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

/// ffmpeg's checksum of the audio decoded to 24-bit PCM, or None where ffmpeg is not
/// installed: a file's identity, by a decoder that is not the program's.
pub fn ffmpeg_md5(song: &Path) -> Option<String> {
    let (stdout, _) = ffmpeg(
        &["-v", "error"],
        song,
        &["-map", "0:a", "-c:a", "pcm_s24le", "-f", "md5", "-"],
    )?;
    Some(stdout.trim().trim_start_matches("MD5=").to_string())
}

/// Says plainly that a check did not run. Written straight to stderr so the
/// test harness does not swallow it: a skipped check must never look passed.
pub fn skip(check: &str, reason: &str) {
    let _ = writeln!(std::io::stderr(), "SKIPPED {check}: {reason}");
}

pub const RATE: u32 = 48_000;

/// Writes interleaved stereo samples in [-1, 1] as a 16-bit PCM WAV file.
pub fn write_wav(path: &Path, left: &[f32], right: &[f32]) {
    write_wav_at(path, RATE, left, right);
}

/// The same, for samples at any number a second.
pub fn write_wav_at(path: &Path, rate: u32, left: &[f32], right: &[f32]) {
    assert_eq!(left.len(), right.len());
    let data_len = (left.len() * 4) as u32;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&2u16.to_le_bytes()); // channels
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 4).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for (l, r) in left.iter().zip(right) {
        for s in [l, r] {
            let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            bytes.extend_from_slice(&v.to_le_bytes());
        }
    }
    std::fs::write(path, bytes).unwrap();
}

pub fn silence(seconds: f64) -> Vec<f32> {
    vec![0.0; (seconds * RATE as f64).round() as usize]
}

pub fn tone(seconds: f64, hz: f64, amplitude: f32) -> Vec<f32> {
    (0..(seconds * RATE as f64).round() as usize)
        .map(|n| amplitude * (std::f64::consts::TAU * hz * n as f64 / RATE as f64).sin() as f32)
        .collect()
}

/// Deterministic white noise from a fixed linear congruential generator.
pub fn noise(seconds: f64, amplitude: f32) -> Vec<f32> {
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    (0..(seconds * RATE as f64).round() as usize)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            amplitude * (((state >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0)
        })
        .collect()
}

/// Adds a short click (a 4 ms burst of decaying noise) starting at each given time.
pub fn add_clicks(signal: &mut [f32], times: &[f64], amplitude: f32) {
    let burst = noise(0.004, 1.0);
    for &t in times {
        let start = (t * RATE as f64).round() as usize;
        for (i, b) in burst.iter().enumerate() {
            if let Some(s) = signal.get_mut(start + i) {
                *s += amplitude * b * (1.0 - i as f32 / burst.len() as f32);
            }
        }
    }
}

/// Times `first`, `first + period`, ... strictly before `end`.
pub fn every(first: f64, period: f64, end: f64) -> Vec<f64> {
    (0..)
        .map(|k| first + k as f64 * period)
        .take_while(|t| *t < end)
        .collect()
}

pub fn num(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("expected a number, got {value}"))
}

/// A list of numbers, such as the values of a curve.
pub fn curve(list: &Value) -> Vec<f64> {
    list.as_array().unwrap().iter().map(num).collect()
}

pub fn times(list: &Value) -> Vec<f64> {
    let times = list.as_array().unwrap().iter();
    times.map(|item| num(&item["time_s"])).collect()
}

/// The one plain line a command that cannot go on says, having failed.
pub fn complaint(output: &Output) -> String {
    let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
    assert!(!output.status.success(), "expected a non-zero exit");
    assert_eq!(message.lines().count(), 1, "one plain line: {message:?}");
    message
}
