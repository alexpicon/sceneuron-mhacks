// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Pairing the Stream Deck on this computer without typing a code: "Set up OBS for me", and OBS
//! when it starts, hand Sceneuron's plugin in the Stream Deck app a fresh pairing code through
//! the same deep link as the editor's one-click pairing, when the plugin is installed and no
//! control surface is paired.

use std::time::Instant;

use crate::server::Context;
use crate::setup::SetupStep;

/// Stream Deck routes `streamdeck://plugins/message/<plugin UUID>/...` to the plugin with that UUID.
const PAIR_LINK: &str = "streamdeck://plugins/message/ai.chaski.sceneuron/pair";

/// What the Stream Deck step is on when it sends a link or finds one paired.
const STREAM_DECK: &str = "Stream Deck";

/// Why no link went out when Sceneuron's Stream Deck plugin is missing, with how to install it.
const NOT_INSTALLED: &str = "Sceneuron's plugin for Stream Deck is not installed on this Mac; double-click \
                             ai.chaski.sceneuron.streamDeckPlugin to install it in the Stream Deck app, then \
                             choose Set up OBS for me again";

/// The deep link that asks Sceneuron's Stream Deck plugin to pair with `code` on `port`, as the
/// editor's one-click pairing writes it. `streamdeck=hidden` makes it a passive deep link, which
/// leaves the Stream Deck window where it is (Stream Deck 7.0 and newer).
pub fn stream_deck_pairing_link(port: u16, code: &str) -> String {
    format!("{PAIR_LINK}?port={port}&code={code}&streamdeck=hidden")
}

/// Sends Sceneuron's Stream Deck plugin a pairing link with a fresh code, replacing any earlier
/// code, unless a control surface is paired already or the plugin is not installed. Added means
/// the link went out; the plugin pairs once it trades the code.
pub(crate) fn pair(context: &Context) -> SetupStep {
    if context.control.paired() {
        return SetupStep::present(STREAM_DECK);
    }
    if !context.host.stream_deck_plugin_installed() {
        return SetupStep::skipped(NOT_INSTALLED);
    }
    let code = context.control.issue_code(Instant::now());
    let link = stream_deck_pairing_link(context.port, &code);
    match context.host.open_stream_deck_link(&link) {
        Ok(()) => SetupStep::added(STREAM_DECK),
        Err(error) => SetupStep::skipped(&format!(
            "the pairing link could not be sent to the Stream Deck app: {error}"
        )),
    }
}
