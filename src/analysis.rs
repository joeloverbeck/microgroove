//! Analysis: samples in; what the song is heard to do, as numbers.
//!
//! Deterministic: the same samples always give the identical analysis.

mod rhythm;
mod sections;
mod spectrum;

use ebur128::{EbuR128, Mode};
use serde::Serialize;

use crate::decode::Song;
pub use rhythm::{Onset, Part, Pulse};
pub use sections::{Boundary, TRUST as BOUNDARY_TRUST};
pub use spectrum::BANDS;
use spectrum::Spectrum;

#[derive(Serialize)]
pub struct Analysis {
    /// How long the song is. Written among the numbers of the file, not here.
    #[serde(skip)]
    pub duration_s: f64,
    pub fingerprint: Fingerprint,
    pub loudness: Loudness,
    /// How hard the song is pushing, from 0 to 1, over song time.
    pub intensity: Curve,
    pub onsets: Vec<Onset>,
    pub pulse: Pulse,
    /// The section boundaries analysis trusts, in song order.
    pub boundaries: Vec<Boundary>,
    /// How much the music changes at each instant, heard close up: its peaks are the
    /// changes among which the boundaries are trusted.
    pub change: Curve,
    /// The stretches between the boundaries, covering the whole song.
    pub sections: Vec<Section>,
    pub stereo: Stereo,
    /// The level in decibels of each band, low to high, at each step of the slow
    /// curves. Drawn on the timeline; too bulky to be worth writing out as numbers.
    #[serde(skip)]
    pub band_levels: Vec<[f32; BANDS]>,
    /// The same for the left channel alone and for the right alone.
    #[serde(skip)]
    pub side_levels: [Vec<[f32; BANDS]>; 2],
}

/// The measurements of the whole song that give its place an identity.
#[derive(Serialize)]
pub struct Fingerprint {
    /// The rate of the pulse, where the song has one.
    pub tempo_bpm: Option<f64>,
    /// How well the song repeats at its beat, from 0 to 1; 0 where it has no pulse.
    pub rhythmic_regularity: f64,
    pub onsets_per_second: f64,
    /// Where the sound sits between the lowest band (0) and the highest (1).
    pub brightness: f64,
    /// From near 0 (the sound sits in a few bands) to 1 (it is spread evenly over all of them).
    pub fullness: f64,
    /// EBU R128 loudness range: how far the loud passages stand above the quiet ones.
    pub dynamic_range_lu: f64,
    /// 0 for identical channels, 0.5 for unrelated ones, 1 for opposed ones.
    pub stereo_width: f64,
}

/// A stretch of song time.
#[derive(Serialize)]
pub struct Span {
    pub start_s: f64,
    pub end_s: f64,
}

/// A value over song time: value `i` describes the stretch starting at `i * hop_s`.
#[derive(Serialize)]
pub struct Curve {
    pub hop_s: f64,
    pub values: Vec<f32>,
}

/// A span of the song heard as one stretch of similar music, with its own measurements.
#[derive(Serialize)]
pub struct Section {
    pub start_s: f64,
    pub end_s: f64,
    /// Average intensity over the section.
    pub intensity: f64,
    pub onsets_per_second: f64,
    /// As in the fingerprint, for this section alone.
    pub brightness: f64,
    pub fullness: f64,
}

/// EBU R128 measurements of the whole song.
#[derive(Serialize)]
pub struct Loudness {
    pub integrated_lufs: Option<f64>,
    pub range_lu: f64,
    pub sample_peak_dbfs: Option<f64>,
}

/// What differs between the left and right channels.
#[derive(Serialize)]
pub struct Stereo {
    /// Seconds covered by each value of `left` and `right`; value `i` starts at `i * hop_s`.
    pub hop_s: f64,
    /// Level (root mean square of the samples) of each channel over song time.
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    /// Whole song: -1 is all left, 0 is even, +1 is all right.
    pub balance: f64,
    /// Whole song: 0 is identical channels, 0.5 is one side only or unrelated sides, 1 is opposed channels.
    pub width: f64,
}

