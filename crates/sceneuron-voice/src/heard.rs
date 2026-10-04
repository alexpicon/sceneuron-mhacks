// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the voice input heard, and whether it was said while push to talk was held.

use serde::Serialize;

/// A transcript, shaped like the core's `Heard`: partial (`is_final` false) while the speaker
/// talks, final once the segment is committed. `addressed` is true for speech said while push
/// to talk was held; whether other speech starts with the wake word is the engine's to decide.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Heard {
    pub text: String,
    pub is_final: bool,
    pub addressed: bool,
}
