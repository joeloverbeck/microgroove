//! Timeline: a picture of what analysis heard, laid out over song time.
//! It is for inspection and is never part of a journey.

use crate::analysis::{Analysis, BOUNDARY_TRUST};

type Colour = [u8; 3];
const SURFACE: Colour = [0x1a, 0x1a, 0x19];
const LANE: Colour = [0x24, 0x24, 0x22];
const GRID: Colour = [0x38, 0x38, 0x35];
const TEXT: Colour = [0xf0, 0xef, 0xec];
const MUTED: Colour = [0xc3, 0xc2, 0xb7];
const BLUE: Colour = [0x39, 0x87, 0xe5];
const PALE_BLUE: Colour = [0xcd, 0xe2, 0xfb];
const ORANGE: Colour = [0xd9, 0x59, 0x26];
const AQUA: Colour = [0x19, 0x9e, 0x70];
const YELLOW: Colour = [0xc9, 0x85, 0x00];
const MAGENTA: Colour = [0xd5, 0x51, 0x81];
const VIOLET: Colour = [0x90, 0x85, 0xe9];
const RED: Colour = [0xe6, 0x67, 0x67];

const WIDTH: usize = 1800;
const LEFT: usize = 120;
const PLOT: usize = WIDTH - LEFT - 20;
const GAP: usize = 10;

const CHARACTERS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ:.-/%=";
/// Five-by-seven dot letters, one row per byte, for the characters above in order.
#[rustfmt::skip]
const GLYPHS: [[u8; 7]; 42] = [
    [0x0E,0x11,0x13,0x15,0x19,0x11,0x0E], [0x04,0x0C,0x04,0x04,0x04,0x04,0x0E], [0x0E,0x11,0x01,0x02,0x04,0x08,0x1F],
    [0x1E,0x01,0x01,0x0E,0x01,0x01,0x1E], [0x02,0x06,0x0A,0x12,0x1F,0x02,0x02], [0x1F,0x10,0x1E,0x01,0x01,0x11,0x0E],
    [0x06,0x08,0x10,0x1E,0x11,0x11,0x0E], [0x1F,0x01,0x02,0x04,0x08,0x08,0x08], [0x0E,0x11,0x11,0x0E,0x11,0x11,0x0E],
    [0x0E,0x11,0x11,0x0F,0x01,0x02,0x0C], [0x0E,0x11,0x11,0x1F,0x11,0x11,0x11], [0x1E,0x11,0x11,0x1E,0x11,0x11,0x1E],
    [0x0E,0x11,0x10,0x10,0x10,0x11,0x0E], [0x1E,0x11,0x11,0x11,0x11,0x11,0x1E], [0x1F,0x10,0x10,0x1E,0x10,0x10,0x1F],
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x10], [0x0E,0x11,0x10,0x17,0x11,0x11,0x0F], [0x11,0x11,0x11,0x1F,0x11,0x11,0x11],
    [0x0E,0x04,0x04,0x04,0x04,0x04,0x0E], [0x07,0x02,0x02,0x02,0x02,0x12,0x0C], [0x11,0x12,0x14,0x18,0x14,0x12,0x11],
    [0x10,0x10,0x10,0x10,0x10,0x10,0x1F], [0x11,0x1B,0x15,0x15,0x11,0x11,0x11], [0x11,0x19,0x15,0x13,0x11,0x11,0x11],
    [0x0E,0x11,0x11,0x11,0x11,0x11,0x0E], [0x1E,0x11,0x11,0x1E,0x10,0x10,0x10], [0x0E,0x11,0x11,0x11,0x15,0x12,0x0D],
    [0x1E,0x11,0x11,0x1E,0x14,0x12,0x11], [0x0F,0x10,0x10,0x0E,0x01,0x01,0x1E], [0x1F,0x04,0x04,0x04,0x04,0x04,0x04],
    [0x11,0x11,0x11,0x11,0x11,0x11,0x0E], [0x11,0x11,0x11,0x11,0x11,0x0A,0x04], [0x11,0x11,0x11,0x15,0x15,0x1B,0x11],
    [0x11,0x11,0x0A,0x04,0x0A,0x11,0x11], [0x11,0x11,0x0A,0x04,0x04,0x04,0x04], [0x1F,0x01,0x02,0x04,0x08,0x10,0x1F],
    [0x00,0x04,0x04,0x00,0x04,0x04,0x00], [0x00,0x00,0x00,0x00,0x00,0x0C,0x0C], [0x00,0x00,0x00,0x1F,0x00,0x00,0x00],
    [0x01,0x01,0x02,0x04,0x08,0x10,0x10], [0x19,0x19,0x02,0x04,0x08,0x13,0x13], [0x00,0x00,0x1F,0x00,0x1F,0x00,0x00],
];
/// Each dot of a letter is drawn this many pixels square.
const DOT: usize = 2;
const LETTER: usize = 6 * DOT;

