// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The editor's camera preview: what camera analysis sees, after any flip, and what on-device
//! analysis found, kept in memory only while the editor asks for them. Nothing here is logged or
//! written anywhere.
//!
//! The video thread calls [`PreviewTap::wants_frame`], a few atomic loads, for every frame. It
//! copies or converts a frame only within [`WATCH_MS`] of the editor's last request, at most one
//! every [`FRAME_INTERVAL_MS`], and never waits for the lock; the request thread encodes the JPEG.
//! Once the editor stops asking, the camera filter's tick drops the frame and the analysis within
//! a video frame, whether or not the camera still sends frames or analysis still runs; removing
//! the filter that fed them drops them at once. Times are on the hub's millisecond clock.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use sceneuron_ai::Frame;
use sceneuron_server::{CameraAnalysis, FaceArea, PREVIEW_MAX_SIDE, PreviewGap};
use sceneuron_vision::{InputSize, Perception, facts};

/// How long one request keeps preview frames coming.
pub const WATCH_MS: u64 = 2_000;
/// The least time between two preview frames: at most five a second.
pub const FRAME_INTERVAL_MS: u64 = 200;
/// A frame or an analysis older than this is not shown: the source stopped sending frames.
pub const STALE_MS: u64 = 2_000;
/// The size the video thread converts a preview frame to when analysis has none to share.
pub const PREVIEW_SIZE: InputSize = InputSize::Fit {
    max_width: PREVIEW_MAX_SIDE,
    max_height: PREVIEW_MAX_SIDE,
};

const NEVER: u64 = u64::MAX;

/// Preview frames and analysis, shared by the camera filter's video and graphics threads, the
/// analysis thread, and the server's request threads.
pub struct PreviewTap {
    /// How many camera analysis filters exist; the vision bridge counts them.
    filters: Arc<AtomicUsize>,
    /// When the editor last asked; [`NEVER`] until it does.
    requested_ms: AtomicU64,
    /// When the editor started asking after a pause.
    watch_since_ms: AtomicU64,
    frame: Held<Frame>,
    analysis: Held<Perception>,
}

impl PreviewTap {
    pub fn new(filters: Arc<AtomicUsize>) -> PreviewTap {
        PreviewTap {
            filters,
            requested_ms: AtomicU64::new(NEVER),
            watch_since_ms: AtomicU64::new(NEVER),
            frame: Held::new(),
            analysis: Held::new(),
        }
    }

    /// The editor asked for the preview at `now_ms`.
    pub fn request(&self, now_ms: u64) {
        let previous = self.requested_ms.swap(now_ms, Ordering::SeqCst);
        if previous == NEVER || now_ms.saturating_sub(previous) > WATCH_MS {
            self.watch_since_ms.store(now_ms, Ordering::SeqCst);
        }
    }

    /// Whether the editor asked within the last [`WATCH_MS`].
    pub fn watching(&self, now_ms: u64) -> bool {
        let requested = self.requested_ms.load(Ordering::SeqCst);
        requested != NEVER && now_ms.saturating_sub(requested) <= WATCH_MS
    }

    /// Whether the video thread should hand over the frame it holds at `now_ms`.
    pub fn wants_frame(&self, now_ms: u64) -> bool {
        let stored = self.frame.captured_ms();
        self.watching(now_ms) && (stored == NEVER || now_ms.saturating_sub(stored) >= FRAME_INTERVAL_MS)
    }

    /// Keeps `frame` as the latest preview frame, unless a request thread holds the lock, in
    /// which case the next frame will do.
    pub fn store(&self, frame: Frame) {
        let captured_ms = frame.captured_ms;
        self.frame.try_put(frame, captured_ms);
    }

    /// The latest frame at `now_ms`, or why there is none. A frame too old to show is dropped.
    pub fn frame(&self, now_ms: u64) -> Result<Frame, PreviewGap> {
        if self.filters.load(Ordering::SeqCst) == 0 {
            return Err(PreviewGap::NoSource);
        }
        if let Some((frame, _)) = self.frame.fresh(now_ms) {
            return Ok(frame);
        }
        let since = self.watch_since_ms.load(Ordering::SeqCst);
        if self.watching(now_ms) && now_ms.saturating_sub(since) <= STALE_MS {
            Err(PreviewGap::Waiting)
        } else {
            Err(PreviewGap::Stale)
        }
    }

    /// Keeps what analysis found in the frame captured at `captured_ms` while the editor asks;
    /// once it stops asking, drops that and the last frame. Runs on the analysis thread, which
    /// may wait for the lock the video thread only tries.
    pub fn observe(&self, perception: &Perception, captured_ms: u64, now_ms: u64) {
        if self.watching(now_ms) {
            self.analysis.put(perception.clone(), captured_ms);
        } else {
            self.release();
        }
    }

