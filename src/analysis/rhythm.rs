//! Onsets, and the pulse where it can be trusted.

use serde::Serialize;

use super::Span;
use super::spectrum::{BANDS, Spectrum};

/// The instant a sound begins.
#[derive(Serialize)]
pub struct Onset {
    pub time_s: f64,
    /// How sharply the sound rose: the rise in log amplitude, averaged over the bands.
    pub strength: f32,
    /// The part of the sound that jumped most, measured against the song's other onsets.
    pub part: Part,
    /// How far the jump leans to the right channel, in decibels: below zero, to the left.
    pub lean_db: f32,
    /// Whether the onset stands out among the song's onsets.
    pub stands_out: bool,
}

/// The lowest, the middle or the highest third of the bands.
#[derive(Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Part {
    Low,
    Middle,
    High,
}

/// The song's steady beat. `bpm` is absent and `trusted` is empty where no steady
/// beat is heard: no pulse is a valid outcome and nothing is guessed in its place.
#[derive(Serialize)]
pub struct Pulse {
    pub bpm: Option<f64>,
    /// How well the whole song repeats at the beat, from 0 (not at all) to 1 (exactly).
    pub regularity: f64,
    /// The stretches of the song that keep the beat.
    pub trusted: Vec<Span>,
}

/// Amplitudes are compressed as ln(1 + GAIN * amplitude), so a rise counts by its
/// ratio once a band is above about -60 dBFS and counts for little below that.
const COMPRESSION_GAIN: f32 = 1000.0;
/// Tuning: a rise must beat this floor plus `ONSET_OVER_LOCAL` times the average rise nearby.
const ONSET_FLOOR: f32 = 0.02;
const ONSET_OVER_LOCAL: f32 = 1.5;
const ONSET_LOCAL_S: f64 = 0.1;
/// An onset is the largest rise within this many seconds either side.
const ONSET_SEPARATION_S: f64 = 0.03;
/// The share of a frame, at its newest edge, in which a new sound is still too
/// tapered to register. Measured with clicks at every alignment to the frames.
const TAPER_HIDES: f64 = 0.25;
/// A jump is taken over this many frames from the onset's, against the loudest of this
/// many frames before it.
const JUMP_FRAMES: (usize, usize) = (3, 5);
/// Tuning: an onset stands out when its jump scores this much against the jumps of the
/// song's strong onsets, those at this percentile, and no onset within this many seconds
/// of it scores higher. Both bars are the song's own, so a song with no strong onsets
/// still has some that stand out.
const STANDS_OUT: f32 = 0.75;
const STRONG_PERCENTILE: usize = 90;
const STANDS_APART_S: f64 = 0.2;

/// Rise of each band's compressed amplitude above the loudest it has just been,
/// averaged over the bands. "Just" is the frames that overlap this one: a steady low
/// tone reads a little differently in each frame, and that ripple is not a rise.
fn rises(spectrum: &Spectrum) -> Vec<f32> {
    let compressed: Vec<f32> = (0..spectrum.frames())
        .flat_map(|n| {
            spectrum
                .frame(n)
                .iter()
                .map(|a| (1.0 + COMPRESSION_GAIN * a).ln())
        })
        .collect();
    let overlapping = (spectrum.window_s / spectrum.hop_s).ceil() as usize;
    let mut rises = vec![0.0; spectrum.frames()];
    for n in 1..spectrum.frames() {
        let mut rise = 0.0;
        for band in 0..BANDS {
            let before = (n.saturating_sub(overlapping)..n).map(|m| compressed[m * BANDS + band]);
            rise += (compressed[n * BANDS + band] - before.fold(0.0, f32::max)).max(0.0);
        }
        rises[n] = rise / BANDS as f32;
    }
    rises
}

/// How far each rise stands above its threshold; zero where it does not.
fn excesses(rises: &[f32], hop_s: f64) -> Vec<f32> {
    let reach = (ONSET_LOCAL_S / hop_s).round() as usize;
    (0..rises.len())
        .map(|n| {
            let nearby = &rises[n.saturating_sub(reach)..(n + reach + 1).min(rises.len())];
            let local = nearby.iter().sum::<f32>() / nearby.len() as f32;
            (rises[n] - ONSET_FLOOR - ONSET_OVER_LOCAL * local).max(0.0)
        })
        .collect()
}

/// How each band jumps at an onset in frame `n`: the rise of its compressed amplitude,
/// which counts a quiet sound like a loud one, and the power it gains, which does not.
fn jumps(spectrum: &Spectrum, n: usize) -> impl Iterator<Item = (f32, f32)> + '_ {
    let loudest = move |frames: std::ops::Range<usize>, band: usize| {
        frames.map(|m| spectrum.frame(m)[band]).fold(0.0, f32::max)
    };
    (0..BANDS).map(move |band| {
        let before = loudest(n.saturating_sub(JUMP_FRAMES.1)..n, band);
        let after = loudest(n..(n + JUMP_FRAMES.0).min(spectrum.frames()), band);
        let rise = ((1.0 + COMPRESSION_GAIN * after) / (1.0 + COMPRESSION_GAIN * before)).ln();
        (rise.max(0.0), (after * after - before * before).max(0.0))
    })
}

