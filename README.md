# microgroove

Give it a song; it carries you through a place made from that song.

`microgroove` is a personal Rust program. It analyses a whole local audio file before playing it, then flies the listener, hands off, through a dream landscape whose geography is the song's structure and whose future lies visibly ahead. Under magnification a record's groove is a canyon whose two walls are the left and right channels; the name takes that literally.

**Status (2026-11-11):** an experiment in progress on the `first-journey` branch. The program decodes one audio file, analyses it, builds a place from the analysis, and rides it: full screen, with the song playing. It also writes a timeline of what it heard and a still of any moment of the place. The journey has not yet been seen on a display, by the owner or by a measurement, and the owner has accepted nothing.

## Build

Needs a Rust toolchain; it is built and tested with Rust 1.97.1. On Ubuntu the audio output also needs `libasound2-dev` and `libpipewire-0.3-dev` at build time, and libclang, which generates the PipeWire bindings. Pictures are drawn on the GPU through Vulkan. ffmpeg is optional: where it is installed, the tests compare the program's decode and loudness with it.

```sh
cargo build --release
```

## Ride

```sh
cargo run --release -- /path/to/song.flac
```

The program analyses the whole file, then opens a borderless full-screen window and plays the song, unaltered, through the default audio device. Each picture shows the moment for the song time being heard at that instant, taken from the audio device's own clock. A journey needs no input and ends by itself when the file ends.

| Key | What it does |
| --- | --- |
| Space | pauses sound and picture together, and resumes both |
| Escape or Q | quits |
| Right arrow | shows the picture 5 ms earlier against the audio; press it when what you see lands after what you hear |
| Left arrow | shows the picture 5 ms later |
| Up arrow | turns the dial up a step: the camera rides a tenth nearer the ground, so the ground passes faster |
| Down arrow | turns the dial down a step: the camera rides a tenth higher, so the ground passes slower |

The nudge and the dial are aids for this experiment. The dial goes eight steps either way and changes only the camera's height: the place, the speed along the route and the land beside it stay as they are.

As it ends, a journey prints the nudge and the dial in use (`nudge at exit: +10 ms; dial at exit: +2`) and then writes what it measured of itself to `captures/journey.json`: the time between pictures, how steadily song time ran, the keys pressed with their times, and anything the audio device reported. If that file cannot be written the journey says so and still ends well. `--size <width>x<height>` draws the picture smaller than the display and scales it up; `--out <directory>` writes the measurements somewhere else.

`captures/` and `stills/`, here and below, are made under the directory the program is run from. Inside this repository both are git-ignored, at any depth.

The program does not resample: it asks the audio device for the file's own sample rate. With no screen attached and switched on, a journey is refused before a window or the audio device is opened. A journey whose window the desktop never showed still writes its measurements, and then fails.

On the owner's machine the song's path lives in `MICROGROOVE_SONG` in the git-ignored `.env` (see `.env.example`). The program never reads `.env`; it takes the path on the command line. Load the variable into the shell first:

```sh
set -a; . ./.env; set +a
cargo run --release -- "$MICROGROOVE_SONG"
```

`.env` is untracked, so a git worktree does not contain it: load it from the main checkout.

### Silent self-launch

```sh
cargo run --release -- self-launch "$MICROGROOVE_SONG"
```

This is the same journey with every sample multiplied by zero, so it makes no sound. It opens the real window and the real audio device, presses its own keys (both nudges, both ways of the dial, pause, resume, quit), quits by itself after about ten seconds, and is ended outright at 15 seconds if it has not. It takes over the screen while it runs. It prints its measurements and writes them to `captures/self-launch.json`, with the last picture it showed as `captures/self-launch-last.png` and that moment's numbers beside it; a still made with `still --at <that song time> --dial <the dial at exit>` is the same moment drawn with no display. `--from <song time>` starts it somewhere other than the beginning. Its measurements say whether the window was ever shown, how many pictures were drawn without the desktop asking (they are left out of the time between pictures), and each time the audio clock laid its steady line afresh or passed over a stray saying of the device, with the song time and the size in ms.

A self-launch is not a viewing: nobody judges it.

## Inspect

Make a timeline, without a display:

```sh
cargo run --release -- timeline /path/to/song.flac
```

This writes `captures/timeline.png`, a picture of what analysis heard over song time, and `captures/analysis.json`, the same as numbers: the intensity curve, the onsets (each with the part of the sound that jumped, the channel it leans to and whether it stands out in the song), the pulse and where it is trusted, the section boundaries with their confidence, the fingerprint, and what differs between the left and right channels. `--out <directory>` writes them somewhere else. Both files are disposable and can be made again from the audio. FLAC, WAV and Ogg Vorbis are read. A file that is missing, unreadable or in another format is refused with one plain line and a non-zero exit; so is one with more than two channels, or with a sample rate analysis cannot handle (below 3,364 Hz or above 2,822,400 Hz).

Make a still of one or more moments, without a display:

```sh
cargo run --release -- still /path/to/song.flac --at 1:50 --at 3:25.5
```

A song time is seconds (`95.5`) or minutes and seconds (`1:35.5`). For each moment this writes `stills/still-<seconds>.png` and, beside it, `still-<seconds>.json` with the numbers behind the moment: position on the route, the region and the kind of place it is, the kind of every region in song order, the camera's pose and the height of its view, the light level, how thick the air is, the speed, how fast the ground and the land beside the route pass, the land's profile across the route at the listener's position (its height on the left and on the right, from beside the route outwards), the impulses in play, and the forms in view with their kind, side, song time and the boxes they stand as. A form stands a little further along the route than the route is at its onset's song time, so that the last of it leaves the picture as its onset sounds: the picture of the camera at that moment with the dial at rest, taken as 16 wide to 9 tall. A still is 3840×2160 unless `--size <width>x<height>` says otherwise, `--dial <steps>` shows the moment as a journey does with its dial turned that far (from -8 to 8), and `--out <directory>` writes somewhere else. A moment depends only on the audio and the song time, so the same moment gives the same still on every run, in whatever order stills are asked for. A song whose route runs further than the GPU can hold ground for, about 225 km today, is refused with one line that says how many minutes of it the place can hold.

Measure a run of consecutive moments, without a display:

```sh
cargo run --release -- run /path/to/song.flac --from 3:30 --to 3:47
```

This draws every moment of the stretch at 144 a second and 3840×2160 and writes `captures/run-<from>-<to>.json`: the most full-screen flashes in any one second (after the "general flash" of WCAG 2.2, as a measuring aid and not a conformance claim), the most times the camera's pose turns back on itself in any one second, how fast what is near passes the camera, how long each moment took to draw, and the position, pose, speed, light, luminance and draw time of every moment, with the luminance of each twelfth of the picture, so that a flash can be found in time and in place. The passing rates are in times a second: the speed divided by the camera's height above the ground under it, and the speed divided by the distance to where the land beside the route begins. It prints the summary. Without `--from` and `--to` it covers the whole song; `--rate <moments a second>` and `--size` make a cheaper run, and `--dial <steps>` measures it as a journey shows it with its dial turned.

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
