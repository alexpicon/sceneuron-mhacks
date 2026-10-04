// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Spoken cards: a card raised on a deck that speaks becomes a speech cue, which the host turns
//! into audio and the overlay plays once.

use super::SpeechCue;
use super::normalize::speakable;
use crate::decks::Card;
use crate::engine::Engine;
use crate::subtitles::{MAX_LINE_CHARS, MAX_LINES};
use crate::template::{self, Fit};
use crate::types::Outcome;

/// Longest text a cue says, in characters; longer card text is cut at a word.
const MAX_SPOKEN_CHARS: usize = 200;

impl Engine {
    /// Makes the card just raised the speech cue when its deck speaks; a cue not spoken yet is
    /// replaced. A covered deck says nothing, since saying its card would give away a hidden vote.
    pub(crate) fn cue_speech(&mut self, deck: &str, card: &str, now_ms: u64) {
        if self.covers.contains_key(deck) {
            return;
        }
        let Some(found) = self.project.deck(deck).filter(|found| found.speak) else {
            return;
        };
        let Some(face) = found.card(card) else {
            return;
        };
        let slot_text = self
            .project
            .element(&found.element)
            .map_or("", |slot| slot.props.text.as_str());
        let text = face.props.text.as_deref().unwrap_or(slot_text);
        let fit = Fit {
            chars: MAX_LINE_CHARS,
            lines: usize::from(MAX_LINES),
            native: false,
        };
        let rendered = template::render(text, &self.sources(), fit);
        let text = cue_text(face, &rendered);
        if text.is_empty() {
            return;
        }
        let (deck, card) = (found.id.clone(), face.id.clone());
        let nonce = self.take_nonce();
        self.voice.speech = Some(SpeechCue {
            text,
            nonce,
            deck,
            card,
            at_ms: now_ms,
            audio: None,
        });
    }

    /// The latest speech cue, if any card was spoken since the engine started.
    pub fn speech(&self) -> Option<&SpeechCue> {
        self.voice.speech.as_ref()
    }

    /// Attaches the generated audio to the cue with `nonce`, so clients play it. A cue that a
    /// newer one replaced, or whose card is no longer up or was covered since, stays silent.
    pub fn speech_ready(&mut self, nonce: u64, audio: &str, _now_ms: u64) -> Outcome {
        let Some(cue) = self
            .voice
            .speech
            .as_ref()
            .filter(|cue| cue.nonce == nonce && cue.audio.is_none())
        else {
            return Outcome::NONE;
        };
        if self.raised_card(&cue.deck) != Some(cue.card.as_str()) || self.covers.contains_key(&cue.deck) {
            return Outcome::NONE;
        }
        if let Some(cue) = self.voice.speech.as_mut() {
            cue.audio = Some(audio.to_owned());
        }
        self.bump(Outcome::CHANGED)
    }
}

/// What a card says: its speech line, else its rendered text without emoji, else its name.
pub(crate) fn cue_text(card: &Card, rendered_text: &str) -> String {
    if let Some(speech) = &card.speech {
        return shorten(speech.trim());
    }
    let text = speakable(rendered_text);
    shorten(if text.is_empty() { speakable(&card.name) } else { text }.as_str())
}

/// Cuts text to [`MAX_SPOKEN_CHARS`] at the last whole word.
fn shorten(text: &str) -> String {
    if text.chars().count() <= MAX_SPOKEN_CHARS {
        return text.to_owned();
    }
    let cut: String = text.chars().take(MAX_SPOKEN_CHARS).collect();
    match cut.rfind(' ') {
        Some(space) if space > 0 => cut[..space].to_owned(),
        _ => cut,
    }
}
