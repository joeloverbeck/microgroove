//! Riding, as far as it can be checked with no display and no audio device. A test never
//! opens a window or plays a sound: the journey itself is exercised by the silent
//! self-launch, which a person or an agent starts on purpose.
mod common;

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use common::{complaint, rising_and_falling_song, scratch};

/// The three ways to start: a journey by path alone, a journey by name, a silent self-launch.
const STARTS: [&[&str]; 3] = [&[], &["ride"], &["self-launch"]];

/// The program, about to start where it can reach neither a display nor the desktop's audio.
fn program(command: &[&str], song: &Path, nowhere: &Path) -> Command {
    let mut program = Command::new(env!("CARGO_BIN_EXE_microgroove"));
    program
        .args(command)
        .arg(song)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env("XDG_RUNTIME_DIR", nowhere);
    program
}

fn start(command: &[&str], song: &Path, nowhere: &Path) -> Output {
    program(command, song, nowhere).output().unwrap()
}

#[test]
fn a_path_alone_starts_a_journey_and_a_file_that_cannot_be_read_is_refused() {
    let dir = scratch("journey-refusal");
    for command in STARTS {
        let output = start(command, &dir.join("no-such-song.flac"), &dir);
        let message = complaint(&output);
        assert!(
            message.contains("refused") && message.contains("does not exist"),
            "{command:?}: {message:?}"
        );
    }
}

#[test]
fn with_no_display_to_ride_on_a_journey_says_so_and_does_not_start() {
    let dir = scratch("journey-no-display");
    let song = dir.join("song.wav");
    rising_and_falling_song(&song);
    let nowhere = dir.join("nowhere");
    std::fs::create_dir(&nowhere).unwrap();
    for command in STARTS {
        let output = start(command, &song, &nowhere);
        let message = complaint(&output);
        assert!(message.contains("no display"), "{command:?}: {message:?}");
        assert!(output.stdout.is_empty(), "{command:?} reported a journey");
    }
}

/// A silent self-launch must never keep the screen, so one that hangs is ended at its
/// limit of 15 seconds. This one hangs before it opens anything, on a file that never
/// yields a byte.
#[test]
fn a_silent_self_launch_that_hangs_is_ended_at_its_limit() {
    let dir = scratch("journey-hang");
    let never = dir.join("never.flac");
    let made = Command::new("mkfifo").arg(&never).status().unwrap();
    assert!(made.success(), "a file that never yields a byte");

    let began = Instant::now();
    let mut hung = program(&["self-launch"], &never, &dir)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    while hung.try_wait().unwrap().is_none() && began.elapsed() < Duration::from_secs(25) {
        std::thread::sleep(Duration::from_millis(50));
    }
    let lasted = began.elapsed().as_secs_f64();
    let _ = hung.kill();
    let output = hung.wait_with_output().unwrap();
    let message = String::from_utf8_lossy(&output.stderr);
    assert!((14.9..16.0).contains(&lasted), "ended after {lasted} s");
    assert_eq!(output.status.code(), Some(3), "{message:?}");
    assert!(message.contains("did not quit by itself"), "{message:?}");
}
