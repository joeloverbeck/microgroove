//! Place: analysis in; the route, the regions and the identity of the place.
//! Moment: a place and a song time in; everything needed to draw that moment.
//!
//! A moment depends only on the analysis and the song time. Nothing here remembers a
//! moment that came before: an impulse's decay is worked out from the time since its onset.
//!
//! Every constant below is tuning, set by inspecting stills of the first-journey song.
//! Lengths are in the place's own metres.

use serde::Serialize;

use crate::analysis::{Analysis, BANDS, Part, Section, smooth};

/// Speed along the route at intensity 0 and at intensity 1, in metres per second of song.
/// Between them each equal step of intensity multiplies it alike.
const SPEED: (f64, f64) = (44.0, 300.0);
/// Height of the camera above the floor of the route at the same two intensities. It
/// rises far less than speed does, so that the ground passes faster where the song pushes.
const ALTITUDE: (f64, f64) = (10.0, 14.0);
/// The camera rises and sinks with intensity as it is over about this many seconds.
const DRIVE_SMOOTH_S: f64 = 3.0;
/// Each step of the dial brings the camera this much nearer the ground, so that the
/// ground passes that much faster. The dial goes this many steps either way.
const DIAL_STEP: f64 = 1.1;
pub const DIAL_MOST: i32 = 8;
/// The camera looks at where it will be this many seconds on, lowered by this share of
/// its height, and along the heading the route has half that long on.
const LOOK_AHEAD_S: f64 = 1.5;
const LOOK_DOWN: f64 = 0.35;
/// At a boundary trusted in full the route turns by this slope, over this long a stretch
/// of the song, whatever the speed does meanwhile.
const TURN: f64 = 0.38;
const TURN_S: f64 = 4.0;
/// The camera leans as its view turns: never as far as this angle, and most of the way
/// to it when the view turns this many degrees a second.
const BANK_MAX_DEG: f64 = 18.0;
const BANK_TURNING_DEG_S: f64 = 16.0;
/// A region takes over from the one before across this length, centred on the boundary.
const BLEND_M: f64 = 120.0;
/// Half the width of the floor the route runs along, at intensity 0 and at intensity 1:
/// it widens as speed does but less.
const FLOOR_M: (f64, f64) = (25.0, 80.0);
/// The land beside the route is the song's spectrum: low sounds stand near the route and
/// high sounds far out, on the left as the left channel has them and on the right as the
/// right. A band stands between 0 and 1 by where its level lies in its own range in the
/// song, from its 10th to its 99th percentile. That range counts as this many decibels at
/// least and at most, and a band that stays this far under the song's loudest band is
/// measured against that band instead, so that what is hardly there does not stand tall.
const BAND_RANGE_DB: (f32, f32) = (6.0, 36.0);
const BAND_UNDER_DB: f32 = 24.0;
/// Along the route a band is smoothed over this many times its own width across the
/// land, and over this many rows at least, so that what it does is about as long as it
/// is wide: a hill, not a rib across the land or a strip along it. What it does over
/// this many times that length is its standing, which counts for this much of its
/// height; what it does apart from its standing counts this many times over.
const ALONG: (f64, usize) = (1.5, 4);
const STANDING: (usize, f32) = (6, 0.7);
const OWN: f32 = 3.0;
/// What a channel has that the other has not counts this many times over, so that the
/// two sides of a song mixed nearly alike are not mirror images.
const APART: f32 = 3.0;
/// A band at full strength stands this tall at the far edge of the land, and nearer the
/// route by this power of the distance. Strength counts by a power between these two:
/// the first where the region is rounded, the second where it is sharp.
const RELIEF_M: f64 = 1500.0;
const SWEEP: f64 = 0.75;
const CREST: (f64, f64) = (1.5, 3.0);
/// The size of the swell of the floor.
const LANDFORM_M: f64 = 800.0;
/// Fixed noise is left only as grain: it makes the land this much taller or lower, in
/// shapes this size and smaller.
const GRAIN: f64 = 0.35;
const GRAIN_M: f64 = 150.0;
/// The ground is laid in rows this far apart, from this far behind the start to this far
/// beyond the end, and reaches this far to either side of the route.
pub const ROW_M: f64 = 10.0;
const BEHIND_M: f64 = 200.0;
const BEYOND_M: f64 = 7000.0;
const HALF_WIDTH_M: f64 = 4200.0;
const COLUMNS: usize = 193;
/// An onset that stands out is met by a form, which is in play for this long after its
/// onset: its light runs up the land beside the route from the floor's edge, at this many
/// floor half-widths a second. A form is in view from this far ahead.
const IMPULSE_LIFE_S: f64 = 0.45;
const LIGHT_RUNS: f64 = 10.0;
const SIGHT_M: f64 = 3000.0;
/// An onset this many decibels louder in one channel stands on that side alone.
const LEAN_DB: f32 = 3.0;
/// The forms: a rib across the floor for a low onset, standing stones beside the route
/// for a middle one, thin masts for a high one. For each, in floor half-widths: where it
/// stands from the middle of the route, half its width across the route, half its depth
/// along it, and its height at level 0 and at level 1. The nearer the route a form
/// stands, the later it leaves the picture before the listener passes it. Its foot is
/// sunk this many metres into the ground, which is nowhere quite level.
const FORMS: [(f64, f64, f64, (f64, f64)); 3] = [
    (0.5, 0.5, 0.015, (0.012, 0.022)),
    (0.75, 0.06, 0.06, (0.5, 1.0)),
    (0.35, 0.012, 0.012, (1.6, 3.0)),
];
const SUNK_M: f64 = 3.0;
/// An onset this strong among the song's own onsets gives an impulse at full level.
const FULL_LEVEL_PERCENTILE: usize = 90;
/// A section fuller than this makes rounded land; the thinner its sound, the sharper its land.
const SHARP_BELOW: f64 = 0.8;

