//! Place: analysis in; the route, the regions and the identity of the place.
//! Moment: a place and a song time in; everything needed to draw that moment.
//!
//! A moment depends only on the analysis and the song time. Nothing here remembers a
//! moment that came before: an impulse's decay is worked out from the time since its onset.
//!
//! Every constant below is tuning, set by inspecting stills of the first-journey song.
//! Lengths are in the place's own metres.

use serde::Serialize;

use crate::analysis::{Analysis, BANDS, Onset, Part, SILENT_DB, Section, percentile, smooth};

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
/// The height of the camera's view, and how many times as wide as it is tall the picture
/// is taken to be where a form is stood to leave it: the display's 16 to 9.
const VIEW_DEG: f64 = 56.0;
const PICTURE: f64 = 16.0 / 9.0;
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
/// No band is measured against less than the least range above silence, so that in a
/// silent file nothing stands.
const BAND_RANGE_DB: (f32, f32) = (6.0, 36.0);
const BAND_UNDER_DB: f32 = 24.0;
/// Along the route a band is smoothed over this many times its own width across the
/// land, and over this many rows at least, so that what it does is about as long as it
/// is wide: a hill, not a rib across the land or a strip along it. What it does over
/// this many times that length is its standing, which counts for this much of its
/// height; what it does apart from its standing counts this many times over. However
/// sharply it does a thing, it stands no stronger than the last of these: a little over
/// the strongest any band stands in the first-journey song, which is 2.03.
const ALONG: (f64, usize) = (1.5, 4);
const STANDING: (usize, f32) = (6, 0.7);
const OWN: f32 = 3.0;
const STRONGEST: f32 = 2.1;
/// What a channel has that the other has not counts this many times over, so that the
/// two sides of a song mixed nearly alike are not mirror images.
const APART: f32 = 3.0;
/// A region's kind carves the land out of the song's spectrum. For still water, dunes,
/// terraced rock and ice in turn: how tall a band at full strength stands at the far edge
/// of the land; the power of the distance by which it stands lower nearer the route; the
/// power strength counts by, from rounded to sharp; and how far fixed noise makes a band
/// stronger or weaker, in shapes of the size after it and smaller. That noise is left
/// only as grain. Below zero it only weakens: it cuts creases, and what stands between
/// them is a sharp ridge, the crest of a dune or an edge of ice.
const CARVED: [(f64, f64, f64, f64, f64); 4] = [
    (1500.0, 0.75, 1.5, 0.25, 150.0),
    (800.0, 0.75, 1.3, -0.45, 500.0),
    (1500.0, 0.55, 1.1, 0.3, 150.0),
    (1500.0, 0.75, 3.0, -0.8, 150.0),
];
/// Still water floods the land this deep: only what stands taller shows, as islands. The
/// flood shallows to nothing at the floor's edge, from this far out, so that beside the
/// floor what the lowest bands raise stands in the shallows, as low bars.
const FLOOD_M: f64 = 12.0;
const SHOAL_M: f64 = 80.0;
/// Beside the floor ice is blunt: strength counts by this power at the floor's edge, and
/// by more with distance, up to the kind's own this far out. Shards stand by the sheet
/// and spires far off.
const SHARD: (f64, f64) = (1.5, 200.0);
/// Land stands where it is this much taller than the floor: as far as the floor's own
/// swell rises. Anything lower cannot be told from the floor as it passes.
const STANDS_M: f64 = 3.0;
/// Terraced rock stands in steps. Its first tread lies this high and each next one this
/// many times higher, the same everywhere in the place; the last share of each step is
/// its riser.
const TREAD_M: (f64, f64) = (8.0, 1.6);
const RISER: f64 = 0.3;
/// How thick the air is over each kind, of the place's own: clear over still water and
/// over rock, dust over the dunes, a mist over the ice.
const AIR: [f32; 4] = [0.7, 2.2, 0.8, 1.3];
/// The size of the swell of the floor.
const LANDFORM_M: f64 = 800.0;
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
/// stands, the nearer the listener is to it as it leaves the picture. Its foot is sunk
/// this many metres into the ground, which is nowhere quite level.
const FORMS: [(f64, f64, f64, (f64, f64)); 3] = [
    (0.5, 0.5, 0.015, (0.012, 0.022)),
    (0.75, 0.06, 0.06, (0.5, 1.0)),
    (0.35, 0.012, 0.012, (1.6, 3.0)),
];
const SUNK_M: f64 = 3.0;
/// An onset this strong among the song's own onsets gives an impulse at full level.
const FULL_LEVEL_PERCENTILE: usize = 90;
/// What chooses a region's kind: its section is quiet below this intensity and loud from
/// that one, dark below this brightness, and thin below this fullness. Each is set in a
/// gap between the first-journey song's sections.
const QUIET_BELOW: f64 = 0.3;
const LOUD_FROM: f64 = 0.55;
const DARK_BELOW: f64 = 0.4;
const THIN_BELOW: f64 = 0.5;
/// A last section shorter than this is the song's tail, and takes the kind before it.
const TAIL_S: f64 = 4.0;

