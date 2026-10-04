// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Simulated subtitles in the development server: while subtitles are on and agreed to, a
//! scripted speaker says a line every few seconds, word by word as partial words and then as a
//! final segment, and each target language gets a canned translation shortly after, as if the
//! provider had answered. Its English shows as detected while the spoken language is left to
//! detection. The settings are checked like the real ones and live in memory; nothing leaves the
//! computer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use sceneuron_ai::{SubtitleSettings, SubtitleSettingsUpdate};
use sceneuron_core::{Heard, SubtitleStatus};
use sceneuron_server::{AssistantError, Hub};
use sceneuron_voice::VoiceStatus as InputStatus;
use serde_json::{Value, json};

const SIMULATED: &str = "Simulated subtitles: a scripted speaker, and nothing is sent";
/// Between the words of a line, while it is being said.
const WORD_GAP: Duration = Duration::from_millis(150);
/// From a final segment to its translations.
const TRANSLATION_DELAY: Duration = Duration::from_millis(400);
/// From one line to the next: longer than the default clear delay, so the bars hide between lines.
const LINE_GAP: Duration = Duration::from_millis(5_000);
/// How often a paused speaker looks whether subtitles came on.
const IDLE: Duration = Duration::from_millis(100);

/// What the scripted speaker says in English, each line with its translations by language; the
/// constructed languages are in their romanization, as a provider would answer.
const SCRIPT: [(&str, &[(&str, &str)]); 3] = [
    (
        "Good morning everyone, and welcome to the stream.",
        &[
            ("es", "Buenos días a todos y bienvenidos a la transmisión."),
            ("fr", "Bonjour à tous et bienvenue sur le stream."),
            ("pt", "Bom dia a todos e bem-vindos à transmissão."),
            ("ja", "皆さん、おはようございます。配信へようこそ。"),
            ("qya", "Elen síla lúmenn' omentielvo!"),
            ("sjn", "Mae govannen, mellyn!"),
            ("tlh", "maj po, Hoch! nuqneH!"),
            ("tok", "suno pona, jan ale! kama pona tawa toki ni."),
        ],
    ),
    (
        "Today we are planning the next sprint together.",
        &[
            ("es", "Hoy planificamos juntos el próximo sprint."),
            ("fr", "Aujourd'hui, nous planifions ensemble le prochain sprint."),
            ("pt", "Hoje planejamos juntos o próximo sprint."),
            ("ja", "今日は一緒に次のスプリントを計画します。"),
            ("qya", "Síra carilmë vinya tië."),
            ("sjn", "Si ceriam lend vîn."),
            ("tlh", "DaHjaj Qu' veb wInab."),
            ("tok", "tenpo suno ni la mi mute li pali e tenpo kama."),
        ],
    ),
    (
        "Let's start with the questions from the chat.",
        &[
            ("es", "Empecemos con las preguntas del chat."),
            ("fr", "Commençons par les questions du chat."),
            ("pt", "Vamos começar com as perguntas do chat."),
            ("ja", "チャットの質問から始めましょう。"),
            ("qya", "Nai yestuvalvë!"),
            ("sjn", "Lasto beth lammen!"),
            ("tlh", "ghItlhmey wIlaD. Qapla'!"),
            ("tok", "o open e toki tan jan pi tomo toki!"),
        ],
    ),
];
/// The language speech to text would report for the scripted speaker when the spoken language is
/// left to detection.
const DETECTED: &str = "eng";