/// Floats per ground vertex: position, normal, accent colour, glow, and how far past
/// the floor's edge it lies, in metres: below zero on the left, zero on the floor.
pub const VERTEX_FLOATS: usize = 11;
/// Floats per lantern: position, level, radius.
pub const LANTERN_FLOATS: usize = 5;
/// Floats per box of a form: see `Place::forms`.
pub const FORM_FLOATS: usize = 6;

/// What a song becomes: the route, the regions along it and the impulses on it.
pub struct Place {
    pub duration_s: f64,
    hop_s: f64,
    intensity: Vec<f32>,
    /// Intensity as the camera follows it.
    drive: Vec<f32>,
    /// A faster pulse travels faster: every speed is multiplied by this.
    pace: f64,
    /// Where the route turns: the song time of the middle of the turn, and the change of slope.
    turns: Vec<(f64, f64)>,
    /// Distance along the route, and the route's sideways offset, at the start of each
    /// step of the intensity curve.
    distance: Vec<f64>,
    across: Vec<f64>,
    regions: Vec<Region>,
    /// How strongly each band stands on each row of ground, from 0 to 1: the left
    /// channel's bands and the right's, low to high.
    land: Vec<[[f32; BANDS]; 2]>,
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

/// The place's answer to one onset, standing where the route is as the onset sounds: a
/// form where the onset stands out, and a small lantern on the route where it does not.
struct Impulse {
    onset_s: f64,
    origin_m: [f64; 2],
    level: f64,
    /// Height of the camera as it passes, and half the width of the floor there.
    altitude_m: f64,
    floor_m: f64,
    /// The kind of form, by the part of the sound that jumped, and the side it stands on.
    form: Option<(Part, Side)>,
}

/// Where a form stands: on the side of the channel its onset is louder in, or on both.
#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Centre,
    Right,
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
    /// Speed along the route, in metres per second of song.
    pub speed_m_s: f64,
    /// How fast what is near passes the camera, in times a second: speed over its height
    /// above the ground under it, and over the distance to where the land beside begins.
    pub over_ground_per_s: f64,
    pub past_land_per_s: f64,
    /// How many times nearer the ground the dial has brought the camera, and with it
    /// the lanterns and rings, which are sized to its height: 1 with the dial at rest.
    pub nearer: f64,
    /// The accent colour of the land around the listener, which tints the air.
    pub accent: [f32; 3],
    pub land: Profile,
    /// The impulses in play, oldest first: the forms whose onsets have just sounded.
    pub impulses: Vec<ImpulseInPlay>,
    /// The forms in view, as the listener meets them: those in play, then those ahead.
    pub forms: Vec<FormInView>,
}

