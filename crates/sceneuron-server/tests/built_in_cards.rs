// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the Stream Deck plugin's copy of the built-in card packs
//! (streamdeck/src/lib/built-in-cards.ts): every deck of every pack, with each card's name, text,
//! and colors exactly as the control view sends them, so keys look like their cards before the
//! plugin is paired. Run with SCENEURON_WRITE_BUILT_IN_CARDS=1 to rewrite the copy after a pack
//! changes.

mod common;

use std::fs;
use std::path::PathBuf;

use common::{TestServer, project_with_packs};
use sceneuron_core::packs;
use sceneuron_server::HubConfig;
use serde_json::{Value, json};

const HEADER: &str = "\
// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * The decks of Sceneuron's built-in card packs as the control view describes them, so card keys
 * look like their cards before the plugin is paired or while Sceneuron is out of reach. Written by
 * crates/sceneuron-server/tests/built_in_cards.rs, which fails when it no longer matches the packs.
 */

import type { ControlCard } from \"./view.ts\";

export interface BuiltInDeck {
  readonly id: string;
  readonly name: string;
  readonly cards: readonly ControlCard[];
}

export const BUILT_IN_DECKS: readonly BuiltInDeck[] = ";

fn copy_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../streamdeck/src/lib/built-in-cards.ts")
}

/// The decks of every built-in pack, from the control view of a project holding all of them.
fn built_in_decks() -> Value {
    let ids: Vec<String> = packs().into_iter().map(|pack| pack.id).collect();
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    let server = TestServer::with(project_with_packs(&ids), HubConfig::default());
    let view = server.control("GET", "/api/control/state").send(server.port).json();
    let decks = view["decks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|deck| json!({"id": deck["id"], "name": deck["name"], "cards": deck["cards"]}));
    Value::Array(decks.collect())
}

#[test]
fn the_stream_deck_plugin_draws_the_built_in_cards_as_the_packs_define_them() {
    let decks = built_in_decks();
    assert!(
        decks.as_array().unwrap().len() >= 4,
        "every pack with a deck is included"
    );
    let expected = format!("{HEADER}{};\n", serde_json::to_string_pretty(&decks).unwrap());
    let path = copy_path();
    if std::env::var_os("SCENEURON_WRITE_BUILT_IN_CARDS").is_some() {
        fs::write(&path, &expected).unwrap();
    }
    let actual = fs::read_to_string(&path).unwrap();
    assert!(
        actual == expected,
        "{} differs from the built-in packs; rerun this test with SCENEURON_WRITE_BUILT_IN_CARDS=1",
        path.display()
    );
}
