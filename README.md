# microgroove

Give it a song; it carries you through a place made from that song.

`microgroove` is a personal Rust program at the foundation stage. It will analyse a whole local audio file before playing it, then fly the listener, hands off, through a dream landscape whose geography is the song's structure and whose future lies visibly ahead. Under magnification a record's groove is a canyon whose two walls are the left and right channels; the name takes that literally.

**Status (2026-11-11):** an experiment in progress on the `first-journey` branch. The program decodes one audio file, analyses it and writes a timeline of what it heard. No journey can be ridden yet, and the owner has accepted nothing.

## Build and inspect

Needs a Rust toolchain; it is built and tested with Rust 1.97.1. ffmpeg is optional: where it is installed, the tests compare the program's decode and loudness with it.

```sh
cargo build --release
```

Make a timeline, without a display:

```sh
cargo run --release -- timeline /path/to/song.flac
```

This writes `captures/timeline.png`, a picture of what analysis heard over song time, and `captures/analysis.json`, the same as numbers: the intensity curve, the onsets, the pulse and where it is trusted, the section boundaries with their confidence, the fingerprint, and what differs between the left and right channels. `--out <directory>` writes them somewhere else. `captures/` is git-ignored; both files are disposable and can be made again from the audio. FLAC, WAV and Ogg Vorbis are read. A file that is missing, unreadable or in another format is refused with one plain line and a non-zero exit.

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
