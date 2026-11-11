//! Journey: one audio file ridden end to end, hands off. The song plays through the audio
//! device while a borderless full-screen window shows, picture after picture, the moment
//! for the song time being heard. A silent self-launch is the same journey with the sound
//! turned down to nothing, a few keys pressed for it, and an end after a few seconds.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::decode::Song;
use crate::place::{Moment, Place};
use crate::playback::Playback;
use crate::render::{FORMAT, Gpu, Renderer, Screen, Size, Still};

/// Tuning: the nudge a journey starts with, which shows the picture this far ahead of
/// the audio. A journey says the nudge in use as it ends; one the listener settles on
/// belongs here.
const NUDGE_MS: i32 = 0;
/// One press of a nudge key shifts the picture against the audio by this much.
const NUDGE_STEP_MS: i32 = 5;
/// The keys a silent self-launch presses for itself, and when, in seconds after it
/// starts to open: both nudge keys, pause, resume and quit.
const SCRIPT: [(f64, KeyCode); 6] = [
    (1.5, KeyCode::ArrowRight),
    (2.0, KeyCode::ArrowRight),
    (2.5, KeyCode::ArrowLeft),
    (5.0, KeyCode::Space),
    (6.0, KeyCode::Space),
    (10.5, KeyCode::Escape),
];
/// A silent self-launch that has not quit by itself within this long is ended outright.
pub const LIMIT: Duration = Duration::from_secs(15);
/// Song time is given this long to settle, each time it starts moving, before its
/// steadiness is measured.
const SETTLE_S: f64 = 0.5;
/// With no picture to draw, the journey still looks this often at whether it is over.
const TICK: Duration = Duration::from_millis(20);

/// What a journey leaves behind.
pub struct Ridden {
    /// The nudge in use at the end, to be made the default if the listener settled on it.
    pub nudge_ms: i32,
    /// What the journey measured of itself.
    pub measured: Value,
    /// The last picture a silent self-launch showed, and its moment: a still to set
    /// beside the one the `still` command makes of the same song time.
    pub last: Option<(Moment, Vec<u8>)>,
    pub adapter: String,
}

/// Ends the process outright once `limit` has passed, whatever the rest of it is doing.
pub fn end_within(limit: Duration) {
    std::thread::spawn(move || {
        std::thread::sleep(limit);
        let said = b"microgroove: the self-launch did not quit by itself in time and was ended\n";
        // SAFETY: `said` is valid for its length, and neither call takes a lock or runs
        // any of the program's own code, so both work while another thread hangs.
        unsafe {
            libc::write(2, said.as_ptr().cast(), said.len());
            libc::_exit(3);
        }
    });
}

/// Rides the song from `from_s`, drawing each picture `size` large, until the song ends
/// or a key quits. A silent journey plays at zero gain and presses its own keys.
pub fn ride(
    song: Song,
    place: &Place,
    size: Size,
    from_s: f64,
    silent: bool,
) -> Result<Ridden, String> {
    let opened = Instant::now();
    let event_loop = EventLoop::new().map_err(|error| {
        let error = error.to_string();
        let reason = error.rsplit(": ").next().unwrap_or(&error);
        format!("there is no display to ride on ({reason})")
    })?;
    let mut journey = Journey {
        place,
        size,
        from_s,
        silent,
        song: Some(song),
        opened,
        tick: opened,
        in_sight: false,
        stage: None,
        outcome: None,
        log: Log::default(),
    };
    event_loop
        .run_app(&mut journey)
        .map_err(|error| format!("the journey broke off ({error})"))?;
    let never = || Err("the journey never began".to_string());
    journey.outcome.unwrap_or_else(never)
}

struct Journey<'a> {
    place: &'a Place,
    size: Size,
    from_s: f64,
    silent: bool,
    /// The song, until the audio device takes it.
    song: Option<Song>,
    opened: Instant,
    /// When a picture is next drawn unasked, if the desktop has asked for none by then.
    tick: Instant,
    /// Whether the desktop has shown a picture: a journey nobody could see is a failure.
    in_sight: bool,
    stage: Option<Stage>,
    outcome: Option<Result<Ridden, String>>,
    log: Log,
}

