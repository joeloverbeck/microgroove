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
/// Tuning: changes are looked for by comparing the music this long before an instant
/// with the music this long after it.
const NEAR_S: f64 = 5.0;
/// Tuning: a change found that way is measured again over up to this long either side.
const FAR_S: f64 = 10.0;
/// With less than this on a side no change is measured.
const LEAST_S: f64 = 2.0;
/// Music steadier than this many decibels is treated as varying by this much, so that
/// a small step in a very steady sound is not taken for a change of section.
const STEADIEST_DB: f64 = 1.0;
/// Tuning: the size at which a change is trusted. Set in the gap between the
/// first-journey song's changes that last (9.4 and up) and those that do not (up to 6.9).
pub const TRUST: f32 = 8.0;
/// Tuning: a change is the largest within this many seconds either side.
const SEPARATION_S: f64 = 2.0;

/// The level of each broad band over any stretch of the song.
struct Levels {
    /// Running sums of the power in each broad band.
    running: Vec<[f64; GROUPS]>,
    /// Slow steps in a piece of `SETTLE_S`.
    piece: usize,
}

impl Levels {
    fn of(levels: &[[f32; BANDS]], hop_s: f64) -> Levels {
        let mut running = vec![[0.0f64; GROUPS]; levels.len() + 1];
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
        Levels {
            running,
            piece: ((SETTLE_S / hop_s).round() as usize).max(1),
        }
    }

    /// How different the music in the `before` pieces up to step `i` is from the music
    /// in the `after` pieces from it. The band levels of each side are summed up as an
    /// average and a spread; the difference is how far apart those two summaries are
    /// (the symmetric Kullback-Leibler divergence of two bell curves, averaged over the
    /// bands). It is 0 where the two sides are alike, and it counts a shift in level by
    /// how large it is against how much the music varies anyway. The pieces are counted
    /// away from the instant, so that none mixes the music before it with the music after.
    fn difference(&self, i: usize, before: usize, after: usize) -> f64 {
        let least = (LEAST_S / SETTLE_S).round() as usize;
        if before < least || after < least {
            return 0.0;
        }
        let piece = self.piece;
        let summary = |starts: &mut dyn Iterator<Item = usize>, group: usize| -> (f64, f64) {
            let (mut sum, mut squares, mut pieces) = (0.0, 0.0, 0.0);
            for start in starts {
                let power = self.running[start + piece][group] - self.running[start][group];
                let level = 10.0 * (power / piece as f64).log10();
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
        let total: f64 = (0..GROUPS)
            .map(|group| {
                let (earlier, earlier_spread) =
                    summary(&mut (1..=before).map(|k| i - k * piece), group);
                let (later, later_spread) = summary(&mut (0..after).map(|k| i + k * piece), group);
                let shift = (later - earlier).powi(2);
                0.5 * ((later_spread + shift) / earlier_spread
                    + (earlier_spread + shift) / later_spread)
                    - 1.0
            })
            .sum();
        total / GROUPS as f64
    }
}

/// How much the music changes at the start of each slow step, heard close up, and the
/// boundaries trusted among those changes.
///
/// A change is trusted when it lasts. The candidates are the instants where the music
/// `NEAR_S` either side differs most. Each is measured a second time over up to `FAR_S`
/// either side, and its size is the geometric mean of the two measurements: a moment's
/// gap or hit is large close up and small from further off, and a change that lasts is
/// large both ways. Candidates are judged from the largest down, and the second
/// measurement never reaches past a boundary already trusted, where it would be
/// measuring that boundary's change instead.
pub fn hear(levels: &[[f32; BANDS]], hop_s: f64) -> (Vec<f32>, Vec<Boundary>) {
    let count = levels.len();
    let levels = Levels::of(levels, hop_s);
    let pieces = |steps: usize, reach_s: f64| {
        (steps / levels.piece).min((reach_s / SETTLE_S).round() as usize)
    };
    let change: Vec<f32> = (0..count)
        .map(|i| levels.difference(i, pieces(i, NEAR_S), pieces(count - i, NEAR_S)) as f32)
        .collect();

    let reach = (SEPARATION_S / hop_s).round() as usize;
    let mut candidates: Vec<usize> = (0..count)
        .filter(|&i| {
            let here = change[i];
            here > 0.0
                && change[i.saturating_sub(reach)..i].iter().all(|c| *c < here)
                && change[i + 1..(i + reach + 1).min(count)]
                    .iter()
                    .all(|c| *c <= here)
        })
        .collect();
    candidates.sort_by(|a, b| change[*b].total_cmp(&change[*a]).then(a.cmp(b)));

    let mut trusted: Vec<(usize, f64)> = Vec::new();
    for i in candidates {
        let earlier = trusted.iter().map(|(at, _)| *at).filter(|at| *at < i).max();
        let later = trusted.iter().map(|(at, _)| *at).filter(|at| *at > i).min();
        let far = levels.difference(
            i,
            pieces(i - earlier.unwrap_or(0), FAR_S),
            pieces(later.unwrap_or(count) - i, FAR_S),
        );
        let size = (f64::from(change[i]) * far).sqrt();
        if size >= f64::from(TRUST) {
            trusted.push((i, size));
        }
    }
    trusted.sort_by_key(|(at, _)| *at);
    let boundaries = trusted
        .into_iter()
        .map(|(at, size)| Boundary {
            time_s: at as f64 * hop_s,
            confidence: (size / (2.0 * f64::from(TRUST))).min(1.0),
        })
        .collect();
    (change, boundaries)
}