/// Floats per ground vertex: position, normal, accent colour, glow, how far past the
/// floor's edge it lies, in metres (below zero on the left, zero on the floor), and how
/// far it is each kind of place.
pub const VERTEX_FLOATS: usize = 15;
/// Floats per lantern: position, level, radius.
pub const LANTERN_FLOATS: usize = 5;
/// Floats per box of a form: the middle of its foot (across, up, along), half its width
/// across the route, its height, and half its depth along the route.
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
    /// How strongly each band stands on each row of ground, from 0 to `STRONGEST`: the
    /// left channel's bands and the right's, low to high.
    land: Vec<[[f32; BANDS]; 2]>,
    impulses: Vec<Impulse>,
    pub sky: Sky,
}

/// The stretch of the place made from one section; its character is that section's own measurements.
struct Region {
    start_s: f64,
    start_m: f64,
    kind: Kind,
    accent: [f32; 3],
}

/// The place's answer to one onset: a form where the onset stands out, and a small
/// lantern on the route where it does not. `origin_m` is where the route is as the onset
/// sounds, across and along.
struct Impulse {
    onset_s: f64,
    origin_m: [f64; 2],
    level: f64,
    /// Height of the camera as it passes, and half the width of the floor there.
    altitude_m: f64,
    floor_m: f64,
    /// The part of the sound that jumped, which makes it a rib, stones or masts, and the
    /// side it stands on.
    form: Option<(Part, Side)>,
    /// The boxes the form stands as: see `Place::stood`.
    boxes: Vec<[f64; FORM_FLOATS]>,
}

/// What kind of place a region is. Its section's own sound chooses it: still water where
/// the song is at rest, whatever its sound; terraced rock where it is loud and dark; ice
/// where it is thin and not dark; dunes everywhere else.
#[derive(Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Water,
    Dunes,
    Rock,
    Ice,
}

const KINDS: [Kind; 4] = [Kind::Water, Kind::Dunes, Kind::Rock, Kind::Ice];

impl Kind {
    fn of(section: &Section) -> Kind {
        let dark = section.brightness < DARK_BELOW;
        match section.intensity {
            intensity if intensity < QUIET_BELOW => Kind::Water,
            intensity if dark && intensity >= LOUD_FROM => Kind::Rock,
            _ if !dark && section.fullness < THIN_BELOW => Kind::Ice,
            _ => Kind::Dunes,
        }
    }
}

/// Where a form stands: on the side of the channel its onset is louder in, or on both.
#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Centre,
    Right,
}