pub fn analyse(song: &Song) -> Analysis {
    let mid: Vec<f32> = song
        .left()
        .iter()
        .zip(song.right())
        .map(|(l, r)| (l + r) / 2.0)
        .collect();
    let spectrum = Spectrum::of(&mid, song.sample_rate);
    let sides = [song.left(), song.right()].map(|side| Spectrum::of(side, song.sample_rate));
    let (onsets, pulse) = rhythm::onsets_and_pulse(&spectrum, &sides);
    let stereo = stereo(song);
    let hop_s = stereo.hop_s;

    let (loudness, momentary) = loudness(song, hop_s);
    let intensity = intensity(&momentary, hop_s);
    let levels = |spectrum: &Spectrum| band_levels(spectrum, hop_s, stereo.left.len());
    let (band_levels, side_levels) = (levels(&spectrum), sides.each_ref().map(levels));
    let (change, boundaries) = sections::hear(&band_levels, hop_s);

    let onset_rate = |start_s: f64, end_s: f64| {
        let within = onsets
            .iter()
            .filter(|onset| (start_s..end_s).contains(&onset.time_s))
            .count();
        within as f64 / (end_s - start_s)
    };
    let starts = std::iter::once(0.0).chain(boundaries.iter().map(|boundary| boundary.time_s));
    let ends = boundaries
        .iter()
        .map(|boundary| boundary.time_s)
        .chain(std::iter::once(song.duration_s()));
    let sections = starts
        .zip(ends)
        .map(|(start_s, end_s)| {
            let steps = (start_s / hop_s).round() as usize
                ..((end_s / hop_s).round() as usize).min(intensity.len());
            let (brightness, fullness) = shape(&band_levels[steps.clone()]);
            let total: f64 = intensity[steps.clone()]
                .iter()
                .map(|value| f64::from(*value))
                .sum();
            Section {
                start_s,
                end_s,
                intensity: total / steps.len().max(1) as f64,
                onsets_per_second: onset_rate(start_s, end_s),
                brightness,
                fullness,
            }
        })
        .collect();

    let (brightness, fullness) = shape(&band_levels);
    Analysis {
        duration_s: song.duration_s(),
        fingerprint: Fingerprint {
            tempo_bpm: pulse.bpm,
            rhythmic_regularity: pulse.regularity,
            onsets_per_second: onset_rate(0.0, song.duration_s()),
            brightness,
            fullness,
            dynamic_range_lu: loudness.range_lu,
            stereo_width: stereo.width,
        },
        loudness,
        intensity: Curve {
            hop_s,
            values: intensity,
        },
        onsets,
        pulse,
        boundaries,
        change: Curve {
            hop_s,
            values: change,
        },
        sections,
        stereo,
        band_levels,
        side_levels,
    }
}

/// Brightness and fullness of a stretch, from the power it holds in each band.
fn shape(levels: &[[f32; BANDS]]) -> (f64, f64) {
    let mut power = [0.0f64; BANDS];
    for step in levels {
        for band in 0..BANDS {
            power[band] += 10f64.powf(f64::from(step[band]) / 10.0);
        }
    }
    let total: f64 = power.iter().sum();
    if total <= 0.0 {
        return (0.0, 0.0);
    }
    let centre: f64 = power
        .iter()
        .enumerate()
        .map(|(band, p)| band as f64 * p)
        .sum::<f64>()
        / total;
    let evenness =
        (power.iter().map(|p| p.ln()).sum::<f64>() / BANDS as f64).exp() / (total / BANDS as f64);
    (centre / (BANDS - 1) as f64, evenness)
}

/// Intensity is the song's momentary loudness placed within the song's own range:
/// 0 at its quiet passages and 1 at its loud ones. These are the two percentiles EBU
/// R128 uses for loudness range. Loudness is the only ingredient so far.
const QUIET_PERCENTILE: usize = 10;
const LOUD_PERCENTILE: usize = 95;
/// A song that hardly varies is not stretched to fill the scale: its range counts as at least this.
const LEAST_RANGE_LU: f32 = 6.0;
/// Below this the song is silent and takes no part in finding its range (the R128 absolute gate).
const SILENT_LUFS: f32 = -70.0;
/// Tuning: intensity follows the song over about this many seconds, not beat by beat.
const INTENSITY_SMOOTH_S: f64 = 1.5;

/// How hard the song is pushing at each slow step, from 0 to 1.
fn intensity(momentary: &[f32], hop_s: f64) -> Vec<f32> {
    let mut sounding: Vec<f32> = momentary
        .iter()
        .copied()
        .filter(|lufs| *lufs > SILENT_LUFS)
        .collect();
    sounding.sort_by(f32::total_cmp);
    let of = |share: usize| percentile(&sounding, share);
    let (Some(loud), Some(quiet)) = (of(LOUD_PERCENTILE), of(QUIET_PERCENTILE)) else {
        return vec![0.0; momentary.len()];
    };
    let quiet = quiet.min(loud - LEAST_RANGE_LU);
    let raw: Vec<f32> = momentary
        .iter()
        .map(|lufs| ((lufs - quiet) / (loud - quiet)).clamp(0.0, 1.0))
        .collect();
    smooth(&raw, (INTENSITY_SMOOTH_S / hop_s).round() as usize)
}

/// The value so many hundredths of the way up a sorted list; none in an empty one.
pub(crate) fn percentile<T: Copy>(sorted: &[T], of: usize) -> Option<T> {
    sorted
        .get(sorted.len().saturating_sub(1) * of / 100)
        .copied()
}

/// Whether a value is the largest within `reach` steps either side: of equals, the first.
fn is_peak(values: &[f32], at: usize, reach: usize) -> bool {
    let earlier = &values[at.saturating_sub(reach)..at];
    let later = &values[at + 1..(at + reach + 1).min(values.len())];
    earlier.iter().all(|v| *v < values[at]) && later.iter().all(|v| *v <= values[at])
}

