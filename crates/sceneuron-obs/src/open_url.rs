// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Opens a local editor URL in the user's default browser, and a pairing link in the Stream Deck
//! app.

use std::process::Command;

use crate::ffi;

/// The start of every pairing link for Sceneuron's Stream Deck plugin.
const PAIRING_LINK: &str = "streamdeck://plugins/message/ai.chaski.sceneuron/pair?";

/// Opens `url` without blocking. Only loopback http URLs are accepted.
pub fn open(url: &str) {
    if !url.starts_with("http://127.0.0.1:") {
        ffi::warn("refusing to open a non-local editor URL");
        return;
    }
    if let Err(error) = launch(&[url], "the browser") {
        ffi::warn(&format!("could not open the browser: {error}"));
    }
}

/// Opens a pairing link for Sceneuron's Stream Deck plugin without blocking, in the background
/// (`open -g`), so neither the Stream Deck app nor its window comes forward. The link carries a
/// pairing code, so no message here repeats it.
pub fn open_stream_deck_link(link: &str) -> Result<(), String> {
    if !link.starts_with(PAIRING_LINK) {
        return Err("that is not a pairing link for Sceneuron's Stream Deck plugin".to_owned());
    }
    launch(&["-g", link], "the pairing link in the Stream Deck app")
}

/// Starts macOS `open` with `args`. The launcher is reaped so it does not linger as a zombie
/// process, and one that fails is logged, naming only `what`, since a failure otherwise leaves no
/// trace.
fn launch(args: &[&str], what: &str) -> Result<(), String> {
    let mut child = Command::new("open")
        .args(args)
        .spawn()
        .map_err(|error| error.to_string())?;
    let what = what.to_owned();
    drop(std::thread::spawn(move || match child.wait() {
        Ok(status) if !status.success() => ffi::warn(&format!("could not open {what}: the launcher {status}")),
        Ok(_) => {}
        Err(error) => ffi::warn(&format!("could not open {what}: {error}")),
    }));
    Ok(())
}
