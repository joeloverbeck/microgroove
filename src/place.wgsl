// Draws one moment of a place: the sky, then the ground, then the forms and the lanterns.
// Every input is the moment or the place; nothing is kept from one picture to the next.
// Colours are linear light; the target's sRGB format encodes them.

struct Moment {
    view: mat4x4<f32>,
    origin: vec4<f32>,   // xyz: a point near the camera that every position is taken from. w: light, 0 to 1.
    eye: vec4<f32>,      // xyz: the camera, from the origin. w: haze per metre.
    right: vec4<f32>,    // xyz: camera right, scaled by half the width of the view. w: impulses in play.
    up: vec4<f32>,       // xyz: camera up, scaled by half the height of the view. w: the size the dial leaves lanterns, of their own.
    forward: vec4<f32>,  // w: how thick the air is around the listener, of the place's own.
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    sun: vec4<f32>,
    sun_toward: vec4<f32>,
    lights: array<vec4<f32>, 12>, // x: the side lit, -1 the left, 1 the right, 0 both. z: reach. w: level.
}

@group(0) @binding(0) var<uniform> moment: Moment;

// Tuning: the light the place keeps when the song gives none, the spacing of the seams
// of light in the land, how strongly the land glows and an impulse shows, and how far
// from the camera an impulse's light on the land is still at about a third.
const LEAST_LIGHT: f32 = 0.14;
const SEAM_M: f32 = 34.0;
const GLOW: f32 = 0.55;
const LIT: f32 = 0.6;
const LIT_M: f32 = 1200.0;
// Tuning, for still water, dunes, terraced rock and ice in turn: how far level ground
// shines, and how coarse the grain of a slope is. Then the width and the length of a
// streak of sand and the thickness of a band of rock, in metres.
const SHINES = vec4<f32>(1.0, 0.0, 0.0, 0.6);
const COARSE = vec4<f32>(0.6, 0.15, 0.8, 0.3);
const STREAK_M = vec2<f32>(2.0, 90.0);
const STRATUM_M: f32 = 7.0;
// An impulse's light counts each metre it climbs a wall of rock as this many it runs out.
const CLIMBS: f32 = 2.5;

fn light() -> f32 {
    return mix(LEAST_LIGHT, 1.0, moment.origin.w);
}

// The air in a direction, before the song's light is applied: a bright band along the
// horizon under a dark zenith, and a low sun with a wide glow. Thick air veils the sun:
// its disc dims and its glow spreads.
fn air(toward: vec3<f32>) -> vec3<f32> {
    let height = clamp(toward.y, 0.0, 1.0);
    let sun = max(dot(toward, moment.sun_toward.xyz), 0.0);
    let veil = max(moment.forward.w, 1.0);
    return mix(moment.horizon.rgb, moment.zenith.rgb, 1.0 - exp(-4.5 * height))
        + moment.sun.rgb * (0.12 * veil * pow(sun, 8.0) + 0.7 / veil * pow(sun, 80.0) + 8.0 / (veil * veil) * pow(sun, 2500.0));
}

// A fixed value from 0 to 1 for each point of a grid, and smooth noise through them.
// Integer arithmetic, so it is the same wherever and whenever it is drawn.
fn lattice(p: vec2<i32>) -> f32 {
    var h = (u32(p.x) * 0x27d4eb2du) ^ (u32(p.y) * 0x165667b1u);
    h = (h ^ (h >> 15u)) * 0x2c1b3c6du;
    h = (h ^ (h >> 12u)) * 0x297a2d39u;
    return f32((h ^ (h >> 15u)) >> 8u) / 16777216.0;
}

fn noise(p: vec2<f32>) -> f32 {
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(lattice(cell), lattice(cell + vec2<i32>(1, 0)), u.x),
        mix(lattice(cell + vec2<i32>(0, 1)), lattice(cell + vec2<i32>(1, 1)), u.x),
        u.y,
    );
}

// The sky: the air, with long thin clouds lying low over the horizon, dark against it
// and lit where the sun is behind them. They are fixed in the sky, like the sun.
fn sky(toward: vec3<f32>) -> vec3<f32> {
    let streak = noise(vec2<f32>(atan2(toward.x, toward.z) * 5.0, toward.y * 55.0));
    let cloud = smoothstep(0.5, 0.85, streak) * smoothstep(0.01, 0.06, toward.y) * (1.0 - smoothstep(0.1, 0.4, toward.y));
    let sun = max(dot(toward, moment.sun_toward.xyz), 0.0);
    return mix(air(toward), moment.zenith.rgb * 0.7 + moment.sun.rgb * 0.6 * pow(sun, 6.0), 0.6 * cloud);
}

