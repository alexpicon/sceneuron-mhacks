// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The engine's side of voice control: heard or typed words become commands, push-to-talk is held
//! and let go, and the host reports how voice control is set up.

use super::matcher::{Intent, interpret};
use super::normalize::{literal_words, words};
use super::scoring::contains;
use super::{Heard, HeardView, VoiceStatus, VoiceView, clean};
use crate::engine::Engine;
use crate::types::Outcome;

/// How long a push-to-talk control stays held without news; a control that never reports its
/// release, such as a Stream Deck that went away, stops the microphone after this.
const MAX_TALK_MS: u64 = 60_000;
/// How long after a card was raised its spoken words, heard back through the microphone, are
/// taken for an echo rather than for speech: time to generate the audio, say up to 200
/// characters, and transcribe them.
const ECHO_MS: u64 = 20_000;

impl Engine {
    /// Acts on words heard at `now_ms`: they become subtitles while subtitles are on (see
    /// [`crate::SubtitlesView`]), and voice control reads them while it is on.
    ///
    /// For voice control, words are addressed when said with push to talk held, and in always-on
    /// mode also when they start with the wake word; in push-to-talk mode the wake word alone is
    /// not a command, even while subtitles keep the stream open. Partial words only update the
    /// heard line, as throttled telemetry, and only while they are addressed. Final words are read
    /// as a command, a card, a reaction, or a trigger phrase (see [`interpret`]), act like a manual
    /// change, and land in the event log; what only the host can do waits for it as a
    /// [`VoiceRequest`](super::VoiceRequest). Trigger phrases are automation, so they do nothing
    /// while automation is paused, they never raise the card their deck already shows, and a
    /// reaction's phrase plays at most once per cooldown. Of words not addressed to Sceneuron,
    /// only a trigger phrase they contain is shown or logged, never the rest; words that contain
    /// none, or that repeat a card just spoken, are dropped without a trace, and so is everything
    /// while voice control is off.
    pub fn hear(&mut self, heard: &Heard, now_ms: u64) -> Outcome {
        let subtitles = self.hear_subtitles(heard, now_ms);
        let voice = self.hear_command(heard, now_ms);
        Outcome {
            changed: subtitles.changed || voice.changed,
            project_dirty: voice.project_dirty,
            telemetry: subtitles.telemetry || voice.telemetry,
        }
    }

    /// Carries out typed words as a command said with push to talk held, whether or not voice
    /// control is on, so commands can be tried without speaking; they never become subtitles.
    pub fn type_command(&mut self, text: &str, now_ms: u64) -> Outcome {
        let text = clean(text);
        if text.is_empty() {
            return Outcome::NONE;
        }
        self.follow(&text, true, true, now_ms)
    }

    fn hear_command(&mut self, heard: &Heard, now_ms: u64) -> Outcome {
        let text = clean(&heard.text);
        let status = &self.voice.status;
        if !status.enabled || text.is_empty() {
            return Outcome::NONE;
        }
        let addressed = status.addresses(&text, heard.addressed);
        if !heard.is_final {
            if !addressed {
                return Outcome::NONE;
            }
            // The line shows the new words now, so an older request's reply only goes to the log.
            self.voice.awaited = None;
            self.voice.heard = Some(HeardView {
                text,
                is_final: false,
                typed: false,
                at_ms: now_ms,
                result: String::new(),
                deck: None,
                card: None,
            });
            return Outcome {
                telemetry: true,
                ..Outcome::NONE
            };
        }
        if !addressed && self.echoes_speech(&text, now_ms) {
            return self.forget_partial();
        }
        self.follow(&text, heard.addressed, false, now_ms)
    }

