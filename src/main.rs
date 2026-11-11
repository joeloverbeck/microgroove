//! Command line: `microgroove timeline <file> [--out <dir>]`.

mod analysis;
mod decode;
mod timeline;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;

const USAGE: &str = "usage: microgroove timeline <audio file> [--out <directory>]";

/// Where a timeline goes unless `--out` says otherwise; git-ignored in this repository.
const DEFAULT_OUT: &str = "captures";

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

fn main() -> ExitCode {
    let Some((song, out)) = parse(std::env::args_os().skip(1).collect()) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match timeline(&song, &out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("microgroove: {message}");
            ExitCode::FAILURE
        }
    }
}

/// `timeline <file> [--out <dir>]` in any order after the command; None for anything else.
fn parse(args: Vec<OsString>) -> Option<(PathBuf, PathBuf)> {
    let mut args = args.into_iter();
    if args.next()? != "timeline" {
        return None;
    }
    let (mut song, mut out) = (None, None);
    while let Some(arg) = args.next() {
        let slot = if arg == "--out" { &mut out } else { &mut song };
        let value = if arg == "--out" { args.next()? } else { arg };
        if slot.replace(PathBuf::from(value)).is_some() {
            return None;
        }
    }
    Some((song?, out.unwrap_or_else(|| PathBuf::from(DEFAULT_OUT))))
}

/// Decodes and analyses the whole file, then writes the timeline and the analysis numbers.
fn timeline(path: &Path, out: &Path) -> Result<(), String> {
    let song =
        decode::decode(path).map_err(|refusal| format!("refused {}: {refusal}", path.display()))?;
    let analysis = analysis::analyse(&song);
    let numbers = Numbers {
        sample_rate: song.sample_rate,
        channels: song.channels.len(),
        frames: song.frames(),
        duration_s: song.duration_s(),
        decoded_audio_md5: song.pcm24_md5(),
        analysis: &analysis,
    };
    let write = |name: &str, bytes: Vec<u8>| {
        let file = out.join(name);
        std::fs::write(&file, bytes)
            .map_err(|error| format!("cannot write {}: {error}", file.display()))?;
        println!("{}", file.display());
        Ok::<(), String>(())
    };
    std::fs::create_dir_all(out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    let mut json = serde_json::to_vec_pretty(&numbers).map_err(|error| error.to_string())?;
    json.push(b'\n');
    write("timeline.png", timeline::draw(&analysis, song.duration_s()))?;
    write("analysis.json", json)
}