struct SkyOut {
    @builtin(position) position: vec4<f32>,
    @location(0) screen: vec2<f32>,
}

@vertex
fn sky_vertex(@builtin(vertex_index) index: u32) -> SkyOut {
    // One triangle that covers the whole picture, as far away as anything can be.
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    return SkyOut(vec4<f32>(corner, 1.0, 1.0), corner);
}

@fragment
fn sky_fragment(in: SkyOut) -> @location(0) vec4<f32> {
    let toward = normalize(moment.forward.xyz + in.screen.x * moment.right.xyz + in.screen.y * moment.up.xyz);
    return vec4<f32>(sky(toward) * light(), 1.0);
}

struct GroundOut {
    @builtin(position) position: vec4<f32>,
    @location(0) point: vec3<f32>, // of the ground, from the origin
    @location(1) normal: vec3<f32>,
    @location(2) paint: vec4<f32>, // rgb: the region's accent. a: the intensity of the song here.
    @location(3) beyond: f32,      // metres past the floor's edge: below zero on the left, zero on the floor.
    @location(4) kind: vec4<f32>,  // how far the ground here is still water, dunes, terraced rock and ice.
}

@vertex
fn ground_vertex(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) paint: vec4<f32>,
    @location(3) beyond: f32,
    @location(4) kind: vec4<f32>,
) -> GroundOut {
    let point = position - moment.origin.xyz;
    return GroundOut(moment.view * vec4<f32>(point, 1.0), point, normal, paint, beyond, kind);
}

