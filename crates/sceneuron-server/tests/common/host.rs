// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A fake host that records what the server asked of it.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use sceneuron_server::{
    AudioSourceInfo, CameraAnalysis, FaceArea, Host, PreviewGap, PreviewImage, SetupReport, SetupStep, SourceInfo,
    camera_to_analyze,
};

/// The smallest byte string that starts and ends like a JPEG.
pub const FAKE_JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0xFF, 0xD9,
];

/// Records what the server asked of the host.
pub struct FakeHost {
    pub attached: Mutex<Vec<String>>,
    /// Every source the server asked to add the voice filter to, oldest first.
    pub voice_attached: Mutex<Vec<String>>,
    /// Every control token the server asked to keep, oldest first.
    pub control_tokens: Mutex<Vec<String>>,
    /// Makes keeping a control token fail, like a full disk.
    pub refuse_control_token: AtomicBool,
    /// What the camera preview answers.
    pub preview: Mutex<Result<PreviewImage, PreviewGap>>,
    /// How many times the server asked for the preview or the analysis.
    pub camera_requests: AtomicUsize,
    /// How many overlays the server asked to add.
    pub overlays: AtomicUsize,
    /// Makes Sceneuron's Stream Deck plugin installed; it is not at first.
    pub stream_deck_installed: AtomicBool,
    /// Every Stream Deck pairing link the server opened, oldest first.
    pub stream_deck_links: Mutex<Vec<String>>,
    /// Makes opening a pairing link fail, like a Mac without the Stream Deck app.
    pub refuse_stream_deck_link: AtomicBool,
}

impl Default for FakeHost {
    fn default() -> Self {
        Self {
            attached: Mutex::default(),
            voice_attached: Mutex::default(),
            control_tokens: Mutex::default(),
            refuse_control_token: AtomicBool::default(),
            preview: Mutex::new(Ok(PreviewImage {
                jpeg: FAKE_JPEG.to_vec(),
                simulated: false,
            })),
            camera_requests: AtomicUsize::default(),
            overlays: AtomicUsize::default(),
            stream_deck_installed: AtomicBool::default(),
            stream_deck_links: Mutex::default(),
            refuse_stream_deck_link: AtomicBool::default(),
        }
    }
}

impl FakeHost {
    /// The control token the server saved last.
    pub fn saved_control_token(&self) -> Option<String> {
        self.control_tokens.lock().unwrap().last().cloned()
    }
}

pub const FAKE_SOURCE: &str = "cam-1";
pub const FAKE_MIC: &str = "mic-1";

impl Host for FakeHost {
    fn list_sources(&self) -> Vec<SourceInfo> {
        let analyzed = self.attached.lock().unwrap().iter().any(|uuid| uuid == FAKE_SOURCE);
        vec![SourceInfo {
            uuid: FAKE_SOURCE.to_owned(),
            name: "Camera".to_owned(),
            kind: "av_capture_input".to_owned(),
            analyzed,
        }]
    }

    fn attach_filter(&self, uuid: &str) -> Result<(), String> {
        if uuid != FAKE_SOURCE {
            return Err(format!("no source {uuid}"));
        }
        self.attached.lock().unwrap().push(uuid.to_owned());
        Ok(())
    }

    fn list_audio_sources(&self) -> Option<Vec<AudioSourceInfo>> {
        let voice = self.voice_attached.lock().unwrap().iter().any(|uuid| uuid == FAKE_MIC);
        Some(vec![AudioSourceInfo {
            uuid: FAKE_MIC.to_owned(),
            name: "Mic/Aux".to_owned(),
            kind: "coreaudio_input_capture".to_owned(),
            global: true,
            voice,
            in_use: voice,
        }])
    }

    fn attach_voice_filter(&self, uuid: &str) -> Result<(), String> {
        if uuid != FAKE_MIC {
            return Err(format!("no audio source {uuid}"));
        }
        self.voice_attached.lock().unwrap().push(uuid.to_owned());
        Ok(())
    }

    fn camera_preview(&self) -> Result<PreviewImage, PreviewGap> {
        self.camera_requests.fetch_add(1, Ordering::SeqCst);
        self.preview.lock().unwrap().clone()
    }

    fn camera_analysis(&self) -> CameraAnalysis {
        self.camera_requests.fetch_add(1, Ordering::SeqCst);
        CameraAnalysis {
            seen: vec!["face in view (0.98)".to_owned()],
            faces: vec![FaceArea {
                x1: 0.25,
                y1: 0.125,
                x2: 0.5,
                y2: 0.5,
                score: 0.98,
            }],
            age_ms: Some(40),
        }
    }

    fn add_overlay(&self) -> Result<String, String> {
        self.overlays.fetch_add(1, Ordering::SeqCst);
        Ok("Sceneuron overlay".to_owned())
    }

    /// Adds what is missing through the other requests, so a second call finds it all present.
    fn set_up(&self) -> Result<SetupReport, String> {
        let overlay = match self.overlays.load(Ordering::SeqCst) {
            0 => SetupStep::added(&self.add_overlay()?),
            _ => SetupStep::present("Sceneuron overlay"),
        };
        let sources = self.list_sources();
        let camera = match sources.iter().find(|source| source.analyzed) {
            Some(source) => SetupStep::present(&source.name),
            None => {
                let source = camera_to_analyze(&sources).ok_or("no camera")?;
                self.attach_filter(&source.uuid)?;
                SetupStep::added(&source.name)
            }
        };
        let mic = &self.list_audio_sources().unwrap_or_default()[0];
        let microphone = if mic.voice {
            SetupStep::present(&mic.name)
        } else {
            self.attach_voice_filter(&mic.uuid)?;
            SetupStep::added(&mic.name)
        };
        Ok(SetupReport {
            overlay,
            camera,
            microphone,
        })
    }

    fn stream_deck_plugin_installed(&self) -> bool {
        self.stream_deck_installed.load(Ordering::SeqCst)
    }

    fn open_stream_deck_link(&self, link: &str) -> Result<(), String> {
        if self.refuse_stream_deck_link.load(Ordering::SeqCst) {
            return Err("no application can open streamdeck links".to_owned());
        }
        self.stream_deck_links.lock().unwrap().push(link.to_owned());
        Ok(())
    }

    fn save_control_token(&self, token: &str) -> Result<(), String> {
        if self.refuse_control_token.load(Ordering::SeqCst) {
            return Err("the disk is full".to_owned());
        }
        self.control_tokens.lock().unwrap().push(token.to_owned());
        Ok(())
    }
}
