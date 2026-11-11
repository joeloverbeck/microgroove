//! Place: analysis in; the route, the regions and the identity of the place.
//! Moment: a place and a song time in; everything needed to draw that moment.
//!
//! A moment depends only on the analysis and the song time. Nothing here remembers a
//! moment that came before: an impulse's decay is worked out from the time since its onset.
//!
//! Every constant below is tuning, set by inspecting stills of the first-journey song.
//! Lengths are in the place's own metres.

use serde::Serialize;

use crate::analysis::{Analysis, smooth};

/// Speed along the route at intensity 0 and at intensity 1, in metres per second of song.
const SPEED: (f64, f64) = (32.0, 150.0);
/// Height of the camera above the floor of the route at the same two intensities.
const ALTITUDE: (f64, f64) = (10.0, 70.0);
/// The camera rises and sinks with intensity as it is over about this many seconds.
const DRIVE_SMOOTH_S: f64 = 3.0;
/// The camera looks at where it will be this many seconds on, lowered by this share of its height.
const LOOK_AHEAD_S: f64 = 1.5;
const LOOK_DOWN: f64 = 0.35;
/// At a boundary trusted in full the route turns by this slope, and the listener spends
/// this long in the turn.
const TURN: f64 = 0.38;
const TURN_S: f64 = 4.0;
/// The camera banks into a turn: up to this angle, reaching most of it at this sideways pull.
const BANK_MAX_DEG: f64 = 26.0;
const BANK_PULL: f64 = 30.0;
/// A region takes over from the one before across this length, centred on the boundary.
const BLEND_M: f64 = 120.0;
/// Half the width of the floor the route runs along, and the height the land beside it
/// rises to, at intensity 0 and at intensity 1.
const FLOOR_M: (f64, f64) = (25.0, 200.0);
const RELIEF_M: (f64, f64) = (45.0, 650.0);
/// The land climbs from the floor to its full height across this length.
const RISE_M: f64 = 450.0;
/// The size of the largest landforms.
const LANDFORM_M: f64 = 800.0;
/// The ground is laid in rows this far apart, from this far behind the start to this far
/// beyond the end, and reaches this far to either side of the route.
pub const ROW_M: f64 = 10.0;
const BEHIND_M: f64 = 200.0;
const BEYOND_M: f64 = 7000.0;
const HALF_WIDTH_M: f64 = 4200.0;
const COLUMNS: usize = 193;
/// An impulse is in play for this long after its onset. Its ring of light starts this many
/// camera heights from its lantern, where the ground comes into view, and runs ahead at
/// this many camera heights a second.
const IMPULSE_LIFE_S: f64 = 0.45;
const RING_START: f64 = 1.7;
const RING_SPEED: f64 = 13.0;
/// An onset this strong among the song's own onsets gives an impulse at full level.
const FULL_LEVEL_PERCENTILE: usize = 90;
/// A section fuller than this makes rounded land; the thinner its sound, the sharper its land.
const SHARP_BELOW: f64 = 0.8;

/// Floats per ground vertex: position, normal, accent colour, glow.
pub const VERTEX_FLOATS: usize = 10;
/// Floats per lantern: position, level, radius.
pub const LANTERN_FLOATS: usize = 5;

/// What a song becomes: the route, the regions along it and the impulses on it.
pub struct Place {
    pub duration_s: f64,
    hop_s: f64,
    intensity: Vec<f32>,
    /// Intensity as the camera follows it.
    drive: Vec<f32>,
    /// Distance along the route, and speed, at the start of each step of the intensity curve.
    distance: Vec<f64>,
    speed: Vec<f64>,
    /// Where the route turns: the distance of the middle of the turn, the change of
    /// slope, and the length of the turn.
    turns: Vec<(f64, f64, f64)>,
    regions: Vec<Region>,
    impulses: Vec<Impulse>,
    pub sky: Sky,
}

/// The stretch of the place made from one section; its character is that section's own measurements.
struct Region {
    start_s: f64,
    start_m: f64,
    accent: [f32; 3],
    /// From rounded (0) to sharp (1) landforms.
    sharpness: f32,
}

/// The place's answer to one onset: a lantern on the route where the onset sounds, and
/// a ring of light that runs out from it once it has sounded.
struct Impulse {
    onset_s: f64,
    origin_m: [f64; 2],
    level: f64,
    /// Height of the camera as it passes.
    altitude_m: f64,
}

