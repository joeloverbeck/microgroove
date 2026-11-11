//! Command line: `timeline`, `still` and `run`, each on one audio file and with no display.

mod analysis;
mod comfort;
mod decode;
mod place;
mod render;
mod timeline;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;

use place::Place;
use render::{FORMAT, Gpu, Renderer, Size, Still};

const USAGE: &str = "usage: microgroove timeline <audio file> [--out <directory>]
       microgroove still <audio file> --at <song time> [--at <song time> ...] [--size <width>x<height>] [--out <directory>]
       microgroove run <audio file> [--from <song time>] [--to <song time>] [--rate <moments a second>] [--size <width>x<height>] [--out <directory>]
a song time is seconds (95.5) or minutes and seconds (1:35.5)";

/// Where output goes unless `--out` says otherwise; both git-ignored in this repository.
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
    /// Impulses in play.
    impulses: Vec<usize>,
    /// Mean relative luminance of the whole picture.
    luminance: Vec<f32>,
}

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let command = args.next().and_then(|command| command.into_string().ok());
    let (act, options): (Command, &[&str]) = match command.as_deref() {
        Some("timeline") => (timeline, &["--out"]),
        Some("still") => (still, &["--out", "--size", "--at"]),
        Some("run") => (run, &["--out", "--size", "--from", "--to", "--rate"]),
        _ => (|_| Ok(()), &[]),
    };
    let Some(request) = parse(args.collect(), options) else {
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
            _ => request.size.replace(size(text?)?).is_none(),
        };
        if !once {
            return None;
        }
    }
    request.song = song?;
    // A command that takes moments must be given at least one.
    let wants_moments = options.contains(&"--at");
    (!options.is_empty() && request.at.is_empty() != wants_moments).then_some(request)
}

/// Seconds (`95.5`) or minutes and seconds (`1:35.5`).
fn song_time(text: &str) -> Option<f64> {
    let (minutes, seconds) = text.split_once(':').unwrap_or(("0", text));
    let time = f64::from(minutes.parse::<u32>().ok()?) * 60.0 + seconds.parse::<f64>().ok()?;
    (time.is_finite() && time >= 0.0).then_some(time)
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
        duration_s: song.duration_s(),
        decoded_audio_md5: song.pcm24_md5(),
        analysis: &analysis,
    };
    write(
        &out,
        "timeline.png",
        timeline::draw(&analysis, song.duration_s()),
    )?;
    write(&out, "analysis.json", json(&numbers, true)?)
}

/// The place made from a file, on a GPU with no display, with a picture to draw it into.
fn place(request: &Request) -> Result<(f64, Place, Gpu, Renderer, Still), String> {
    let (song, analysis) = hear(&request.song)?;
    let end_s = song.duration_s();
    let late = [request.from, request.to]
        .into_iter()
        .flatten()
        .chain(request.at.iter().copied());
    if let Some(late) = late.into_iter().find(|time| *time > end_s) {
        return Err(format!(
            "no moment at {late} s: the song ends at {end_s:.2} s"
        ));
    }
    let place = Place::of(&analysis, end_s);
    let gpu = Gpu::headless()?;
    let size = request.size.unwrap_or(DISPLAY);
    let renderer = Renderer::new(&gpu.device, FORMAT, size, &place);
    let picture = Still::new(&gpu.device, size);
    Ok((end_s, place, gpu, renderer, picture))
}

/// Writes a still of each moment asked for, with its numbers.
fn still(request: &Request) -> Result<(), String> {
    let (_, place, gpu, renderer, picture) = place(request)?;
    let out = request.out.clone().unwrap_or(PathBuf::from(STILLS));
    for at in &request.at {
        let moment = place.moment(*at);
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
    let (end_s, place, gpu, renderer, picture) = place(request)?;
    let rate = request.rate.unwrap_or(DISPLAY_HZ);
    let (from_s, to_s) = (request.from.unwrap_or(0.0), request.to.unwrap_or(end_s));
    let count = ((to_s - from_s) * rate as f64).round() as usize;
    if to_s <= from_s || count == 0 {
        return Err(format!("no moments from {from_s} s to {to_s} s"));
    }

    let mut series = Series::default();
    let mut tiles = vec![Vec::new(); comfort::TILES.0 * comfort::TILES.1];
    let mut draw_ms = Vec::with_capacity(count);
    for index in 0..count {
        let moment = place.moment(from_s + index as f64 / rate as f64);
        draw_ms.push(gpu.draw(&renderer, &picture.view, &moment)?.as_secs_f64() * 1000.0);
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
    draw_ms.sort_by(f64::total_cmp);
    let share = |of: usize| draw_ms[(count - 1) * of / 100];
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
        "draw_ms": {
            "least": share(0),
            "median": share(50),
            "p95": share(95),
            "p99": share(99),
            "max": share(100),
        },
        "series": series,
    });
    let out = request.out.clone().unwrap_or(PathBuf::from(CAPTURES));
    let name = format!("run-{from_s:08.3}-{to_s:08.3}.json");
    write(&out, &name, json(&numbers, false)?)?;
    println!(
        "{count} moments at {rate} a second: flashes {}, camera {}, draw_ms {}",
        numbers["flashes"], numbers["camera"], numbers["draw_ms"]
    );
    Ok(())
}
