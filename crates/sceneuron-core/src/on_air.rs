// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What OBS's program shows and whether it is on air: the current scene's name, whether OBS
//! streams or records, and since when, as the host reports them. `{{on-air}}` in overlay text shows
//! the scene and "LIVE" while OBS streams or records, and `{{on-air-time}}` the time on air, which
//! the overlay counts every second from `since_ms` (see [`crate::template`]).

use serde::Serialize;

use crate::engine::Engine;
use crate::types::Outcome;

/// The most characters of a scene name the overlay shows.
const MAX_SCENE_CHARS: usize = 80;
/// What `{{on-air}}` adds to the scene's name while OBS streams or records.
const LIVE: &str = "LIVE";
/// Between the scene's name and "LIVE".
const SEPARATOR: &str = " \u{b7} ";

/// OBS's program as the host last reported it; kept in memory only.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OnAir {
    scene: String,
    /// When streaming started, on the engine's clock, while OBS streams.
    streaming_since: Option<u64>,
    /// When recording started, on the engine's clock, while OBS records.
    recording_since: Option<u64>,
}

/// OBS's program, for the overlay and the editor.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct OnAirView {
    /// The current scene's name; empty outside OBS until a host reports one.
    pub scene: String,
    pub streaming: bool,
    pub recording: bool,
    /// When OBS went on air, on the same clock as `now_ms`: the start of whichever of streaming
    /// and recording has run longer; `None` while it does neither.
    pub since_ms: Option<u64>,
}

impl OnAir {
    fn since_ms(&self) -> Option<u64> {
        match (self.streaming_since, self.recording_since) {
            (Some(stream), Some(record)) => Some(stream.min(record)),
            (stream, record) => stream.or(record),
        }
    }

    /// What `{{on-air}}` shows: the scene's name, then "LIVE" while OBS is on air.
    pub(crate) fn text(&self) -> String {
        match (self.scene.is_empty(), self.since_ms().is_some()) {
            (_, false) => self.scene.clone(),
            (true, true) => LIVE.to_owned(),
            (false, true) => format!("{}{SEPARATOR}{LIVE}", self.scene),
        }
    }
}

impl Engine {
    /// Takes what the host reports of OBS's program: the current scene's name and whether OBS
    /// streams and records. Streaming and recording each count from the first report that has them
    /// on, so reporting the same state again changes nothing.
    pub fn set_on_air(&mut self, scene: &str, streaming: bool, recording: bool, now_ms: u64) -> Outcome {
        let was_live = self.on_air.since_ms().is_some();
        let next = OnAir {
            scene: clean(scene),
            streaming_since: streaming.then(|| self.on_air.streaming_since.unwrap_or(now_ms)),
            recording_since: recording.then(|| self.on_air.recording_since.unwrap_or(now_ms)),
        };
        if next == self.on_air {
            return Outcome::NONE;
        }
        self.on_air = next;
        let live = self.on_air.since_ms().is_some();
        if live != was_live {
            let detail = if live { "on air" } else { "off air" };
            self.record(now_ms, "obs", detail);
        }
        self.bump(Outcome::CHANGED)
    }

    pub(crate) fn on_air_view(&self) -> OnAirView {
        OnAirView {
            scene: self.on_air.scene.clone(),
            streaming: self.on_air.streaming_since.is_some(),
            recording: self.on_air.recording_since.is_some(),
            since_ms: self.on_air.since_ms(),
        }
    }
}

/// A scene name on one line, without control characters, and at most [`MAX_SCENE_CHARS`] long.
fn clean(scene: &str) -> String {
    let flat: String = scene.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    flat.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_SCENE_CHARS)
        .collect()
}
