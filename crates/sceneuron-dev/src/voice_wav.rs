// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! `--voice-wav`: speaks a WAV file to voice control once, as if push to talk were held while
//! it plays, for demos and tests without a microphone.
//!
//! With `--ai` the file goes to the assistant's voice input, under the voice and subtitle
//! settings in the AI settings file, and its spend joins the AI status; with subtitles on, its
//! words become subtitles too. Without it, a voice input of its own is
//! started with voice control on in push-to-talk mode, since giving the option is the consent,
//! and the seconds of audio it sends and their estimated cost are printed.
//! Either way the file's audio, and the wake word and the names and trigger phrases of cards
//! that voice can raise, as keyterms, go to ElevenLabs speech to text, or to `--voice-url`.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use sceneuron_ai::{AiSettings, Credentials, ElevenLabs, HubSink, VoiceLink, VoiceUsage, env_file_path, follow_voice};
use sceneuron_core::VoiceMode;
use sceneuron_server::{Hub, HubEvent};
use sceneuron_voice::{DEFAULT_ENDPOINT, Wav, read_wav};

/// Named like the other optional keys in `.env.example`.
const KEY_VAR: &str = "SCENEURON_ELEVENLABS_API_KEY";
/// Audio is handed over in pieces this long, at the pace it would be spoken.
const PIECE: Duration = Duration::from_millis(20);
/// How long the player waits for push to talk to reach the voice input.
const HOLD_WAIT: Duration = Duration::from_secs(2);

/// The thread that plays the file, and the voice input it started, if any; stop it before the
/// hub.
pub(crate) struct VoiceWav {
    own: Option<Arc<VoiceLink>>,
    stop: Arc<AtomicBool>,
    player: Option<JoinHandle<()>>,
}

impl VoiceWav {
    pub(crate) fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(player) = self.player.take() {
            let _ = player.join();
        }
        if let Some(link) = self.own.take() {
            link.shutdown();
        }
    }
}

/// Where ElevenLabs is reached: `url` for speech to text when given, else the real API.
pub(crate) fn elevenlabs(url: Option<&str>) -> ElevenLabs {
    ElevenLabs {
        realtime_url: url.unwrap_or(DEFAULT_ENDPOINT).to_owned(),
        ..ElevenLabs::default()
    }
}

/// Starts playing the file at `path` into `assistant_voice`, or else into a voice input of its
/// own with the key from the environment or `env_file`. Returns the file's length in seconds
/// with the handle.
pub(crate) fn start(
    hub: &Arc<Hub>,
    path: &Path,
    url: Option<&str>,
    env_file: &Path,
    assistant_voice: Option<Arc<VoiceLink>>,
) -> Result<(VoiceWav, f64), String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let wav = read_wav(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    let (link, own) = match assistant_voice {
        Some(link) => (link, None),
        None => {
            let link = Arc::new(own_voice(hub, url, env_file)?);
            follow_voice(hub, &link);
            (Arc::clone(&link), Some(link))
        }
    };
    report(hub);
    let stop = Arc::new(AtomicBool::new(false));
    let seconds = wav.duration_secs();
    let (hub, stopping) = (Arc::clone(hub), Arc::clone(&stop));
    let player = thread::Builder::new()
        .name("sceneuron-voice-wav".to_owned())
        .spawn(move || play(&hub, &link, &wav, &stopping))
        .map_err(|error| format!("cannot start playing {}: {error}", path.display()))?;
    let handle = VoiceWav {
        own,
        stop,
        player: Some(player),
    };
    Ok((handle, seconds))
}

