// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! WAV files: reading recorded speech to feed the voice input, and writing 16-bit PCM.

use std::fmt;

use crate::audio::AudioChunk;

/// Decoded audio: interleaved samples in -1..=1.
#[derive(Clone, Debug, PartialEq)]
pub struct Wav {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl Wav {
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }

    pub fn duration_secs(&self) -> f64 {
        self.frames() as f64 / f64::from(self.sample_rate.max(1))
    }

    /// The audio as mono chunks of up to `frames` frames each.
    pub fn chunks(&self, frames: usize) -> impl Iterator<Item = AudioChunk> + '_ {
        let channels = usize::from(self.channels.max(1));
        self.samples
            .chunks(frames.max(1) * channels)
            .map(move |chunk| AudioChunk::from_interleaved(chunk, channels, self.sample_rate))
    }
}

/// Why a file could not be read as audio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WavError {
    NotWav,
    Missing(&'static str),
    Malformed(&'static str),
    /// A sample encoding this reader does not decode, by WAVE format tag and bits per sample.
    Unsupported {
        format: u16,
        bits: u16,
    },
}

impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WavError::NotWav => f.write_str("not a WAV file"),
            WavError::Missing(chunk) => write!(f, "the WAV file has no {chunk} chunk"),
            WavError::Malformed(what) => write!(f, "the WAV file has a malformed {what}"),
            WavError::Unsupported { format, bits } => {
                write!(f, "WAV format {format} with {bits}-bit samples is not supported")
            }
        }
    }
}

impl std::error::Error for WavError {}

const PCM: u16 = 1;
const IEEE_FLOAT: u16 = 3;
const EXTENSIBLE: u16 = 0xFFFE;

struct Format {
    tag: u16,
    channels: u16,
    sample_rate: u32,
    bits: u16,
}

/// Reads integer PCM (8, 16, 24, or 32 bits) or float (32 or 64 bits) audio, plain or in the
/// extensible layout. Unknown chunks are skipped, and a data chunk cut short, as some
/// streaming writers leave it, is read as far as it goes.
pub fn read_wav(bytes: &[u8]) -> Result<Wav, WavError> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::NotWav);
    }
    let (mut format, mut data) = (None, None);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = u32_at(bytes, at + 4) as usize;
        let body = &bytes[at + 8..(at + 8).saturating_add(size).min(bytes.len())];
        match &bytes[at..at + 4] {
            b"fmt " => format = Some(parse_format(body)?),
            b"data" => data = Some(body),
            _ => {}
        }
        at = (at + 8).saturating_add(size).saturating_add(size & 1);
    }
    let format = format.ok_or(WavError::Missing("fmt"))?;
    let data = data.ok_or(WavError::Missing("data"))?;
    let decode: fn(&[u8]) -> f32 = match (format.tag, format.bits) {
        (PCM, 8) => |b| (f32::from(b[0]) - 128.0) / 128.0,
        (PCM, 16) => |b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0,
        (PCM, 24) => |b| (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0,
        (PCM, 32) => |b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 2_147_483_648.0,
        (IEEE_FLOAT, 32) => |b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        (IEEE_FLOAT, 64) => |b| f64::from_le_bytes(b[..8].try_into().unwrap_or([0; 8])) as f32,
        (format, bits) => return Err(WavError::Unsupported { format, bits }),
    };
    let width = usize::from(format.bits / 8);
    let samples = data.chunks_exact(width).map(decode).collect();
    Ok(Wav {
        sample_rate: format.sample_rate,
        channels: format.channels,
        samples,
    })
}

fn parse_format(body: &[u8]) -> Result<Format, WavError> {
    if body.len() < 16 {
        return Err(WavError::Malformed("fmt chunk"));
    }
    let mut tag = u16_at(body, 0);
    if tag == EXTENSIBLE {
        // The sub-format GUID starts with the format tag it stands for.
        if body.len() < 26 {
            return Err(WavError::Malformed("extensible fmt chunk"));
        }
        tag = u16_at(body, 24);
    }
    let format = Format {
        tag,
        channels: u16_at(body, 2),
        sample_rate: u32_at(body, 4),
        bits: u16_at(body, 14),
    };
    if format.channels == 0 || format.sample_rate == 0 {
        return Err(WavError::Malformed("fmt chunk"));
    }
    Ok(format)
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// A WAV file of interleaved 16-bit PCM samples.
pub fn pcm16_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let block_align = channels * 2;
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&PCM.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
