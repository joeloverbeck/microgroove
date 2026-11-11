//! Section boundaries: where one stretch of similar music gives way to another.

use serde::Serialize;

use super::spectrum::BANDS;

/// A change of section that analysis trusts. Changes it is unsure of are not listed:
/// no boundary is a valid outcome and nothing is guessed in its place.
#[derive(Serialize)]
pub struct Boundary {
    pub time_s: f64,
    /// From 0.5 (just trusted) to 1 (twice the change needed for trust, or more).
    pub confidence: f64,
}

/// Neighbouring bands are merged in threes, so the music is compared in eight broad bands.
const MERGED: usize = 3;
const GROUPS: usize = BANDS / MERGED;
/// Tuning: levels are taken over pieces this long, so that single beats do not count as change.
const SETTLE_S: f64 = 0.5;
/// Tuning: the music this long before an instant is compared with the music this long after it.
const REACH_S: f64 = 5.0;
/// Near the ends of the song less is available; with less than this on a side no change is measured.
const LEAST_S: f64 = 2.0;
/// Music steadier than this many decibels is treated as varying by this much, so that
/// a small step in a very steady sound is not taken for a change of section.
const STEADIEST_DB: f64 = 1.0;
/// Tuning: the change at which a boundary is trusted. Set in the widest gap among the
/// first-journey song's changes: its weaker ones reach about 9.3 and its clear ones
/// start at about 12.
pub const TRUST: f32 = 10.5;
/// Tuning: a boundary is the largest change within this many seconds either side.
const SEPARATION_S: f64 = 2.0;

/// How much the music changes at the start of each slow step. The band levels over
/// `REACH_S` before the step and after it are each summed up as an average and a
/// spread; the change is how far apart those two summaries are (the symmetric
/// Kullback-Leibler divergence of two bell curves, averaged over the bands). It is 0
/// where the two sides are alike, and it counts a shift in level by how large it is
/// against how much the music varies anyway.
pub fn change(levels: &[[f32; BANDS]], hop_s: f64) -> Vec<f32> {
    let count = levels.len();
    // Running sums of the power in each broad band give the level of any stretch.
    let mut running = vec![[0.0f64; GROUPS]; count + 1];
    for (i, bands) in levels.iter().enumerate() {
        for group in 0..GROUPS {
            let merged = &bands[group * MERGED..(group + 1) * MERGED];
            let power: f64 = merged
                .iter()
                .map(|db| 10f64.powf(f64::from(*db) / 10.0))
                .sum();
            running[i + 1][group] = running[i][group] + power;
        }
    }
    // Each side is taken in pieces of `SETTLE_S`, counted away from the instant, so
    // that no piece mixes the music before it with the music after it.
    let piece = ((SETTLE_S / hop_s).round() as usize).max(1);
    let (most, least) = (
        (REACH_S / SETTLE_S).round() as usize,
        (LEAST_S / SETTLE_S).round() as usize,
    );
    let summary = |starts: &mut dyn Iterator<Item = usize>, group: usize| -> (f64, f64) {
        let (mut sum, mut squares, mut pieces) = (0.0, 0.0, 0.0);
        for start in starts {
            let level = 10.0
                * ((running[start + piece][group] - running[start][group]) / piece as f64).log10();
            sum += level;
            squares += level * level;
            pieces += 1.0;
        }
        let average = sum / pieces;
        (
            average,
            (squares / pieces - average * average).max(0.0) + STEADIEST_DB * STEADIEST_DB,
        )
    };

    (0..count)
        .map(|i| {
            let (before, after) = ((i / piece).min(most), ((count - i) / piece).min(most));
            if before < least || after < least {
                return 0.0;
            }
            let total: f64 = (0..GROUPS)
                .map(|group| {
                    let (earlier, earlier_spread) =
                        summary(&mut (1..=before).map(|k| i - k * piece), group);
                    let (later, later_spread) =
                        summary(&mut (0..after).map(|k| i + k * piece), group);
                    let shift = (later - earlier).powi(2);
                    0.5 * ((later_spread + shift) / earlier_spread
                        + (earlier_spread + shift) / later_spread)
                        - 1.0
                })
                .sum();
            (total / GROUPS as f64) as f32
        })
        .collect()
}

pub fn boundaries(change: &[f32], hop_s: f64) -> Vec<Boundary> {
    let reach = (SEPARATION_S / hop_s).round() as usize;
    let mut boundaries = Vec::new();
    for (i, here) in change.iter().enumerate() {
        let earlier = &change[i.saturating_sub(reach)..i];
        let later = &change[i + 1..(i + reach + 1).min(change.len())];
        if *here >= TRUST && earlier.iter().all(|c| c < here) && later.iter().all(|c| c <= here) {
            let confidence = f64::from(here / (2.0 * TRUST)).min(1.0);
            boundaries.push(Boundary {
                time_s: i as f64 * hop_s,
                confidence,
            });
        }
    }
    boundaries
}
