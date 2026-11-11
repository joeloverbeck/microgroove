// Draws one moment of a place: the sky, then the ground, then the lanterns.
// Every input is the moment or the place; nothing is kept from one picture to the next.
// Colours are linear light; the target's sRGB format encodes them.

struct Moment {
    view: mat4x4<f32>,
    origin: vec4<f32>,   // xyz: a point near the camera that every position is taken from. w: light, 0 to 1.
    eye: vec4<f32>,      // xyz: the camera, from the origin. w: haze per metre.
    right: vec4<f32>,    // xyz: camera right, scaled by half the width of the view. w: impulses in play.
    up: vec4<f32>,       // xyz: camera up, scaled by half the height of the view. w: the size the dial leaves lanterns and rings, of their own.
    forward: vec4<f32>,
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    sun: vec4<f32>,
    sun_toward: vec4<f32>,
    rings: array<vec4<f32>, 12>, // xy: the point under the camera, from the origin (across, along). z: reach. w: level.
}

@group(0) @binding(0) var<uniform> moment: Moment;

// Tuning: the light the place keeps when the song gives none, the spacing of the seams
// of light in the land, and how strongly the land glows and an impulse shows.
const LEAST_LIGHT: f32 = 0.14;
const SEAM_M: f32 = 34.0;
const GLOW: f32 = 0.55;
const RING: f32 = 0.8;

fn light() -> f32 {
    return mix(LEAST_LIGHT, 1.0, moment.origin.w);
}

// The air in a direction, before the song's light is applied: a bright band along the
// horizon under a dark zenith, and a low sun with a wide glow.
fn air(toward: vec3<f32>) -> vec3<f32> {
    let height = clamp(toward.y, 0.0, 1.0);
    let sun = max(dot(toward, moment.sun_toward.xyz), 0.0);
    return mix(moment.horizon.rgb, moment.zenith.rgb, 1.0 - exp(-4.5 * height))
        + moment.sun.rgb * (0.12 * pow(sun, 8.0) + 0.7 * pow(sun, 80.0) + 8.0 * pow(sun, 2500.0));
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
}

@vertex
fn ground_vertex(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) paint: vec4<f32>,
) -> GroundOut {
    let point = position - moment.origin.xyz;
    return GroundOut(moment.view * vec4<f32>(point, 1.0), point, normal, paint);
}

@fragment
fn ground_fragment(in: GroundOut) -> @location(0) vec4<f32> {
    let from_eye = in.point - moment.eye.xyz;
    let range = length(from_eye);
    let toward = from_eye / range;
    let accent = in.paint.rgb;
    let across = in.point.xz + moment.origin.xz;

    // Level ground lies wet, dark and smooth. Slopes are rough and grained, at a scale
    // finer than the ground's triangles; roughness too fine to draw fades out.
    let shape = normalize(in.normal);
    let wet = smoothstep(0.86, 0.995, shape.y);
    let near = clamp(1.0 - range / 1500.0, 0.0, 1.0) * (1.0 - wet);
    // Height shifts the grain, so that it does not smear down a steep slope.
    let rough = (across + in.point.y * vec2<f32>(1.3, 0.7)) / 7.0;
    let grain = noise(rough);
    let tilt = vec2<f32>(noise(rough + vec2<f32>(0.3, 0.0)), noise(rough + vec2<f32>(0.0, 0.3))) - grain;
    let normal = normalize(shape - vec3<f32>(tilt.x, 0.0, tilt.y) * 1.5 * near);
    let matter = mix(vec3<f32>(0.05), accent * 0.32, 0.7) * (1.0 + 0.8 * (grain - 0.5) * near) * (1.0 - 0.6 * wet);
    let sunlit = max(dot(normal, moment.sun_toward.xyz), 0.0);
    var colour = matter
        * (moment.sun.rgb * sunlit * 2.2 + moment.zenith.rgb * (0.5 + 0.5 * normal.y) * 1.6 + moment.horizon.rgb * 0.25);

    // It shines like water: it shows the sky beyond it, most of all at a glance.
    let mirrored = reflect(toward, normal);
    let glance = pow(1.0 - max(dot(-toward, normal), 0.0), 4.0);
    let shine = (0.05 + 0.95 * glance) * wet;
    colour = mix(colour, sky(normalize(vec3<f32>(mirrored.x, abs(mirrored.y) + 0.02, mirrored.z))), 0.7 * shine);
    // All of that is as bright as the song gives light now.
    colour *= light();

    // The land glows from within where the song is intense, whatever the light is now:
    // along broken seams at even heights, and in veins that wander over the slopes.
    // Seams and veins too fine to draw fade out.
    let level = in.point.y / SEAM_M;
    let fine = fwidth(level);
    let seam = (1.0 - smoothstep(0.02, 0.04 + fine, abs(fract(level) - 0.5)))
        * clamp(1.0 - 2.5 * fine, 0.0, 1.0)
        * smoothstep(0.42, 0.62, noise(across / 140.0 + vec2<f32>(floor(level) * 7.0)));
    let wander = abs(2.0 * noise(across / 80.0) - 1.0);
    let vein = (1.0 - smoothstep(0.0, 0.015 + 1.5 * fwidth(wander), wander))
        * clamp(1.0 - range / 2500.0, 0.0, 1.0) * (1.0 - wet);
    colour += accent * in.paint.a * in.paint.a * GLOW * (0.04 + seam + 0.5 * vein);

    // Each impulse in play is a ring of light running out over the ground from under the camera.
    for (var i = 0; i < i32(moment.right.w); i++) {
        let ring = moment.rings[i];
        let off = (distance(in.point.xz, ring.xy) - ring.z) / (0.03 * ring.z + 0.5 * moment.up.w);
        colour += mix(accent, vec3<f32>(1.0), 0.5) * ring.w * RING * exp(-off * off);
    }

    // Distance fades the land into the sky that stands behind it.
    let behind = air(normalize(vec3<f32>(toward.x, max(toward.y, 0.0) + 0.03, toward.z))) * light();
    return vec4<f32>(mix(behind, colour, exp(-range * moment.eye.w)), 1.0);
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
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let corner = corners[index];
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
