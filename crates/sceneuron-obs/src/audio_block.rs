// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Views a block of OBS filter audio as float planes for the voice input, without copying or
//! allocating, since it runs on OBS's audio thread.

use sceneuron_obs_sys as sys;

/// The planes of one block, borrowed from OBS for the length of the filter callback.
pub struct Planes<'a> {
    planes: [&'a [f32]; sys::MAX_AV_PLANES],
    count: usize,
}

impl<'a> Planes<'a> {
    pub fn as_slice(&self) -> &[&'a [f32]] {
        &self.planes[..self.count]
    }
}

/// The first `channels` planes of `block`; `None` when one of them is missing or not aligned
/// for floats. More channels than OBS has planes count as the planes there are.
///
/// # Safety
/// Each of the first `channels` planes of `block` must be null or point to `block.frames`
/// readable floats that stay valid for `'a`, which OBS guarantees for the audio it hands a
/// filter until the callback returns.
pub unsafe fn planes<'a>(block: &sys::obs_audio_data, channels: usize) -> Option<Planes<'a>> {
    let count = channels.min(sys::MAX_AV_PLANES);
    let frames = block.frames as usize;
    let mut planes: [&[f32]; sys::MAX_AV_PLANES] = [&[]; sys::MAX_AV_PLANES];
    for (plane, &data) in planes.iter_mut().zip(&block.data).take(count) {
        let samples = data.cast::<f32>();
        if samples.is_null() || !samples.is_aligned() {
            return None;
        }
        // SAFETY: the caller guarantees `frames` readable floats for 'a, and the pointer is
        // aligned.
        *plane = unsafe { std::slice::from_raw_parts(samples, frames) };
    }
    Some(Planes { planes, count })
}
