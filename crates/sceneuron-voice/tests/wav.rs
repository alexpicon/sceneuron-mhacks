// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises WAV reading and writing: 16-bit round trips, other integer and float encodings,
//! the extensible layout, skipped and padded chunks, a cut-short data chunk, mono chunks for
//! feeding, and the errors for files that are not usable audio.

use sceneuron_voice::{Wav, WavError, pcm16_wav, read_wav};

/// A WAV file with a format chunk of the given fields and a data chunk.
fn wav(format: u16, channels: u16, rate: u32, bits: u16, data: &[u8], extensible: bool) -> Vec<u8> {
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&(if extensible { 0xFFFE } else { format }).to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&rate.to_le_bytes());
    let block = channels * bits / 8;
    fmt.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
    fmt.extend_from_slice(&block.to_le_bytes());
    fmt.extend_from_slice(&bits.to_le_bytes());
    if extensible {
        fmt.extend_from_slice(&22u16.to_le_bytes());
        fmt.extend_from_slice(&bits.to_le_bytes());
        fmt.extend_from_slice(&0u32.to_le_bytes());
        fmt.extend_from_slice(&format.to_le_bytes());
        fmt.extend_from_slice(&[0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xAA, 0, 0x38, 0x9B, 0x71]);
    }
    let mut body = b"WAVE".to_vec();
    for (id, chunk) in [(b"fmt ", fmt.as_slice()), (b"data", data)] {
        body.extend_from_slice(id);
        body.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        body.extend_from_slice(chunk);
    }
    let mut file = b"RIFF".to_vec();
    file.extend_from_slice(&(body.len() as u32).to_le_bytes());
    file.extend_from_slice(&body);
    file
}

#[test]
fn sixteen_bit_files_round_trip() {
    let samples = [0i16, 16384, -16384, 32767, -32768, 1];
    let read = read_wav(&pcm16_wav(22_050, 2, &samples)).unwrap();
    assert_eq!((read.sample_rate, read.channels, read.frames()), (22_050, 2, 3));
    let expected: Vec<f32> = samples.iter().map(|s| f32::from(*s) / 32768.0).collect();
    assert_eq!(read.samples, expected);
    assert!((read.duration_secs() - 3.0 / 22_050.0).abs() < 1e-12);
}

#[test]
fn other_integer_widths_and_floats_decode_to_the_same_scale() {
    let eight = read_wav(&wav(1, 1, 8_000, 8, &[128, 255, 0], false)).unwrap();
    assert_eq!(eight.samples, vec![0.0, 127.0 / 128.0, -1.0]);
    let twenty_four = read_wav(&wav(1, 1, 8_000, 24, &[0, 0, 0x40, 0, 0, 0xC0], false)).unwrap();
    assert_eq!(twenty_four.samples, vec![0.5, -0.5]);
    let thirty_two = read_wav(&wav(1, 1, 8_000, 32, &(i32::MIN / 2).to_le_bytes(), false)).unwrap();
    assert_eq!(thirty_two.samples, vec![-0.5]);
    let float: Vec<u8> = [0.25f32, -1.0].iter().flat_map(|s| s.to_le_bytes()).collect();
    assert_eq!(
        read_wav(&wav(3, 1, 48_000, 32, &float, false)).unwrap().samples,
        vec![0.25, -1.0]
    );
    let double: Vec<u8> = 0.75f64.to_le_bytes().to_vec();
    assert_eq!(
        read_wav(&wav(3, 1, 48_000, 64, &double, false)).unwrap().samples,
        vec![0.75]
    );
}

#[test]
fn the_extensible_layout_reads_its_sub_format() {
    let float: Vec<u8> = [0.5f32, 0.25].iter().flat_map(|s| s.to_le_bytes()).collect();
    let read = read_wav(&wav(3, 2, 48_000, 32, &float, true)).unwrap();
    assert_eq!((read.channels, read.samples.clone()), (2, vec![0.5, 0.25]));
}

#[test]
fn unknown_and_odd_sized_chunks_are_skipped() {
    let plain = pcm16_wav(16_000, 1, &[100, -100]);
    let mut file = plain[..12].to_vec();
    file.extend_from_slice(b"LIST");
    file.extend_from_slice(&3u32.to_le_bytes());
    file.extend_from_slice(b"abc\0");
    file.extend_from_slice(&plain[12..]);
    assert_eq!(read_wav(&file).unwrap(), read_wav(&plain).unwrap());
}

#[test]
fn a_data_chunk_cut_short_is_read_as_far_as_it_goes() {
    let mut file = pcm16_wav(16_000, 1, &[1000, 2000, 3000]);
    file.truncate(file.len() - 1);
    assert_eq!(read_wav(&file).unwrap().samples.len(), 2);
}

#[test]
fn mono_chunks_cover_the_file() {
    let samples: Vec<i16> = (0..1000).map(|n| (n % 200) as i16 * 100).collect();
    let read: Wav = read_wav(&pcm16_wav(8_000, 2, &samples)).unwrap();
    let chunks: Vec<_> = read.chunks(160).collect();
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks.iter().map(|chunk| chunk.samples.len()).sum::<usize>(), 500);
    assert!(chunks.iter().all(|chunk| chunk.sample_rate == 8_000));
    assert_eq!(chunks[0].samples[1], (200.0 + 300.0) / 2.0 / 32768.0);
}

#[test]
fn unusable_files_say_why() {
    assert_eq!(read_wav(b"not a wav file at all"), Err(WavError::NotWav));
    assert_eq!(read_wav(b"RIFF\x04\0\0\0WAVE"), Err(WavError::Missing("fmt")));
    let no_data = &pcm16_wav(16_000, 1, &[])[..36];
    assert_eq!(read_wav(no_data), Err(WavError::Missing("data")));
    let mut short_format = pcm16_wav(16_000, 1, &[0]);
    short_format[16] = 8;
    assert_eq!(read_wav(&short_format), Err(WavError::Malformed("fmt chunk")));
    let unsupported = wav(2, 1, 8_000, 4, &[0], false);
    assert_eq!(
        read_wav(&unsupported),
        Err(WavError::Unsupported { format: 2, bits: 4 })
    );
    assert_eq!(
        read_wav(&wav(1, 0, 8_000, 16, &[0, 0], false)),
        Err(WavError::Malformed("fmt chunk"))
    );
    assert_eq!(WavError::Missing("data").to_string(), "the WAV file has no data chunk");
}