#[derive(Serialize)]
pub struct Camera {
    /// Across the route, up, and along the route.
    pub position_m: [f64; 3],
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub bank_deg: f64,
}

/// The land across the route at the listener's position: its height above the floor on
/// the left and on the right, at each distance from the middle of the route where a band
/// of the sound stands. Neither the floor's swell nor the land's grain is in it.
#[derive(Serialize)]
pub struct Profile {
    pub across_m: Vec<f64>,
    pub left_m: Vec<f64>,
    pub right_m: Vec<f64>,
}

#[derive(Serialize)]
pub struct ImpulseInPlay {
    pub onset_s: f64,
    pub age_s: f64,
    /// From 1 at a strong onset as it sounds down to 0 as the impulse dies away.
    pub level: f64,
    /// How far past the floor's edge its light has run up the land, and on which side.
    pub reach_m: f64,
    pub side: Side,
}

#[derive(Serialize)]
pub struct FormInView {
    /// The song time of its onset, and where on the route it stands.
    pub onset_s: f64,
    pub route_m: f64,
    pub kind: Part,
    pub side: Side,
}

/// A summary of the place, written beside each still. No pulse is in it because none
/// is shown: analysis gives its rate but not where its beats fall.
#[derive(Serialize)]
pub struct Summary {
    pub route_length_m: f64,
    pub regions: usize,
    pub impulses: usize,
    pub forms: usize,
}

fn mix(a: f64, b: f64, share: f64) -> f64 {
    a + (b - a) * share
}

/// From the first of two to the second, each equal step of the share multiplying alike.
fn scale((from, to): (f64, f64), share: f64) -> f64 {
    from * (to / from).powf(share)
}

/// What a rate comes to between two song times, taken at four points: exact for
/// anything that runs as smoothly as a turn does within one step of the curves.
fn sum(from_s: f64, to_s: f64, rate: impl Fn(f64) -> f64) -> f64 {
    let half = (to_s - from_s) / 2.0;
    let at = |point: f64| rate(from_s + half * (1.0 + point));
    let (inner, outer) = (0.339_981_043_584_856, 0.861_136_311_594_053);
    half * (0.652_145_154_862_546 * (at(-inner) + at(inner))
        + 0.347_854_845_137_454 * (at(-outer) + at(outer)))
}

