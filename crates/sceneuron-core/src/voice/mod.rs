// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Voice control and spoken cards: what the microphone heard, what it asks for, and which raised
//! card should be spoken.
//!
//! The host transcribes speech and feeds it to [`Engine::hear`](crate::Engine::hear) as
//! [`Heard`]. Addressed speech, said while push-to-talk is held or starting with the wake word,
//! is a command: lower, pause, resume, a vote, a card named by its name or text, a reaction, a
//! theme, an element to show or hide, a look, or subtitles on, off, or in a language. Anything
//! else addressed goes to the host's AI as a [`VoiceRequest`], and so do the look and subtitles,
//! which only the host can carry out. In always-on mode, the trigger phrases of cards and
//! reactions work whenever they are said, like a hand signal. Only decks whose voice is on take
//! part. Of speech that is not addressed, only a trigger phrase it contains is ever shown or
//! logged, never the other words. [`VoiceGuide`] lists what can be said, from the same tables.

mod acting;
mod actions;
mod changes;
mod commands;
mod deck_commands;
mod guide;
mod guide_entries;
mod hearing;
mod local;
mod matcher;
mod normalize;
mod reacting;
mod requests;
mod scoring;
mod speech;

use serde::{Deserialize, Serialize};

use crate::decks::Card;
use crate::model::ValidationError;

pub use actions::{RoomStep, VoiceAction, VoiceReply, VoiceRequest, VoiceStep, VoiceTask};
pub use guide::{GuideEntry, GuideGroup, VoiceGuide};
use matcher::is_addressed;
pub use matcher::{Intent, interpret};
pub use normalize::{normalize_speech, speakable};
pub use requests::VOICE_REQUEST_TIMEOUT_MS;

/// The wake word used until the host reports another.
pub const DEFAULT_WAKE_WORD: &str = "Sceneuron";
const MAX_PHRASES: usize = 4;
const PHRASE_CHARS: (usize, usize) = (2, 60);
const SPEECH_CHARS: (usize, usize) = (1, 200);
/// Longest heard text kept for the editor and the event log.
const MAX_HEARD_CHARS: usize = 200;

/// Words the host transcribed from the microphone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heard {
    pub text: String,
    /// The transcript of this stretch of speech is complete; partial text is only shown.
    pub is_final: bool,
    /// Said while push-to-talk was held, or after the wake word, so it is a command.
    pub addressed: bool,
}

/// When the microphone listens for commands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceMode {
    /// Only while a push-to-talk control is held; everything said then is a command.
    #[default]
    PushToTalk,
    /// All the time: commands start with the wake word, and trigger phrases need none.
    AlwaysOn,
}

/// How voice control is set up and what it is doing, reported by the host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VoiceStatus {
    /// Voice control is on in the settings; heard words are ignored while it is off.
    pub enabled: bool,
    pub mode: VoiceMode,
    /// The word that addresses Sceneuron in always-on mode.
    pub wake_word: String,
    /// Microphone audio is being transcribed right now.
    pub listening: bool,
    /// Why voice control is off, what it needs, or what it is doing.
    pub message: String,
}

impl VoiceStatus {
    /// Whether `text` is addressed to Sceneuron: always when it was said with push to talk held
    /// (`held`), and in always-on mode also when it starts with the wake word. In push-to-talk
    /// mode the wake word alone addresses nothing, even while subtitles keep the stream open.
    pub(crate) fn addresses(&self, text: &str, held: bool) -> bool {
        held || (self.mode == VoiceMode::AlwaysOn && is_addressed(text, &self.wake_word))
    }
}

impl Default for VoiceStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: VoiceMode::PushToTalk,
            wake_word: DEFAULT_WAKE_WORD.to_owned(),
            listening: false,
            message: String::new(),
        }
    }
}

/// The last addressed words, or the last trigger phrase, and what they did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HeardView {
    pub text: String,
    pub is_final: bool,
    /// The words were typed in the editor rather than heard.
    pub typed: bool,
    pub at_ms: u64,
    /// What it did, such as `raised Meeting signals: Wrap it up`, or why it did nothing; empty
    /// while the words are partial, and saying what it waits for while the host works on it.
    pub result: String,
    /// The deck and card it raised.
    pub deck: Option<String>,
    pub card: Option<String>,
}

/// Voice control as the editor and `GET /api/voice` show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VoiceView {
    pub enabled: bool,
    pub mode: VoiceMode,
    pub listening: bool,
    /// A push-to-talk control is held.
    pub talking: bool,
    pub message: String,
    pub heard: Option<HeardView>,
}

/// A raised card to speak; clients play it once, after `audio` is set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SpeechCue {
    /// What to say: the card's speech line, or its text without emoji.
    pub text: String,
    /// Increases with every cue, like effect nonces.
    pub nonce: u64,
    pub deck: String,
    pub card: String,
    /// When the card was raised, on the snapshot clock.
    pub at_ms: u64,
    /// The id of the generated audio, served at `/api/speech/<audio>`; `None` until the host
    /// generated it.
    pub audio: Option<String>,
}

/// The engine's voice state; kept in memory only, like raised cards.
#[derive(Clone, Debug, Default)]
pub(crate) struct VoiceState {
    pub(crate) status: VoiceStatus,
    pub(crate) talking: bool,
    /// When a held push-to-talk control lets go by itself.
    pub(crate) talk_until_ms: Option<u64>,
    pub(crate) heard: Option<HeardView>,
    pub(crate) speech: Option<SpeechCue>,
    /// What waits for the host, oldest first.
    pub(crate) requests: Vec<VoiceRequest>,
    /// A host answers voice requests; while none does, they are refused at once.
    pub(crate) answered: bool,
    /// The request whose reply the heard line waits for, if it waits.
    pub(crate) awaited: Option<u64>,
}

/// Checks a card's trigger phrases and speech line.
pub(crate) fn validate_card(card: &Card) -> Result<(), ValidationError> {
    if card.phrases.len() > MAX_PHRASES {
        let message = format!("a card has at most {MAX_PHRASES} trigger phrases");
        return Err(ValidationError(message));
    }
    for phrase in &card.phrases {
        let count = phrase.chars().count();
        let allowed = |c: char| c.is_alphanumeric() || " '’,.!?¿¡-".contains(c);
        let wordy = phrase.chars().any(char::is_alphanumeric);
        if !(PHRASE_CHARS.0..=PHRASE_CHARS.1).contains(&count) || !phrase.chars().all(allowed) || !wordy {
            let (min, max) = PHRASE_CHARS;
            let message = format!(
                "trigger phrase {phrase:?} must be {min}-{max} characters of letters, digits, spaces, \
                 apostrophes, and basic punctuation"
            );
            return Err(ValidationError(message));
        }
    }
    if let Some(speech) = &card.speech {
        let count = speech.chars().count();
        if !(SPEECH_CHARS.0..=SPEECH_CHARS.1).contains(&count) || speech.chars().any(char::is_control) {
            let (min, max) = SPEECH_CHARS;
            let message = format!("the speech line must be {min}-{max} characters on one line");
            return Err(ValidationError(message));
        }
    }
    Ok(())
}

/// Keeps heard text on one line and caps its length.
fn clean(text: &str) -> String {
    let flat: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let words: Vec<&str> = flat.split_whitespace().collect();
    words.join(" ").chars().take(MAX_HEARD_CHARS).collect()
}