@fragment
fn ground_fragment(in: GroundOut) -> @location(0) vec4<f32> {
    let from_eye = in.point - moment.eye.xyz;
    let range = length(from_eye);
    let toward = from_eye / range;
    let accent = in.paint.rgb;
    let across = in.point.xz + moment.origin.xz;
    let water = in.kind.x;
    let dunes = in.kind.y;
    let rock = in.kind.z;
    let ice = in.kind.w;

    // Ice is cut in facets: each triangle of the ground shows its own face.
    let face = normalize(cross(dpdx(in.point), dpdy(in.point)));
    let shape = normalize(mix(normalize(in.normal), face * sign(face.y), ice));
    // Level ground is smooth, and shines by what it is made of: still water is a mirror,
    // sheet ice shines less, a slab of rock hardly, and sand not at all.
    let flat = smoothstep(0.86, 0.995, shape.y);
    let wet = flat * dot(in.kind, SHINES);
    // Slopes are grained, at a scale finer than the ground's triangles: rock coarsely,
    // sand and ice hardly. Height shifts the grain, so that it does not smear down a
    // steep slope. Roughness too fine to draw fades out. Here and below, what a kind
    // of ground does not show is not worked out for it.
    let coarse = dot(in.kind, COARSE) * clamp(1.0 - range / 1500.0, 0.0, 1.0) * (1.0 - flat);
    var grain = 0.5;
    var tilt = vec2<f32>(0.0);
    if coarse > 0.0 {
        let rough = (across + in.point.y * vec2<f32>(1.3, 0.7)) / 7.0;
        grain = noise(rough);
        tilt = (vec2<f32>(noise(rough + vec2<f32>(0.3, 0.0)), noise(rough + vec2<f32>(0.0, 0.3))) - grain) * 1.5 * coarse;
    }
    // Sand that lies nearly level is combed in streaks that run with the route, so that
    // they stream past the listener and do not beat against the eye. No two are alike:
    // even stripes over much of the picture would be hard on the eye.
    let combed = across / STREAK_M;
    let comb = dunes * smoothstep(0.75, 0.95, shape.y) * clamp(1.0 - fwidth(combed.x), 0.0, 1.0);
    var sand = 0.5;
    if comb > 0.0 {
        sand = noise(combed);
        tilt.x += 1.2 * comb * (noise(combed + vec2<f32>(0.4, 0.0)) - sand);
    }
    let normal = normalize(shape - vec3<f32>(tilt.x, 0.0, tilt.y));

    // What each kind is made of: dark stone standing in the water, with a pale line
    // where the water meets it; pale sand; rock in bands that lie level through the
    // whole place; pale ice.
    let stone = mix(vec3<f32>(0.05), accent * 0.32, 0.7);
    let layer = in.point.y / STRATUM_M;
    let banding = rock * (1.0 - flat) * clamp(1.0 - 1.5 * fwidth(layer), 0.0, 1.0);
    var banded = 1.0;
    if banding > 0.0 {
        let band = smoothstep(0.35, 0.65, noise(vec2<f32>(layer, (across.x + across.y) / 120.0)));
        banded = mix(1.0, mix(0.4, 1.9, band), banding / rock);
    }
    let shore = (1.0 - flat) * (1.0 - smoothstep(0.3, 1.5, in.point.y));
    let matter = (water * mix(stone, vec3<f32>(0.5), shore)
            + dunes * mix(vec3<f32>(0.34), accent, 0.4) * (1.0 + 0.3 * comb * (sand - 0.5))
            + rock * stone * banded
            + ice * mix(vec3<f32>(0.5), accent, 0.3))
        * (1.0 + 0.8 * (grain - 0.5) * coarse) * (1.0 - 0.6 * wet);
    let sunlit = max(dot(normal, moment.sun_toward.xyz), 0.0);
    var colour = matter
        * (moment.sun.rgb * sunlit * 2.2 + moment.zenith.rgb * (0.5 + 0.5 * normal.y) * 1.6 + moment.horizon.rgb * 0.25);

    // What shines shows the sky beyond it, most of all at a glance: still water nearly
    // all of it, and the faces of the ice each their own part of it.
    let mirrored = reflect(toward, normal);
    let glance = pow(1.0 - max(dot(-toward, normal), 0.0), 4.0);
    let shine = (0.05 + 0.3 * water + (0.95 - 0.3 * water) * glance) * wet + ice * (1.0 - wet) * (0.12 + 0.5 * glance);
    if shine > 0.0 {
        colour = mix(colour, sky(normalize(vec3<f32>(mirrored.x, abs(mirrored.y) + 0.02, mirrored.z))), 0.7 * shine);
        colour += ice * moment.sun.rgb * pow(max(dot(mirrored, moment.sun_toward.xyz), 0.0), 40.0);
    }
    // All of that is as bright as the song gives light now.
    colour *= light();

    // The land glows from within where the song is intense, whatever the light is now:
    // rock along broken seams at even heights, and all but still water in veins that
    // wander over whatever does not shine, which on a level slab of rock and on sheet
    // ice are a net of fine cracks. Seams and veins too fine to draw fade out.
    let level = in.point.y / SEAM_M;
    let fine = fwidth(level);
    var seam = 0.0;
    if rock > 0.0 {
        seam = rock * (1.0 - smoothstep(0.02, 0.04 + fine, abs(fract(level) - 0.5)))
            * clamp(1.0 - 2.5 * fine, 0.0, 1.0)
            * smoothstep(0.42, 0.62, noise(across / 140.0 + vec2<f32>(floor(level) * 7.0)));
    }
    let veined = (1.0 - water) * clamp(1.0 - range / 2500.0, 0.0, 1.0) * (1.0 - flat * (water + dunes));
    let cracked = (rock + ice) * flat;
    var wander = 1.0;
    if veined > 0.0 {
        wander = abs(2.0 * noise(across / 80.0) - 1.0);
        if cracked > 0.0 {
            wander = min(wander, mix(1.0, abs(2.0 * noise(across / 23.0) - 1.0), cracked));
        }
    }
    let vein = veined * (1.0 - smoothstep(0.0, 0.015 + 1.5 * fwidth(wander), wander));
    colour += accent * in.paint.a * in.paint.a * GLOW * (0.04 + seam + 0.5 * vein);

    // Each impulse in play lights the land beside the route, never level ground: a
    // front of light that runs up the land from the floor's edge, with a glow behind it.
    // A wall of rock it climbs, so that it is a line on the wall and never the whole of it.
    let climbed = abs(in.beyond) + CLIMBS * rock * max(in.point.y, 0.0);
    for (var i = 0; i < i32(moment.right.w); i++) {
        let lit = moment.lights[i];
        let behind = (lit.z - climbed) / (10.0 + 0.4 * lit.z);
        let front = smoothstep(-0.2, 0.0, behind) * exp(-2.5 * max(behind, 0.0));
        let land = (1.0 - flat) * smoothstep(0.0, 4.0, abs(in.beyond)) * step(-0.5, in.beyond * lit.x);
        colour += mix(accent, vec3<f32>(1.0), 0.5) * lit.w * LIT * (0.5 + grain) * front * land * exp(-range / LIT_M);
    }
    return vec4<f32>(hazed(colour, toward, range), 1.0);
}