/// Everything a journey holds open while it is ridden.
struct Stage {
    window: Arc<Window>,
    gpu: Gpu,
    screen: Screen,
    renderer: Renderer,
    picture: Still,
    playback: Playback,
    /// The moment last shown.
    last: Option<Moment>,
}

/// What is noted of each picture shown: when it was begun, in seconds after the journey
/// started to open; the song time then heard; how long it took to draw and hand over;
/// and how long the display took to have room for it, both in ms.
type Picture = (f64, f64, f64, f64);

/// What a journey notes of itself as it goes.
#[derive(Default)]
struct Log {
    pictures: Vec<Picture>,
    /// Times the desktop asked for a picture, and times the display had no room for one.
    asked_for: usize,
    not_shown: usize,
    /// The keys pressed, with what each left behind.
    pressed: Vec<Value>,
    /// How much of its script a silent self-launch has pressed.
    scripted: usize,
    /// The window as it opened, and when everything was ready to play.
    opening: Value,
    /// What the desktop told the window since: its size, its scale, whether it has the keys.
    told: Vec<Value>,
}

impl Journey<'_> {
    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<Stage, String> {
        // With no screen attached the desktop takes a window and never shows it, so
        // this is refused before a window or the audio device is opened.
        if event_loop.available_monitors().next().is_none() {
            let reason = "the desktop has no screen attached and switched on";
            return Err(format!("there is no display to ride on ({reason})"));
        }
        // Borderless: exclusive full screen does nothing under Wayland.
        let attributes = Window::default_attributes()
            .with_title("microgroove")
            .with_fullscreen(Some(Fullscreen::Borderless(None)));
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| format!("the window cannot be opened ({error})"))?;
        let window = Arc::new(window);
        window.set_cursor_visible(false);
        let display = Box::new(event_loop.owned_display_handle());
        let instance = wgpu::InstanceDescriptor::new_with_display_handle(display);
        let opened = window.inner_size();
        let (gpu, screen) = Screen::open(instance, window.clone(), opened.into())?;
        let renderer = Renderer::new(&gpu.device, FORMAT, self.size, self.place);
        let picture = Still::new(&gpu.device, self.size);
        self.log.opening = json!({
            "window_size": [opened.width, opened.height],
            "ready_s": self.opened.elapsed().as_secs_f64(),
        });
        let song = self.song.take().ok_or("the song is already playing")?;
        let gain = if self.silent { 0.0 } else { 1.0 };
        let playback = Playback::start(song, self.from_s, gain)?;
        playback.heard().clock.nudge_ms = NUDGE_MS;
        Ok(Stage {
            playback,
            window,
            gpu,
            screen,
            renderer,
            picture,
            last: None,
        })
    }

    /// What a key does, whether the listener presses it or a silent self-launch does.
    fn press(&mut self, key: KeyCode, event_loop: &ActiveEventLoop) {
        let Some(stage) = &self.stage else { return };
        let mut heard = stage.playback.heard();
        let clock = &mut heard.clock;
        match key {
            KeyCode::Escape | KeyCode::KeyQ => event_loop.exit(),
            KeyCode::Space => clock.paused = !clock.paused,
            KeyCode::ArrowLeft => clock.nudge_ms -= NUDGE_STEP_MS,
            KeyCode::ArrowRight => clock.nudge_ms += NUDGE_STEP_MS,
            _ => return,
        }
        self.log.pressed.push(json!({
            "at_s": self.opened.elapsed().as_secs_f64(),
            "key": format!("{key:?}"),
            "paused": clock.paused,
            "nudge_ms": clock.nudge_ms,
        }));
    }

    /// Draws the moment for the song time being heard, and shows it. The song time is
    /// read once the display has room for the picture, as late as it can be.
    fn show(&mut self, event_loop: &ActiveEventLoop) {
        let Some(stage) = &mut self.stage else { return };
        let asked = Instant::now();
        let room = match stage.screen.room(&stage.gpu) {
            Ok(Some(room)) => room,
            // Asked for again at the next tick.
            Ok(None) => return self.log.not_shown += 1,
            Err(failure) => {
                self.outcome = Some(Err(failure));
                return event_loop.exit();
            }
        };
        let began = Instant::now();
        let (heard_s, shown_s, _) = stage.playback.now();
        let moment = self.place.moment(shown_s);
        // Telling the window first is what has the desktop ask for the next picture
        // when it has shown this one: one picture for each refresh of the display.
        let about_to_show = || stage.window.pre_present_notify();
        let (gpu, renderer, picture) = (&stage.gpu, &stage.renderer, &stage.picture);
        stage
            .screen
            .show(gpu, renderer, picture, &moment, room, about_to_show);
        self.log.pictures.push((
            (began - self.opened).as_secs_f64(),
            heard_s,
            began.elapsed().as_secs_f64() * 1e3,
            (began - asked).as_secs_f64() * 1e3,
        ));
        stage.last = Some(moment);
        stage.window.request_redraw();
        self.tick = began + TICK;
    }

    /// Closes the journey and says what it measured.
    fn close(&self, mut stage: Stage) -> Result<Ridden, String> {
        if !self.in_sight {
            return Err("the desktop never showed the window (is the screen on?)".to_string());
        }
        let last = match stage.last.take() {
            Some(moment) if self.silent => Some((moment, stage.picture.png(&stage.gpu)?)),
            _ => None,
        };
        let (_, _, ended) = stage.playback.now();
        let nudge_ms = stage.playback.heard().clock.nudge_ms;
        let pictures = &self.log.pictures;
        let between_ms = pictures
            .windows(2)
            .map(|pair| (pair[1].0 - pair[0].0) * 1e3);
        let between_ms: Vec<f64> = between_ms.collect();
        // A picture is late when more than a refresh and a half passed since the last.
        let refresh = stage.window.current_monitor();
        let refresh = refresh.and_then(|display| display.refresh_rate_millihertz());
        let refresh_ms = refresh.map(|millihertz| 1e6 / f64::from(millihertz));
        let late = refresh_ms.map(|refresh_ms| {
            let late = |gap_ms: &&f64| **gap_ms > 1.5 * refresh_ms;
            between_ms.iter().filter(late).count()
        });
        let size = stage.window.inner_size();
        let measured = json!({
            "silent": self.silent,
            "from_s": self.from_s,
            "ended": if ended { "the song ended" } else { "a key quit" },
            "nudge_ms": nudge_ms,
            "opening": self.log.opening,
            "window": {
                "size": [size.width, size.height],
                "scale": stage.window.scale_factor(),
                "told": self.log.told,
                "surface": format!("{:?}", stage.screen.config.format),
                "presenting": format!("{:?}", stage.screen.config.present_mode),
            },
            "pictures": {
                "size": stage.picture.size,
                "adapter": stage.gpu.adapter,
                "shown": pictures.len(),
                "asked_for": self.log.asked_for,
                "not_shown": self.log.not_shown,
                "first_shown_s": pictures.first().map(|first| first.0 + first.2 / 1e3),
                "between_ms": crate::spread(between_ms),
                "refresh_ms": refresh_ms,
                "late": late,
                "drawing_ms": crate::spread(pictures.iter().map(|picture| picture.2).collect()),
                "waiting_for_room_ms":
                    crate::spread(pictures.iter().map(|picture| picture.3).collect()),
            },
            "song_time": steady_stretches(pictures),
            "audio": stage.playback.measured(),
            "pressed": self.log.pressed,
        });
        Ok(Ridden {
            nudge_ms,
            measured,
            last,
            adapter: stage.gpu.adapter.clone(),
        })
    }
}