/// What the fingerprint gives the whole place: its palette and its weather.
pub struct Sky {
    pub zenith: [f32; 3],
    pub horizon: [f32; 3],
    pub sun: [f32; 3],
    pub haze_per_m: f32,
}

/// The ground as triangles: `rows` by `columns` vertices of `VERTEX_FLOATS` floats, row by row.
pub struct Ground {
    pub rows: usize,
    pub columns: usize,
    pub vertices: Vec<f32>,
}

/// The row of ground at or before a distance along the route.
pub fn row_at(route_m: f64) -> usize {
    ((route_m + BEHIND_M) / ROW_M).max(0.0) as usize
}

/// A place as it appears at one song time.
#[derive(Serialize)]
pub struct Moment {
    pub song_time_s: f64,
    /// Distance travelled along the route.
    pub route_position_m: f64,
    /// The region the listener is in, counted from 0.
    pub region: usize,
    pub camera: Camera,
    /// From 0 to 1: how much light the song gives the place now.
    pub light: f64,
    /// The accent colour of the land around the listener, which tints the air.
    pub accent: [f32; 3],
    /// The impulses in play, oldest first.
    pub impulses: Vec<ImpulseInPlay>,
}

#[derive(Serialize)]
pub struct Camera {
    /// Across the route, up, and along the route.
    pub position_m: [f64; 3],
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub bank_deg: f64,
}

#[derive(Serialize)]
pub struct ImpulseInPlay {
    pub onset_s: f64,
    pub age_s: f64,
    /// From 1 at a strong onset as it sounds down to 0 as the impulse dies away.
    pub level: f64,
    /// Where its lantern stands (across, along), and how far its ring has run from there.
    pub origin_m: [f64; 2],
    pub reach_m: f64,
}

/// A summary of the place, written beside each still. No pulse is in it because none
/// is shown: analysis gives its rate but not where its beats fall.
#[derive(Serialize)]
pub struct Summary {
    pub route_length_m: f64,
    pub regions: usize,
    pub impulses: usize,
}

fn mix(a: f64, b: f64, share: f64) -> f64 {
    a + (b - a) * share
}