struct Canvas {
    height: usize,
    pixels: Vec<u8>,
}

impl Canvas {
    fn fill(&mut self, x: usize, y: usize, width: usize, height: usize, colour: Colour) {
        for row in y..(y + height).min(self.height) {
            for column in x..(x + width).min(WIDTH) {
                self.pixels[(row * WIDTH + column) * 3..][..3].copy_from_slice(&colour);
            }
        }
    }

    fn text(&mut self, x: usize, y: usize, text: &str, colour: Colour) {
        for (i, character) in text.chars().enumerate() {
            let Some(glyph) = CHARACTERS.find(character.to_ascii_uppercase()) else {
                continue;
            };
            for (row, bits) in GLYPHS[glyph].iter().enumerate() {
                for column in (0..5).filter(|column| bits >> (4 - column) & 1 == 1) {
                    self.fill(
                        x + i * LETTER + column * DOT,
                        y + row * DOT,
                        DOT,
                        DOT,
                        colour,
                    );
                }
            }
        }
    }
}

fn clock(seconds: f64) -> String {
    format!("{}:{:02}", (seconds / 60.0) as u32, (seconds % 60.0) as u32)
}

fn blend(from: Colour, to: Colour, share: f32) -> Colour {
    std::array::from_fn(|i| {
        (f32::from(from[i]) + (f32::from(to[i]) - f32::from(from[i])) * share.clamp(0.0, 1.0)) as u8
    })
}

