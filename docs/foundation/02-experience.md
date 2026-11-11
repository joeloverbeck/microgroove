# Experience

What a journey is. The first section is decided, the second is a proposal to be tested by riding, the third states what cannot be heard, and the last is what to ask after a viewing.

## Decided by the owner (2026-11-11)

1. **Hands off.** The song carries the listener. A journey needs no input and has nothing to perform, score or fail.
2. **A place.** Landscape and sky that the listener travels through. Not an interior, not pure abstraction, and not a different family of world per song.
3. **A dream landscape.** A real sense of space (horizon, atmosphere, light, vast scale) built from matter that visibly answers the song. Believable as a place, impossible as geology. Neither naturalistic footage nor flat graphic stylization.
4. **Sound only.** The world comes from the audio signal alone, never from lyrics, titles, metadata or subject matter, so every file is treated alike.
5. **The same world every time.** One file always yields the same journey. A song becomes a place the owner can return to and get to know.
6. **Strong, never strobing.** Speed, scale and light may swing widely, and the camera may bank and dive. Full-screen flashes stay below about three per second, and the camera does not shake.
7. **One file, one journey.** Whatever the file's length or contents, it is ridden as a single journey.
8. **Rust.** All project-owned code is Rust. The owner expects, and the agent recommends, no game engine.

## Proposed design: time is distance

Proposed by the agent, unproven, and replaceable wherever riding shows it wrong (`PURP-2`).

The file is known in full before playback starts, so the whole song is analysed first and its future is laid out in space ahead of the listener. Song time maps to distance along the route. Four consequences follow:

- **Foreknowledge.** What is coming is visible before it sounds: a loud section stands on the horizon during the quiet one before it.
- **Sync by placement.** An event four seconds ahead is an object four seconds down the route, and the listener passes it as it sounds.
- **Form is geography.** Sections are regions. A repeated section is the same region revisited and changed.
- **Fingerprint is identity.** Whole-song measurements (tempo, tonal centre, brightness, roughness, dynamic range, stereo width, rhythmic regularity) set the place's palette, shape language, scale, weather and the camera's temperament.

One rule holds it together: **nothing moves unless the song moves it.**

The song acts at three time scales:

| Scale | Carries | Example mapping |
| --- | --- | --- |
| Whole song | identity and geography | fingerprint → what kind of place; sections → regions |
| Phrase and section | the energy arc | intensity → speed, altitude, openness, light |
| Beat and onset | impact | transients → impulses in ground, light and sky |

These mappings are hypotheses for the first journey, not a requirements inventory.

## Limits of hearing

- From a stereo mix, plain signal analysis can estimate loudness, spectral balance, onsets, beat and tempo, a percussive/tonal split, key, and section boundaries. Each estimate can be wrong.
- It cannot isolate a voice or an instrument. That needs learned source separation, which is outside the outcome.
- Where a measurement is unreliable (no steady pulse, no clear sections, ambiguous key), the world falls back to what is reliable (energy, onsets, spectrum) and shows no pulse or boundary rather than a guessed one (`SEM-1`).
- The agent cannot see motion or hear. It can inspect still frames and numbers. Feel, sync and beauty are judged only by the owner.

## After a viewing

The owner's verdict is the evidence. These questions make it easy to give:

1. Did the parts of the song feel different from each other, and did the place change where the music changed?
2. Could you see things coming?
3. Did hits land on time?
4. Did the place suit this song?
5. Did anything move for no reason you could hear?
6. Was it comfortable for the whole length?