// Distance fades what stands in the place into the sky that stands behind it.
fn hazed(colour: vec3<f32>, toward: vec3<f32>, range: f32) -> vec3<f32> {
    let behind = air(normalize(vec3<f32>(toward.x, max(toward.y, 0.0) + 0.03, toward.z))) * light();
    return mix(behind, colour, exp(-range * moment.eye.w));
}

// The corners of a face, as two triangles.
const CORNERS = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
);

struct FormOut {
    @builtin(position) position: vec4<f32>,
    @location(0) point: vec3<f32>, // of the form, from the origin
    @location(1) normal: vec3<f32>,
    @location(2) up: f32,          // how far up the form: 0 at its foot, 1 at its top.
}

@vertex
fn form_vertex(
    @builtin(vertex_index) index: u32,
    @location(0) foot: vec3<f32>, // the middle of its foot
    @location(1) size: vec3<f32>, // half its width across the route, its height, half its depth along it
) -> FormOut {
    // A box: each of its six faces looks along one axis and spans the other two.
    let corner = CORNERS[index % 6u];
    let face = index / 6u;
    var normal = vec3<f32>(0.0);
    normal[face % 3u] = f32(face / 3u) * 2.0 - 1.0;
    var unit = normal;
    unit[(face + 1u) % 3u] = corner.x;
    unit[(face + 2u) % 3u] = corner.y;
    let base = foot - moment.origin.xyz;
    // A form too thin to draw from where the camera is is drawn a pixel or two wide.
    let half = max(size.xz, vec2<f32>(0.0006 * length(base - moment.eye.xyz)));
    let point = base + vec3<f32>(unit.x * half.x, (unit.y + 1.0) * 0.5 * size.y, unit.z * half.y);
    return FormOut(moment.view * vec4<f32>(point, 1.0), point, normal, 0.5 * unit.y + 0.5);
}

@fragment
fn form_fragment(in: FormOut) -> @location(0) vec4<f32> {
    let from_eye = in.point - moment.eye.xyz;
    let range = length(from_eye);
    // Dark matter under the sun and the sky, with a light of its own that is brightest at
    // its top: dim enough that a form filling a part of the picture as it passes is no flash.
    let sunlit = max(dot(in.normal, moment.sun_toward.xyz), 0.0);
    let colour = vec3<f32>(0.03) * (moment.sun.rgb * sunlit * 2.2 + moment.zenith.rgb * 1.6 + moment.horizon.rgb * 0.25) * light()
        + moment.sun.rgb * (0.04 + 0.2 * pow(in.up, 4.0));
    return vec4<f32>(hazed(colour, from_eye / range, range), 1.0);
}

struct LanternOut {
    @builtin(position) position: vec4<f32>,
    @location(0) corner: vec2<f32>,
    @location(1) shine: f32,
}

@vertex
fn lantern_vertex(
    @builtin(vertex_index) index: u32,
    @location(0) lantern: vec4<f32>, // xyz: where it stands. w: level.
    @location(1) its_radius: f32,
) -> LanternOut {
    let radius = its_radius * moment.up.w;
    let corner = CORNERS[index];
    let foot = lantern.xyz - moment.origin.xyz;
    let range = length(foot - moment.eye.xyz);
    // A lantern is never drawn smaller than a few pixels: it dims instead.
    let drawn = max(radius, range * 0.0016);
    let centre = foot
        + drawn * (corner.x * normalize(moment.right.xyz) + (corner.y + 1.0) * normalize(moment.up.xyz));
    let shine = (0.25 + 0.75 * lantern.w) * (radius * radius) / (drawn * drawn) * exp(-range * moment.eye.w);
    return LanternOut(moment.view * vec4<f32>(centre, 1.0), corner, shine);
}

@fragment
fn lantern_fragment(in: LanternOut) -> @location(0) vec4<f32> {
    let glow = exp(-5.0 * dot(in.corner, in.corner)) - exp(-5.0);
    return vec4<f32>(moment.sun.rgb * max(glow, 0.0) * in.shine * 1.6, 1.0);
}
