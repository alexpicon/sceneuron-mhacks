// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The live description bar: what the camera sees, as on-device analysis describes it with every
//! analyzed frame, and below it the vision model's caption with its age while scene understanding
//! is on. It has no deck.

use super::Pack;
use super::catalog::{DARK, LIGHT};
use super::parts::slot;
use crate::props::{ElementProps, EnterAnimation, TextAlign};

/// The live description bar, in the band between the status banner and the classroom cards, as
/// wide as the meeting card. Its words change often, so it plays no enter animation, and it hides
/// while both lines are empty, as while no camera is analyzed.
pub(super) fn live_description() -> Pack {
    let props = ElementProps {
        x: 380.0,
        y: 196.0,
        width: 1160.0,
        height: 180.0,
        font_size: 32.0,
        corner_radius: 16.0,
        align: TextAlign::Left,
        enter: EnterAnimation::None,
        visible: true,
        text: "{{seen}}\n{{scene:age}}".to_owned(),
        color: LIGHT.to_owned(),
        background: DARK.to_owned(),
        ..ElementProps::default()
    };
    Pack {
        id: "live-description".to_owned(),
        name: "Live description".to_owned(),
        description: "A bar that says what the camera sees, such as \"1 person; right hand raised, peace sign\". \
                      Sceneuron writes it on this computer from every analyzed frame: free, and nothing is sent. \
                      While scene understanding is on, the AI's latest caption and its age show below it; each \
                      caption comes from a look, which costs money within the hourly budget, so remove \
                      {{scene:age}} from its text to keep it free."
            .to_owned(),
        elements: vec![slot("live-description", "Live description", props)],
        decks: Vec::new(),
    }
}
