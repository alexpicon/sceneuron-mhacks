// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The simulated camera picture and microphones that `--demo-presence` adds to the development
//! host, so the editor's Camera & mic tab can be tried without OBS. The picture is drawn, never
//! a camera frame, and the microphones hear nothing.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use sceneuron_ai::encode_jpeg;
use sceneuron_core::Presence;
use sceneuron_server::{AudioSourceInfo, CameraAnalysis, Hub, PREVIEW_MAX_SIDE, PreviewGap, PreviewImage};

use crate::demo_picture;

pub(crate) struct DemoDevices {
    hub: Arc<Hub>,
    started: Instant,
    microphones: Mutex<Vec<AudioSourceInfo>>,
}

impl DemoDevices {
    pub(crate) fn new(hub: Arc<Hub>) -> Self {
        let device = |uuid: &str, name: &str, kind: &str| AudioSourceInfo {
            uuid: uuid.to_owned(),
            name: name.to_owned(),
            kind: kind.to_owned(),
            global: true,
            voice: false,
            in_use: false,
        };
        Self {
            hub,
            started: Instant::now(),
            microphones: Mutex::new(vec![
                device(
                    "5f0c1d2e-0000-4000-8000-000000000011",
                    "Mic/Aux (simulated)",
                    "coreaudio_input_capture",
                ),
                device(
                    "5f0c1d2e-0000-4000-8000-000000000012",
                    "Desktop Audio (simulated)",
                    "coreaudio_output_capture",
                ),
            ]),
        }
    }

    pub(crate) fn microphones(&self) -> Vec<AudioSourceInfo> {
        self.lock().clone()
    }

    /// Moves the simulated voice filter to the microphone with this UUID, as OBS does; the one
    /// filter is always the one heard.
    pub(crate) fn attach_voice(&self, uuid: &str) -> Result<String, String> {
        let mut microphones = self.lock();
        if !microphones.iter().any(|microphone| microphone.uuid == uuid) {
            return Err(format!("no audio source has the UUID {uuid:?}"));
        }
        let mut chosen = String::new();
        for microphone in microphones.iter_mut() {
            let this = microphone.uuid == uuid;
            (microphone.voice, microphone.in_use) = (this, this);
            if this {
                chosen.clone_from(&microphone.name);
            }
        }
        Ok(chosen)
    }

    /// The drawn picture now.
    pub(crate) fn preview(&self) -> Result<PreviewImage, PreviewGap> {
        let frame = demo_picture::draw(self.present(), self.elapsed_ms());
        let jpeg = encode_jpeg(&frame, PREVIEW_MAX_SIDE).map_err(|_| PreviewGap::Stale)?;
        Ok(PreviewImage { jpeg, simulated: true })
    }

    /// What analysis would find in the drawn picture: the face while demo presence is in view.
    pub(crate) fn analysis(&self) -> CameraAnalysis {
        let (seen, faces) = if self.present() {
            (vec!["face in view (0.95)".to_owned()], vec![demo_picture::face()])
        } else {
            (Vec::new(), Vec::new())
        };
        CameraAnalysis {
            seen,
            faces,
            age_ms: Some(0),
        }
    }

    fn present(&self) -> bool {
        self.hub.read(|engine| engine.presence()) == Presence::Present
    }

    fn elapsed_ms(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn lock(&self) -> MutexGuard<'_, Vec<AudioSourceInfo>> {
        self.microphones.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
