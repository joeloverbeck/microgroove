//! Playback clock: plays the samples through the audio device and answers "what song
//! time is being heard now". The picture is locked to this clock and to nothing else.

use std::ops::Range;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde_json::{Value, json};

use crate::decode::Song;
use crate::place::DIAL_MOST;

/// The audio device playing the song, and the clock it keeps.
pub struct Playback {
    stream: cpal::Stream,
    shared: Arc<Mutex<Shared>>,
    /// Which device plays, and how it was asked to.
    device: String,
}

/// What the audio device and the journey share. The device holds it only long enough to
/// say what it wants; the journey only long enough to read the time.
pub struct Shared {
    pub clock: Clock,
    /// How often the device has asked for frames, the fewest and the most it asked for
    /// at once, and the least and the most time it gave before they would sound.
    asks: u64,
    lot: [usize; 2],
    ahead_ns: [u64; 2],
    /// When the device first asked, after it was told to play.
    first_ask: Option<Duration>,
    /// What the device said went wrong, such as a gap in the sound.
    problems: Vec<String>,
}

impl Playback {
    /// Starts playing the song from `from_s`, each sample multiplied by `gain`: 1 leaves
    /// the song exactly as it is, and 0 goes through every step without a sound.
    pub fn start(song: Song, from_s: f64, gain: f32) -> Result<Playback, String> {
        let host = cpal::default_host();
        let output = host
            .default_output_device()
            .ok_or("there is no audio device to play through")?;
        let rate = song.sample_rate;
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: rate,
            buffer_size: cpal::BufferSize::Default,
        };
        let from = ((from_s * f64::from(rate)).round() as usize).min(song.frames());
        let shared = Arc::new(Mutex::new(Shared {
            clock: Clock::new(rate, song.frames(), from),
            asks: 0,
            lot: [usize::MAX, 0],
            ahead_ns: [u64::MAX, 0],
            first_ask: None,
            problems: Vec::new(),
        }));
        let (asking, failing) = (shared.clone(), shared.clone());
        let told = Instant::now();
        let stream = output
            .build_output_stream(
                config,
                move |out: &mut [f32], info: &cpal::OutputCallbackInfo| {
                    let said = info.timestamp();
                    let due_ns = said.playback.as_nanos() as u64;
                    let ahead_ns = due_ns.saturating_sub(said.callback.as_nanos() as u64);
                    let wanted = out.len() / 2;
                    let lot = {
                        let mut shared = asking.lock().unwrap();
                        shared.asks += 1;
                        shared.lot = [shared.lot[0].min(wanted), shared.lot[1].max(wanted)];
                        shared.ahead_ns = [
                            shared.ahead_ns[0].min(ahead_ns),
                            shared.ahead_ns[1].max(ahead_ns),
                        ];
                        shared.first_ask.get_or_insert_with(|| told.elapsed());
                        shared.clock.take(wanted, due_ns)
                    };
                    out.fill(0.0);
                    for (pair, frame) in out.chunks_exact_mut(2).zip(lot) {
                        pair[0] = song.left()[frame] * gain;
                        pair[1] = song.right()[frame] * gain;
                    }
                },
                move |problem: cpal::Error| {
                    let problems = &mut failing.lock().unwrap().problems;
                    if problems.len() < 20 {
                        problems.push(format!("{:?}: {problem}", problem.kind()));
                    }
                },
                Some(Duration::from_secs(2)),
            )
            .map_err(|error| {
                format!("the audio device will not play {rate} Hz stereo as it is ({error})")
            })?;
        stream
            .play()
            .map_err(|error| format!("the audio device will not start ({error})"))?;
        Ok(Playback {
            stream,
            shared,
            device: format!(
                "{:?}, {}, {rate} Hz, 2 channels, f32",
                host.id(),
                output.id().map_or("?".to_string(), |id| id.to_string())
            ),
        })
    }

    pub fn shared(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().unwrap()
    }

    /// The song time being heard now, the song time of the moment to show now, and
    /// whether the song has ended.
    pub fn now(&self) -> (f64, f64, bool) {
        let now_ns = self.stream.now().as_nanos() as u64;
        let mut shared = self.shared();
        let clock = &mut shared.clock;
        (clock.heard_s(now_ns), clock.shown_s(now_ns), clock.ended())
    }

    /// What the device was and did, for a journey's measurements.
    pub fn measured(&self) -> Value {
        let shared = self.shared();
        let ms = |ns: u64| ns as f64 / 1e6;
        json!({
            "device": self.device,
            "asks": shared.asks,
            "frames_asked_for": shared.lot,
            "ms_given_before_sounding": shared.ahead_ns.map(ms),
            "first_ask_ms_after_play": shared.first_ask.map(|after| after.as_secs_f64() * 1e3),
            "sayings_wander_ms": shared.clock.wander_ms(),
            "line_laid_at_song_s_moving_it_ms": shared.clock.laid,
            "sayings_passed_over_at_song_s_off_by_ms": shared.clock.passed_over,
            "problems": shared.problems,
        })
    }
}