/// A voice input with voice control on in push-to-talk mode and the key from the environment
/// or the env file, which prints what it spends.
fn own_voice(hub: &Arc<Hub>, url: Option<&str>, env_file: &Path) -> Result<VoiceLink, String> {
    let (credentials, warnings) = Credentials::load(Some(&env_file_path(env_file)));
    warnings
        .iter()
        .for_each(|warning| eprintln!("sceneuron-dev: {warning}"));
    if credentials.elevenlabs.trim().is_empty() {
        return Err(format!(
            "--voice-wav needs an ElevenLabs API key in {KEY_VAR}, in the environment or in {}",
            env_file_path(env_file).display()
        ));
    }
    let mut settings = AiSettings {
        elevenlabs_api_key: credentials.elevenlabs.clone(),
        ..AiSettings::default()
    };
    settings.voice.enabled = true;
    settings.voice.consent = true;
    settings.voice.mode = VoiceMode::PushToTalk;
    let sink = Arc::new(HubSink::new(Arc::clone(hub)));
    Ok(VoiceLink::standalone(&settings, &elevenlabs(url), sink, print_spend()))
}

/// Prints the audio sent so far and its estimated cost at list prices after each report, since
/// without `--ai` no AI status shows them.
fn print_spend() -> impl FnMut(VoiceUsage, f64) + Send + 'static {
    let mut total = SpendTotal::default();
    move |usage, usd| println!("{}", total.add(usage.audio_seconds, usd))
}

/// The running total of audio sent and its cost across spend reports.
#[derive(Default)]
struct SpendTotal {
    seconds: f64,
    cost: f64,
}

impl SpendTotal {
    fn add(&mut self, seconds: f64, usd: f64) -> String {
        self.seconds += seconds;
        self.cost += usd;
        format!(
            "Voice: sent {:.1} s of audio so far, about ${:.4}",
            self.seconds, self.cost
        )
    }
}

/// Prints each final thing voice control heard and did, and each change of its status.
fn report(hub: &Arc<Hub>) {
    let printed = std::sync::Mutex::new((0u64, String::new()));
    let hub_ref = Arc::downgrade(hub);
    hub.add_listener(Box::new(move |event| {
        let Some(hub) = hub_ref
            .upgrade()
            .filter(|_| matches!(event, HubEvent::StateChanged { .. }))
        else {
            return;
        };
        let view = hub.read(|engine| engine.voice_view());
        let mut printed = printed.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(heard) = view.heard.filter(|heard| heard.is_final && heard.at_ms > printed.0) {
            println!("Voice heard: {:?}: {}", heard.text, heard.result);
            printed.0 = heard.at_ms;
        }
        if view.message != printed.1 {
            println!("Voice: {}", view.message);
            printed.1 = view.message;
        }
    }));
}

/// Holds push to talk while the file plays at its own pace, then lets go.
fn play(hub: &Hub, link: &VoiceLink, wav: &Wav, stop: &AtomicBool) {
    let input = link.input();
    hub.update(|engine, now| engine.set_talking(true, now));
    let waited = Instant::now();
    while !input.wants_audio() && waited.elapsed() < HOLD_WAIT && !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(5));
    }
    if !input.wants_audio() {
        eprintln!(
            "sceneuron-dev: voice control and subtitles are off; turn one on and agree to send audio in the AI tab"
        );
    }
    let frames = (u64::from(wav.sample_rate) * PIECE.as_millis() as u64 / 1000).max(1) as usize;
    let started = Instant::now();
    for (index, chunk) in wav.chunks(frames).enumerate() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        input.feed(&chunk);
        let due = started + PIECE * (index as u32 + 1);
        thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    hub.update(|engine, now| engine.set_talking(false, now));
}

#[cfg(test)]
mod tests {
    use super::SpendTotal;

    #[test]
    fn each_report_adds_to_the_running_total() {
        let mut total = SpendTotal::default();
        assert_eq!(
            total.add(0.6, 0.0001),
            "Voice: sent 0.6 s of audio so far, about $0.0001"
        );
        assert_eq!(
            total.add(1.4, 0.0002),
            "Voice: sent 2.0 s of audio so far, about $0.0003"
        );
    }
}
