// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises audio conversion: mixing planar and interleaved channels to mono, 16-bit
//! little-endian output, and the resampler's gain, frequency response, timing, and streaming.

use std::f32::consts::PI;

use sceneuron_voice::{AudioChunk, Converter, Resampler, SAMPLE_RATE};

fn sine(frequency: f32, rate: u32, seconds: f32, amplitude: f32) -> Vec<f32> {
    let count = (rate as f32 * seconds) as usize;
    (0..count)
        .map(|n| amplitude * (2.0 * PI * frequency * n as f32 / rate as f32).sin())
        .collect()
}

fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    let mut out = Vec::new();
    Resampler::new(from, to).process(input, &mut out);
    out
}

/// Root mean square of the middle of `samples`, away from the edges' transients.
fn rms_middle(samples: &[f32]) -> f32 {
    let middle = &samples[samples.len() / 4..samples.len() * 3 / 4];
    (middle.iter().map(|s| s * s).sum::<f32>() / middle.len() as f32).sqrt()
}

fn decode(pcm: &[u8]) -> Vec<i16> {
    pcm.as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect()
}

// Mixing to mono

#[test]
fn planar_channels_are_averaged() {
    let (left, right) = ([1.0, 0.5, -1.0], [0.0, 0.5, 1.0]);
    let chunk = AudioChunk::from_planar(&[&left, &right], 48_000);
    assert_eq!(chunk.samples, vec![0.5, 0.5, 0.0]);
    assert_eq!(chunk.sample_rate, 48_000);
    let mono = AudioChunk::from_planar(&[&left], 48_000);
    assert_eq!(mono.samples, left.to_vec());
}

#[test]
fn a_short_plane_cuts_the_chunk_and_bad_numbers_become_silence() {
    let (left, right) = ([1.0, f32::NAN, 1.0], [1.0, 1.0]);
    let chunk = AudioChunk::from_planar(&[&left, &right], 16_000);
    assert_eq!(chunk.samples, vec![1.0, 0.5]);
    assert!(AudioChunk::from_planar(&[], 16_000).samples.is_empty());
}

#[test]
fn interleaved_frames_are_averaged_and_a_partial_frame_is_dropped() {
    let chunk = AudioChunk::from_interleaved(&[1.0, 0.0, 0.25, 0.75, 0.5], 2, 44_100);
    assert_eq!(chunk.samples, vec![0.5, 0.5]);
    assert_eq!(
        AudioChunk::from_interleaved(&[0.5, f32::INFINITY], 0, 8_000).samples,
        vec![0.5, 0.0]
    );
    assert!((AudioChunk::from_interleaved(&[0.0; 32_000], 2, 16_000).duration_secs() - 1.0).abs() < 1e-9);
}

// Conversion to Scribe's format

#[test]
fn sixteen_kilohertz_passes_through_as_little_endian_samples() {
    let chunk = AudioChunk {
        sample_rate: SAMPLE_RATE,
        samples: vec![0.0, 1.0, -1.0, 0.5, 2.0, -3.0],
    };
    let mut out = Vec::new();
    Converter::new().convert(&chunk, &mut out);
    assert_eq!(decode(&out), vec![0, 32767, -32767, 16384, 32767, -32767]);
    assert_eq!(&out[2..4], &[0xff, 0x7f]);
}

#[test]
fn a_zero_rate_is_ignored_and_other_rates_are_resampled() {
    let mut converter = Converter::new();
    let mut out = Vec::new();
    converter.convert(
        &AudioChunk {
            sample_rate: 0,
            samples: vec![0.5; 100],
        },
        &mut out,
    );
    assert!(out.is_empty());
    let one_second = AudioChunk::from_planar(&[&sine(440.0, 48_000, 1.0, 0.5)], 48_000);
    converter.convert(&one_second, &mut out);
    let samples = decode(&out).len();
    assert!((15_900..=16_000).contains(&samples), "{samples} samples for one second");
}

#[test]
fn a_new_rate_or_a_reset_starts_a_new_stream() {
    let mut converter = Converter::new();
    let mut out = Vec::new();
    let tone = |rate| AudioChunk::from_planar(&[&sine(300.0, rate, 0.5, 0.5)], rate);
    converter.convert(&tone(48_000), &mut out);
    let after_48k = out.len();
    converter.convert(&tone(44_100), &mut out);
    converter.reset();
    converter.convert(&tone(22_050), &mut out);
    let total = decode(&out).len();
    assert!(
        after_48k > 0 && (23_800..=24_000).contains(&total),
        "{total} samples for 1.5 seconds"
    );
}

// The resampler

#[test]
fn silence_and_a_constant_keep_their_level() {
    let out = resample(&[0.25; 48_000], 48_000, 16_000);
    let middle = &out[1_000..out.len() - 1_000];
    assert!(
        middle.iter().all(|sample| (sample - 0.25).abs() < 1e-4),
        "DC gain is one"
    );
    assert!(
        resample(&[0.0; 4_800], 48_000, 16_000)
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn speech_frequencies_keep_their_level_at_common_rates() {
    for rate in [8_000, 22_050, 24_000, 32_000, 44_100, 48_000, 96_000] {
        let out = resample(&sine(1_000.0, rate, 1.0, 0.5), rate, 16_000);
        let level = rms_middle(&out);
        let expected = 0.5 / 2f32.sqrt();
        assert!(
            (level - expected).abs() < 0.01 * expected,
            "{rate} Hz: level {level}, expected {expected}"
        );
    }
}

#[test]
fn frequencies_above_eight_kilohertz_are_filtered_instead_of_folded_back() {
    // At 16 kHz, 12 kHz would alias to 4 kHz, right in the speech band.
    for frequency in [9_000.0, 12_000.0, 20_000.0] {
        let out = resample(&sine(frequency, 48_000, 1.0, 0.5), 48_000, 16_000);
        let level = rms_middle(&out);
        assert!(level < 0.5 * 0.01, "{frequency} Hz leaked through at {level}");
    }
}

#[test]
fn output_length_follows_the_rate_ratio() {
    for (from, to) in [(48_000, 16_000), (44_100, 16_000), (8_000, 16_000), (16_000, 16_000)] {
        let out = resample(&vec![0.1; from as usize], from, to);
        let expected = to as usize;
        assert!(
            out.len() <= expected && out.len() + 400 >= expected,
            "{from} -> {to}: {}",
            out.len()
        );
    }
}

#[test]
fn a_stream_in_chunks_matches_the_stream_at_once() {
    let input = sine(700.0, 44_100, 0.5, 0.8);
    let whole = resample(&input, 44_100, 16_000);
    let mut resampler = Resampler::new(44_100, 16_000);
    let mut chunked = Vec::new();
    for chunk in input.chunks(441) {
        resampler.process(chunk, &mut chunked);
    }
    assert_eq!(chunked.len(), whole.len());
    let largest = whole
        .iter()
        .zip(&chunked)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(largest < 1e-6, "chunked output differs by {largest}");
}

#[test]
fn a_tone_stays_in_phase_with_the_input() {
    // Output sample n falls on input time n / 16000 s, so a 500 Hz sine stays a sine of the
    // same phase rather than arriving late.
    let out = resample(&sine(500.0, 48_000, 1.0, 0.5), 48_000, 16_000);
    let expected = sine(500.0, 16_000, 1.0, 0.5);
    for n in 4_000..12_000 {
        assert!(
            (out[n] - expected[n]).abs() < 0.01,
            "sample {n}: {} vs {}",
            out[n],
            expected[n]
        );
    }
}
