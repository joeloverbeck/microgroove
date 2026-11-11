# The first journey

The one bounded experiment these foundations authorize. It becomes work only through a GitHub issue.

## Claim and input

Claim: one song, ridden end to end with hands off, feels like travelling through that song, and its parts feel different from each other. This is claim 1 in [the mission](01-mission.md#purpose).

Input: the owner's song **"Mammalian Machine (post-hardcore version)"**: stereo FLAC, 48 kHz, 24-bit, 4:06, loudness range 9.2 LU. The owner selected it. Its stark quiet and loud passages, distortion and hard drums exercise section geography, intensity and impact together, and its length keeps each viewing cheap. The file lives outside the repository.

Path: the real one (`EVID-3`). The program reads that file, plays it through the owner's audio device and presents full screen on the owner's display for the whole length. A debug view, a clip or a recording is not a journey.

Before owner acceptance the implementation is an experiment (`WORK-2`). Once accepted, it is the starting implementation, not a general system.

## Appetite

- **12 viewings.** A viewing is any launch the owner judges, whatever its length: one the owner is asked to judge, or one the owner starts unprompted and reports a verdict on that the work then acts on. Accepted, rejected and inconclusive viewings all count (`EVID-8`).
- The default streak in `WORK-3` and the default stops in `OPS-4`.

Viewings are scarce, so the agent gets as far as still frames and measurements allow before asking for one, and each request names what the viewing decides.

When the appetite is used up, work stops for the owner's terminal decision (`WORK-5`): adopt, narrow, reframe, park or reject.

## Uncertainties, in order

Attack the earliest first (`EVID-2`):

1. **Can analysis hear this song's shape?** Whether the sections, pulse and energy arc recovered from the file match the song. The agent checks with numbers and rendered timelines; remaining doubt is settled inside a viewing.
2. **Does the place feel like the song?** Only viewings decide.
3. **Does it hold frame rate?** The agent can measure this. Rendering below display resolution and scaling up is implementation judgment.

## The minimum

What the first journey needs (`WORK-6`):

- decode the file and analyse it whole;
- a world determined by that analysis and by song time alone, so that any moment can be reproduced exactly;
- playback with the picture locked to the audio clock;
- full-screen presentation with pause and quit;
- a still image of any moment, rendered without a display, for the agent to inspect.

Tuning against this one song is expected (`EVID-5`). Behaviour keyed to a file's identity is not, because it would break "sound only". Whether the result suits other songs is claim 2 and is not tested here.

Wait until a journey is accepted before adding settings files or screens, a stable cache format, presets, plugin points, a second kind of world, or publication as a crate.

## Starting points

Implementation judgment, replaceable, and to be re-checked when work starts: Rust with `wgpu` (GPU), `winit` (window), `cpal` (audio output), `symphonia` (decoding) and `rustfft` (analysis). Structure analysis is written here, since no mature crate was found. [Prior art](../research/prior-art.md) holds versions, platform caveats and algorithm references as of 2026-11-11.

Once code exists, `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` pass before any claim of completion.

## Operation

- **Input:** a path to a local audio file on the command line. There is no expected location and no corpus in the repository; `.gitignore` excludes audio.
- **Bad input:** a missing, unreadable or unsupported file is reported plainly with a non-zero exit. Nothing is substituted.
- **Derived data:** analysis caches and stills are disposable, reproducible and kept out of tracked files.
- **Dependencies:** no other repository, service or network access.
- **Today:** there is no code and nothing to run. The first issue creates the build and launch path and records it in the README.
