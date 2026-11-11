# Mission

## Authority and status

The repository owner (GitHub `joeloverbeck`) is the only acceptance authority. This document records the owner's founding decisions from the interview of **2026-11-11**. It specializes [the constitution](../principles/principles-for-repos.md), chiefly `PURP`, `EVID`, `BOUND` and `GOV`, with this repository's facts; the constitution governs wherever the two differ.

Nothing here is built or demonstrated. Every design statement in these foundations is a proposal until the owner has ridden a real journey and accepted it.

## Purpose

Purpose class: **recreation** (`PURP-1`).

Primary outcome: the owner gives the program one local audio file and is carried, hands off, through a place made from that song, so that listening feels like travelling through the song instead of sitting in front of a still picture.

The sign that makes continuing worthwhile is private and owner-judged: after a journey, the owner would rather have heard the song this way than with a static video.

That outcome bundles four separable claims (`PURP-2`). Each needs its own evidence, and evidence for one says nothing about the next:

1. **One journey.** A single song, ridden end to end, feels like that song, and its parts feel different from each other. *Active:* see [the first journey](03-first-journey.md).
2. **Songs differ.** A materially different song becomes a recognizably different place, for reasons the owner can hear. *Not active.*
3. **Any song.** The same holds for arbitrary files beyond the owner's own catalogue. *Not active;* a generalization claim under `EVID-6`.
4. **Returning.** The owner chooses this as a way to listen, again and unprompted. *Not active;* a recurrence claim under `EVID-3`.

Expansion event: owner acceptance of claim 1 permits selecting claim 2 through its own bounded issue. Nothing else enlarges scope.

Appetite for claim 1: one bounded GitHub issue and **12 owner viewings**, defined in [the first journey](03-first-journey.md#appetite), under the default limits of `WORK-3` and `OPS-4`.

Portfolio: the owner states that this repository runs alongside current work and displaces nothing. No change to any other repository's status is authorized.

## Incumbent and comparison

Without this repository the owner listens through YouTube videos or a music player (`PURP-4`).

The motivating memory is Audiosurf, played long ago. The owner loved riding any chosen song and remembers that different songs, and the parts within one song, felt sensorially different. Of Beat Saber the owner is unsure, adding: "Sometimes I just want to experience the song without having to perform myself." The memory was not re-checked by replaying the game and is **unverified** (`EVID-1`).

- Preserve: any song can be ridden; songs and their parts feel different.
- Change: no performing; a place instead of a game track.

The owner selected a custom hands-off program as part of the outcome and declined a replay, so no competitive comparison was run and no superiority over existing software is claimed. [Prior art](../research/prior-art.md) lists the alternatives found and the cheapest replay, should the owner ever want one.

## Outside the outcome

Deferred possibilities. They carry no authority and are not a backlog:

- steering, looking around, or any gameplay, score or failure;
- lyrics, titles, subject matter, generated imagery, or any model or network use at run time;
- separating a mix into stems;
- VR, other operating systems, live or streamed input;
- video export, a music library, playlists;
- per-song hint files, presets, scripting, plugins, or a reusable engine or published crate.

## Inputs, consumer and boundary

Real input: audio files the owner supplies by path. The first is named in [the first journey](03-first-journey.md#claim-and-input). Audio is private and is never committed.

Consumer: the owner, at an Ubuntu desktop with one 4K 144 Hz display and an RTX 2080 Ti, wearing headphones.

One pipeline (`BOUND-1`):

> local audio file → whole-file analysis → deterministic world with synchronized playback → the owner's experience → owner acceptance, or an honest refusal of a file that cannot be read

The whole repository has one release and one abandonment decision. A second pipeline, such as video rendering, analysis offered as a library, or library management, is a `BOUND-1` stop.

No other repository is imported, exported to, or read. The program receives a file path and nothing else: it works on a bare audio file and never depends on the layout of whatever tool produced it (`INT-1`).

## Work control

GitHub Issues and pull requests of this repository own active work (`GOV-2`). These documents record decisions; they are not tickets.
