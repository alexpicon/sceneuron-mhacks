// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The reaction presets: what each overlay-wide flood shows, for how long, how the camera plays it
//! by itself, and the trigger phrases that play it when said. The overlay, the editor, and the
//! Stream Deck plugin read copies of this table that tests keep equal to it.

use serde::{Deserialize, Serialize};

/// A reaction preset, named by its id in JSON, such as `"email"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Flood {
    /// Faces sticking their tongues out.
    Tongue,
    /// "This meeting could have been an email", for a meeting that drags.
    Email,
    /// "Mind blown", for an idea that lands.
    MindBlown,
    /// "Ship it!", for a decision made.
    ShipIt,
    /// "Coffee break?", for a meeting that needs one.
    Coffee,
    /// "You're absolutely right!", for agreement as eager as a chatbot's.
    AbsolutelyRight,
    /// "We need more tokens!", for a plan that will take more AI than anyone budgeted.
    MoreTokens,
    /// "Works on my GPU", for a demo that only ever ran on one machine.
    WorksOnMyGpu,
    /// "Meatproxy alert!", for a person only passing along what a language model said.
    Meatproxy,
    /// "My clanker will do it!", for a job handed to an AI.
    Clanker,
}

/// How the camera plays a preset by itself, while fun reactions are on and automation runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Camera {
    /// The vision model takes a look once on-device analysis shows the preset's cue, as an open
    /// mouth for a tongue sticking out; see [`reactions`](crate::reactions).
    Look,
    /// On-device analysis alone: a hand held over the upper half of the face for
    /// [`FACEPALM_HOLD_MS`](crate::FACEPALM_HOLD_MS), with no AI call.
    Facepalm,
}

/// One preset: the emojis that rain and burst across the overlay, the banner line in the middle,
/// and how long it all lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Preset {
    #[serde(rename = "id")]
    pub flood: Flood,
    /// What the editor calls it, such as "Could have been an email".
    pub name: &'static str,
    /// A Stream Deck key's label, such as "Email".
    pub short: &'static str,
    /// The emojis of the flood; the first one stands for the preset on keys and in the banner.
    pub emojis: &'static [&'static str],
    /// The line shown large in the middle of the overlay, if any.
    pub banner: Option<&'static str>,
    /// The banner's and a key's background, `#rrggbb`; white text reads on it.
    pub color: &'static str,
    /// How long the flood plays; a page that learns of it later skips it.
    pub duration_ms: u64,
    pub camera: Option<Camera>,
    /// Trigger phrases that play the preset when said anywhere in always-on mode, without the
    /// wake word, while fun reactions are on and its voice is; see
    /// [`Reactions::hears`](super::Reactions::hears). Words addressed to Sceneuron also name it
    /// by these, its name, its short name, or its banner.
    pub phrases: &'static [&'static str],
}

const PRESETS: [Preset; 10] = [
    Preset {
        flood: Flood::Tongue,
        name: "Tongue out",
        short: "Tongue",
        emojis: &["😜", "😛", "🤪", "😝"],
        banner: None,
        color: "#be185d",
        duration_ms: 3_000,
        camera: Some(Camera::Look),
        phrases: &[],
    },
    Preset {
        flood: Flood::Email,
        name: "Could have been an email",
        short: "Email",
        emojis: &["📧", "✉️", "📨", "📬", "🤦"],
        banner: Some("This meeting could have been an email"),
        color: "#1d4ed8",
        duration_ms: 4_000,
        camera: Some(Camera::Facepalm),
        phrases: &["could have been an email", "should have been an email"],
    },
    Preset {
        flood: Flood::MindBlown,
        name: "Mind blown",
        short: "Mind blown",
        emojis: &["🤯", "💥", "🧠", "✨"],
        banner: Some("Mind blown"),
        color: "#6d28d9",
        duration_ms: 4_000,
        camera: None,
        phrases: &["mind blown", "blew my mind", "my mind is blown"],
    },
    Preset {
        flood: Flood::ShipIt,
        name: "Ship it",
        short: "Ship it",
        emojis: &["🚀", "🎉", "✅", "🙌"],
        banner: Some("Ship it!"),
        color: "#15803d",
        duration_ms: 4_000,
        camera: None,
        phrases: &["ship it"],
    },
    Preset {
        flood: Flood::Coffee,
        name: "Coffee break",
        short: "Coffee",
        emojis: &["☕", "🍩", "🥐", "⏸️"],
        banner: Some("Coffee break?"),
        color: "#92400e",
        duration_ms: 4_000,
        camera: None,
        phrases: &["coffee break"],
    },
    Preset {
        flood: Flood::AbsolutelyRight,
        name: "You're absolutely right",
        short: "So right",
        emojis: &["💯", "🙇", "✅", "👏"],
        banner: Some("You're absolutely right!"),
        color: "#0f766e",
        duration_ms: 4_000,
        camera: None,
        phrases: &["you're absolutely right", "you are absolutely right"],
    },
    Preset {
        flood: Flood::MoreTokens,
        name: "We need more tokens",
        short: "Tokens",
        emojis: &["🪙", "💸", "🔥", "🤖"],
        banner: Some("We need more tokens!"),
        color: "#854d0e",
        duration_ms: 4_000,
        camera: None,
        phrases: &["need more tokens", "out of tokens", "ran out of tokens"],
    },
    Preset {
        flood: Flood::WorksOnMyGpu,
        name: "Works on my GPU",
        short: "My GPU",
        emojis: &["🖥️", "🔥", "🤷", "⚙️"],
        banner: Some("Works on my GPU"),
        color: "#3f6212",
        duration_ms: 4_000,
        camera: None,
        phrases: &["works on my gpu", "works on my machine"],
    },
    Preset {
        flood: Flood::Meatproxy,
        name: "Meatproxy alert",
        short: "Meatproxy",
        emojis: &["🥩", "🤖", "📋", "🚨"],
        banner: Some("Meatproxy alert!"),
        color: "#9f1239",
        duration_ms: 4_000,
        camera: None,
        phrases: &["meat proxy", "meat proxy alert"],
    },
    Preset {
        flood: Flood::Clanker,
        name: "My clanker will do it",
        short: "Clanker",
        emojis: &["🤖", "🦾", "⚙️", "🔧"],
        banner: Some("My clanker will do it!"),
        color: "#334155",
        duration_ms: 4_000,
        camera: None,
        phrases: &["my clanker will do it", "my clanker will", "clanker will do it"],
    },
];

/// Every preset, in the order the editor and the Stream Deck list them.
pub fn presets() -> &'static [Preset] {
    &PRESETS
}

impl Flood {
    /// The preset this names.
    pub fn preset(self) -> &'static Preset {
        PRESETS
            .iter()
            .find(|preset| preset.flood == self)
            .expect("every flood has a preset")
    }

    /// What the flood shows, for the event log: its emojis, and its banner line when it has one,
    /// such as `📧 ✉️ 📨 📬 🤦 and "This meeting could have been an email"`.
    pub fn describe(self) -> String {
        let preset = self.preset();
        let emojis = preset.emojis.join(" ");
        match preset.banner {
            Some(banner) => format!("{emojis} and {banner:?}"),
            None => emojis,
        }
    }
}
