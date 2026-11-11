# Prior art and stack notes

Collected by a research agent for the founding of this repository. Claims carry their source; anything marked UNVERIFIED was not confirmed. Re-check before relying on any of it.

Retrieved 2026-11-11. Reference material, not a work queue. UNVERIFIED = not confirmed today from a primary or reputable source. Steam and crate facts were read through the store and crates.io APIs.

## 1. Incumbents and prior art

### Games that build a level from your own song
- **Audiosurf** (2008): ride a track shaped by the song. Windows only; ProtonDB gold. Local MP3, M4A, WMA, OGG, CD; no FLAC listed. Developer: it "analyzes the entire song before play begins". $9.99; legacy. https://store.steampowered.com/app/12900 · https://www.protondb.com/app/12900 · https://www.gamedeveloper.com/game-platforms/road-to-the-igf-i-audiosurf-i-making-waves-with-music
- **Audiosurf 2** (2015): same idea, moddable. Native Linux; ProtonDB gold. Local files; no format list on the store page (FLAC on Linux UNVERIFIED). Whole-song pre-analysis UNVERIFIED for this title. $14.99; free demo allows 3 songs of your choice. Official maintenance UNVERIFIED; a community patch exists. https://store.steampowered.com/app/235800 · https://store.steampowered.com/app/373960 · https://audiosurf2.info/download/linux
- **Riff Racer** (ex Drive Any Track): racetrack from the song; "Verses, choruses and drops become checkpoints". The only incumbent found that claims section detection. Windows; MP3/FLAC/OGG; analysed locally. Delisted from Steam; dead. https://store.steampowered.com/app/351990 · https://www.mcvuk.com/the-develop-post-mortem-riff-racer/
- **Melody's Escape** (2016): runner whose pace follows intensity. Native Linux; MP3, OGG, FLAC, AAC; song analysed "within a few seconds" before play. $9.99. https://store.steampowered.com/app/270210
- **Melody's Escape 2** (2026-06-12): Windows only; ProtonDB platinum (7 reports). MP3/FLAC/OGG/WAV/M4A; "vastly improved tempo, intensity and voice detection". $14.99; active. https://store.steampowered.com/app/1829470
- **Symphony** (2012): shooter, levels from your songs. Native Linux (MP3, OGG, WAV, FLAC, AIFF); analysis undocumented. $4.99. https://store.steampowered.com/app/207750
- **Beat Hazard 2 / 3** (2019 / 2024): music-driven shooters. Windows only; ProtonDB gold / platinum. "Open Mic" listens to any source, so at least partly real time; local formats and whole-track analysis UNVERIFIED. $18.99 / $19.99. https://store.steampowered.com/app/618740 · https://store.steampowered.com/app/1493520
- **Polynomial 2** (2016): fly through fractal nebulae that "animate to your music". Native Linux; own files/M3U or system audio; real-time reactive. $11.99. https://store.steampowered.com/app/379420
- **AUDIOMECH** (Audiosurf's developer, unreleased): Windows; real-time reaction to "any audio playing on your machine". https://store.steampowered.com/app/3925150

### Reactive visualizers (no lookahead)
- **projectM**: MilkDrop reimplementation, LGPL-2.1; libprojectM v4.1.8 (2026-10-06), maintained. Linux frontends: projectMSDL 2.0.0-pre1 (.deb/.tar.gz, pre-release), free Steam app, Qt frontend with PipeWire/PulseAudio/JACK backends, Kodi add-on, GStreamer plugin. https://github.com/projectM-visualizer/projectm · https://github.com/projectM-visualizer/frontend-sdl-cpp/releases · https://store.steampowered.com/app/1358800
  - Ubuntu 26.04 apt has only libprojectm 2.1.0 and `qmmp-plugin-projectm`; 26.10 adds `projectm-pulseaudio`/`projectm-sdl` 3.1.12. https://packages.ubuntu.com/search?keywords=projectm&searchon=names&suite=all&section=all
  - PipeWire output: the SDL frontend captures an input device and may not list monitors; user-reported fix: in pavucontrol's Recording tab point "SDL Application" at the output's Monitor. https://github.com/projectM-visualizer/frontend-sdl-cpp/issues/100
  - Local file: qmmp plus its plugin, or gst-projectm (`filesrc ! decodebin ! … ! projectm`, renders to video). https://github.com/projectM-visualizer/gst-projectm
- **Butterchurn**: WebGL MilkDrop, MIT, last push 2026-04-20; takes a WebAudio node, runs in any browser. Demo's local-file loading UNVERIFIED. https://github.com/jberg/butterchurn
- **Kodi**: Ubuntu ships shadertoy, spectrum, waveform, fishbmc (21.0.2); projectM add-on is upstream. https://packages.ubuntu.com/search?keywords=kodi-visualization&searchon=names&suite=all&section=all · https://github.com/xbmc/visualization.projectm
- **VLC**: upstream projectM module exists; Ubuntu 26.04's `vlc-plugin-visualization` 3.0.23 lists no libprojectm dependency. https://wiki.videolan.org/Documentation:Modules/projectm/ · https://packages.ubuntu.com/resolute/vlc-plugin-visualization
- **Synesthesia**: live VJ tool; Windows/macOS only; $99 Standard, $249 Pro (sale), perpetual. https://synesthesia.live/pricing
- **Plane9**: 3D scenes; Windows 7-10 only; free; v2.5.1.3 (2016-12-29); reacts to system audio. https://www.plane9.com/download

### Gap
Nothing found is Linux-native, maintained, takes a local FLAC, analyses the whole song and renders a 3D journey with upcoming structure visible. One open-ended search found no such tool (absence of evidence only).

### Least-effort test (under an hour): Audiosurf 2 Demo
1. `sudo apt install steam-installer` (multiverse, amd64). https://packages.ubuntu.com/search?keywords=steam-installer&searchon=names&suite=all&section=all
2. In Steam, install "Audiosurf 2 Demo" (free, Linux listed).
3. Put a song under `~/Music` (user-reported as the only folder read; UNVERIFIED). If FLAC fails, convert: `ffmpeg -i song.flac song.ogg`. https://www.protondb.com/app/235800
4. If the native build fails, force Proton in Properties > Compatibility. Behaviour on GNOME Wayland + NVIDIA UNVERIFIED.

Reactive control: `sudo apt install qmmp qmmp-plugin-projectm`, play the same FLAC (menu path UNVERIFIED). https://packages.ubuntu.com/resolute/qmmp-plugin-projectm

## 2. Rust stack facts

### Versions
- **wgpu 30.0.1** (2026-08-22), MIT OR Apache-2.0. Breaking major roughly quarterly (v28 Dec 2025, v29 Mar 2026, v30 Jul 2026); MSRV 1.87 as of v29. https://crates.io/crates/wgpu · https://github.com/gfx-rs/wgpu/releases
- **winit 0.30.13** (2026-03-02), Apache-2.0. 0.31.0-beta.4 (2026-10-24) is pre-release; scope of its API changes UNVERIFIED. https://crates.io/crates/winit
- **cpal 0.18.2** (2026-08-16), Apache-2.0, MSRV 1.85. 0.18.0 broke API: streams no longer auto-start, errors unified. PipeWire host is the opt-in `pipewire` feature (`libpipewire-0.3-dev`; `libasound2-dev` always). Unreleased master renames `play()` to `start()` and merges the timestamp types. https://github.com/RustAudio/cpal/blob/master/CHANGELOG.md · https://github.com/RustAudio/cpal
- **symphonia 0.6.1** (2026-08-13), MPL-2.0 (file-level copyleft), MSRV 1.85. Default: FLAC, PCM, ADPCM, Vorbis; WAV, OGG, MKV; ID3v1/ID3v2/APE; SIMD. Opt-in: `mp3`/`mp2`/`mp1` (`mpa`), `aac`, `alac`, `isomp4`, `aiff`, `caf`. The master README also lists Opus and WavPack, absent from 0.6.1's features. https://docs.rs/crate/symphonia/0.6.1/features · https://github.com/pdeljanov/Symphonia
- **rustfft 6.4.1** (2025-09-18), MIT OR Apache-2.0. "RustFFT does not normalize outputs"; AVX/SSE4.1/NEON on by default. https://docs.rs/rustfft/latest/rustfft/
- **realfft 3.5.1** (2026-10-26), MIT. Real-input wrapper over RustFFT; also unnormalised. https://docs.rs/realfft/latest/realfft/

### (a) wgpu/winit on Wayland + NVIDIA
- Present modes: Fifo everywhere; Mailbox on "NVidia on Vulkan and Wayland on Vulkan"; Immediate on "Most platforms except older DX12 and Wayland". https://docs.rs/wgpu/latest/wgpu/enum.PresentMode.html
- `desired_maximum_frame_latency` defaults to 2; Vulkan swapchain length is that plus 1. https://docs.rs/wgpu/30.0.1/wgpu/type.SurfaceConfiguration.html
- Since v29 `get_current_texture` returns `CurrentSurfaceTexture`: skip on `Timeout`/`Occluded`, reconfigure on `Outdated`/`Suboptimal`; pass `DisplayHandle` to `InstanceDescriptor` on Wayland. https://github.com/gfx-rs/wgpu/releases/tag/v29.0.0
- NVIDIA fixes: v28.0.1 "Fixed crash on nvidia cards when presenting from another thread"; v30.0.1 stopped passing an un-waited fence to `vkAcquireNextImageKHR`. https://github.com/gfx-rs/wgpu/releases
- Driver: explicit sync needs kernel 6.8+. A forum thread reports Wayland Vulkan present blocking; current status UNVERIFIED. https://download.nvidia.com/XFree86/Linux-x86_64/610.43.02/README/wayland-issues.html · https://forums.developer.nvidia.com/t/vulkan-wayland-vkqueuepresentkhr-waits-for-gpu-to-finish/302203
- winit on Wayland: exclusive fullscreen "will no-op a request" (use Borderless); no positioning, `set_visible` or `focus_window`; `pre_present_notify` schedules the frame callback that throttles `RedrawRequested`. https://docs.rs/winit/0.30.13/winit/window/struct.Window.html
- 144 Hz pacing, VRR and tearing under Mutter: UNVERIFIED.
- HDR: wgpu 30 adds `SurfaceConfiguration::color_space` and `Surface::display_hdr_info`; `Auto` "never selects HDR"; "wgpu does not tonemap or gamut-map for you". Mutter gained wp_color_management_v1 in GNOME 48. End-to-end HDR from wgpu on GNOME + NVIDIA UNVERIFIED. https://wgpu.rs/doc/wgpu/documentation/color/hdr_surfaces/index.html · https://www.phoronix.com/news/GNOME-wp_color_management_v1

### (b) cpal timestamps
- The output callback receives `OutputStreamTimestamp { callback, playback }`; `playback` is "the predicted instant that data written will be delivered to the device". 0.18 adds `StreamTrait::now()` and `buffer_size()`. Clocks are per stream. https://docs.rs/cpal/latest/cpal/struct.OutputStreamTimestamp.html
- Clock sources: ALSA `snd_pcm_status_get_htstamp()`, PipeWire `pw_stream_get_time_n()`, JACK `jack_get_time()`, PulseAudio host `std::time::Instant` (no device clock). https://docs.rs/crate/cpal/0.18.2/source/src/timestamp.rs
- Reliability: recently repaired, not long proven. 0.18.0 fixed ALSA timestamps stepping backward; 0.18.2 made PipeWire timestamps monotonic across graph changes; the PipeWire host dates from June 2026. Measured accuracy on either backend UNVERIFIED; calibrate with a click test. A PipeWire issue titled "alsa-plugin: status timestamps a lot more imprecise than raw alsa" (unread, UNVERIFIED) argues for the `pipewire` feature. https://github.com/RustAudio/cpal/blob/master/CHANGELOG.md · https://gitlab.freedesktop.org/pipewire/pipewire/-/issues/3774

### (c) MIR crates (assessments from metadata and READMEs, not code review)
- **ebur128** 0.1.11 (2026-10-26), MIT: EBU R128; 1.2M downloads; the one clearly mature crate here. https://crates.io/crates/ebur128
- **stratum-dsp** 1.0.0 (2025-12-19), MIT OR Apache-2.0, pure Rust: tempogram BPM, HMM beats, Krumhansl-Kessler key. Self-reported 87.7% BPM (within 2 BPM) and 72.1% key on DJ tracks, 56.7% BPM on FMA Small; one release, DJ-tuned. https://github.com/HLLMR/stratum-dsp
- **aubio** 0.2.1 / **aubio-rs** 0.2.0 (2021), GPL-3.0: C bindings for onset, tempo, pitch; stale. **bliss-audio-aubio-rs** 0.2.4 (2025-08-04), GPL-3.0: fork kept for bliss. https://crates.io/crates/aubio · https://crates.io/crates/bliss-audio-aubio-rs
- **bliss-audio** 0.13.0 (2026-08-18), GPL-3.0-only: song-similarity distances for playlists, built on aubio; maintained, but no beats/sections API. https://github.com/Polochon-street/bliss-rs
- **essentia** 0.1.5 (2025-07-03), MIT wrapper over C++ Essentia (AGPLv3): broadest coverage, heavy build, copyleft. https://crates.io/crates/essentia · https://essentia.upf.edu/licensing_information.html
- Young, low adoption, unproven, MIT and/or Apache: **beat-track-rs** 0.1.2 (2026-09-09; Ellis DP), **dsp-spectral** 0.1.1 (2026-10-24; STFT, mel, HPSS), **resonant-analysis** 0.4.0 (2026-05-01; onsets, beats, pitch), **bpm-key** 0.1.0 (2026-10-05), **dasp-rs** 0.6.0 (2026-11-01; CQT, MIR features), **librosa-rs** 0.1.0 (2026-11-10; nine downloads). https://crates.io/search?q=beat%20tracking%20tempo
- Structure segmentation: three crates.io searches found no dedicated crate (oximedia-mir 0.2.1, Apache-2.0, unchecked); expect to implement it. https://crates.io/search?q=music%20structure%20segmentation

## 3. Plain-DSP algorithms

Failure-mode lines without their own URL are general MIR knowledge, not re-verified today.

- **Spectral-flux onsets**: Bello, Daudet, Abdallah, Duxbury, Davies, Sandler (2005), "A Tutorial on Onset Detection in Music Signals", https://doi.org/10.1109/TSA.2005.851998; Dixon (2006), "Onset Detection Revisited", https://dafx.de/paper-archive/details/_kscRgr98aSFW4l-j1YJuA. Fails on vibrato (false positives) and soft bowed or sung onsets; see Böck and Widmer (2013), "Maximum Filter Vibrato Suppression for Onset Detection", https://dafx.de/paper-archive/details/0oee-99Z88WL7pSo749gcA.
- **DP beat tracking**: Ellis (2007), "Beat Tracking by Dynamic Programming", J. New Music Research 36(1), https://doi.org/10.1080/09298210701653344. Commits to one global tempo: breaks on tempo changes and rubato, and locks to half or double tempo.
- **Tempo**: Grosche and Müller (2011), "Extracting Predominant Local Pulse Information from Music Recordings", https://doi.org/10.1109/TASL.2010.2096216; Grosche, Müller, Kurth (2010), "Cyclic Tempogram", https://doi.org/10.1109/ICASSP.2010.5495219. Octave errors (2:1, 3:2) dominate; stratum-dsp reports about 12% such confusions.
- **Key**: Krumhansl (1990), "Cognitive Foundations of Musical Pitch", Oxford University Press (no URL checked); Temperley (1999), "What's Key for Key? The Krumhansl-Schmuckler Key-Finding Algorithm Reconsidered", https://doi.org/10.2307/40285812. Confuses relative, parallel and fifth-related keys; modulation, modal music and percussive or detuned chroma hurt.
- **HPSS**: FitzGerald (2010), "Harmonic/Percussive Separation using Median Filtering", DAFx-10, https://dafx.de/paper-archive/details/DsmIVcydPX66AuaqEmKyTQ. Vocals, vibrato and noisy sustained sounds leak into both parts; hence the residual in Driedger, Müller, Disch (2014), https://audiolabs-erlangen.com/resources/2014-ISMIR-ExtHPSep/.
- **Foote novelty**: Foote (2000), "Automatic audio segmentation using a measure of audio novelty", ICME, https://doi.org/10.1109/ICME.2000.869637. Finds only boundaries between contrasting homogeneous blocks; kernel size fixes the time scale; gives no labels. https://www.audiolabs-erlangen.de/resources/MIR/FMP/C4/C4S4_NoveltySegmentation.html
- **Laplacian segmentation**: McFee and Ellis (2014), "Analyzing Song Structure with Spectral Clustering", ISMIR, https://archives.ismir.net/ismir2014/paper/000319.pdf. Needs a chosen cluster count; beat-tracking errors propagate; low-repetition music yields arbitrary partitions.
- **Loudness**: EBU R 128 v5.0 (2023), "Loudness normalisation and permitted maximum level of audio signals", https://tech.ebu.ch/publications/r128; measurement in ITU-R BS.1770, https://www.itu.int/rec/R-REC-BS.1770 (URL not opened today). Integrated loudness is one number per track: use momentary (400 ms) or short-term (3 s) curves for an arc; heavily limited masters give a nearly flat curve.
