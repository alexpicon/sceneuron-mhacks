// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Overlay themes: one look for the whole overlay, chosen per project. The project stores only the
//! theme's id; the overlay page draws each theme over the elements' positions, sizes, and text,
//! which no theme changes.

use serde::{Deserialize, Serialize};

/// How the overlay draws every element. JSON: the theme's id, such as `"windows-95"`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    /// Each element's own colors, borders, and corners.
    #[default]
    #[serde(rename = "none")]
    None,
    /// Graphite panels with warm amber accents: cards with an amber icon tile, the newest card
    /// edged in amber, lower thirds with an amber bar, and on-air pills.
    #[serde(rename = "graphite")]
    Graphite,
    /// Neon green monospace on black glass, with a prompt, a blinking cursor, and scanlines.
    #[serde(rename = "hacker")]
    Hacker,
    /// An amber phosphor screen in a curved CRT, with scanlines.
    #[serde(rename = "eighties")]
    Eighties,
    /// Silver bevelled windows with navy title bars; cards are dialog boxes.
    #[serde(rename = "windows-95")]
    Windows95,
    /// One-bit black and white windows with striped title bars; cards are alert boxes.
    #[serde(rename = "classic-mac")]
    ClassicMac,
    /// Pink and cyan neon on purple gradients, in italics.
    #[serde(rename = "vaporwave")]
    Vaporwave,
}

impl Theme {
    /// Every theme, in the order the editor offers them.
    pub const ALL: [Theme; 7] = [
        Theme::None,
        Theme::Graphite,
        Theme::Hacker,
        Theme::Eighties,
        Theme::Windows95,
        Theme::ClassicMac,
        Theme::Vaporwave,
    ];

    /// The id a project and the overlay use for this theme.
    pub fn id(self) -> &'static str {
        match self {
            Theme::None => "none",
            Theme::Graphite => "graphite",
            Theme::Hacker => "hacker",
            Theme::Eighties => "eighties",
            Theme::Windows95 => "windows-95",
            Theme::ClassicMac => "classic-mac",
            Theme::Vaporwave => "vaporwave",
        }
    }

    /// The theme with this id, if this build knows it.
    pub fn from_id(id: &str) -> Option<Theme> {
        Theme::ALL.into_iter().find(|theme| theme.id() == id)
    }
}