/// The mapping from a reading of the audio clock to a song time. The audio device says
/// when the frames it takes will sound; between those sayings the audio clock itself
/// carries song time forward. Readings are in nanoseconds, as the device gives them.
pub struct Clock {
    /// Shifts the picture against the audio: the moment shown is this far ahead of what is heard.
    pub nudge_ms: i32,
    /// The dial the moment shown is seen with: steps up, where the ground passes faster,
    /// or down. An aid like the nudge; it shifts nothing in time.
    pub dial: i32,
    /// While set, the device is handed no frames, so sound and picture stop together.
    pub paused: bool,
    /// Each time the steady line through the device's sayings was laid afresh: the song
    /// time it was laid at, and how far that moved it, in ms. Once where sound begins,
    /// again where it begins again after a pause, and wherever the sound breaks off.
    pub laid: Vec<[f64; 2]>,
    /// Each saying that stood beyond the break by itself and was passed over: the song
    /// time it was for, and how far from the line it stood, in ms.
    pub passed_over: Vec<[f64; 2]>,
    rate: f64,
    frames: usize,
    /// Frames of the song handed to the device so far.
    handed: usize,
    /// The furthest frame yet reported heard.
    heard: f64,
    /// The steady line: the reading at which the song's first frame would have sounded,
    /// had the song played without a break from there to here.
    began_ns: Option<f64>,
    /// How far the device's sayings stand from the steady line, as a running average.
    wander_ns: f64,
    /// No frames were handed over at the last asking: sound begins with the next that are.
    idle: bool,
    /// Where the last saying would put the line, if it stood beyond the break.
    stray_ns: Option<f64>,
}

/// Tuning. Each saying moves the steady line this share of the way toward itself, so
/// that the small unevenness of a device's sayings does not reach the picture. A saying
/// further than `BREAK_NS` from the line is not unevenness. By itself it is a stray and
/// moves nothing; when the next saying agrees with it, the sound has broken off, and the
/// line is laid afresh through that one.
const FOLLOW: f64 = 1.0 / 32.0;
const BREAK_NS: f64 = 3e6;

impl Clock {
    /// A clock for a song of `frames` frames at `rate` a second, to be heard from frame `from`.
    pub fn new(rate: u32, frames: usize, from: usize) -> Clock {
        Clock {
            nudge_ms: 0,
            dial: 0,
            paused: false,
            laid: Vec::new(),
            passed_over: Vec::new(),
            rate: f64::from(rate),
            frames,
            handed: from,
            heard: from as f64,
            began_ns: None,
            wander_ns: 0.0,
            idle: true,
            stray_ns: None,
        }
    }

    /// The device asks for `wanted` frames and will sound the first of them at `due_ns`.
    /// Returns the frames of the song to hand it: fewer, or none, when paused or at the end.
    pub fn take(&mut self, wanted: usize, due_ns: u64) -> Range<usize> {
        let lot = if self.paused {
            0
        } else {
            wanted.min(self.frames - self.handed)
        };
        if lot > 0 {
            let said_ns = due_ns as f64 - self.handed as f64 * 1e9 / self.rate;
            let off_ns = said_ns - self.began_ns.unwrap_or(said_ns);
            let noted = [self.handed as f64 / self.rate, off_ns / 1e6];
            let far = off_ns.abs() > BREAK_NS;
            let agreed = |stray_ns: f64| (said_ns - stray_ns).abs() <= BREAK_NS;
            let lays = self.idle || far && self.stray_ns.is_some_and(agreed);
            self.stray_ns = (far && !lays).then_some(said_ns);
            if lays {
                self.began_ns = Some(said_ns);
                self.laid.push(noted);
            } else if far {
                self.passed_over.push(noted);
            } else {
                self.began_ns = Some(said_ns - off_ns * (1.0 - FOLLOW));
                self.wander_ns += (off_ns.abs() - self.wander_ns) * FOLLOW;
            }
            self.handed += lot;
        }
        self.idle = lot == 0;
        self.handed - lot..self.handed
    }

    /// The song time being heard at a reading of the audio clock. It never runs backwards,
    /// and never passes what the device has been handed.
    pub fn heard_s(&mut self, now_ns: u64) -> f64 {
        if let Some(began_ns) = self.began_ns {
            let reached = (now_ns as f64 - began_ns) * self.rate / 1e9;
            self.heard = self.heard.max(reached.min(self.handed as f64));
        }
        self.heard / self.rate
    }

