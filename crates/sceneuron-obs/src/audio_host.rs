// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The editor's microphone choice inside OBS: the sources that carry audio, OBS's global audio
//! devices from Settings > Audio included, adding the "Sceneuron Voice" filter to one, and giving
//! it to Mic/Aux when OBS is set up in one click.
//!
//! Voice control and subtitles hear one microphone at a time: the first Sceneuron Voice filter
//! that hands over audio feeds them until it is removed (see `voice`). Choosing a microphone in
//! the editor therefore removes the filter from every other source, as choosing a camera does,
//! so the choice is the one they hear.

use std::ffi::c_void;

use sceneuron_obs_sys as sys;
use sceneuron_server::{AudioSourceInfo, SetupStep};

use crate::ffi::{self, SourceRef};
use crate::overlay_source::OVERLAY_ID;
use crate::source_filters::{
    add_filter, filters_of, for_each_live_source, identity, is_type, remove_filters_except, source_by_uuid,
};
use crate::voice_filter::FILTER_ID;

/// The output channels where the OBS frontend keeps its global audio devices: Mic/Aux to
/// Mic/Aux 4 on channels 3 to 6, listed first, then Desktop Audio and Desktop Audio 2 on 1 and 2.
const DEVICE_CHANNELS: [u32; 6] = [3, 4, 5, 6, 1, 2];
/// The channels of the microphone devices, Mic/Aux to Mic/Aux 4.
const MIC_CHANNELS: [u32; 4] = [3, 4, 5, 6];

/// Every source that carries audio, the global audio devices first; `owner` is the id of the
/// voice filter that feeds the voice input, or 0.
pub fn list(owner: usize) -> Vec<AudioSourceInfo> {
    let devices = global_devices(&DEVICE_CHANNELS);
    let mut sources: Vec<AudioSourceInfo> = devices
        .iter()
        .filter(|device| carries_audio(device.as_ptr()))
        .map(|device| describe(device.as_ptr(), true, owner))
        .collect();
    for_each_live_source(|source| {
        let device = devices.iter().any(|device| device.as_ptr() == source);
        if !device && carries_audio(source) {
            sources.push(describe(source, false, owner));
        }
    });
    sources
}

/// Adds the voice filter to the source with this UUID, unless it has one, and removes it from
/// every other source. Runs on the UI thread.
pub fn attach(uuid: &str) -> Result<(), String> {
    let source = source_by_uuid(uuid)?;
    let (_, name, _) = identity(source.as_ptr());
    if is_type(source.as_ptr(), OVERLAY_ID) {
        return Err(format!(
            "{name} plays Sceneuron's own voice, so it cannot be the microphone"
        ));
    }
    if !carries_audio(source.as_ptr()) {
        return Err(format!("{name} carries no audio"));
    }
    if filters_of(source.as_ptr(), FILTER_ID).is_empty() {
        add_filter(source.as_ptr(), FILTER_ID, c"Sceneuron Voice")?;
    }
    remove_filters_except(source.as_ptr(), FILTER_ID);
    ffi::info(&format!("voice control and subtitles listen to {name:?}"));
    Ok(())
}

/// Gives Mic/Aux, the first microphone device OBS has, the voice filter, unless a source has
/// one. Runs on the UI thread.
pub fn set_up() -> SetupStep {
    if let Some(heard) = list(0).into_iter().find(|source| source.voice) {
        return SetupStep::present(&heard.name);
    }
    let Some(mic) = global_devices(&MIC_CHANNELS)
        .into_iter()
        .find(|device| carries_audio(device.as_ptr()))
    else {
        return SetupStep::skipped("OBS has no Mic/Aux device; choose your microphone in Settings > Audio");
    };
    let (uuid, name, _) = identity(mic.as_ptr());
    attach(&uuid).map_or_else(|error| SetupStep::skipped(&error), |()| SetupStep::added(&name))
}

/// The global audio devices that are set on `channels`, in that order.
fn global_devices(channels: &[u32]) -> Vec<SourceRef> {
    channels
        .iter()
        // SAFETY: any channel number is accepted; returns an owned reference or null.
        .filter_map(|&channel| unsafe { SourceRef::from_owned((sys::obs().obs_get_output_source)(channel)) })
        .collect()
}

/// An input that outputs audio, other than a Sceneuron overlay, whose audio is Sceneuron's own
/// spoken cards.
fn carries_audio(source: *mut sys::obs_source_t) -> bool {
    // SAFETY: `source` is live; both getters only read it.
    let (kind, flags) = unsafe {
        (
            (sys::obs().obs_source_get_type)(source),
            (sys::obs().obs_source_get_output_flags)(source),
        )
    };
    kind == sys::OBS_SOURCE_TYPE_INPUT && flags & sys::OBS_SOURCE_AUDIO != 0 && !is_type(source, OVERLAY_ID)
}

fn describe(source: *mut sys::obs_source_t, global: bool, owner: usize) -> AudioSourceInfo {
    let (uuid, name, kind) = identity(source);
    let filters = filters_of(source, FILTER_ID);
    // A filter's private data is the address its create callback returned, which is the id it
    // claims the voice input with.
    let in_use = owner != 0
        && filters.iter().any(|filter| {
            // SAFETY: `filter` is a live source held by the reference above.
            let data = unsafe { (sys::obs().obs_obj_get_data)(filter.as_ptr().cast::<c_void>()) };
            data as usize == owner
        });
    AudioSourceInfo {
        uuid,
        name,
        kind,
        global,
        voice: !filters.is_empty(),
        in_use,
    }
}
