//! Command line: a journey through one audio file, which is what a path alone asks for;
//! the silent self-launch that tries a journey out; and `timeline`, `still` and `run`,
//! which inspect the same file with no display.

mod analysis;
mod comfort;
mod decode;
mod journey;
mod place;
mod playback;
mod render;
mod timeline;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;

use place::Place;
use render::{Gpu, Renderer, Size, Still};

const USAGE: &str = "usage: microgroove [ride] <audio file> [--size <width>x<height>] [--out <directory>]
       microgroove self-launch <audio file> [--from <song time>] [--size <width>x<height>] [--out <directory>]
       microgroove timeline <audio file> [--out <directory>]
       microgroove still <audio file> --at <song time> [--at <song time> ...] [--dial <steps>] [--size <width>x<height>] [--out <directory>]
       microgroove run <audio file> [--from <song time>] [--to <song time>] [--rate <moments a second>] [--dial <steps>] [--size <width>x<height>] [--out <directory>]
a song time is seconds (95.5) or minutes and seconds (1:35.5); the dial takes steps from -8 to 8";

/// Where output goes unless `--out` says otherwise: under the current directory, and
/// git-ignored anywhere in this repository.
const CAPTURES: &str = "captures";
const STILLS: &str = "stills";
/// The display a journey is ridden on: a picture is this large, and a run draws this many
/// moments a second, unless asked otherwise.
const DISPLAY: Size = (3840, 2160);
const DISPLAY_HZ: usize = 144;
/// A part of the camera's pose has turned back on itself when it reverses by this much:
/// a hundredth of a metre, or two hundredths of a degree (under a pixel of the display).
const LEAST_REVERSAL: (f32, f32) = (0.01, 0.02);

type Command = fn(&Request) -> Result<(), String>;

/// Each command and the options it takes. A journey comes first: it is what a path alone asks for.
const COMMANDS: [(&str, Command, &[&str]); 5] = [
    ("ride", ride, &["--size", "--out"]),
    ("self-launch", self_launch, &["--size", "--from", "--out"]),
    ("timeline", timeline, &["--out"]),
    ("still", still, &["--out", "--size", "--at", "--dial"]),
    (
        "run",
        run,
        &["--out", "--size", "--from", "--to", "--rate", "--dial"],
    ),
];

/// What the command line asks for.
#[derive(Default)]
struct Request {
    song: PathBuf,
    out: Option<PathBuf>,
    at: Vec<f64>,
    from: Option<f64>,
    to: Option<f64>,
    rate: Option<usize>,
    size: Option<Size>,
    /// The dial a journey's keys turn, for a still or a run to be seen as a journey was.
    dial: Option<i32>,
}

/// The analysis numbers as written beside the timeline.
#[derive(Serialize)]
struct Numbers<'a> {
    sample_rate: u32,
    channels: usize,
    frames: usize,
    duration_s: f64,
    decoded_audio_md5: String,
    #[serde(flatten)]
    analysis: &'a analysis::Analysis,
}

/// The numbers behind a still: its moment, and what it was drawn from and on.
#[derive(Serialize)]
struct StillNumbers<'a> {
    #[serde(flatten)]
    moment: &'a place::Moment,
    place: place::Summary,
    size: Size,
    adapter: &'a str,
}

/// One value for each moment of a run.
#[derive(Serialize, Default)]
struct Series {
    song_time_s: Vec<f64>,
    route_position_m: Vec<f64>,
    across_m: Vec<f32>,
    up_m: Vec<f32>,
    yaw_deg: Vec<f32>,
    pitch_deg: Vec<f32>,
    bank_deg: Vec<f32>,
    light: Vec<f32>,
    speed_m_s: Vec<f64>,
    over_ground_per_s: Vec<f64>,
    past_land_per_s: Vec<f64>,
    /// Impulses in play.
    impulses: Vec<usize>,
    /// Mean relative luminance of the whole picture.
    luminance: Vec<f32>,
    /// How long the moment took to draw.
    draw_ms: Vec<f64>,
}