    /// The song time of the moment to show: what is being heard, shifted by the nudge.
    pub fn shown_s(&mut self, now_ns: u64) -> f64 {
        self.heard_s(now_ns) + f64::from(self.nudge_ms) / 1e3
    }

    /// Turns the dial by a press of one of its keys. It stops at its ends.
    pub fn turn(&mut self, press: i32) {
        self.dial = (self.dial + press).clamp(-DIAL_MOST, DIAL_MOST);
    }

    /// Whether the last frame of the song has been heard.
    pub fn ended(&self) -> bool {
        self.heard >= self.frames as f64
    }

    /// How far the device's sayings stand from the steady line through them, on average.
    pub fn wander_ms(&self) -> f64 {
        self.wander_ns / 1e6
    }
}

/// The inner seam issue #1 accepts beside the command line: a real audio device cannot
/// run inside a test, so the mapping is driven by a fake one whose answers are known by
/// construction. A second inner seam exists in `src/comfort.rs`, kept pending the owner's word.
#[cfg(test)]
mod tests {
    use super::*;

    /// A device on a clock that reads `START_NS` when it first asks for frames. It asks
    /// for 960 frames every 20 ms and sounds each lot 40 ms after asking for it.
    const START_NS: u64 = 5_000_000_000_000;
    const ASKS_EVERY_NS: u64 = 20_000_000;
    const SOUNDS_AFTER_NS: u64 = 40_000_000;

    struct Device {
        clock: Clock,
        now_ns: u64,
        asked: u64,
        /// The frames it has been handed, lot by lot.
        handed: Vec<Range<usize>>,
        /// How far each of its sayings is off, in turn, in nanoseconds.
        off_ns: Vec<i64>,
    }

    impl Device {
        fn playing(clock: Clock) -> Device {
            Device {
                clock,
                now_ns: START_NS,
                asked: 0,
                handed: Vec::new(),
                off_ns: vec![0],
            }
        }

