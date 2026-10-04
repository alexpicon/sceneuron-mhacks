// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Audio in any channel layout and sample rate, converted to what Scribe expects.

use crate::resample::Resampler;

/// Scribe's input format `pcm_16000`: 16 kHz, mono, 16-bit little-endian PCM, from
/// <https://elevenlabs.io/docs/eleven-api/guides/how-to/speech-to-text/realtime/server-side-streaming>.
pub const SAMPLE_RATE: u32 = 16_000;
/// Bytes in one second of converted audio.
pub const BYTES_PER_SECOND: usize = SAMPLE_RATE as usize * 2;

/// Mono audio at its source's sample rate, as the audio thread hands it over. Samples are
/// floats in -1..=1; anything that is not a finite number becomes silence.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioChunk {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

impl AudioChunk {
    /// Mixes planar channels, OBS's float layout with one slice per channel, down to mono.
    /// Channels shorter than the first cut the chunk to their length.
    pub fn from_planar(planes: &[&[f32]], sample_rate: u32) -> AudioChunk {
        let frames = planes.iter().map(|plane| plane.len()).min().unwrap_or(0);
        let mut samples = vec![0.0f32; frames];
        for plane in planes {
            for (mixed, sample) in samples.iter_mut().zip(plane.iter()) {
                *mixed += finite(*sample);
            }
        }
        if planes.len() > 1 {
            let scale = 1.0 / planes.len() as f32;
            samples.iter_mut().for_each(|sample| *sample *= scale);
        }
        AudioChunk { sample_rate, samples }
    }

    /// Mixes interleaved frames, a WAV file's layout, down to mono. A trailing partial frame
    /// is dropped.
    pub fn from_interleaved(samples: &[f32], channels: usize, sample_rate: u32) -> AudioChunk {
        let channels = channels.max(1);
        let samples = samples
            .chunks_exact(channels)
            .map(|frame| frame.iter().copied().map(finite).sum::<f32>() / channels as f32)
            .collect();
        AudioChunk { sample_rate, samples }
    }

    pub fn duration_secs(&self) -> f64 {
        self.samples.len() as f64 / f64::from(self.sample_rate.max(1))
    }
}

fn finite(sample: f32) -> f32 {
    if sample.is_finite() { sample } else { 0.0 }
}

/// Converts the chunks of one stream to Scribe's PCM, keeping the resampler's state from one
/// chunk to the next. Call [`Converter::reset`] between separate stretches of audio.
#[derive(Default)]
pub struct Converter {
    resampler: Option<(u32, Resampler)>,
    resampled: Vec<f32>,
}

impl Converter {
    pub fn new() -> Converter {
        Converter::default()
    }

    /// Appends `chunk` as 16 kHz little-endian 16-bit samples to `out`. A chunk at a new sample
    /// rate starts a new stream; a zero rate is ignored.
    pub fn convert(&mut self, chunk: &AudioChunk, out: &mut Vec<u8>) {
        self.convert_samples(chunk.sample_rate, &chunk.samples, out);
    }

    /// [`Converter::convert`] for mono `samples` at `sample_rate`.
    pub fn convert_samples(&mut self, sample_rate: u32, samples: &[f32], out: &mut Vec<u8>) {
        if sample_rate == 0 {
            return;
        }
        let samples: &[f32] = if sample_rate == SAMPLE_RATE {
            samples
        } else {
            let resampler = match &mut self.resampler {
                Some((rate, resampler)) if *rate == sample_rate => resampler,
                slot => &mut slot.insert((sample_rate, Resampler::new(sample_rate, SAMPLE_RATE))).1,
            };
            self.resampled.clear();
            resampler.process(samples, &mut self.resampled);
            &self.resampled
        };
        out.reserve(samples.len() * 2);
        for sample in samples {
            out.extend_from_slice(&to_i16(*sample).to_le_bytes());
        }
    }

    pub fn reset(&mut self) {
        self.resampler = None;
    }
}

/// Full scale is 32767 both ways, so a sine and its negation stay symmetric; louder clips.
fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16
}
