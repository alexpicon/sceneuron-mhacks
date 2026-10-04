// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A streaming windowed-sinc resampler for mono audio.
//!
//! Each output sample is a weighted sum of the input around its position, with weights from a
//! Blackman-windowed sinc whose cutoff sits below the lower of the two Nyquist frequencies. So
//! 48 kHz speech loses what 16 kHz cannot carry instead of folding it back as noise. Weights are
//! precomputed for [`PHASES`] fractional positions; a position between two blends their sums.
//! Positions advance in exact integer steps, so long streams do not drift.

use std::f64::consts::PI;

/// Sinc lobes on each side of an output position; more give a sharper cutoff for more work.
const ZERO_CROSSINGS: f64 = 16.0;
/// The passband ends this far below the Nyquist frequency, so the filter has rolled off by
/// the frequency where aliasing would begin.
const ROLLOFF: f64 = 0.9;
/// Fractional positions between two input samples that have precomputed weights.
const PHASES: usize = 64;

/// Converts one stream from `input_rate` to `output_rate`, keeping the samples it still needs
/// between calls. Output stays aligned with input; each sample is produced once the `half`
/// input samples after its position have arrived.
pub struct Resampler {
    input_rate: u64,
    output_rate: u64,
    /// Input samples on each side of an output position that carry weight.
    half: usize,
    /// `PHASES + 1` rows of `2 * half` weights; row `p` is for the position `p / PHASES` past
    /// a sample, and every row sums to one.
    weights: Vec<f32>,
    history: Vec<f32>,
    /// The next output falls at `history[index]` plus `remainder / output_rate` samples.
    index: usize,
    remainder: u64,
}

impl Resampler {
    /// A resampler between two rates; a zero rate counts as 1 Hz.
    pub fn new(input_rate: u32, output_rate: u32) -> Resampler {
        let (input_rate, output_rate) = (u64::from(input_rate.max(1)), u64::from(output_rate.max(1)));
        let ratio = output_rate as f64 / input_rate as f64;
        // In cycles per input sample.
        let cutoff = 0.5 * ratio.min(1.0) * ROLLOFF;
        let half = (ZERO_CROSSINGS / (2.0 * cutoff)).ceil() as usize;
        let mut weights = Vec::with_capacity((PHASES + 1) * 2 * half);
        for phase in 0..=PHASES {
            let offset = phase as f64 / PHASES as f64;
            let row: Vec<f64> = (0..2 * half)
                .map(|tap| {
                    let distance = offset + half as f64 - 1.0 - tap as f64;
                    kernel(distance, cutoff, half as f64)
                })
                .collect();
            let sum: f64 = row.iter().sum();
            weights.extend(row.iter().map(|weight| (weight / sum) as f32));
        }
        Resampler {
            input_rate,
            output_rate,
            half,
            weights,
            history: vec![0.0; half - 1],
            index: half - 1,
            remainder: 0,
        }
    }

    /// Appends the output that `input` completes to `out`.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.history.extend_from_slice(input);
        let width = 2 * self.half;
        while self.index + self.half < self.history.len() {
            let window = &self.history[self.index + 1 - self.half..][..width];
            let scaled = self.remainder * PHASES as u64;
            let phase = (scaled / self.output_rate) as usize;
            let blend = (scaled % self.output_rate) as f32 / self.output_rate as f32;
            let below = dot(window, &self.weights[phase * width..][..width]);
            let above = dot(window, &self.weights[(phase + 1) * width..][..width]);
            out.push(below + (above - below) * blend);
            self.remainder += self.input_rate;
            self.index += (self.remainder / self.output_rate) as usize;
            self.remainder %= self.output_rate;
        }
        let consumed = (self.index + 1 - self.half).min(self.history.len());
        self.history.drain(..consumed);
        self.index -= consumed;
    }
}

/// The windowed sinc at `distance` input samples from the output position.
fn kernel(distance: f64, cutoff: f64, half: f64) -> f64 {
    let x = distance / half;
    if x.abs() >= 1.0 {
        return 0.0;
    }
    let window = 0.42 + 0.5 * (PI * x).cos() + 0.08 * (2.0 * PI * x).cos();
    let t = 2.0 * cutoff * distance;
    let sinc = if t.abs() < 1e-12 {
        1.0
    } else {
        (PI * t).sin() / (PI * t)
    };
    2.0 * cutoff * sinc * window
}

fn dot(samples: &[f32], weights: &[f32]) -> f32 {
    samples
        .iter()
        .zip(weights)
        .map(|(sample, weight)| sample * weight)
        .sum()
}