/// 0 below 0, 1 above 1, and a smooth climb between with no corner at either end.
fn ease(share: f64) -> f64 {
    let u = share.clamp(0.0, 1.0);
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

/// A curve from 0 to 1 read at any song time without corners at its steps.
fn read(values: &[f32], hop_s: f64, time_s: f64) -> f64 {
    through(values, hop_s, time_s).clamp(0.0, 1.0)
}

/// Any curve read between its steps without corners (Catmull-Rom through the middle of
/// each step), which may take it a little past the values either side.
fn through(values: &[f32], hop_s: f64, time_s: f64) -> f64 {
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
    b + 0.5 * u * (p + u * (q + u * r))
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

        let mut place = Place {
            duration_s: analysis.duration_s,
            hop_s,
            intensity,
            drive,
            // With no trusted pulse the pace is neutral.
            pace: (fingerprint.tempo_bpm.unwrap_or(120.0) / 120.0).clamp(0.6, 1.6),
            turns: Vec::new(),
            distance: vec![0.0],
            across: vec![0.0],
            regions: Vec::new(),
            land: Vec::new(),
            impulses: Vec::new(),
            sky: Sky {
                zenith: colour(hue(fingerprint.brightness) - 0.13, 0.75, 0.1),
                horizon: colour(hue(fingerprint.brightness) + 0.08, 0.6, 0.42),
                sun: colour(hue(fingerprint.brightness) + 0.2, 0.45, 1.0),
                // A fuller sound thickens the air.
                haze_per_m: (1.0 / mix(11000.0, 5500.0, fingerprint.fullness)) as f32,
            },
        };

        // The route turns into each region, one way and then the other, by as much as
        // the boundary is trusted.
        let mut heading = 0.0;
        for (index, boundary) in analysis.boundaries.iter().enumerate() {
            let side = if index % 2 == 0 { 1.0 } else { -1.0 };
            let turn = side * TURN * boundary.confidence - heading;
            place.turns.push((boundary.time_s, turn));
            heading += turn;
        }
        // The route is laid out step by step, each from where the one before ended.
        for step in 1..=place.intensity.len() {
            let to_s = step as f64 * hop_s;
            let (along, across) = (place.distance_at(to_s), place.across(to_s));
            place.distance.push(along);
            place.across.push(across);
        }
        // A region for each section.
        let region = |section: &Section| Region {
            start_s: section.start_s,
            start_m: place.distance_at(section.start_s),
            accent: colour(
                hue(section.brightness),
                mix(0.55, 1.0, section.fullness),
                1.0,
            ),
            sharpness: ((SHARP_BELOW - section.fullness) / 0.5).clamp(0.0, 1.0) as f32,
        };
        place.regions = analysis.sections.iter().map(region).collect();
        place.land = place.raised(&analysis.side_levels);

        let mut strengths: Vec<f32> = analysis.onsets.iter().map(|onset| onset.strength).collect();
        strengths.sort_by(f32::total_cmp);
        let full = strengths
            .get(strengths.len().saturating_sub(1) * FULL_LEVEL_PERCENTILE / 100)
            .map_or(1.0, |strength| f64::from(*strength));
        place.impulses = analysis
            .onsets
            .iter()
            .map(|onset| Impulse {
                onset_s: onset.time_s,
                origin_m: [place.across(onset.time_s), place.distance_at(onset.time_s)],
                level: (f64::from(onset.strength) / full).min(1.0),
                altitude_m: place.altitude(onset.time_s),
                floor_m: scale(FLOOR_M, read(&place.intensity, hop_s, onset.time_s)),
                form: onset.stands_out.then_some((
                    onset.part,
                    match onset.lean_db {
                        lean if lean < -LEAN_DB => Side::Left,
                        lean if lean > LEAN_DB => Side::Right,
                        _ => Side::Centre,
                    },
                )),
            })
            .collect();
        place
    }

    pub fn summary(&self) -> Summary {
        Summary {
            route_length_m: self.distance_at(self.duration_s),
            regions: self.regions.len(),
            impulses: self.impulses.len(),
            forms: self.impulses.iter().filter(|i| i.form.is_some()).count(),
        }
    }

    /// Speed along the route at a song time: the song's intensity then, and nothing else.
    fn speed(&self, time_s: f64) -> f64 {
        self.pace * scale(SPEED, read(&self.intensity, self.hop_s, time_s))
    }

    /// The last step of the intensity curve laid out by a song time, and when it starts.
    fn step(&self, time_s: f64) -> (usize, f64) {
        let step = ((time_s / self.hop_s).max(0.0) as usize).min(self.distance.len() - 1);
        (step, step as f64 * self.hop_s)
    }

    /// Distance along the route at a song time: the speed, summed from the step before.
    fn distance_at(&self, time_s: f64) -> f64 {
        let (step, from_s) = self.step(time_s);
        self.distance[step] + sum(from_s, time_s, |at| self.speed(at))
    }

    /// The route's sideways offset at a song time: its slope over the distance travelled.
    fn across(&self, time_s: f64) -> f64 {
        let (step, from_s) = self.step(time_s);
        self.across[step] + sum(from_s, time_s, |at| self.heading(at).0 * self.speed(at))
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
            let from_s = if route_m < 0.0 { 0.0 } else { self.duration_s };
            return from_s + (route_m - self.distance_at(from_s)) / self.speed(from_s);
        }
        (step as f64 + (route_m - self.distance[step]) / span) * self.hop_s
    }

    /// The route's sideways slope at a song time, and how fast that slope is changing.
    /// Each turn eases it over the same stretch of song, as gently out as in.
    fn heading(&self, time_s: f64) -> (f64, f64) {
        let (mut slope, mut changing) = (0.0, 0.0);
        for (middle_s, turn) in &self.turns {
            let u = ((time_s - middle_s) / TURN_S + 0.5).clamp(0.0, 1.0);
            slope += turn * ease(u);
            changing += turn * 30.0 * u * u * (1.0 - u) * (1.0 - u) / TURN_S;
        }
        (slope, changing)
    }

    /// Height of the camera above the floor at a song time.
    fn altitude(&self, time_s: f64) -> f64 {
        let drive = read(&self.drive, self.hop_s, time_s);
        mix(ALTITUDE.0, ALTITUDE.1, drive)
    }

    /// Where the camera is at a song time, the dial having brought it `nearer` the ground.
    fn flight(&self, time_s: f64, nearer: f64) -> [f64; 3] {
        [
            self.across(time_s),
            self.altitude(time_s) / nearer,
            self.distance_at(time_s),
        ]
    }

    /// The place at a song time, seen with the dial so many steps up or down.
    pub fn moment(&self, song_time_s: f64, dial: i32) -> Moment {
        let time_s = song_time_s.clamp(0.0, self.duration_s);
        let nearer = DIAL_STEP.powi(dial.clamp(-DIAL_MOST, DIAL_MOST));
        let eye = self.flight(time_s, nearer);
        let ahead = self.flight(time_s + LOOK_AHEAD_S, nearer);
        let (along, speed) = (eye[2], self.speed(time_s));
        let sink = ahead[1] - eye[1] - LOOK_DOWN * eye[1];
        // The view turns as the route's heading does, and the camera leans as the view turns.
        let (slope, changing) = self.heading(time_s + LOOK_AHEAD_S / 2.0);
        let turning_deg_s = (changing / (1.0 + slope * slope)).to_degrees();
        let light = read(&self.intensity, self.hop_s, time_s);
        let first = self
            .impulses
            .partition_point(|i| i.onset_s <= time_s - IMPULSE_LIFE_S);
        let end = self.impulses.partition_point(|i| i.onset_s <= time_s);
        let seen = self
            .impulses
            .partition_point(|i| i.origin_m[1] <= along + SIGHT_M);
        let forms = |impulses: std::ops::Range<usize>| {
            let impulses = self.impulses[impulses].iter();
            impulses.filter_map(|impulse| Some((impulse, impulse.form?)))
        };
        Moment {
            song_time_s: time_s,
            route_position_m: along,
            region: self.regions.partition_point(|r| r.start_s <= time_s).max(1) - 1,
            camera: Camera {
                position_m: eye,
                yaw_deg: slope.atan().to_degrees(),
                pitch_deg: sink
                    .atan2((ahead[0] - eye[0]).hypot(ahead[2] - eye[2]))
                    .to_degrees(),
                bank_deg: BANK_MAX_DEG * (turning_deg_s / BANK_TURNING_DEG_S).tanh(),
            },
            light,
            speed_m_s: speed,
            over_ground_per_s: speed / (eye[1] - f64::from(swell(eye[0] as f32, along as f32))),
            past_land_per_s: speed / scale(FLOOR_M, light),
            nearer,
            accent: self.character(along).0,
            land: self.profile(along, light),
            impulses: forms(first..end)
                .map(|(impulse, (_, side))| {
                    let age_s = time_s - impulse.onset_s;
                    ImpulseInPlay {
                        onset_s: impulse.onset_s,
                        age_s,
                        level: impulse.level * (1.0 - age_s / IMPULSE_LIFE_S).powi(2),
                        reach_m: impulse.floor_m * LIGHT_RUNS * age_s,
                        side,
                    }
                })
                .collect(),
            forms: forms(first..seen.max(end))
                .map(|(impulse, (kind, side))| FormInView {
                    onset_s: impulse.onset_s,
                    route_m: impulse.origin_m[1],
                    kind,
                    side,
                })
                .collect(),
        }
    }

    /// One lantern for each onset that does not stand out, where it sounds: across, up,
    /// along, level, radius.
    pub fn lanterns(&self) -> Vec<f32> {
        let lantern = |i: &Impulse| {
            let [x, z] = i.origin_m.map(|m| m as f32);
            let radius = i.altitude_m * (0.02 + 0.06 * i.level);
            [x, swell(x, z), z, i.level as f32, radius as f32]
        };
        let lesser = self.impulses.iter().filter(|i| i.form.is_none());
        lesser.flat_map(lantern).collect()
    }

    /// The forms as boxes, one on each side a form stands on: the middle of its foot
    /// (across, up, along), half its width across the route, its height, and half its
    /// depth along the route.
    pub fn forms(&self) -> Vec<f32> {
        let mut boxes = Vec::new();
        for impulse in &self.impulses {
            let Some((part, side)) = impulse.form else {
                continue;
            };
            let (out, across, along, tall) = FORMS[part as usize];
            let sides: &[f64] = match side {
                Side::Left => &[-1.0],
                Side::Centre => &[-1.0, 1.0],
                Side::Right => &[1.0],
            };
            let ([x, z], floor) = (impulse.origin_m, impulse.floor_m);
            let foot = f64::from(swell(x as f32, z as f32)) - SUNK_M;
            let height = mix(tall.0, tall.1, impulse.level) * floor + SUNK_M;
            for toward in sides {
                let x = x + toward * out * floor;
                let form = [x, foot, z, across * floor, height, along * floor];
                boxes.extend(form.map(|part| part as f32));
            }
        }
        boxes
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

    /// How strongly each band of each side stands on each row of ground.
    fn raised(&self, sides: &[Vec<[f32; BANDS]>; 2]) -> Vec<[[f32; BANDS]; 2]> {
        let range = |band: usize| {
            let mut levels: Vec<f32> = sides.iter().flatten().map(|step| step[band]).collect();
            levels.sort_by(f32::total_cmp);
            let share = |of: usize| levels.get(levels.len().saturating_sub(1) * of / 100);
            [10, 99].map(|of| share(of).map_or(0.0, |level| *level))
        };
        let ranges: [[f32; 2]; BANDS] = std::array::from_fn(range);
        let loudest = ranges.iter().map(|range| range[1]).fold(f32::MIN, f32::max);
        let rows = self.rows();
        // The song time at which each row begins, and that at which the last one ends.
        let edges: Vec<f64> = (0..=rows)
            .map(|row| self.time_at((row as f64 - 0.5) * ROW_M - BEHIND_M))
            .collect();
        let mut land = vec![[[0.0; BANDS]; 2]; rows];
        for (side, levels) in sides.iter().enumerate() {
            for band in 0..BANDS {
                let high = ranges[band][1].max(loudest - BAND_UNDER_DB);
                let low = ranges[band][0].clamp(high - BAND_RANGE_DB.1, high - BAND_RANGE_DB.0);
                // How the band stands as a running total over the song, so that a row of
                // ground takes the mean over the time it covers.
                let mut total = vec![0.0f64];
                let stands = |level: f32| ((level - low) / (high - low)).clamp(0.0, 1.0);
                for (step, other) in levels.iter().zip(&sides[1 - side]) {
                    let (here, there) = (stands(step[band]), stands(other[band]));
                    let here = (here + (APART - 1.0) / 2.0 * (here - there)).clamp(0.0, 1.0);
                    total.push(total[total.len() - 1] + f64::from(here));
                }
                let until = |time_s: f64| {
                    let x = time_s / self.hop_s;
                    let step = (x.max(0.0) as usize).min(total.len().saturating_sub(2));
                    let stands = total.get(step + 1).map_or(0.0, |next| next - total[step]);
                    total[step] + stands * (x - step as f64)
                };
                let mean = |row: &[f64]| (until(row[1]) - until(row[0])) / (row[1] - row[0]);
                let stands: Vec<f32> = edges
                    .windows(2)
                    .map(|row| (mean(row) * self.hop_s) as f32)
                    .collect();
                let width = band_m(band as f64 + 0.5) - band_m(band as f64 - 0.5);
                let reach = ((ALONG.0 * width / ROW_M).round() as usize).max(ALONG.1);
                let stands = smooth(&stands, reach);
                let standing = smooth(&stands, STANDING.0 * reach);
                for (row, (stands, standing)) in stands.iter().zip(&standing).enumerate() {
                    land[row][side][band] =
                        (STANDING.1 * standing + OWN * (stands - standing)).max(0.0);
                }
            }
        }
        land
    }

    /// The height of the land above the floor on a row of ground, on the left (0) or the
    /// right (1), so far past the floor's edge.
    fn stands(&self, row: usize, side: usize, beyond_m: f64, sharpness: f32) -> f64 {
        if beyond_m <= 0.0 {
            return 0.0;
        }
        let strength = through(&self.land[row][side], 1.0, band_at(beyond_m) + 0.5).max(0.0);
        let crest = mix(CREST.0, CREST.1, f64::from(sharpness));
        // The land climbs out of the floor over the reach of the lowest band.
        RELIEF_M
            * (beyond_m / HALF_WIDTH_M).powf(SWEEP)
            * ease(beyond_m / band_m(0.0))
            * strength.powf(crest)
    }

    /// The land across the route at a distance along it, where the song's intensity is as given.
    fn profile(&self, route_m: f64, intensity: f64) -> Profile {
        let row = row_at(route_m + ROW_M / 2.0).min(self.land.len() - 1);
        let sharpness = self.character(route_m).1;
        let side = |side: usize| {
            (0..BANDS)
                .map(|band| self.stands(row, side, band_m(band as f64), sharpness))
                .collect()
        };
        Profile {
            across_m: (0..BANDS)
                .map(|band| scale(FLOOR_M, intensity) + band_m(band as f64))
                .collect(),
            left_m: side(0),
            right_m: side(1),
        }
    }

    /// Builds the ground of the whole place: a floor along the route that widens with
    /// intensity, and to either side the land the song raises.
    pub fn ground(&self) -> Ground {
        let rows = self.rows();
        let mut positions = Vec::with_capacity(rows * COLUMNS);
        let mut paint = Vec::with_capacity(rows * COLUMNS);
        for row in 0..rows {
            let along = row as f64 * ROW_M - BEHIND_M;
            let time_s = self.time_at(along);
            let intensity = read(&self.intensity, self.hop_s, time_s);
            let (accent, sharpness) = self.character(along);
            let centre = self.across(time_s);
            let floor = scale(FLOOR_M, intensity);
            for column in 0..COLUMNS {
                // Columns crowd toward the route, where the camera is.
                let across = crowd(column as f64 / (COLUMNS - 1) as f64 * 2.0 - 1.0);
                let x = centre + across;
                let beyond = (across.abs() - floor).max(0.0);
                let grain = fractal((x / GRAIN_M) as f32, (along / GRAIN_M) as f32);
                let height = self.stands(row, usize::from(across > 0.0), beyond, sharpness)
                    * (1.0 + GRAIN * (f64::from(grain) - 0.5))
                    + f64::from(swell(x as f32, along as f32));
                positions.push([x as f32, height as f32, along as f32]);
                let beyond = beyond.copysign(across) as f32;
                paint.push([accent[0], accent[1], accent[2], intensity as f32, beyond]);
            }
        }

        let mut vertices = Vec::with_capacity(rows * COLUMNS * VERTEX_FLOATS);
        let at = |row: usize, column: usize| positions[row * COLUMNS + column];
        for row in 0..rows {
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
                vertices.extend(paint[row * COLUMNS + column]);
            }
        }
        Ground {
            rows,
            columns: COLUMNS,
            vertices,
        }
    }
}

/// How far from the middle of the land a share of its half-width lies: the columns of
/// ground crowd toward the route, and the bands of the sound with them.
fn crowd(share: f64) -> f64 {
    HALF_WIDTH_M * (0.12 * share + 0.88 * share * share * share)
}

/// How far past the floor's edge a band stands: the middle of band 0 at 0, and so on.
fn band_m(band: f64) -> f64 {
    crowd((band + 0.5) / BANDS as f64)
}

/// The band that stands so far past the floor's edge, as `band_m` has it.
fn band_at(beyond_m: f64) -> f64 {
    let above = (1..BANDS - 1)
        .find(|band| band_m(*band as f64) > beyond_m)
        .unwrap_or(BANDS - 1);
    let (near, far) = (band_m(above as f64 - 1.0), band_m(above as f64));
    above as f64 - 1.0 + (beyond_m - near) / (far - near)
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
