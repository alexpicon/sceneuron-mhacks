// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The audio thread's hand-off to the voice thread: a ring of samples allocated once, written by
//! one thread and read by another with atomics only, so writing never locks, allocates, or
//! makes a system call.
//!
//! Audio goes in blocks: a header of the sample rate and the frame count, then the mono
//! samples. A block that does not fit is dropped whole. Positions only grow, so a position is
//! also a point in the stream of audio written, which is how push to talk marks where a press
//! or a release happened.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// Header words before each block's samples: the sample rate and the frame count.
const HEADER: usize = 2;

pub(crate) struct AudioRing {
    /// Each slot holds a header word or a sample's bits.
    slots: Box<[AtomicU32]>,
    mask: usize,
    /// The position after the last block written, published once the whole block is in.
    written: AtomicUsize,
    /// The position after the last block read, so the writer knows what it may reuse.
    read: AtomicUsize,
    /// Held while a block is written, so a second writer drops its block instead of racing.
    writing: AtomicBool,
}

impl AudioRing {
    /// A ring of `capacity` slots, rounded up to a power of two.
    pub(crate) fn new(capacity: usize) -> AudioRing {
        let capacity = capacity.max(HEADER + 1).next_power_of_two();
        AudioRing {
            slots: (0..capacity).map(|_| AtomicU32::new(0)).collect(),
            mask: capacity - 1,
            written: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            writing: AtomicBool::new(false),
        }
    }

    /// Writes a block of `frames` samples at `sample_rate`, sample `n` being `sample(n)`; false
    /// when it does not fit or another thread is writing, and the block is dropped.
    pub(crate) fn write(&self, sample_rate: u32, frames: usize, mut sample: impl FnMut(usize) -> f32) -> bool {
        let Ok(count) = u32::try_from(frames) else {
            return false;
        };
        if frames == 0 || self.writing.swap(true, Ordering::Acquire) {
            return false;
        }
        let start = self.written.load(Ordering::Relaxed);
        let used = start.wrapping_sub(self.read.load(Ordering::Acquire));
        let fits = used + HEADER + frames <= self.slots.len();
        if fits {
            self.slot(start).store(sample_rate, Ordering::Relaxed);
            self.slot(start + 1).store(count, Ordering::Relaxed);
            for n in 0..frames {
                self.slot(start + HEADER + n)
                    .store(sample(n).to_bits(), Ordering::Relaxed);
            }
            self.written.store(start + HEADER + frames, Ordering::Release);
        }
        self.writing.store(false, Ordering::Release);
        fits
    }

    /// The position after the last block written, the point in the audio where "now" is for a
    /// thread that is not writing.
    pub(crate) fn position(&self) -> usize {
        self.written.load(Ordering::Acquire)
    }

    /// Reads the next block that starts before `until`, appending its samples to `out`, and
    /// returns its sample rate; `None` when no such block is waiting.
    pub(crate) fn read_before(&self, until: usize, out: &mut Vec<f32>) -> Option<u32> {
        let start = self.read.load(Ordering::Relaxed);
        let end = self.written.load(Ordering::Acquire);
        if start == end || until.wrapping_sub(start) as isize <= 0 {
            return None;
        }
        let sample_rate = self.slot(start).load(Ordering::Relaxed);
        let frames = self.slot(start + 1).load(Ordering::Relaxed) as usize;
        out.extend((0..frames).map(|n| f32::from_bits(self.slot(start + HEADER + n).load(Ordering::Relaxed))));
        self.read.store(start + HEADER + frames, Ordering::Release);
        Some(sample_rate)
    }

    fn slot(&self, position: usize) -> &AtomicU32 {
        &self.slots[position & self.mask]
    }
}
