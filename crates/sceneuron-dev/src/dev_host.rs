// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A stand-in for OBS with two simulated cameras, so every editor control can be tried. With
//! `--demo-presence` it also has a camera picture and microphones, all simulated; without, the
//! Camera & mic tab says that Sceneuron is not running inside OBS. With `--stream-deck-links`,
//! Sceneuron's Stream Deck plugin counts as installed, and pairing links go to a file instead of
//! the Stream Deck app.

use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sceneuron_server::{
    AudioSourceInfo, CameraAnalysis, Host, Hub, PreviewGap, PreviewImage, SetupReport, SetupStep, SourceInfo,
    camera_to_analyze,
};

use crate::demo_devices::DemoDevices;

const NOT_IN_OBS: &str = "Sceneuron is not running inside OBS";

pub(crate) struct DevHost {
    sources: Mutex<Vec<SourceInfo>>,
    overlays: AtomicUsize,
    demo: Option<DemoDevices>,
    /// Where pairing links go; `None` when the Stream Deck plugin is not installed.
    stream_deck_links: Option<PathBuf>,
}

impl DevHost {
    /// The host, with the simulated picture and microphones when `demo` holds the hub that demo
    /// presence drives, and the Stream Deck plugin installed when `stream_deck_links` says where
    /// its pairing links go.
    pub(crate) fn new(demo: Option<Arc<Hub>>, stream_deck_links: Option<PathBuf>) -> Self {
        let source = |uuid: &str, name: &str, kind: &str| SourceInfo {
            uuid: uuid.to_owned(),
            name: name.to_owned(),
            kind: kind.to_owned(),
            analyzed: false,
        };
        Self {
            sources: Mutex::new(vec![
                source(
                    "5f0c1d2e-0000-4000-8000-000000000001",
                    "Webcam (simulated)",
                    "av_capture_input",
                ),
                source(
                    "5f0c1d2e-0000-4000-8000-000000000002",
                    "Capture card (simulated)",
                    "av_capture_input",
                ),
            ]),
            overlays: AtomicUsize::new(0),
            demo: demo.map(DemoDevices::new),
            stream_deck_links,
        }
    }

    fn sources(&self) -> std::sync::MutexGuard<'_, Vec<SourceInfo>> {
        self.sources.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The simulated devices, once a simulated camera is analyzed.
    fn analyzing(&self) -> Result<&DemoDevices, PreviewGap> {
        let demo = self.demo.as_ref().ok_or(PreviewGap::NotInObs)?;
        let analyzed = self.sources().iter().any(|source| source.analyzed);
        if analyzed { Ok(demo) } else { Err(PreviewGap::NoSource) }
    }
}

impl Host for DevHost {
    fn list_sources(&self) -> Vec<SourceInfo> {
        self.sources().clone()
    }

    fn attach_filter(&self, uuid: &str) -> Result<(), String> {
        let mut sources = self.sources();
        let source = sources
            .iter_mut()
            .find(|source| source.uuid == uuid)
            .ok_or_else(|| format!("no source has the UUID {uuid:?}"))?;
        source.analyzed = true;
        println!("Analyzing {}", source.name);
        Ok(())
    }

    fn list_audio_sources(&self) -> Option<Vec<AudioSourceInfo>> {
        self.demo.as_ref().map(DemoDevices::microphones)
    }

    fn attach_voice_filter(&self, uuid: &str) -> Result<(), String> {
        let demo = self.demo.as_ref().ok_or(NOT_IN_OBS)?;
        let name = demo.attach_voice(uuid)?;
        println!("Voice control and subtitles listen to {name}");
        Ok(())
    }

    fn camera_preview(&self) -> Result<PreviewImage, PreviewGap> {
        self.analyzing()?.preview()
    }

    fn camera_analysis(&self) -> CameraAnalysis {
        self.analyzing().map(DemoDevices::analysis).unwrap_or_default()
    }

    fn add_overlay(&self) -> Result<String, String> {
        let count = self.overlays.fetch_add(1, Ordering::Relaxed) + 1;
        let name = overlay_name(count);
        println!("Added {name} to the current scene");
        Ok(name)
    }

    /// The simulated scene is the current one, so any overlay added is in it; without
    /// `--demo-presence` there are no microphones.
    fn set_up(&self) -> Result<SetupReport, String> {
        let overlay = match self.overlays.load(Ordering::Relaxed) {
            0 => self
                .add_overlay()
                .map_or_else(|error| SetupStep::skipped(&error), |name| SetupStep::added(&name)),
            _ => SetupStep::present(&overlay_name(1)),
        };
        let sources = self.list_sources();
        let camera = match sources.iter().find(|source| source.analyzed) {
            Some(source) => SetupStep::present(&source.name),
            None => match camera_to_analyze(&sources) {
                Some(source) => self
                    .attach_filter(&source.uuid)
                    .map_or_else(|error| SetupStep::skipped(&error), |()| SetupStep::added(&source.name)),
                None => SetupStep::skipped("there is no camera device to analyze"),
            },
        };
        let microphone = self
            .demo
            .as_ref()
            .map_or_else(|| SetupStep::skipped(NOT_IN_OBS), set_up_microphone);
        Ok(SetupReport {
            overlay,
            camera,
            microphone,
        })
    }

    fn stream_deck_plugin_installed(&self) -> bool {
        self.stream_deck_links.is_some()
    }

    /// Appends the link to the `--stream-deck-links` file, which only its owner can read, so a
    /// test can trade its code as the plugin would; the terminal never shows the code.
    fn open_stream_deck_link(&self, link: &str) -> Result<(), String> {
        let path = self
            .stream_deck_links
            .as_deref()
            .ok_or("Sceneuron's Stream Deck plugin is not installed")?;
        let written = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path)
            .and_then(|mut file| writeln!(file, "{link}"));
        written.map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        println!(
            "Sent a Stream Deck pairing link to {} instead of opening it",
            path.display()
        );
        Ok(())
    }

    /// A revoked control token stays revoked until the server stops; the next start uses
    /// `--control-token` or a new random token again.
    fn save_control_token(&self, _token: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Gives the simulated Mic/Aux the voice filter, unless a microphone has it.
fn set_up_microphone(demo: &DemoDevices) -> SetupStep {
    let microphones = demo.microphones();
    if let Some(heard) = microphones.iter().find(|microphone| microphone.voice) {
        return SetupStep::present(&heard.name);
    }
    // OBS lists Mic/Aux first among its global audio devices, and so does the simulation.
    let Some(mic_aux) = microphones.first() else {
        return SetupStep::skipped("there is no Mic/Aux device");
    };
    match demo.attach_voice(&mic_aux.uuid) {
        Ok(name) => {
            println!("Voice control and subtitles listen to {name}");
            SetupStep::added(&name)
        }
        Err(error) => SetupStep::skipped(&error),
    }
}

/// The name of the `count`th overlay added.
fn overlay_name(count: usize) -> String {
    if count == 1 {
        "Sceneuron overlay".to_owned()
    } else {
        format!("Sceneuron overlay {count}")
    }
}