    /// Reads final words and carries out what they ask; words said with push to talk `held`, and
    /// `typed` words, are addressed.
    fn follow(&mut self, text: &str, held: bool, typed: bool, now_ms: u64) -> Outcome {
        let status = &self.voice.status;
        let addressed = status.addresses(text, held);
        let intent = interpret(text, held, status.mode, &status.wake_word, &self.project);
        let shown = match &intent {
            _ if addressed => {
                let verb = if typed { "typed" } else { "heard" };
                self.record(now_ms, "voice", &format!("{verb}: {text:?}"));
                text.to_owned()
            }
            Intent::Play { phrase, .. } if self.spoke_recently(phrase, now_ms) => return self.forget_partial(),
            Intent::Raise {
                phrase: Some(phrase), ..
            }
            | Intent::Play { phrase, .. } => {
                self.record(now_ms, "voice", &format!("heard the trigger phrase {phrase:?}"));
                phrase.clone()
            }
            Intent::Unclear { phrase, .. } => phrase.clone(),
            _ => return self.forget_partial(),
        };
        self.voice.awaited = None;
        let (result, raised, saved) = self.act_on(intent, now_ms);
        let (deck, card) = raised.unzip();
        self.voice.heard = Some(HeardView {
            text: shown,
            is_final: true,
            typed,
            at_ms: now_ms,
            result,
            deck,
            card,
        });
        self.bump(if saved { Outcome::SAVED } else { Outcome::CHANGED })
    }

    /// Partial words shown while they seemed addressed are not left behind.
    fn forget_partial(&mut self) -> Outcome {
        if self.voice.heard.as_ref().is_some_and(|shown| !shown.is_final) {
            self.voice.heard = None;
            return Outcome {
                telemetry: true,
                ..Outcome::NONE
            };
        }
        Outcome::NONE
    }

    /// Whether words contain the card spoken in the last [`ECHO_MS`], as when the microphone
    /// hears the overlay.
    fn echoes_speech(&self, text: &str, now_ms: u64) -> bool {
        self.recent_speech(now_ms)
            .is_some_and(|spoken| contains(&words(text), &words(spoken)))
    }

    /// Whether a reaction's trigger phrase is part of the card spoken in the last [`ECHO_MS`], so
    /// the microphone may have heard the overlay say it.
    fn spoke_recently(&self, phrase: &str, now_ms: u64) -> bool {
        self.recent_speech(now_ms)
            .is_some_and(|spoken| contains(&literal_words(spoken), &literal_words(phrase)))
    }

    /// What the overlay said in the last [`ECHO_MS`], if it said anything.
    fn recent_speech(&self, now_ms: u64) -> Option<&str> {
        let cue = self.voice.speech.as_ref()?;
        let recent = cue.audio.is_some() && now_ms.saturating_sub(cue.at_ms) <= ECHO_MS;
        recent.then_some(cue.text.as_str())
    }

    /// Holds or lets go of push-to-talk. A held control lets go by itself after a minute
    /// unless it is pressed again.
    pub fn set_talking(&mut self, down: bool, now_ms: u64) -> Outcome {
        self.voice.talk_until_ms = down.then(|| now_ms.saturating_add(MAX_TALK_MS));
        if self.voice.talking == down {
            return Outcome::NONE;
        }
        self.voice.talking = down;
        self.bump(Outcome::CHANGED)
    }

    /// Whether a push-to-talk control is held.
    pub fn talking(&self) -> bool {
        self.voice.talking
    }

    /// Lets go of push-to-talk once its hold time has passed.
    pub(crate) fn expire_talk(&mut self, now_ms: u64) -> Outcome {
        if !self.voice.talking || self.voice.talk_until_ms.is_some_and(|until| until > now_ms) {
            return Outcome::NONE;
        }
        self.voice.talking = false;
        self.voice.talk_until_ms = None;
        self.record(now_ms, "voice", "push to talk let go after a minute without news");
        self.bump(Outcome::CHANGED)
    }

    /// Takes the host's report on voice control; turning it on or off is logged.
    pub fn set_voice_status(&mut self, status: VoiceStatus, now_ms: u64) -> Outcome {
        if self.voice.status == status {
            return Outcome::NONE;
        }
        if self.voice.status.enabled != status.enabled {
            let detail = if status.enabled {
                "voice control turned on"
            } else {
                "voice control turned off"
            };
            self.record(now_ms, "voice", detail);
        }
        self.voice.status = status;
        self.bump(Outcome::CHANGED)
    }

    /// Voice control as the editor shows it.
    pub fn voice_view(&self) -> VoiceView {
        let status = &self.voice.status;
        VoiceView {
            enabled: status.enabled,
            mode: status.mode,
            listening: status.listening,
            talking: self.voice.talking,
            message: status.message.clone(),
            heard: self.voice.heard.clone(),
        }
    }
}