/// Each value averaged with its neighbours up to `reach` steps away, nearer ones counting more.
pub(crate) fn smooth(values: &[f32], reach: usize) -> Vec<f32> {
    (0..values.len())
        .map(|i| {
            let first = i.saturating_sub(reach);
            let nearby = &values[first..(i + reach + 1).min(values.len())];
            let weight = |j: usize| (reach + 1 - i.abs_diff(first + j)) as f32;
            let total: f32 = nearby
                .iter()
                .enumerate()
                .map(|(j, value)| weight(j) * value)
                .sum();
            total / (0..nearby.len()).map(weight).sum::<f32>()
        })
        .collect()
}

/// Seconds per value of the slow curves (levels, change, intensity).
const SLOW_HOP_S: f64 = 0.1;
/// Band amplitudes below this all count as silence, which is this level in decibels.
const SILENCE: f32 = 1e-4;
pub const SILENT_DB: f32 = -80.0;

/// The level of each band in decibels, averaged over each slow step.
fn band_levels(spectrum: &Spectrum, hop_s: f64, steps: usize) -> Vec<[f32; BANDS]> {
    let per_step = hop_s / spectrum.hop_s;
    let mut levels = vec![[20.0 * SILENCE.log10(); BANDS]; steps];
    for step in 0..steps {
        let first = (step as f64 * per_step).round() as usize;
        let end = (((step + 1) as f64 * per_step).round() as usize).min(spectrum.frames());
        if first >= end {
            // The last frames of the file hold less than a whole window: carry the level on.
            levels[step] = levels[step.saturating_sub(1)];
            continue;
        }
        for (band, level) in levels[step].iter_mut().enumerate() {
            let power: f32 = (first..end).map(|n| spectrum.frame(n)[band].powi(2)).sum();
            *level = 10.0 * (power / (end - first) as f32 + SILENCE * SILENCE).log10();
        }
    }
    levels
}

fn stereo(song: &Song) -> Stereo {
    let block = ((f64::from(song.sample_rate) * SLOW_HOP_S).round() as usize).max(1);
    let level = |channel: &[f32]| -> Vec<f32> {
        let rms = |block: &[f32]| {
            (block.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / block.len() as f64).sqrt()
                as f32
        };
        channel.chunks(block).map(rms).collect()
    };
    let energy = |of: fn(f64, f64) -> f64| {
        song.left()
            .iter()
            .zip(song.right())
            .map(|(l, r)| of(f64::from(*l), f64::from(*r)).powi(2))
            .sum::<f64>()
    };
    let (left, right) = (energy(|l, _| l), energy(|_, r| r));
    let (mid, side) = (energy(|l, r| (l + r) / 2.0), energy(|l, r| (l - r) / 2.0));
    let ratio = |part: f64, whole: f64| if whole > 0.0 { part / whole } else { 0.0 };
    Stereo {
        hop_s: block as f64 / f64::from(song.sample_rate),
        left: level(song.left()),
        right: level(song.right()),
        balance: ratio(right - left, right + left),
        width: ratio(side, mid + side),
    }
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

/// The whole song's EBU R128 measurements, and its momentary loudness (LUFS over
/// 0.4 s) around each slow step; silence reads as negative infinity. A mono file is
/// measured as its one channel, as ffmpeg measures it.
fn loudness(song: &Song, hop_s: f64) -> (Loudness, Vec<f32>) {
    let block = (hop_s * f64::from(song.sample_rate)).round() as usize;
    let channels = song.channels.len() as u32;
    let mode = Mode::I | Mode::LRA | Mode::SAMPLE_PEAK;
    let mut meter = EbuR128::new(channels, song.sample_rate, mode)
        .expect("decode accepts only channel counts and sample rates the meter takes");
    let mut momentary = Vec::new();
    for from in (0..song.frames()).step_by(block) {
        let step = from..(from + block).min(song.frames());
        let heard: Vec<&[f32]> = song.channels.iter().map(|c| &c[step.clone()]).collect();
        meter.add_frames_planar_f32(&heard).expect("equal channels");
        momentary.push(meter.loudness_momentary().unwrap_or(f64::NEG_INFINITY) as f32);
    }
    // A reading covers the 0.4 s before it, so the one taken a step later is centred here.
    momentary.rotate_left(1);
    if let [.., last, carried] = momentary.as_mut_slice() {
        *carried = *last;
    }
    let peak = (0..channels)
        .map(|channel| meter.sample_peak(channel).unwrap_or(0.0))
        .fold(0.0, f64::max);
    let loudness = Loudness {
        integrated_lufs: meter.loudness_global().ok().and_then(finite),
        range_lu: meter.loudness_range().unwrap_or(0.0),
        sample_peak_dbfs: finite(20.0 * peak.log10()),
    };
    (loudness, momentary)
}
