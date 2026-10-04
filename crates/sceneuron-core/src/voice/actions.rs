// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What voice control carries out at once, as a local command or as one of the AI's actions, and
//! the requests that only the host can answer: a look, a subtitles change, or words for the AI.

use serde::{Deserialize, Serialize};

use crate::plan::EditOp;
use crate::reactions::Flood;
use crate::theme::Theme;

/// One thing voice control does at once. Local commands carry out the first kinds; the AI may
/// answer with any of them. JSON: tagged by `action`, such as `{"action": "look"}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum VoiceAction {
    /// Plays a reaction preset at once, like the editor's Try it.
    React { reaction: Flood },
    /// Switches the overlay's theme in the saved project.
    Theme { theme: Theme },
    /// Shows or hides an element, held until it is released, like the control API's element
    /// show and hide.
    ShowElement { element: String, visible: bool },
    /// Raises a card of a deck whose voice is on.
    RaiseCard { deck: String, card: String },
    /// Lowers the card of `deck`, or of every deck whose voice is on when `deck` is empty.
    LowerCards { deck: String },
    /// Holds new text on a text element until it is released.
    SetText { element: String, text: String },
    /// Releases what is held on an element, so rules and cards decide again.
    ReleaseElement { element: String },
    /// Resumes or pauses automation.
    Automation { enabled: bool },
    /// Turns live subtitles on or off; only the host can.
    Subtitles { enabled: bool },
    /// Translates subtitles into `language`, a BCP 47 code, first; only the host can.
    SubtitleLanguage { language: String },
    /// Asks the vision model to look at the camera now; only the host can.
    Look,
    /// Reveals the shared room's votes or starts its next round; only the host can, as it holds
    /// the room.
    Room { step: RoomStep },
    /// Changes the saved project with edit-plan operations, as an applied assistant plan does.
    EditProject { summary: String, operations: Vec<EditOp> },
}

impl VoiceAction {
    /// Whether only the host can carry it out, because it changes the AI settings or the vision
    /// model.
    pub fn for_host(&self) -> bool {
        matches!(
            self,
            VoiceAction::Subtitles { .. }
                | VoiceAction::SubtitleLanguage { .. }
                | VoiceAction::Look
                | VoiceAction::Room { .. }
        )
    }
}

/// What a voice command does in the shared planning poker room.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomStep {
    /// Shows everyone's votes at once.
    Reveal,
    /// Lowers every card and clears the votes for the next round.
    Next,
}

/// What voice control asks of the host.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "task", rename_all = "snake_case")]
pub enum VoiceTask {
    /// An action only the host can carry out; see [`VoiceAction::for_host`].
    Act { action: VoiceAction },
    /// Words addressed to Sceneuron that no local command understood, for the AI to read.
    Ask { text: String },
}

/// A task waiting for the host, by the nonce the host answers with.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VoiceRequest {
    /// Increases with every request, like effect nonces.
    pub nonce: u64,
    pub task: VoiceTask,
    /// When it was asked, on the snapshot clock.
    pub at_ms: u64,
}

/// The host's answer to a [`VoiceRequest`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VoiceReply {
    /// What the AI took the words to mean, or why it found nothing to do; empty for an action.
    pub understood: String,
    /// What to do or what was done, in order.
    pub steps: Vec<VoiceStep>,
}

/// One part of a [`VoiceReply`].
#[derive(Clone, Debug, PartialEq)]
pub enum VoiceStep {
    /// An action for the engine to carry out.
    Act(VoiceAction),
    /// What the host did itself, or why it could not, in words.
    Done(String),
}