/// The onsets of the song as its two channels together have it, each with the part of
/// the sound that jumped, the channel it leans to, and whether it stands out; and the pulse.
pub fn onsets_and_pulse(spectrum: &Spectrum, sides: &[Spectrum; 2]) -> (Vec<Onset>, Pulse) {
    let rises = rises(spectrum);
    let excess = excesses(&rises, spectrum.hop_s);
    let reach = (ONSET_SEPARATION_S / spectrum.hop_s).round() as usize;
    let frames: Vec<usize> = (0..rises.len())
        .filter(|n| excess[*n] > 0.0 && super::is_peak(&rises, *n, reach))
        .collect();
    // Six measures of each onset: in each part of the sound, the rise averaged over its
    // bands and then the amplitude gained there.
    let jumped = |n: &usize| -> [f32; 6] {
        let jumps: Vec<(f32, f32)> = jumps(spectrum, *n).collect();
        std::array::from_fn(|measure| {
            let bands = &jumps[measure / 2 * BANDS / 3..(measure / 2 + 1) * BANDS / 3];
            if measure % 2 == 0 {
                bands.iter().map(|jump| jump.0).sum::<f32>() / bands.len() as f32
            } else {
                bands.iter().map(|jump| jump.1).sum::<f32>().sqrt()
            }
        })
    };
    let jumped: Vec<[f32; 6]> = frames.iter().map(jumped).collect();
    let strong: [f32; 6] = std::array::from_fn(|measure| {
        let mut values: Vec<f32> = jumped.iter().map(|jump| jump[measure]).collect();
        values.sort_by(f32::total_cmp);
        super::percentile(&values, STRONG_PERCENTILE).unwrap_or(0.0)
    });
    // An onset's score in a part of the sound sets its rise and its gain there against
    // those of the song's strong onsets; its part is the one where it scores highest.
    let score = |jump: &[f32; 6], part: usize| {
        let (rise, gain) = (2 * part, 2 * part + 1);
        let score = (jump[rise] / strong[rise] * jump[gain] / strong[gain]).sqrt();
        if score.is_finite() { score } else { 0.0 }
    };
    let mut onsets: Vec<(f32, Onset)> = frames
        .iter()
        .zip(&jumped)
        .map(|(n, jump)| {
            let highest = |a: &usize, b: &usize| score(jump, *a).total_cmp(&score(jump, *b));
            let part = (0..3).rev().max_by(highest).unwrap_or(0);
            let gain = |side: &Spectrum| jumps(side, *n).map(|jump| jump.1).sum::<f32>().sqrt();
            let [left, right] = sides.each_ref().map(|side| gain(side).max(1e-6));
            // A rise shows once the new sound is far enough into the frame for the
            // window's taper to let it through, so the sound began near the frame's
            // newest edge, not at its centre.
            let time_s = *n as f64 * spectrum.hop_s + (1.0 - TAPER_HIDES) * spectrum.window_s;
            let onset = Onset {
                time_s,
                strength: rises[*n],
                part: [Part::Low, Part::Middle, Part::High][part],
                lean_db: 20.0 * (right / left).log10(),
                stands_out: false,
            };
            (score(jump, part), onset)
        })
        .collect();
    // The highest scores first: each stands out unless one that already does is too near.
    let mut order: Vec<usize> = (0..onsets.len()).collect();
    order.sort_by(|a, b| onsets[*b].0.total_cmp(&onsets[*a].0));
    for index in order {
        let time_s = onsets[index].1.time_s;
        let near = |other: &(f32, Onset)| {
            other.1.stands_out && (other.1.time_s - time_s).abs() < STANDS_APART_S
        };
        onsets[index].1.stands_out = onsets[index].0 >= STANDS_OUT && !onsets.iter().any(near);
    }
    let onsets = onsets.into_iter().map(|(_, onset)| onset).collect();
    (onsets, pulse(&excess, spectrum.hop_s))
}