/// The simulated subtitle settings and the running speaker.
#[derive(Default)]
pub(crate) struct DemoSubtitles {
    settings: Mutex<SubtitleSettings>,
    stop: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl DemoSubtitles {
    /// Adds the subtitle settings, with simulated prices of nothing, to the AI settings JSON.
    pub(crate) fn add_to(&self, json: &mut Value) {
        let mut view = serde_json::to_value(&*lock(&self.settings)).unwrap_or_else(|_| json!({}));
        view["translation_price_known"] = json!(true);
        view["translation_input_price_per_mtok"] = json!(0.0);
        view["translation_output_price_per_mtok"] = json!(0.0);
        if let Some(fields) = json.as_object_mut() {
            fields.insert("subtitles".to_owned(), view);
        }
    }

    /// The settings with the `subtitles` object of an update applied, or why it is refused; the
    /// caller stores them with [`DemoSubtitles::set`] once the whole update is accepted.
    pub(crate) fn updated(&self, update: &Value) -> Result<SubtitleSettings, AssistantError> {
        let current = lock(&self.settings).clone();
        let Some(subtitles) = update.get("subtitles") else {
            return Ok(current);
        };
        let change: SubtitleSettingsUpdate = serde_json::from_value(subtitles.clone())
            .map_err(|error| AssistantError::new(400, format!("invalid subtitle settings: {error}")))?;
        let next = current.updated(&change);
        next.validate()
            .map_err(|error| AssistantError::new(400, error.to_string()))?;
        Ok(next)
    }

    pub(crate) fn set(&self, settings: SubtitleSettings) {
        *lock(&self.settings) = settings;
    }

    pub(crate) fn settings(&self) -> SubtitleSettings {
        lock(&self.settings).clone()
    }

    /// The status the engine shows, which says that subtitles are simulated, with the speaker's
    /// English as if detected while the spoken language is left to detection.
    pub(crate) fn status(&self, has_key: bool) -> SubtitleStatus {
        let settings = lock(&self.settings);
        let mut status = settings.status(has_key, &InputStatus::Listening, false, &settings.language);
        if status.enabled {
            SIMULATED.clone_into(&mut status.message);
            if settings.language.is_empty() {
                DETECTED.clone_into(&mut status.detected);
            }
        }
        status
    }
}

/// Starts the scripted speaker, which says lines while subtitles are on.
pub(crate) fn start(hub: &Arc<Hub>, state: &Arc<DemoSubtitles>) -> std::io::Result<JoinHandle<()>> {
    let (hub, state) = (Arc::downgrade(hub), Arc::clone(state));
    thread::Builder::new()
        .name("sceneuron-demo-subtitles".to_owned())
        .spawn(move || {
            let mut line = 0;
            while !state.stop.load(Ordering::SeqCst) {
                let Some(hub) = hub.upgrade() else {
                    return;
                };
                if !hub.read(|engine| engine.subtitle_status().enabled) {
                    thread::sleep(IDLE);
                    continue;
                }
                say(&hub, &state, SCRIPT[line % SCRIPT.len()]);
                line += 1;
                pause(&state, LINE_GAP);
            }
        })
}

/// Stops the speaker; the caller joins its thread.
pub(crate) fn stop(state: &DemoSubtitles) {
    state.stop.store(true, Ordering::SeqCst);
}

/// Says one line word by word, then answers its translation job.
fn say(hub: &Hub, state: &DemoSubtitles, (english, translations): (&str, &[(&str, &str)])) {
    let words: Vec<&str> = english.split_whitespace().collect();
    for count in 1..words.len() {
        let heard = Heard {
            text: words[..count].join(" "),
            is_final: false,
            addressed: false,
        };
        hub.update(|engine, now| engine.hear(&heard, now));
        pause(state, WORD_GAP);
    }
    let heard = Heard {
        text: english.to_owned(),
        is_final: true,
        addressed: false,
    };
    hub.update(|engine, now| engine.hear(&heard, now));
    pause(state, TRANSLATION_DELAY);
    for job in hub.read(|engine| engine.translation_jobs(0)) {
        let translations = job
            .targets
            .iter()
            .map(|code| {
                let language = code.split('-').next().unwrap_or_default();
                let canned = translations
                    .iter()
                    .find(|(listed, _)| listed.eq_ignore_ascii_case(language));
                let text = canned.map_or_else(|| format!("[{code}] {}", job.text), |(_, text)| (*text).to_owned());
                (code.clone(), text)
            })
            .collect();
        hub.update(|engine, now| engine.subtitles_translated(job.id, &translations, now));
    }
}

fn pause(state: &DemoSubtitles, length: Duration) {
    let mut left = length;
    while !left.is_zero() && !state.stop.load(Ordering::SeqCst) {
        let step = left.min(IDLE);
        thread::sleep(step);
        left -= step;
    }
}