/// Draws the timeline and returns it as a PNG file.
pub fn draw(analysis: &Analysis, duration_s: f64) -> Vec<u8> {
    let bands = analysis
        .band_levels
        .first()
        .map_or(0, |levels| levels.len());
    let lanes = [
        ("BANDS", bands * 5),
        ("INTENSITY", 110),
        ("STEREO", 100),
        ("ONSETS", 70),
        ("PULSE", 30),
        ("CHANGE", 100),
    ];
    // Above the lanes: one line of summary, then three rows for the boundary times.
    let row = 7 * DOT + GAP / 2;
    let top = 4 * row + 2 * GAP;
    let bottom = top + lanes.iter().map(|(_, height)| height + GAP).sum::<usize>();
    let height = bottom + 7 * DOT + 2 * GAP;
    let mut canvas = Canvas {
        height,
        pixels: SURFACE.repeat(WIDTH * height),
    };
    let x = |seconds: f64| {
        LEFT + ((seconds / duration_s) * PLOT as f64)
            .round()
            .clamp(0.0, PLOT as f64) as usize
    };
    // The value of a slow curve under plot column `column`.
    let at = |values: &[f32], hop_s: f64, column: usize| -> f32 {
        let seconds = (column as f64 + 0.5) / PLOT as f64 * duration_s;
        values
            .get((seconds / hop_s) as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let peak = |values: &[f32]| values.iter().copied().fold(f32::MIN_POSITIVE, f32::max);

    let pulse = match analysis.pulse.bpm {
        Some(bpm) => format!("PULSE {bpm:.1} BPM"),
        None => "NO PULSE".to_string(),
    };
    let loudness = analysis
        .loudness
        .integrated_lufs
        .map_or("SILENT".to_string(), |lufs| format!("{lufs:.1} LUFS"));
    let summary = format!(
        "{} LONG   {loudness}   RANGE {:.1} LU   {} ONSETS   {pulse}   {} BOUNDARIES",
        clock(duration_s),
        analysis.loudness.range_lu,
        analysis.onsets.len(),
        analysis.boundaries.len()
    );
    canvas.text(LEFT, GAP, &summary, TEXT);

    let mut y = top;
    let mut lane_tops = Vec::new();
    for (name, lane_height) in lanes {
        canvas.fill(LEFT, y, PLOT, lane_height, LANE);
        canvas.text(GAP, y + lane_height / 2 - 7, name, MUTED);
        lane_tops.push(y);
        y += lane_height + GAP;
    }

    // Song time along the bottom, with a faint line up through every lane.
    let step = [1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0]
        .into_iter()
        .find(|step| step / duration_s * PLOT as f64 >= 60.0);
    for tick in (0..)
        .map(|i| i as f64 * step.unwrap_or(3600.0))
        .take_while(|tick| *tick <= duration_s)
    {
        canvas.fill(x(tick), top, 1, bottom - top, GRID);
        canvas.text(
            x(tick).saturating_sub(2 * LETTER),
            bottom + GAP / 2,
            &clock(tick),
            MUTED,
        );
    }

    let hop_s = analysis.intensity.hop_s;
    // Band levels are shaded over the 60 dB below the loudest level any band reaches.
    let loudest_level = analysis
        .band_levels
        .iter()
        .flatten()
        .copied()
        .fold(f32::MIN, f32::max);
    for column in 0..PLOT {
        let step = (((column as f64 + 0.5) / PLOT as f64 * duration_s) / hop_s) as usize;
        if let Some(levels) = analysis.band_levels.get(step) {
            for (band, level) in levels.iter().enumerate() {
                let share = 1.0 + (level - loudest_level) / 60.0;
                let colour = if share < 0.6 {
                    blend(LANE, BLUE, share / 0.6)
                } else {
                    blend(BLUE, PALE_BLUE, (share - 0.6) / 0.4)
                };
                canvas.fill(
                    LEFT + column,
                    lane_tops[0] + (bands - 1 - band) * 5,
                    1,
                    5,
                    colour,
                );
            }
        }

        let bar = (at(&analysis.intensity.values, hop_s, column) * 110.0).round() as usize;
        canvas.fill(
            LEFT + column,
            lane_tops[1] + 110 - bar.min(110),
            1,
            bar.min(110),
            ORANGE,
        );

        let loudest = peak(&analysis.stereo.left).max(peak(&analysis.stereo.right));
        let left = (at(&analysis.stereo.left, hop_s, column) / loudest * 49.0).round() as usize;
        let right = (at(&analysis.stereo.right, hop_s, column) / loudest * 49.0).round() as usize;
        canvas.fill(LEFT + column, lane_tops[2] + 49 - left, 1, left, BLUE);
        canvas.fill(LEFT + column, lane_tops[2] + 51, 1, right, MAGENTA);

        let change = at(&analysis.change.values, hop_s, column) / (4.0 * BOUNDARY_TRUST);
        let bar = ((change * 100.0).round() as usize).min(100);
        canvas.fill(LEFT + column, lane_tops[5] + 100 - bar, 1, bar, VIOLET);
        if column % 6 < 3 {
            canvas.fill(LEFT + column, lane_tops[5] + 75, 1, 1, TEXT); // the change at which a boundary is trusted
        }
    }
    canvas.text(LEFT + 4, lane_tops[2] + 4, "LEFT", MUTED);
    canvas.text(LEFT + 4, lane_tops[2] + 100 - 18, "RIGHT", MUTED);
    canvas.text(LEFT + 4, lane_tops[5] + 75 - 18, "TRUSTED ABOVE", MUTED);

    let strongest = analysis
        .onsets
        .iter()
        .map(|onset| onset.strength)
        .fold(f32::MIN_POSITIVE, f32::max);
    for onset in &analysis.onsets {
        let bar = ((onset.strength / strongest * 70.0).round() as usize).max(2);
        canvas.fill(x(onset.time_s), lane_tops[3] + 70 - bar, 1, bar, YELLOW);
    }
    for span in &analysis.pulse.trusted {
        canvas.fill(
            x(span.start_s),
            lane_tops[4] + 8,
            x(span.end_s) - x(span.start_s),
            14,
            AQUA,
        );
    }

    // Trusted boundaries cut through every lane, each with its time above.
    for (i, boundary) in analysis.boundaries.iter().enumerate() {
        canvas.fill(x(boundary.time_s), top - GAP, 1, bottom - top + GAP, RED);
        let label = format!(
            "{} {:.0}%",
            clock(boundary.time_s),
            boundary.confidence * 100.0
        );
        canvas.text(
            (x(boundary.time_s) + 3).min(WIDTH - label.len() * LETTER),
            GAP + (1 + i % 3) * row,
            &label,
            TEXT,
        );
    }

    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, WIDTH as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .expect("a PNG header can be written to memory");
    writer
        .write_image_data(&canvas.pixels)
        .expect("the pixels match the header");
    writer.finish().expect("a PNG can be finished in memory");
    png
}