fn main() -> ExitCode {
    let mut args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let named = |name: &OsString| COMMANDS.iter().position(|command| name == command.0);
    let command = args.first().and_then(named);
    let (_, act, options) = COMMANDS[command.map_or(0, |command| {
        args.remove(0);
        command
    })];
    let Some(request) = parse(args, options) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match act(&request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("microgroove: {message}");
            ExitCode::FAILURE
        }
    }
}

/// One audio file and the options a command takes, in any order; None for anything else.
fn parse(args: Vec<OsString>, options: &[&str]) -> Option<Request> {
    let mut request = Request::default();
    let mut song = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let Some(option) = arg.to_str().filter(|arg| arg.starts_with("--")) else {
            if song.replace(PathBuf::from(arg)).is_some() {
                return None;
            }
            continue;
        };
        let value = args.next().filter(|_| options.contains(&option))?;
        let text = value.to_str();
        let once = match option {
            "--out" => request.out.replace(PathBuf::from(&value)).is_none(),
            "--at" => {
                request.at.push(song_time(text?)?);
                true
            }
            "--from" => request.from.replace(song_time(text?)?).is_none(),
            "--to" => request.to.replace(song_time(text?)?).is_none(),
            "--rate" => request
                .rate
                .replace(text?.parse().ok().filter(|rate| *rate > 0)?)
                .is_none(),
            "--dial" => {
                let steps = text?.parse().ok();
                let within = |steps: &i32| (-place::DIAL_MOST..=place::DIAL_MOST).contains(steps);
                request.dial.replace(steps.filter(within)?).is_none()
            }
            _ => request.size.replace(size(text?)?).is_none(),
        };
        if !once {
            return None;
        }
    }
    request.song = song?;
    // A command that takes moments must be given at least one.
    let wants_moments = options.contains(&"--at");
    (request.at.is_empty() != wants_moments).then_some(request)
}

/// Seconds (`95.5`) or minutes and seconds (`1:35.5`).
fn song_time(text: &str) -> Option<f64> {
    let (minutes, seconds) = text.split_once(':').unwrap_or(("0", text));
    let seconds = seconds.parse::<f64>().ok()?;
    let time = f64::from(minutes.parse::<u32>().ok()?) * 60.0 + seconds;
    (time.is_finite() && seconds >= 0.0).then_some(time)
}

/// `<width>x<height>` in pixels.
fn size(text: &str) -> Option<Size> {
    let (width, height) = text.split_once('x')?;
    let size = (width.parse().ok()?, height.parse().ok()?);
    ((1..=8192).contains(&size.0) && (1..=8192).contains(&size.1)).then_some(size)
}

/// Decodes and analyses the whole file.
fn hear(path: &Path) -> Result<(decode::Song, analysis::Analysis), String> {
    let song =
        decode::decode(path).map_err(|refusal| format!("refused {}: {refusal}", path.display()))?;
    let analysis = analysis::analyse(&song);
    Ok((song, analysis))
}

