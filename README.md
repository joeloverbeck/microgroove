# microgroove

Give it a song; it carries you through a place made from that song.

`microgroove` is a personal Rust program at the foundation stage. It will analyse a whole local audio file before playing it, then fly the listener, hands off, through a dream landscape whose geography is the song's structure and whose future lies visibly ahead. Under magnification a record's groove is a canyon whose two walls are the left and right channels; the name takes that literally.

**Status (2026-11-11):** an experiment in progress on the `first-journey` branch. The program decodes one audio file, analyses it, builds a place from the analysis, and writes a timeline of what it heard and a still of any moment of the place. No journey can be ridden yet, and the owner has accepted nothing.

## Build and inspect

Needs a Rust toolchain; it is built and tested with Rust 1.97.1. Stills are drawn on the GPU through Vulkan, with no window and no display. ffmpeg is optional: where it is installed, the tests compare the program's decode and loudness with it.

```sh
cargo build --release
```

Make a timeline, without a display:

```sh
cargo run --release -- timeline /path/to/song.flac
```

This writes `captures/timeline.png`, a picture of what analysis heard over song time, and `captures/analysis.json`, the same as numbers: the intensity curve, the onsets, the pulse and where it is trusted, the section boundaries with their confidence, the fingerprint, and what differs between the left and right channels. `--out <directory>` writes them somewhere else. `captures/` is git-ignored; both files are disposable and can be made again from the audio. FLAC, WAV and Ogg Vorbis are read. A file that is missing, unreadable or in another format is refused with one plain line and a non-zero exit.

Make a still of one or more moments, without a display:

```sh
cargo run --release -- still /path/to/song.flac --at 1:50 --at 3:25.5
```

A song time is seconds (`95.5`) or minutes and seconds (`1:35.5`). For each moment this writes `stills/still-<seconds>.png` and, beside it, `still-<seconds>.json` with the numbers behind the moment: position on the route, the region, the camera's pose, the light level and the impulses in play. A still is 3840×2160 unless `--size <width>x<height>` says otherwise, and `--out <directory>` writes somewhere else. A moment depends only on the audio and the song time, so the same moment gives the same still on every run, in whatever order stills are asked for. `stills/` is git-ignored.

Measure a run of consecutive moments, without a display:

```sh
cargo run --release -- run /path/to/song.flac --from 3:30 --to 3:47
```

This draws every moment of the stretch at 144 a second and 3840×2160 and writes `captures/run-<from>-<to>.json`: the most full-screen flashes in any one second (after the "general flash" of WCAG 2.2, as a measuring aid and not a conformance claim), the most times the camera's pose turns back on itself in any one second, how long each moment took to draw, and the position, pose, light and luminance of every moment. It prints the summary. Without `--from` and `--to` it covers the whole song; `--rate <moments a second>` and `--size` make a cheaper run.

Check the work:

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

The checks against the first-journey song run only where `MICROGROOVE_SONG` names that song (see `.env.example`; the program itself never reads `.env`). Without it each of them prints a `SKIPPED` line and says why.

## Read in this order

1. [CLAUDE.md](CLAUDE.md): agent entry point. `AGENTS.md` is a symlink to it.
2. [Mission](docs/foundation/01-mission.md): purpose, outcome, limits and boundary.
3. [Experience](docs/foundation/02-experience.md): what the owner decided a journey is, and the proposed design.
4. [The first journey](docs/foundation/03-first-journey.md): the one bounded experiment, its appetite and its minimum.

[Prior art](docs/research/prior-art.md) is reference material, not a work queue. The [constitution](docs/principles/principles-for-repos.md) is an unchanged copy of the owner's file.

## Working here

Active work lives in this repository's GitHub Issues and pull requests. Songs are supplied by path at run time and are never committed. No other repository or service is needed.

## License

Original project code uses the [MIT license](LICENSE). This is a personal project with no support promise.
