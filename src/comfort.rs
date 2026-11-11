//! Comfort, measured over a run of consecutive moments: full-screen flashes and camera
//! jitter. These are measuring aids, not conformance claims: the owner's comfort verdict
//! remains the judge.

use crate::render::Size;

/// A general flash, after WCAG 2.2 (success criterion 2.3.1): "a pair of opposing changes
/// in relative luminance of 10% or more of the maximum relative luminance (1.0) where the
/// relative luminance of the darker image is below 0.80".
const SWING: f32 = 0.10;
const DARK_BELOW: f32 = 0.80;
/// The picture is measured whole and in tiles, because a large part of it can flash
/// while its mean stays level.
pub const TILES: (usize, usize) = (4, 3);

/// The peaks and valleys of a series that stand at least `swing` apart, with where the
/// first move began and where the last one had got to. Smaller wobbles are passed over.
fn turning_points(series: &[f32], swing: f32) -> Vec<(usize, f32)> {
    let mut points = Vec::new();
    let Some(&first) = series.first() else {
        return points;
    };
    let (mut low, mut high, mut furthest) = ((0, first), (0, first), (0, first));
    let mut rising = None;
    for (at, &value) in series.iter().enumerate() {
        match rising {
            None => {
                if value < low.1 {
                    low = (at, value);
                }
                if value > high.1 {
                    high = (at, value);
                }
                if high.1 - low.1 >= swing {
                    let up = high.0 > low.0;
                    points.push(if up { low } else { high });
                    furthest = if up { high } else { low };
                    rising = Some(up);
                }
            }
            Some(up) if (value > furthest.1) == up && value != furthest.1 => furthest = (at, value),
            Some(up) if (value - furthest.1).abs() >= swing => {
                points.push(furthest);
                furthest = (at, value);
                rising = Some(!up);
            }
            Some(_) => {}
        }
    }
    if rising.is_some() {
        points.push(furthest);
    }
    points
}

/// The most events that fall within any stretch of `window` moments.
fn most_within(events: &[usize], window: usize) -> usize {
    let from = |first: usize| {
        events[first..]
            .iter()
            .take_while(move |at| **at < events[first] + window)
    };
    (0..events.len())
        .map(|first| from(first).count())
        .max()
        .unwrap_or(0)
}

/// The most general flashes that complete within any one second of a series of relative
/// luminance. A flash is two opposing changes; each change belongs to one flash only.
pub fn worst_flashes_in_a_second(luminance: &[f32], moments_per_second: usize) -> usize {
    let mut flashes = Vec::new();
    let mut waiting = false;
    for change in turning_points(luminance, SWING).windows(2) {
        let counts = change[0].1.min(change[1].1) < DARK_BELOW;
        if counts && waiting {
            flashes.push(change[1].0);
        }
        waiting = counts && !waiting;
    }
    most_within(&flashes, moments_per_second)
}

/// The most times within any one second that a part of the camera's pose turns back on
/// itself by at least `least`. A camera that shakes does this many times a second.
pub fn worst_reversals_in_a_second(pose: &[f32], least: f32, moments_per_second: usize) -> usize {
    let turns: Vec<usize> = turning_points(pose, least)
        .iter()
        .map(|point| point.0)
        .collect();
    // The first and last points are where the series starts and stops, not reversals.
    match turns.as_slice() {
        [_, reversals @ .., _] => most_within(reversals, moments_per_second),
        _ => 0,
    }
}

/// The widest range a series covers within any one second.
pub fn widest_range_in_a_second(series: &[f32], moments_per_second: usize) -> f32 {
    let range = |window: &[f32]| {
        let (low, high) = window
            .iter()
            .fold((f32::MAX, f32::MIN), |(low, high), value| {
                (low.min(*value), high.max(*value))
            });
        high - low
    };
    series
        .windows(moments_per_second.clamp(1, series.len().max(1)))
        .map(range)
        .fold(0.0, f32::max)
}

/// The mean relative luminance (WCAG 2.2) of a picture and of each of its tiles, from
/// its sRGB bytes: the whole picture first, then the tiles row by row. Pixels are sampled
/// on an even grid; light is averaged, not bytes.
pub fn luminance(rows: &[u8], stride: usize, size: Size) -> Vec<f32> {
    let linear: Vec<f32> = (0..256)
        .map(|byte| {
            let c = byte as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        })
        .collect();
    let (width, height) = (size.0 as usize, size.1 as usize);
    let step = (width / 640).max(1);
    let mut tiles = vec![(0.0f64, 0usize); TILES.0 * TILES.1];
    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let pixel = &rows[y * stride + x * 4..];
            let light = 0.2126 * linear[pixel[0] as usize]
                + 0.7152 * linear[pixel[1] as usize]
                + 0.0722 * linear[pixel[2] as usize];
            let tile = &mut tiles[(y * TILES.1 / height) * TILES.0 + x * TILES.0 / width];
            *tile = (tile.0 + f64::from(light), tile.1 + 1);
        }
    }
    let (total, count) = tiles
        .iter()
        .fold((0.0, 0), |sum, tile| (sum.0 + tile.0, sum.1 + tile.1));
    let mean = |(total, count): (f64, usize)| (total / count.max(1) as f64) as f32;
    std::iter::once((total, count))
        .chain(tiles)
        .map(mean)
        .collect()
}

/// The measuring aids are checked on series whose answers are known by construction,
/// because the place is built never to flash or shake: no song given on the command
/// line can make them count.
#[cfg(test)]
mod tests {
    use super::*;

    /// A square wave between two levels, at 144 moments a second for three seconds.
    fn square(low: f32, high: f32, per_second: usize) -> Vec<f32> {
        let half = 144 / (2 * per_second);
        (0..432)
            .map(|at| if (at / half) % 2 == 0 { low } else { high })
            .collect()
    }

    #[test]
    fn flashes_are_counted_as_pairs_of_opposing_changes_within_a_second() {
        assert_eq!(worst_flashes_in_a_second(&square(0.2, 0.7, 4), 144), 4);
        assert_eq!(worst_flashes_in_a_second(&square(0.2, 0.7, 2), 144), 2);
        assert_eq!(
            worst_flashes_in_a_second(&square(0.2, 0.29, 4), 144),
            0,
            "too small a change"
        );
        assert_eq!(
            worst_flashes_in_a_second(&square(0.85, 0.97, 4), 144),
            0,
            "too bright to count"
        );
        let swell: Vec<f32> = (0..432).map(|at| 0.3 + 0.2 * (at as f32 / 432.0)).collect();
        assert_eq!(
            worst_flashes_in_a_second(&swell, 144),
            0,
            "one slow rise is not a flash"
        );
    }

    #[test]
    fn a_shaking_pose_reverses_many_times_a_second_and_a_swooping_one_does_not() {
        assert_eq!(
            worst_reversals_in_a_second(&square(-1.0, 1.0, 8), 0.05, 144),
            16
        );
        assert_eq!(
            worst_reversals_in_a_second(&square(-0.01, 0.01, 8), 0.05, 144),
            0,
            "too small to see"
        );
        let swoop: Vec<f32> = (0..432).map(|at| (at as f32 / 432.0 * 4.0).sin()).collect();
        assert_eq!(
            worst_reversals_in_a_second(&swoop, 0.05, 144),
            1,
            "one slow rise and fall"
        );
    }
}