/// Song time against the system clock, over each stretch in which every picture heard
/// more of the song than the picture before it: how many seconds of song passed in a
/// second, and how far song time stood from that steady line.
fn steady_stretches(pictures: &[Picture]) -> Vec<Value> {
    let mut stretches = Vec::new();
    let mut from = 0;
    for at in 1..=pictures.len() {
        if at < pictures.len() && pictures[at].1 > pictures[at - 1].1 {
            continue;
        }
        let stretch = &pictures[from..at];
        from = at;
        let (first, last) = (stretch[0], stretch[stretch.len() - 1]);
        if last.0 - first.0 < 2.0 * SETTLE_S {
            continue;
        }
        // The steady line is the least-squares line through the pictures after settling.
        let settling = stretch.partition_point(|picture| picture.0 < first.0 + SETTLE_S);
        let settled = &stretch[settling..];
        let mean = |part: fn(&Picture) -> f64| {
            settled.iter().map(part).sum::<f64>() / settled.len() as f64
        };
        let (mean_at, mean_song) = (mean(|picture| picture.0), mean(|picture| picture.1));
        let (mut along, mut across) = (0.0, 0.0);
        for picture in settled {
            along += (picture.0 - mean_at) * (picture.1 - mean_song);
            across += (picture.0 - mean_at).powi(2);
        }
        let rate = along / across;
        let furthest_ms = |pictures: &[Picture]| {
            let off =
                |picture: &Picture| ((picture.1 - mean_song) - rate * (picture.0 - mean_at)).abs();
            pictures.iter().map(off).fold(0.0, f64::max) * 1e3
        };
        stretches.push(json!({
            "from_s": first.0,
            "to_s": last.0,
            "song_from_s": first.1,
            "song_to_s": last.1,
            "pictures": stretch.len(),
            "song_seconds_a_second": rate,
            "furthest_from_steady_ms": furthest_ms(settled),
            "furthest_while_settling_ms": furthest_ms(&stretch[..settling]),
        }));
    }
    stretches
}