impl Side {
    /// Each side of the route this stands on: -1 the left, 1 the right.
    pub fn signs(self) -> &'static [f64] {
        match self {
            Side::Left => &[-1.0],
            Side::Centre => &[-1.0, 1.0],
            Side::Right => &[1.0],
        }
    }
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
    /// The region the listener is in, counted from 0, and the kind of place it is.
    pub region: usize,
    pub kind: Kind,
    pub camera: Camera,
    /// From 0 to 1: how much light the song gives the place now.
    pub light: f64,
    /// Speed along the route, in metres per second of song.
    pub speed_m_s: f64,
    /// How fast what is near passes the camera, in times a second: speed over its height
    /// above the ground under it, and over the distance from the middle of the route to
    /// the nearest land that stands beside it, which is 0 where none stands.
    pub over_ground_per_s: f64,
    pub past_land_per_s: f64,
    /// How many times nearer the ground the dial has brought the camera, and with it
    /// the lanterns, which are sized to its height: 1 with the dial at rest.
    pub nearer: f64,
    /// The accent colour of the land around the listener, which tints the air, and how
    /// thick the air is there, of the place's own: each kind of place has its air.
    pub accent: [f32; 3],
    pub air: f32,
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
    /// The height of its view.
    pub view_deg: f64,
}

impl Camera {
    /// The ways the camera's right, its up and its view point: across, up, along.
    pub fn axes(&self) -> [[f64; 3]; 3] {
        let [yaw, pitch, bank] = [self.yaw_deg, self.pitch_deg, self.bank_deg].map(f64::to_radians);
        let forward = [
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        ];
        let level = [yaw.cos(), 0.0, -yaw.sin()];
        let over = [
            forward[1] * level[2] - forward[2] * level[1],
            forward[2] * level[0] - forward[0] * level[2],
            forward[0] * level[1] - forward[1] * level[0],
        ];
        // Banking tips the camera toward the inside of the turn.
        let tip = |a: [f64; 3], b: [f64; 3], sign: f64| {
            [0, 1, 2].map(|i| a[i] * bank.cos() + sign * b[i] * bank.sin())
        };
        [tip(level, over, -1.0), tip(over, level, 1.0), forward]
    }
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
    /// The song time of its onset, as which the last of it leaves the picture.
    pub onset_s: f64,
    pub part: Part,
    pub side: Side,
    /// The boxes it stands as, one on each side it stands on: see `FORM_FLOATS`.
    pub boxes_m: Vec<[f64; FORM_FLOATS]>,
}