/// Writes one file into the output directory and says where it went.
fn write(out: &Path, name: &str, bytes: Vec<u8>) -> Result<(), String> {
    std::fs::create_dir_all(out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    let file = out.join(name);
    std::fs::write(&file, bytes)
        .map_err(|error| format!("cannot write {}: {error}", file.display()))?;
    println!("{}", file.display());
    Ok(())
}

fn json(numbers: &impl Serialize, pretty: bool) -> Result<Vec<u8>, String> {
    let json = if pretty {
        serde_json::to_vec_pretty(numbers)
    } else {
        serde_json::to_vec(numbers)
    };
    let mut json = json.map_err(|error| error.to_string())?;
    json.push(b'\n');
    Ok(json)
}

/// Writes the timeline and the analysis numbers.
fn timeline(request: &Request) -> Result<(), String> {
    let (song, analysis) = hear(&request.song)?;
    let out = request.out.clone().unwrap_or(PathBuf::from(CAPTURES));
    let numbers = Numbers {
        sample_rate: song.sample_rate,
        channels: song.channels.len(),
        frames: song.frames(),
        duration_s: analysis.duration_s,
        decoded_audio_md5: song.pcm24_md5(),
        analysis: &analysis,
    };
    write(&out, "timeline.png", timeline::draw(&analysis))?;
    write(&out, "analysis.json", json(&numbers, true)?)
}

/// The least, the median, the 95th and 99th in a hundred, and the greatest of some values.
fn spread(mut values: Vec<f64>) -> serde_json::Value {
    values.sort_by(f64::total_cmp);
    let share = |of: usize| analysis::percentile(&values, of);
    serde_json::json!({
        "least": share(0),
        "median": share(50),
        "p95": share(95),
        "p99": share(99),
        "max": share(100),
    })
}

/// The song in a file and the place made from it.
fn place_of(request: &Request) -> Result<(decode::Song, Place), String> {
    let (song, analysis) = hear(&request.song)?;
    let end_s = analysis.duration_s;
    let late = [request.from, request.to]
        .into_iter()
        .flatten()
        .chain(request.at.iter().copied());
    if let Some(late) = late.into_iter().find(|time| *time > end_s) {
        return Err(format!(
            "no moment at {late} s: the song ends at {end_s:.2} s"
        ));
    }
    let place = Place::of(&analysis);
    if let Some(holds_s) = place.outgrows(render::MOST_BUFFER_BYTES) {
        let (song, minutes) = (request.song.display(), holds_s / 60.0);
        return Err(format!(
            "refused {song}: the place cannot hold a song this long yet, only {minutes:.0} minutes of it"
        ));
    }
    Ok((song, place))
}

/// Rides the song: full screen, the song playing, until it ends or a key quits.
fn ride(request: &Request) -> Result<(), String> {
    journey(request, false)
}

/// Tries a journey out for a few seconds without a sound, and reports what it measured.
fn self_launch(request: &Request) -> Result<(), String> {
    journey::end_within(journey::LIMIT);
    journey(request, true)
}

fn journey(request: &Request, silent: bool) -> Result<(), String> {
    let (song, place) = place_of(request)?;
    let size = request.size.unwrap_or(DISPLAY);
    let from_s = request.from.unwrap_or(0.0);
    let ridden = journey::ride(song, &place, size, from_s, silent)?;
    // Said before anything that can fail.
    let (nudge_ms, dial) = (ridden.nudge_ms, ridden.dial);
    println!("nudge at exit: {nudge_ms:+} ms; dial at exit: {dial:+}");
    let name = if silent { "self-launch" } else { "journey" };
    let record = || -> Result<(), String> {
        let out = request.out.clone().unwrap_or(PathBuf::from(CAPTURES));
        let measured = json(&ridden.measured, true)?;
        if silent {
            print!("{}", String::from_utf8_lossy(&measured));
        }
        write(&out, &format!("{name}.json"), measured)?;
        if let Some((moment, picture)) = ridden.last {
            let numbers = StillNumbers {
                moment: &moment,
                place: place.summary(),
                size,
                adapter: &ridden.adapter,
            };
            write(&out, &format!("{name}-last.png"), picture?)?;
            write(&out, &format!("{name}-last.json"), json(&numbers, true)?)?;
        }
        Ok(())
    };
    // A journey that was ridden does not fail because its record could not be written.
    if let Err(warning) = record() {
        eprintln!("microgroove: warning: {warning}");
    }
    ridden.failure.map_or(Ok(()), Err)
}

/// The place made from a file, on a GPU with no display, with a picture to draw it into.
fn prepare(request: &Request) -> Result<(Place, Gpu, Renderer, Still), String> {
    let (_, place) = place_of(request)?;
    let gpu = Gpu::headless()?;
    let size = request.size.unwrap_or(DISPLAY);
    let renderer = Renderer::new(&gpu.device, size, &place);
    let picture = Still::new(&gpu.device, size);
    Ok((place, gpu, renderer, picture))
}

/// Writes a still of each moment asked for, with its numbers.
fn still(request: &Request) -> Result<(), String> {
    let (place, gpu, renderer, picture) = prepare(request)?;
    let out = request.out.clone().unwrap_or(PathBuf::from(STILLS));
    for at in &request.at {
        let moment = place.moment(*at, request.dial.unwrap_or(0));
        gpu.draw(&renderer, &picture.view, &moment)?;
        let numbers = StillNumbers {
            moment: &moment,
            place: place.summary(),
            size: picture.size,
            adapter: &gpu.adapter,
        };
        let name = format!("still-{:08.3}", moment.song_time_s);
        write(&out, &format!("{name}.png"), picture.png(&gpu)?)?;
        write(&out, &format!("{name}.json"), json(&numbers, true)?)?;
    }
    Ok(())
}

/// Draws consecutive moments across a stretch of the song, and writes what they measure:
/// flashes, camera reversals and the time each took to draw.
fn run(request: &Request) -> Result<(), String> {
    let (place, gpu, renderer, picture) = prepare(request)?;
    let rate = request.rate.unwrap_or(DISPLAY_HZ);
    let from_s = request.from.unwrap_or(0.0);
    let to_s = request.to.unwrap_or(place.duration_s);
    let count = ((to_s - from_s) * rate as f64).round() as usize;
    if to_s <= from_s || count == 0 {
        return Err(format!("no moments from {from_s} s to {to_s} s"));
    }

    let mut series = Series::default();
    let mut tiles = vec![Vec::new(); comfort::TILES.0 * comfort::TILES.1];
    for index in 0..count {
        let at = from_s + index as f64 / rate as f64;
        let moment = place.moment(at, request.dial.unwrap_or(0));
        let drawn = gpu.draw(&renderer, &picture.view, &moment)?;
        series.draw_ms.push(drawn.as_secs_f64() * 1000.0);
        let luminance = picture.read(&gpu, |rows, stride| {
            comfort::luminance(rows, stride, picture.size)
        })?;
        series.luminance.push(luminance[0]);
        for (tile, value) in tiles.iter_mut().zip(&luminance[1..]) {
            tile.push(*value);
        }
        series.song_time_s.push(moment.song_time_s);
        series.route_position_m.push(moment.route_position_m);
        series.across_m.push(moment.camera.position_m[0] as f32);
        series.up_m.push(moment.camera.position_m[1] as f32);
        series.yaw_deg.push(moment.camera.yaw_deg as f32);
        series.pitch_deg.push(moment.camera.pitch_deg as f32);
        series.bank_deg.push(moment.camera.bank_deg as f32);
        series.light.push(moment.light as f32);
        series.speed_m_s.push(moment.speed_m_s);
        series.over_ground_per_s.push(moment.over_ground_per_s);
        series.past_land_per_s.push(moment.past_land_per_s);
        series.impulses.push(moment.impulses.len());
    }

    let (metres, degrees) = LEAST_REVERSAL;
    let poses = [
        (&series.across_m, metres),
        (&series.up_m, metres),
        (&series.yaw_deg, degrees),
        (&series.pitch_deg, degrees),
        (&series.bank_deg, degrees),
    ];
    let reversals =
        |(pose, least): &(&Vec<f32>, f32)| comfort::worst_reversals_in_a_second(pose, *least, rate);
    let flashes = |luminance: &Vec<f32>| comfort::worst_flashes_in_a_second(luminance, rate);
    // Flashes are counted on the mean relative luminance of the whole picture and of each
    // tile of it. The camera's worst is over every part of its pose. Drawing is timed
    // from submitting a moment to the GPU having finished it.
    let numbers = serde_json::json!({
        "from_s": from_s,
        "to_s": to_s,
        "moments_per_second": rate,
        "size": picture.size,
        "adapter": gpu.adapter,
        "flashes": {
            "worst_in_any_second": flashes(&series.luminance),
            "worst_tile_in_any_second": tiles.iter().map(flashes).max(),
            "widest_luminance_range_in_any_second":
                comfort::widest_range_in_a_second(&series.luminance, rate),
        },
        "camera": { "worst_reversals_in_any_second": poses.iter().map(reversals).max() },
        "passing": {
            "over_ground_per_s": spread(series.over_ground_per_s.clone()),
            "past_land_per_s": spread(series.past_land_per_s.clone()),
        },
        "draw_ms": spread(series.draw_ms.clone()),
        "series": series,
        "tiles": tiles,
    });
    let out = request.out.clone().unwrap_or(PathBuf::from(CAPTURES));
    let name = format!("run-{from_s:08.3}-{to_s:08.3}.json");
    write(&out, &name, json(&numbers, false)?)?;
    println!(
        "{count} moments at {rate} a second: flashes {}, camera {}, passing {}, draw_ms {}",
        numbers["flashes"], numbers["camera"], numbers["passing"], numbers["draw_ms"]
    );
    Ok(())
}