        /// Lets the audio clock run on, the device asking for frames as it goes, and
        /// returns the song time then heard and the song time then shown.
        fn after(&mut self, seconds: f64) -> (f64, f64) {
            self.now_ns += (seconds * 1e9).round() as u64;
            while START_NS + self.asked * ASKS_EVERY_NS <= self.now_ns {
                let asks_ns = START_NS + self.asked * ASKS_EVERY_NS;
                let off_ns = self.off_ns[self.asked as usize % self.off_ns.len()];
                let due_ns = (asks_ns + SOUNDS_AFTER_NS).saturating_add_signed(off_ns);
                let lot = self.clock.take(960, due_ns);
                if !lot.is_empty() {
                    self.handed.push(lot);
                }
                self.asked += 1;
            }
            (
                self.clock.heard_s(self.now_ns),
                self.clock.shown_s(self.now_ns),
            )
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn song_time_follows_the_audio_clock_at_one_second_a_second_from_when_sound_begins() {
        let mut device = Device::playing(Clock::new(48_000, 480_000, 0));
        assert_eq!(device.clock.heard_s(START_NS), 0.0, "nothing is heard yet");
        // The first frame sounds 40 ms after the device first asked.
        assert!(
            close(device.after(0.03).0, 0.0),
            "handed over, not yet heard"
        );
        assert!(close(device.after(0.97).0, 0.96));
        assert!(close(device.after(0.5).0, 1.46));
        assert!(close(device.after(0.007).0, 1.467), "between two askings");
        assert!(close(device.after(2.0).0, 3.467));
    }

    #[test]
    fn pausing_stops_song_time_where_the_sound_stops_and_resuming_carries_on_from_there() {
        let mut device = Device::playing(Clock::new(48_000, 480_000, 0));
        assert!(close(device.after(1.0).0, 0.96));
        device.clock.paused = true;
        // The device already holds 51 lots, 1.02 s of song: that much still sounds.
        assert!(close(device.after(0.05).0, 1.01), "still sounding");
        assert!(close(device.after(0.05).0, 1.02));
        assert!(close(device.after(3.9).0, 1.02), "song time stands still");
        device.clock.paused = false;
        // It asks again at 5.02 s and sounds what it gets at 5.06 s.
        assert!(close(device.after(0.03).0, 1.02), "never backwards");
        assert!(close(device.after(0.03).0, 1.02), "sound begins again");
        assert!(
            close(device.after(1.0).0, 2.02),
            "one second a second again"
        );
        let handed: Vec<usize> = device.handed.iter().map(|lot| lot.start).collect();
        assert_eq!(handed[50..53], [48_000, 48_960, 49_920], "no frame skipped");
        // Laid at the start and at the resume, which came four seconds late for the old line.
        assert_eq!(device.clock.laid, [[0.0, 0.0], [1.02, 4000.0]]);
    }

    #[test]
    fn a_nudge_shifts_the_moment_shown_against_what_is_heard() {
        let mut device = Device::playing(Clock::new(48_000, 480_000, 0));
        device.clock.nudge_ms = 15;
        let (heard, shown) = device.after(1.0);
        assert!(close(heard, 0.96) && close(shown, 0.975), "{heard} {shown}");
        device.clock.nudge_ms = -20;
        let (heard, shown) = device.after(1.0);
        assert!(close(heard, 1.96) && close(shown, 1.94), "{heard} {shown}");
    }

    #[test]
    fn the_dial_turns_a_step_at_a_press_as_far_as_eight_either_way_and_shifts_no_song_time() {
        let mut device = Device::playing(Clock::new(48_000, 480_000, 0));
        let undialled = device.after(1.0);
        for press in [1, 1, -1] {
            device.clock.turn(press);
        }
        assert_eq!(device.clock.dial, 1, "two presses up and one down");
        assert_eq!(device.after(0.0), undialled, "what is heard and shown");
        for (press, end) in [(1, 8), (-1, -8)] {
            (0..20).for_each(|_| device.clock.turn(press));
            assert_eq!(device.clock.dial, end, "the dial stops at its end");
        }
    }

    #[test]
    fn every_frame_is_handed_over_once_in_order_and_the_song_ends_when_the_last_is_heard() {
        // Two seconds of song, heard from half a second in. The last lot is a short one.
        let mut device = Device::playing(Clock::new(48_000, 96_100, 24_000));
        assert!(
            close(device.after(0.54).0, 1.0),
            "half a second in, plus 0.5"
        );
        assert!(!device.clock.ended());
        assert!(close(device.after(1.0).0, 2.0));
        assert!(!device.clock.ended(), "a hundred frames are still to sound");
        assert!(close(device.after(1.0).0, 96_100.0 / 48_000.0));
        assert!(device.clock.ended());
        let (first, last) = (&device.handed[0], device.handed.last().unwrap());
        assert_eq!((first.start, last.clone()), (24_000, 96_000..96_100));
        let in_order = |pair: &[Range<usize>]| pair[0].end == pair[1].start;
        assert!(device.handed.windows(2).all(in_order));
    }

    #[test]
    fn a_wandering_device_is_followed_steadily_and_one_that_falls_behind_once_it_says_so_twice() {
        let mut device = Device::playing(Clock::new(48_000, 4_800_000, 0));
        // Each saying is half a millisecond early or late.
        device.off_ns = vec![500_000, -500_000, -500_000, 500_000, 500_000, -500_000];
        device.after(4.0);
        let mut heard_s = 3.96;
        for _ in 0..300 {
            heard_s += 0.007;
            let heard = device.after(0.007).0;
            assert!((heard - heard_s).abs() < 0.000_05, "{heard} at {heard_s}");
        }
        let wander = device.clock.wander_ms();
        assert!((0.4..0.6).contains(&wander), "wander of {wander} ms");
        assert_eq!(device.clock.laid.len(), 1, "one steady line all the while");

        // The device stalls: from here on everything sounds 30 ms later than it would have.
        // Its first late saying could be a stray; its second agrees with the first.
        device.off_ns = vec![30_000_000];
        let before = device.after(0.0).0;
        let stalled = device.after(0.04).0;
        assert!(stalled >= before && stalled < before + 0.04, "{stalled}");
        heard_s += 0.5 - 0.03;
        assert!((device.after(0.46).0 - heard_s).abs() < 0.000_05);
        let (laid, passed_over) = (device.clock.laid.len(), device.clock.passed_over.len());
        assert_eq!((laid, passed_over), (2, 1), "a new line, one saying late");
    }

    #[test]
    fn one_saying_far_from_the_line_neither_lays_it_afresh_nor_moves_song_time() {
        for stray_ns in [5_000_000, -5_000_000] {
            let mut device = Device::playing(Clock::new(48_000, 4_800_000, 0));
            assert!(close(device.after(1.0).0, 0.96));
            // One saying is 5 ms off, late or early. Those before and after it are true.
            device.off_ns = vec![stray_ns];
            assert!(close(device.after(0.02).0, 0.98), "at the stray saying");
            device.off_ns = vec![0];
            let mut heard_s = 0.98;
            for _ in 0..100 {
                heard_s += 0.007;
                let heard = device.after(0.007).0;
                assert!(close(heard, heard_s), "{heard} at {heard_s}");
            }
            assert_eq!(device.clock.laid.len(), 1, "one steady line all the while");
            assert_eq!(device.clock.passed_over, [[1.02, stray_ns as f64 / 1e6]]);
        }
    }
}