/// A summary of the place, written beside each still. No pulse is in it because none
/// is shown: analysis gives its rate but not where its beats fall.
#[derive(Serialize)]
pub struct Summary {
    pub route_length_m: f64,
    pub regions: usize,
    /// The kind of place each region is, in song order.
    pub kinds: Vec<Kind>,
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
            kind: Kind::of(section),
            accent: colour(
                hue(section.brightness),
                mix(0.55, 1.0, section.fullness),
                1.0,
            ),
        };
        place.regions = analysis.sections.iter().map(region).collect();
        // The song dying away is not a place of its own.
        if let [.., before, tail] = place.regions.as_mut_slice()
            && analysis.duration_s - tail.start_s < TAIL_S
        {
            tail.kind = before.kind;
        }
        place.land = place.raised(&analysis.side_levels);

        let mut strengths: Vec<f32> = analysis.onsets.iter().map(|onset| onset.strength).collect();
        strengths.sort_by(f32::total_cmp);
        let full = percentile(&strengths, FULL_LEVEL_PERCENTILE).map_or(1.0, f64::from);
        let impulse = |onset: &Onset| {
            let mut impulse = Impulse {
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
                boxes: Vec::new(),
            };
            impulse.boxes = place.stood(&impulse);
            impulse
        };
        place.impulses = analysis.onsets.iter().map(impulse).collect();
        place
    }

    pub fn summary(&self) -> Summary {
        Summary {
            route_length_m: self.distance_at(self.duration_s),
            regions: self.regions.len(),
            kinds: self.regions.iter().map(|region| region.kind).collect(),
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

    /// The camera at a song time, the dial having brought it `nearer` the ground.
    fn camera(&self, time_s: f64, nearer: f64) -> Camera {
        let eye = self.flight(time_s, nearer);
        let ahead = self.flight(time_s + LOOK_AHEAD_S, nearer);
        let sink = ahead[1] - eye[1] - LOOK_DOWN * eye[1];
        // The view turns as the route's heading does, and the camera leans as the view turns.
        let (slope, changing) = self.heading(time_s + LOOK_AHEAD_S / 2.0);
        let turning_deg_s = (changing / (1.0 + slope * slope)).to_degrees();
        Camera {
            position_m: eye,
            yaw_deg: slope.atan().to_degrees(),
            pitch_deg: sink
                .atan2((ahead[0] - eye[0]).hypot(ahead[2] - eye[2]))
                .to_degrees(),
            bank_deg: BANK_MAX_DEG * (turning_deg_s / BANK_TURNING_DEG_S).tanh(),
            view_deg: VIEW_DEG,
        }
    }

    /// The boxes a form stands as, one on each side it stands on. The camera looks ahead,
    /// so a thing beside it is already out of the picture: each box stands further along
    /// the route than the route is as its onset sounds, by as far as has the last of it
    /// leave the picture at that moment. The picture is the camera's own then, with the
    /// dial at rest.
    fn stood(&self, impulse: &Impulse) -> Vec<[f64; FORM_FLOATS]> {
        let Some((part, side)) = impulse.form else {
            return Vec::new();
        };
        let (out, across, along, tall) = FORMS[part as usize];
        let (sides, floor) = (side.signs(), impulse.floor_m);
        let height = mix(tall.0, tall.1, impulse.level) * floor + SUNK_M;
        let stand = |(toward, ahead): (&f64, &f64)| {
            let z = impulse.origin_m[1] + ahead;
            let x = self.across(self.time_at(z));
            let foot = f64::from(self.level(x, z)) - SUNK_M;
            let x = x + toward * out * floor;
            [x, foot, z, across * floor, height, along * floor]
        };
        let camera = self.camera(impulse.onset_s, 1.0);
        let slope = self.heading(impulse.onset_s).0;
        let mut ahead = vec![0.0; sides.len()];
        // Where a box stands sets the ground under it and the bend of the route up to
        // it, and those move how far ahead it must stand, a little: three goes settle it.
        for _ in 0..3 {
            let further = |at: (&f64, &f64)| further_m(&camera, slope, stand(at));
            let further: Vec<f64> = sides.iter().zip(&ahead).map(further).collect();
            // A rib is one thing across the floor: both halves stand where the last of it leaves.
            let least = further.iter().copied().fold(f64::MAX, f64::min);
            for (ahead, further) in ahead.iter_mut().zip(further) {
                *ahead += if part == Part::Low { least } else { further };
            }
        }
        sides.iter().zip(&ahead).map(stand).collect()
    }

    /// The place at a song time, seen with the dial so many steps up or down.
    pub fn moment(&self, song_time_s: f64, dial: i32) -> Moment {
        let time_s = song_time_s.clamp(0.0, self.duration_s);
        let nearer = DIAL_STEP.powi(dial.clamp(-DIAL_MOST, DIAL_MOST));
        let camera = self.camera(time_s, nearer);
        let eye = camera.position_m;
        let (along, speed) = (eye[2], self.speed(time_s));
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
        let region = self.regions.partition_point(|r| r.start_s <= time_s).max(1) - 1;
        let (accent, kinds) = self.character(along);
        Moment {
            song_time_s: time_s,
            route_position_m: along,
            region,
            kind: self.regions[region].kind,
            camera,
            light,
            speed_m_s: speed,
            over_ground_per_s: speed / (eye[1] - f64::from(self.level(eye[0], along))),
            past_land_per_s: speed / self.nearest_land_m(along),
            nearer,
            accent,
            air: kinds.iter().zip(AIR).map(|(kind, air)| kind * air).sum(),
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
                .map(|(impulse, (part, side))| FormInView {
                    onset_s: impulse.onset_s,
                    part,
                    side,
                    boxes_m: impulse.boxes.clone(),
                })
                .collect(),
        }
    }

    /// One lantern for each onset that does not stand out, where it sounds: across, up,
    /// along, level, radius.
    pub fn lanterns(&self) -> Vec<f32> {
        let lantern = |i: &Impulse| {
            let [x, z] = i.origin_m;
            let radius = (i.altitude_m * (0.02 + 0.06 * i.level)) as f32;
            [x as f32, self.level(x, z), z as f32, i.level as f32, radius]
        };
        let lesser = self.impulses.iter().filter(|i| i.form.is_none());
        lesser.flat_map(lantern).collect()
    }

    /// The boxes every form stands as, one after another.
    pub fn forms(&self) -> Vec<f32> {
        let boxes = self.impulses.iter().flat_map(|impulse| &impulse.boxes);
        boxes.flatten().map(|part| *part as f32).collect()
    }

    /// The accent colour of the land at a distance along the route, and how far it is each
    /// kind of place, in the order of `Kind`: each region's own, taking over from the one
    /// before across its boundary.
    fn character(&self, route_m: f64) -> ([f32; 3], [f32; 4]) {
        let mut character = (self.regions[0].accent, [0.0; 4]);
        character.1[self.regions[0].kind as usize] = 1.0;
        for region in &self.regions[1..] {
            let share = ease((route_m - region.start_m) / BLEND_M + 0.5) as f32;
            for (channel, accent) in character.0.iter_mut().zip(region.accent) {
                *channel += (accent - *channel) * share;
            }
            for (kind, weight) in character.1.iter_mut().enumerate() {
                *weight += (f32::from(kind == region.kind as usize) - *weight) * share;
            }
        }
        character
    }

    /// The height of the floor and of all level ground at a point of the place: its swell,
    /// which still water lies flat over.
    fn level(&self, x: f64, z: f64) -> f32 {
        swell(x as f32, z as f32) * (1.0 - self.character(z).1[Kind::Water as usize])
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
            [10, 99].map(|of| percentile(&levels, of).unwrap_or(0.0))
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
                let high =
                    (ranges[band][1].max(loudest - BAND_UNDER_DB)).max(SILENT_DB + BAND_RANGE_DB.0);
                let low = ranges[band][0].clamp(high - BAND_RANGE_DB.1, high - BAND_RANGE_DB.0);
                // Where the band's level lies in its range, as a running total over the
                // song, so that a row of ground takes the mean over the time it covers.
                let mut total = vec![0.0f64];
                let lies = |level: f32| ((level - low) / (high - low)).clamp(0.0, 1.0);
                for (step, other) in levels.iter().zip(&sides[1 - side]) {
                    let (here, there) = (lies(step[band]), lies(other[band]));
                    let here = (here + (APART - 1.0) / 2.0 * (here - there)).clamp(0.0, 1.0);
                    total.push(total[total.len() - 1] + f64::from(here));
                }
                let until = |time_s: f64| {
                    let x = time_s / self.hop_s;
                    let step = (x.max(0.0) as usize).min(total.len().saturating_sub(2));
                    let gained = total.get(step + 1).map_or(0.0, |next| next - total[step]);
                    total[step] + gained * (x - step as f64)
                };
                let mean = |row: &[f64]| (until(row[1]) - until(row[0])) / (row[1] - row[0]);
                let means = edges.windows(2).map(|row| (mean(row) * self.hop_s) as f32);
                let width = band_m(band as f64 + 0.5) - band_m(band as f64 - 0.5);
                let reach = ((ALONG.0 * width / ROW_M).round() as usize).max(ALONG.1);
                let does = smooth(&means.collect::<Vec<f32>>(), reach);
                let standing = smooth(&does, STANDING.0 * reach);
                for (row, (does, standing)) in does.iter().zip(&standing).enumerate() {
                    land[row][side][band] =
                        (STANDING.1 * standing + OWN * (does - standing)).clamp(0.0, STRONGEST);
                }
            }
        }
        land
    }

    /// The height of the land above the floor on a row of ground, on the left (0) or the
    /// right (1), so far past the floor's edge: the song's spectrum, carved as each kind
    /// of place the ground is there carves it. `grain` is each kind's fixed noise there,
    /// from -0.5 to 0.5.
    fn stands(
        &self,
        row: usize,
        side: usize,
        beyond_m: f64,
        kinds: [f32; 4],
        grain: [f64; 4],
    ) -> f64 {
        if beyond_m <= 0.0 {
            return 0.0;
        }
        let strength = through(&self.land[row][side], 1.0, band_at(beyond_m) + 0.5).max(0.0);
        let carved = |kind: &Kind| {
            let (relief, sweep, crest, grained, _) = CARVED[*kind as usize];
            let grain = grain[*kind as usize];
            let grain = if grained < 0.0 {
                2.0 * grain.abs()
            } else {
                grain
            };
            let crest = match kind {
                Kind::Ice => mix(SHARD.0, crest, ease(beyond_m / SHARD.1)),
                _ => crest,
            };
            // The land climbs out of the floor over the reach of the lowest band.
            let height = relief
                * (beyond_m / HALF_WIDTH_M).powf(sweep)
                * ease(beyond_m / band_m(0.0))
                * (strength * (1.0 + grained * grain)).powf(crest);
            let height = match kind {
                Kind::Water => (height - FLOOD_M * ease(beyond_m / SHOAL_M)).max(0.0),
                Kind::Rock => terraced(height),
                _ => height,
            };
            f64::from(kinds[*kind as usize]) * height
        };
        let here = |kind: &&Kind| kinds[**kind as usize] > 0.0;
        KINDS.iter().filter(here).map(carved).sum()
    }

    /// How far from the middle of the route the nearest land stands, on either side, on the
    /// row of ground at a distance along the route: the nearest column of that row on
    /// which the land as it is drawn, grain and all, is `STANDS_M` tall. Infinite where
    /// none stands.
    fn nearest_land_m(&self, route_m: f64) -> f64 {
        let row = row_at(route_m + ROW_M / 2.0).min(self.land.len() - 1);
        let along = row as f64 * ROW_M - BEHIND_M;
        let time_s = self.time_at(along);
        let floor = scale(FLOOR_M, read(&self.intensity, self.hop_s, time_s));
        let (centre, kinds) = (self.across(time_s), self.character(along).1);
        let stands = |across: &f64| {
            [-1.0, 1.0].iter().any(|toward| {
                let grain = grain(kinds, centre + toward * across, along);
                let side = usize::from(*toward > 0.0);
                self.stands(row, side, across - floor, kinds, grain) >= STANDS_M
            })
        };
        let share = |column: usize| column as f64 / (COLUMNS - 1) as f64 * 2.0;
        let mut columns = (1..=COLUMNS / 2).map(|column| crowd(share(column)));
        columns.find(stands).unwrap_or(f64::INFINITY)
    }

    /// The land across the route at a distance along it, where the song's intensity is as given.
    fn profile(&self, route_m: f64, intensity: f64) -> Profile {
        let row = row_at(route_m + ROW_M / 2.0).min(self.land.len() - 1);
        let kinds = self.character(route_m).1;
        let side = |side: usize| {
            (0..BANDS)
                .map(|band| self.stands(row, side, band_m(band as f64), kinds, [0.0; 4]))
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
            let (accent, kinds) = self.character(along);
            let centre = self.across(time_s);
            let floor = scale(FLOOR_M, intensity);
            let dry = 1.0 - kinds[Kind::Water as usize];
            for column in 0..COLUMNS {
                // Columns crowd toward the route, where the camera is.
                let across = crowd(column as f64 / (COLUMNS - 1) as f64 * 2.0 - 1.0);
                let x = centre + across;
                let beyond = (across.abs() - floor).max(0.0);
                let side = usize::from(across > 0.0);
                let grain = grain(kinds, x, along);
                let height = self.stands(row, side, beyond, kinds, grain) as f32
                    + dry * swell(x as f32, along as f32);
                positions.push([x as f32, height, along as f32]);
                let beyond = beyond.copysign(across) as f32;
                let [red, green, blue] = accent;
                let [water, dunes, rock, ice] = kinds;
                let intensity = intensity as f32;
                paint.push([red, green, blue, intensity, beyond, water, dunes, rock, ice]);
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

/// Each kind's fixed noise at a point of the place, from -0.5 to 0.5: none for a kind of
/// place the ground there is not.
fn grain(kinds: [f32; 4], x: f64, along: f64) -> [f64; 4] {
    std::array::from_fn(|kind| {
        let size = CARVED[kind].4;
        let noise = || fractal((x / size) as f32, (along / size) as f32) - 0.5;
        f64::from(if kinds[kind] > 0.0 { noise() } else { 0.0 })
    })
}

/// How much further along a route of the given slope a box (see `Place::stood`) would
/// have to stand for the last of it to be on the edge of a camera's picture: below zero
/// while some of it is in the picture. The last of a box to leave is on one of its edges.
fn further_m(camera: &Camera, slope: f64, form: [f64; FORM_FLOATS]) -> f64 {
    let [right, up, forward] = camera.axes();
    let tall = (camera.view_deg / 2.0).to_radians().tan();
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    // How far along the route a corner must move to be on each side of the picture.
    let needs = |corner: usize| {
        let to = |axis: usize| (corner >> axis & 1) as f64;
        let from_eye = [
            form[0] + (2.0 * to(0) - 1.0) * form[3] - camera.position_m[0],
            form[1] + to(1) * form[4] - camera.position_m[1],
            form[2] + (2.0 * to(2) - 1.0) * form[5] - camera.position_m[2],
        ];
        [(right, -PICTURE), (right, PICTURE), (up, -1.0), (up, 1.0)].map(|(axis, wide)| {
            let side = [0, 1, 2].map(|i| tall * wide.abs() * forward[i] + wide.signum() * axis[i]);
            -dot(side, from_eye) / dot(side, [slope, 0.0, 1.0])
        })
    };
    let edges = (0..8).flat_map(|corner| (0..3).map(move |axis| (corner, corner | 1 << axis)));
    edges
        .filter(|(from, to)| from != to)
        .map(|(from, to)| least_of_greatest(needs(from), needs(to)))
        .fold(f64::MAX, f64::min)
}

/// The least, anywhere between two ends, of the greatest of four values that each run
/// evenly from one end to the other: it lies at an end or where two of them cross.
fn least_of_greatest(from: [f64; 4], to: [f64; 4]) -> f64 {
    let greatest = |share: f64| {
        let values = from.iter().zip(to).map(|(from, to)| mix(*from, to, share));
        values.fold(f64::MIN, f64::max)
    };
    let cross = |one: usize, other: usize| {
        (from[other] - from[one]) / (to[one] - from[one] - to[other] + from[other])
    };
    let crossings = (0..4).flat_map(|one| (0..one).map(move |other| cross(one, other)));
    crossings
        .filter(|share| (0.0..1.0).contains(share))
        .map(greatest)
        .fold(greatest(0.0).min(greatest(1.0)), f64::min)
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

/// A height as terraced rock has it: level on the tread at or below it, and climbing to
/// the next tread over the last share of the step between them.
fn terraced(height: f64) -> f64 {
    let (first, times) = TREAD_M;
    let step = (height / first).ln() / times.ln();
    if step <= 0.0 {
        return height;
    }
    first * times.powf(step.floor() + ease((step.fract() - 1.0 + RISER) / RISER))
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