    /// What analysis found lately; empty when it found nothing in the last [`STALE_MS`], and
    /// what it found earlier is dropped.
    pub fn analysis(&self, now_ms: u64) -> CameraAnalysis {
        let Some((perception, captured)) = self.analysis.fresh(now_ms) else {
            return CameraAnalysis::default();
        };
        CameraAnalysis {
            seen: facts(&perception),
            faces: perception
                .faces
                .iter()
                .map(|face| FaceArea {
                    x1: face.x1,
                    y1: face.y1,
                    x2: face.x2,
                    y2: face.y2,
                    score: face.score,
                })
                .collect(),
            age_ms: Some(now_ms.saturating_sub(captured)),
        }
    }

    /// Drops the frame and the analysis once the editor stopped asking. Runs on OBS's graphics
    /// thread for every video frame, so it never waits: a lock another thread holds leaves the
    /// drop to the next call, and with nothing to drop it costs a few atomic loads.
    pub fn expire(&self, now_ms: u64) {
        if !self.watching(now_ms) {
            self.frame.try_clear();
            self.analysis.try_clear();
        }
    }

    /// A camera analysis filter was removed; `owned` when it fed analysis, and so the preview.
    /// Drops the frame and the analysis then, and once no filter is left, since no tick of
    /// that filter will.
    pub fn filter_removed(&self, owned: bool) {
        if owned || self.filters.load(Ordering::SeqCst) == 0 {
            self.release();
        }
    }

    /// Drops the frame and the analysis, waiting for the locks.
    pub fn release(&self) {
        self.frame.clear();
        self.analysis.clear();
    }

    /// Whether the tap holds neither a frame nor an analysis.
    pub fn is_empty(&self) -> bool {
        self.frame.captured_ms() == NEVER && self.analysis.captured_ms() == NEVER
    }
}

/// A value and when it was captured. Both change together under the lock, so the time, which
/// reads without it, is [`NEVER`] whenever the lock is free and there is no value.
struct Held<T> {
    captured_ms: AtomicU64,
    value: Mutex<Option<T>>,
}

impl<T: Clone> Held<T> {
    fn new() -> Held<T> {
        Held {
            captured_ms: AtomicU64::new(NEVER),
            value: Mutex::new(None),
        }
    }

    /// When the value was captured; [`NEVER`] when there is none.
    fn captured_ms(&self) -> u64 {
        self.captured_ms.load(Ordering::SeqCst)
    }

    fn lock(&self) -> MutexGuard<'_, Option<T>> {
        self.value.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set(&self, slot: &mut Option<T>, value: Option<T>, captured_ms: u64) {
        self.captured_ms.store(captured_ms, Ordering::SeqCst);
        *slot = value;
    }

    /// Keeps `value`, waiting for the lock.
    fn put(&self, value: T, captured_ms: u64) {
        self.set(&mut self.lock(), Some(value), captured_ms);
    }

    /// Keeps `value` unless another thread holds the lock; never waits.
    fn try_put(&self, value: T, captured_ms: u64) {
        if let Ok(mut slot) = self.value.try_lock() {
            self.set(&mut slot, Some(value), captured_ms);
        }
    }

    /// Drops the value, waiting for the lock.
    fn clear(&self) {
        self.set(&mut self.lock(), None, NEVER);
    }

    /// Drops the value unless there is none or another thread holds the lock; never waits.
    fn try_clear(&self) {
        if self.captured_ms() != NEVER
            && let Ok(mut slot) = self.value.try_lock()
        {
            self.set(&mut slot, None, NEVER);
        }
    }

    /// The value and when it was captured, if at most [`STALE_MS`] before `now_ms`. A staler
    /// value is dropped, since it would never be shown.
    fn fresh(&self, now_ms: u64) -> Option<(T, u64)> {
        let mut slot = self.lock();
        let captured_ms = self.captured_ms();
        if captured_ms != NEVER && now_ms.saturating_sub(captured_ms) <= STALE_MS {
            return slot.clone().map(|value| (value, captured_ms));
        }
        self.set(&mut slot, None, NEVER);
        None
    }
}

/// Whether an analysis sample of `width` x `height` keeps the aspect ratio of a frame of
/// `frame_width` x `frame_height`, so the preview can show a copy of it. A perceiver with a
/// fixed input size stretches frames, and the preview then converts a frame of its own.
pub fn keeps_aspect(width: u32, height: u32, frame_width: u32, frame_height: u32) -> bool {
    let fit = InputSize::Fit {
        max_width: width,
        max_height: height,
    };
    fit.for_frame(frame_width, frame_height) == (width, height)
}