/// 0 below 0, 1 above 1, and a smooth climb between with no corner at either end.
fn ease(share: f64) -> f64 {
    let u = share.clamp(0.0, 1.0);
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

/// A curve read at any song time without corners at its steps (Catmull-Rom through the
/// middle of each step).
fn read(values: &[f32], hop_s: f64, time_s: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let at = |i: isize| f64::from(values[i.clamp(0, values.len() as isize - 1) as usize]);
    let x = time_s / hop_s - 0.5;
    let (i, u) = (x.floor() as isize, x - x.floor());
    let (a, b, c, d) = (at(i - 1), at(i), at(i + 1), at(i + 2));
    let (p, q, r) = (
        c - a,
        2.0 * a - 5.0 * b + 4.0 * c - d,
        3.0 * (b - c) + d - a,
    );
    (b + 0.5 * u * (p + u * (q + u * r))).clamp(0.0, 1.0)
}

/// A colour from hue (in turns), saturation and value, as linear light.
fn colour(hue: f64, saturation: f64, value: f64) -> [f32; 3] {
    let channel = |offset: f64| {
        let k = (offset + hue.rem_euclid(1.0) * 6.0) % 6.0;
        (value - value * saturation * k.min(4.0 - k).clamp(0.0, 1.0)) as f32
    };
    [channel(5.0), channel(3.0), channel(1.0)]
}

/// Dark, bass-heavy sound is hot (red at brightness 0.2); bright sound is cold (cyan at 0.55).
fn hue(brightness: f64) -> f64 {
    1.02 - 1.5 * (brightness - 0.2)
}

impl Place {
    pub fn of(analysis: &Analysis) -> Place {
        let hop_s = analysis.intensity.hop_s;
        let intensity = analysis.intensity.values.clone();
        let drive = smooth(&intensity, (DRIVE_SMOOTH_S / hop_s).round() as usize);
        let fingerprint = &analysis.fingerprint;

        // A faster pulse travels faster. With no trusted pulse the pace is neutral.
        let pace = (fingerprint.tempo_bpm.unwrap_or(120.0) / 120.0).clamp(0.6, 1.6);
        let speed: Vec<f64> = (0..=intensity.len())
            .map(|step| {
                pace * mix(
                    SPEED.0,
                    SPEED.1,
                    read(&intensity, hop_s, step as f64 * hop_s),
                )
            })
            .collect();
        let mut distance = vec![0.0; speed.len()];
        for step in 1..speed.len() {
            distance[step] = distance[step - 1] + (speed[step - 1] + speed[step]) / 2.0 * hop_s;
        }

        let mut place = Place {
            duration_s: analysis.duration_s,
            hop_s,
            intensity,
            drive,
            distance,
            speed,
            turns: Vec::new(),
            regions: Vec::new(),
            impulses: Vec::new(),
            sky: Sky {
                zenith: colour(hue(fingerprint.brightness) - 0.13, 0.75, 0.1),
                horizon: colour(hue(fingerprint.brightness) + 0.08, 0.6, 0.42),
                sun: colour(hue(fingerprint.brightness) + 0.2, 0.45, 1.0),
                // A fuller sound thickens the air.
                haze_per_m: (1.0 / mix(11000.0, 5500.0, fingerprint.fullness)) as f32,
            },
        };

        // A region for each section. The route turns into each region, one way and then
        // the other, by as much as the boundary is trusted.
        let mut heading = 0.0;
        for (index, section) in analysis.sections.iter().enumerate() {
            let (start_m, speed) = place.travel(section.start_s);
            if index > 0 {
                let confidence = analysis.boundaries[index - 1].confidence;
                let side = if index % 2 == 1 { 1.0 } else { -1.0 };
                let turn = side * TURN * confidence - heading;
                place.turns.push((start_m, turn, TURN_S * speed));
                heading += turn;
            }
            place.regions.push(Region {
                start_s: section.start_s,
                start_m,
                accent: colour(
                    hue(section.brightness),
                    mix(0.55, 1.0, section.fullness),
                    1.0,
                ),
                sharpness: ((SHARP_BELOW - section.fullness) / 0.5).clamp(0.0, 1.0) as f32,
            });
        }

        let mut strengths: Vec<f32> = analysis.onsets.iter().map(|onset| onset.strength).collect();
        strengths.sort_by(f32::total_cmp);
        let full = strengths
            .get(strengths.len().saturating_sub(1) * FULL_LEVEL_PERCENTILE / 100)
            .map_or(1.0, |strength| f64::from(*strength));
        place.impulses = analysis
            .onsets
            .iter()
            .map(|onset| {
                let along = place.distance_at(onset.time_s);
                Impulse {
                    onset_s: onset.time_s,
                    origin_m: [place.bend(along).0, along],
                    level: (f64::from(onset.strength) / full).min(1.0),
                    altitude_m: place.altitude(onset.time_s),
                }
            })
            .collect();
        place
    }

    pub fn summary(&self) -> Summary {
        Summary {
            route_length_m: self.distance_at(self.duration_s),
            regions: self.regions.len(),
            impulses: self.impulses.len(),
        }
    }

    /// Distance along the route at a song time, and the speed there. Speed runs straight
    /// from each step of the intensity curve to the next, so distance has no corners.
    fn travel(&self, time_s: f64) -> (f64, f64) {
        let last = self.speed.len() - 1;
        let step = ((time_s / self.hop_s).max(0.0) as usize).min(last);
        let into = time_s - step as f64 * self.hop_s;
        if step == last || time_s < 0.0 {
            return (
                self.distance[step] + self.speed[step] * into,
                self.speed[step],
            );
        }
        let gain = (self.speed[step + 1] - self.speed[step]) / self.hop_s;
        (
            self.distance[step] + (self.speed[step] + gain * into / 2.0) * into,
            self.speed[step] + gain * into,
        )
    }

    fn distance_at(&self, time_s: f64) -> f64 {
        self.travel(time_s).0
    }

    /// The song time at which the listener is a given distance along the route.
    fn time_at(&self, route_m: f64) -> f64 {
        let last = self.distance.len() - 1;
        let step = self
            .distance
            .partition_point(|d| *d <= route_m)
            .clamp(1, last.max(1))
            - 1;
        let span = (self.distance[(step + 1).min(last)] - self.distance[step]).max(1e-9);
        if route_m < 0.0 || route_m > self.distance[last] {
            let from = if route_m < 0.0 { 0 } else { last };
            return from as f64 * self.hop_s + (route_m - self.distance[from]) / self.speed[from];
        }
        (step as f64 + (route_m - self.distance[step]) / span) * self.hop_s
    }

    /// The route's sideways offset and curvature at a distance along it.
    fn bend(&self, route_m: f64) -> (f64, f64) {
        let (mut offset, mut curvature) = (0.0, 0.0);
        for (middle, turn, length) in &self.turns {
            let u = ((route_m - middle) / length + 0.5).max(0.0);
            if u >= 1.0 {
                offset += turn * (route_m - middle);
            } else {
                // The route's slope eases in; its integral and its derivative are exact.
                let u4 = u * u * u * u;
                offset += turn * length * u4 * (u * u - 3.0 * u + 2.5);
                curvature += turn * 30.0 * u * u * (1.0 - u) * (1.0 - u) / length;
            }
        }
        (offset, curvature)
    }

    /// Height of the camera above the floor at a song time.
    fn altitude(&self, time_s: f64) -> f64 {
        let drive = read(&self.drive, self.hop_s, time_s);
        mix(ALTITUDE.0, ALTITUDE.1, drive)
    }

    /// Where the camera is at a song time.
    fn flight(&self, time_s: f64) -> [f64; 3] {
        let along = self.distance_at(time_s);
        [self.bend(along).0, self.altitude(time_s), along]
    }

    pub fn moment(&self, song_time_s: f64) -> Moment {
        let time_s = song_time_s.clamp(0.0, self.duration_s);
        let (eye, ahead) = (self.flight(time_s), self.flight(time_s + LOOK_AHEAD_S));
        let (along, speed) = self.travel(time_s);
        let toward = [
            ahead[0] - eye[0],
            ahead[1] - eye[1] - LOOK_DOWN * eye[1],
            ahead[2] - eye[2],
        ];
        let pull = speed * speed * self.bend(along).1;
        let first = self
            .impulses
            .partition_point(|i| i.onset_s <= time_s - IMPULSE_LIFE_S);
        let end = self.impulses.partition_point(|i| i.onset_s <= time_s);
        Moment {
            song_time_s: time_s,
            route_position_m: along,
            region: self.regions.partition_point(|r| r.start_s <= time_s).max(1) - 1,
            camera: Camera {
                position_m: eye,
                yaw_deg: toward[0].atan2(toward[2]).to_degrees(),
                pitch_deg: toward[1].atan2(toward[0].hypot(toward[2])).to_degrees(),
                bank_deg: BANK_MAX_DEG * (pull / BANK_PULL).tanh(),
            },
            light: read(&self.intensity, self.hop_s, time_s),
            accent: self.character(along).0,
            impulses: self.impulses[first..end]
                .iter()
                .map(|impulse| {
                    let age_s = time_s - impulse.onset_s;
                    ImpulseInPlay {
                        onset_s: impulse.onset_s,
                        age_s,
                        level: impulse.level * (1.0 - age_s / IMPULSE_LIFE_S).powi(2),
                        origin_m: impulse.origin_m,
                        reach_m: impulse.altitude_m * (RING_START + RING_SPEED * age_s),
                    }
                })
                .collect(),
        }
    }

    /// One lantern for each impulse, where its onset sounds: across, up, along, level, radius.
    pub fn lanterns(&self) -> Vec<f32> {
        let lantern = |i: &Impulse| {
            let [x, z] = i.origin_m.map(|m| m as f32);
            let radius = i.altitude_m * (0.02 + 0.06 * i.level);
            [x, swell(x, z), z, i.level as f32, radius as f32]
        };
        self.impulses.iter().flat_map(lantern).collect()
    }

    /// The accent colour and sharpness of the land at a distance along the route: each
    /// region's own, taking over from the one before across its boundary.
    fn character(&self, route_m: f64) -> ([f32; 3], f32) {
        let mut character = (self.regions[0].accent, self.regions[0].sharpness);
        for region in &self.regions[1..] {
            let share = ease((route_m - region.start_m) / BLEND_M + 0.5) as f32;
            for (channel, accent) in character.0.iter_mut().zip(region.accent) {
                *channel += (accent - *channel) * share;
            }
            character.1 += (region.sharpness - character.1) * share;
        }
        character
    }

    /// Rows the ground is laid in.
    fn rows(&self) -> usize {
        let length = self.distance_at(self.duration_s);
        ((BEHIND_M + length + BEYOND_M) / ROW_M).ceil() as usize + 1
    }

    /// The ground goes to the GPU whole, in one buffer. None where it fits in `bytes`;
    /// otherwise the song time by which the route has run as far as ground can be laid.
    pub fn outgrows(&self, bytes: u64) -> Option<f64> {
        let rows = bytes as usize / (4 * VERTEX_FLOATS * COLUMNS);
        let route_m = rows.saturating_sub(1) as f64 * ROW_M - BEHIND_M - BEYOND_M;
        (self.rows() > rows).then(|| self.time_at(route_m))
    }

    /// Builds the ground of the whole place: a floor along the route that widens with
    /// intensity, and land to either side that rises higher the harder the song pushes.
    pub fn ground(&self) -> Ground {
        let rows = self.rows();
        let mut positions = Vec::with_capacity(rows * COLUMNS);
        let mut paint = Vec::with_capacity(rows);
        for row in 0..rows {
            let along = row as f64 * ROW_M - BEHIND_M;
            let intensity = read(&self.intensity, self.hop_s, self.time_at(along));
            let (accent, sharpness) = self.character(along);
            let centre = self.bend(along).0;
            let (floor, relief) = (
                mix(FLOOR_M.0, FLOOR_M.1, intensity),
                mix(RELIEF_M.0, RELIEF_M.1, intensity),
            );
            for column in 0..COLUMNS {
                // Columns crowd toward the route, where the camera is.
                let u = column as f64 / (COLUMNS - 1) as f64 * 2.0 - 1.0;
                let across = HALF_WIDTH_M * (0.12 * u + 0.88 * u * u * u);
                let x = centre + across;
                let (px, pz) = ((x / LANDFORM_M) as f32, (along / LANDFORM_M) as f32);
                let rounded = fractal(px, pz);
                let ridged = 1.0 - (2.0 * fractal(1.7 * px + 37.0, 1.7 * pz + 11.0) - 1.0).abs();
                let shape = rounded + (ridged * ridged * ridged - rounded) * sharpness;
                let rise = ease((across.abs() - floor) / RISE_M)
                    * (0.6 + 0.4 * (across.abs() / HALF_WIDTH_M));
                let height = relief * rise * (0.2 + 0.8 * f64::from(shape))
                    + f64::from(swell(x as f32, along as f32));
                positions.push([x as f32, height as f32, along as f32]);
            }
            paint.push([accent[0], accent[1], accent[2], intensity as f32]);
        }

        let mut vertices = Vec::with_capacity(rows * COLUMNS * VERTEX_FLOATS);
        let at = |row: usize, column: usize| positions[row * COLUMNS + column];
        for (row, paint) in paint.iter().enumerate() {
            for column in 0..COLUMNS {
                // The normal stands across the two directions the ground runs in here.
                let along = sub(
                    at((row + 1).min(rows - 1), column),
                    at(row.saturating_sub(1), column),
                );
                let across = sub(
                    at(row, (column + 1).min(COLUMNS - 1)),
                    at(row, column.saturating_sub(1)),
                );
                let normal = [
                    along[1] * across[2] - along[2] * across[1],
                    along[2] * across[0] - along[0] * across[2],
                    along[0] * across[1] - along[1] * across[0],
                ];
                let length = normal.iter().map(|n| n * n).sum::<f32>().sqrt();
                vertices.extend(at(row, column));
                vertices.extend(normal.map(|n| n / length));
                vertices.extend(paint);
            }
        }
        Ground {
            rows,
            columns: COLUMNS,
            vertices,
        }
    }
}

/// The gentle rise and fall of the ground everywhere, the floor of the route included.
fn swell(x: f32, z: f32) -> f32 {
    let scale = 4.0 / LANDFORM_M as f32;
    6.0 * (fractal(x * scale, z * scale) - 0.5)
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// A fixed value from 0 to 1 for each point of a grid. Integer arithmetic only, so it is
/// the same on every run.
fn lattice(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d) ^ (y as u32).wrapping_mul(0x1656_67b1);
    h = (h ^ (h >> 15)).wrapping_mul(0x2c1b_3c6d);
    h = (h ^ (h >> 12)).wrapping_mul(0x297a_2d39);
    ((h ^ (h >> 15)) >> 8) as f32 / 16_777_216.0
}

/// Smooth noise from 0 to 1: large shapes with smaller ones on top.
fn fractal(x: f32, y: f32) -> f32 {
    let (mut total, mut weight, mut scale) = (0.0, 0.5, 1.0);
    for _ in 0..5 {
        let (px, py) = (x * scale, y * scale);
        let (ix, iy) = (px.floor() as i32, py.floor() as i32);
        let (fx, fy) = (px - px.floor(), py - py.floor());
        let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
        let top = lattice(ix, iy) + (lattice(ix + 1, iy) - lattice(ix, iy)) * ux;
        let bottom = lattice(ix, iy + 1) + (lattice(ix + 1, iy + 1) - lattice(ix, iy + 1)) * ux;
        total += weight * (top + (bottom - top) * uy);
        weight *= 0.5;
        scale *= 2.03;
    }
    total / 0.96875
}