impl ApplicationHandler for Journey<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.stage.is_none() && self.outcome.is_none() {
            match self.open(event_loop) {
                Ok(stage) => self.stage = Some(stage),
                Err(failure) => {
                    self.outcome = Some(Err(failure));
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let told = match &event {
            WindowEvent::Resized(size) => Some(format!("size {}x{}", size.width, size.height)),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                Some(format!("scale {scale_factor}"))
            }
            WindowEvent::Focused(focused) => Some(format!("focus {focused}")),
            WindowEvent::Occluded(occluded) => Some(format!("out of sight {occluded}")),
            _ => None,
        };
        if let Some(told) = told {
            let at_s = self.opened.elapsed().as_secs_f64();
            self.log.told.push(json!({ "at_s": at_s, "window": told }));
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(stage) = &mut self.stage {
                    stage.screen.resize(&stage.gpu, size.into());
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => self.press(key, event_loop),
            WindowEvent::RedrawRequested => {
                // Once a picture has been handed over, the desktop asks for another only
                // when it has shown one: from then on the window is known to be in sight.
                self.in_sight |= !self.log.pictures.is_empty();
                self.log.asked_for += 1;
                self.show(event_loop);
            }
            _ => {}
        }
    }

    /// Between pictures. The end of the song is looked for here and not while drawing,
    /// because a window that is out of sight is asked for no pictures: at each tick
    /// without one, a picture is drawn unasked.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let at_s = self.opened.elapsed().as_secs_f64();
        while let Some((_, key)) = SCRIPT
            .get(self.log.scripted)
            .filter(|(due_s, _)| self.silent && *due_s <= at_s)
        {
            self.log.scripted += 1;
            self.press(*key, event_loop);
        }
        if self
            .stage
            .as_ref()
            .is_some_and(|stage| stage.playback.now().2)
        {
            event_loop.exit();
        }
        let now = Instant::now();
        if now >= self.tick {
            self.tick = now + TICK;
            self.show(event_loop);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.tick));
    }

    /// Everything is closed here, while the display it was opened on is still there.
    fn exiting(&mut self, _: &ActiveEventLoop) {
        if let Some(stage) = self.stage.take()
            && self.outcome.is_none()
        {
            self.outcome = Some(self.close(stage));
        }
    }
}