/// The slowest and fastest beat considered, in beats per minute.
const SLOWEST_BPM: f64 = 60.0;
const FASTEST_BPM: f64 = 200.0;
const BPM_STEP: f64 = 0.1;
/// Tuning: rates near this one are preferred, over this width, because listeners settle
/// on a beat near two a second: a rhythm far from it, such as one at one and a half
/// times the beat, must repeat much better to be taken for the beat. The preference
/// decides close calls, and is right only near its centre. Without it the first-journey
/// song reads 66 BPM, half its beat. With it, in review, synthetic beats of 100 to 132
/// under a rhythm one and a half times as fast were read rightly; beats of 70, 80 and
/// 90 were read as that rhythm; beats of 150 and 160 as three quarters of themselves;
/// and a beat of 70 by itself as 140. Each misreading was trusted.
const PREFERRED_BPM: f64 = 120.0;
const PREFERENCE_WIDTH_OCTAVES: f64 = 0.5;
/// A beat is checked at this many successive beats ahead.
const BEATS_AHEAD: usize = 4;
/// Tuning: how well the song must repeat at the beat, overall and nearby, to be trusted.
const TRUST_OVERALL: f64 = 0.15;
const TRUST_NEARBY: f64 = 0.25;
const NEARBY_S: f64 = 8.0;
const NEARBY_STEP_S: f64 = 1.0;
/// Tuning: trust that comes or goes for no longer than this is not a change in the pulse.
const BRIEF_S: f64 = 3.0;

/// How well a signal repeats `lag` frames later, as a correlation from -1 to 1.
fn correlation(signal: &[f32]) -> impl Fn(usize) -> f64 {
    let mean = signal.iter().map(|v| f64::from(*v)).sum::<f64>() / signal.len().max(1) as f64;
    let centred: Vec<f64> = signal.iter().map(|v| f64::from(*v) - mean).collect();
    let power: f64 = centred.iter().map(|v| v * v).sum();
    move |lag| {
        if power <= 0.0 {
            return 0.0;
        }
        centred
            .iter()
            .zip(centred.iter().skip(lag))
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / power
    }
}

/// How well a signal repeats at a beat of `period` frames (which may be fractional):
/// its correlation 1..=BEATS_AHEAD beats later, averaged.
fn repeats(correlation: impl Fn(usize) -> f64, period: f64) -> f64 {
    let mut total = 0.0;
    for beats in 1..=BEATS_AHEAD {
        let lag = period * beats as f64;
        let (below, part) = (lag.floor() as usize, lag.fract());
        total += correlation(below) * (1.0 - part) + correlation(below + 1) * part;
    }
    total / BEATS_AHEAD as f64
}

fn pulse(excess: &[f32], hop_s: f64) -> Pulse {
    let none = Pulse {
        bpm: None,
        regularity: 0.0,
        trusted: Vec::new(),
    };
    // Soften by one frame either side so that a beat a frame early or late still counts.
    let envelope: Vec<f32> = (0..excess.len())
        .map(|n| {
            let at = |m: usize| excess.get(m).copied().unwrap_or(0.0);
            0.25 * at(n.wrapping_sub(1)) + 0.5 * at(n) + 0.25 * at(n + 1)
        })
        .collect();
    if envelope.iter().all(|v| *v == 0.0) {
        return none;
    }

    let period = |bpm: f64| 60.0 / bpm / hop_s;
    let longest = (period(SLOWEST_BPM) * BEATS_AHEAD as f64) as usize + 2;
    let whole = correlation(&envelope);
    let whole: Vec<f64> = (0..longest).map(whole).collect();
    let steps = ((FASTEST_BPM - SLOWEST_BPM) / BPM_STEP).round() as usize;
    let mut best = (f64::MIN, SLOWEST_BPM, 0.0);
    for step in 0..=steps {
        let bpm = SLOWEST_BPM + step as f64 * BPM_STEP;
        let regularity = repeats(|lag| whole[lag], period(bpm));
        let octaves = (bpm / PREFERRED_BPM).log2() / PREFERENCE_WIDTH_OCTAVES;
        let score = regularity * (-0.5 * octaves * octaves).exp();
        if score > best.0 {
            best = (score, bpm, regularity);
        }
    }
    let (_, bpm, regularity) = best;
    if regularity < TRUST_OVERALL {
        return none;
    }

    // Trust the beat at each instant whose surroundings repeat at it.
    let half = (NEARBY_S / 2.0 / hop_s).round() as usize;
    let step = ((NEARBY_STEP_S / hop_s).round() as usize).max(1);
    let mut trusted: Vec<Span> = Vec::new();
    for centre in (0..envelope.len()).step_by(step) {
        let nearby = &envelope[centre.saturating_sub(half)..(centre + half).min(envelope.len())];
        if repeats(correlation(nearby), period(bpm)) < TRUST_NEARBY {
            continue;
        }
        let start_s = (centre as f64 - step as f64 / 2.0).max(0.0) * hop_s;
        let end_s = (centre as f64 + step as f64 / 2.0).min(envelope.len() as f64) * hop_s;
        match trusted.last_mut() {
            // A beat that is lost for a moment and found again was kept all along.
            Some(last) if start_s - last.end_s <= BRIEF_S => last.end_s = end_s,
            _ => trusted.push(Span { start_s, end_s }),
        }
    }
    // A beat found only for a moment was never kept.
    trusted.retain(|span| span.end_s - span.start_s > BRIEF_S);
    if trusted.is_empty() {
        return none;
    }
    Pulse {
        bpm: Some(bpm),
        regularity,
        trusted,
    }
}
