//! The song as band amplitudes over time: the shared front end of analysis.

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

/// Frames per second of the spectrum; fine enough to time an onset to a few milliseconds.
const FRAMES_PER_SECOND: f64 = 200.0;
/// Seconds of sound each frame covers, before rounding up to a power of two in samples.
const WINDOW_S: f64 = 0.02;
pub const BANDS: usize = 24;
const LOWEST_HZ: f64 = 50.0;
const HIGHEST_HZ: f64 = 16_000.0;

/// Amplitude in `BANDS` bands, low to high and evenly spaced in pitch, for each frame.
/// Frame `n` covers song time `n * hop_s .. n * hop_s + window_s`. A sine of amplitude
/// `a` reads about `a` in its band.
pub struct Spectrum {
    pub hop_s: f64,
    pub window_s: f64,
    amplitude: Vec<f32>,
}

impl Spectrum {
    pub fn of(samples: &[f32], sample_rate: u32) -> Spectrum {
        let rate = f64::from(sample_rate);
        let hop = ((rate / FRAMES_PER_SECOND).round() as usize).max(1);
        let window = ((rate * WINDOW_S) as usize).next_power_of_two();
        let hann: Vec<f32> = (0..window)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / window as f32).cos())
            .collect();
        let scale = 2.0 / hann.iter().sum::<f32>();

        // Band edges as FFT bins: evenly spaced in pitch, each band at least one bin wide.
        let top_hz = HIGHEST_HZ.min(rate / 2.0);
        let mut edges = [0usize; BANDS + 1];
        for i in 0..=BANDS {
            let hz = LOWEST_HZ * (top_hz / LOWEST_HZ).powf(i as f64 / BANDS as f64);
            let bin = ((hz * window as f64 / rate).round() as usize).max(1);
            edges[i] = if i == 0 {
                bin
            } else {
                bin.max(edges[i - 1] + 1)
            };
        }
        let fft = FftPlanner::<f32>::new().plan_fft_forward(window);
        let frames = if samples.len() >= window {
            (samples.len() - window) / hop + 1
        } else {
            0
        };
        let mut amplitude = Vec::with_capacity(frames * BANDS);
        let mut buffer = vec![Complex::new(0.0f32, 0.0); window];
        for frame in 0..frames {
            let block = &samples[frame * hop..frame * hop + window];
            for ((slot, sample), weight) in buffer.iter_mut().zip(block).zip(&hann) {
                *slot = Complex::new(sample * weight, 0.0);
            }
            fft.process(&mut buffer);
            for band in edges.windows(2) {
                let power: f32 = buffer[band[0]..band[1]]
                    .iter()
                    .map(|bin| bin.norm_sqr())
                    .sum();
                amplitude.push(power.sqrt() * scale);
            }
        }
        Spectrum {
            hop_s: hop as f64 / rate,
            window_s: window as f64 / rate,
            amplitude,
        }
    }

    pub fn frames(&self) -> usize {
        self.amplitude.len() / BANDS
    }

    pub fn frame(&self, n: usize) -> &[f32] {
        &self.amplitude[n * BANDS..(n + 1) * BANDS]
    }
}
