// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the copies of the reaction presets that the overlay and the editor
//! (web/reaction-presets.js) and the Stream Deck plugin (streamdeck/src/lib/built-in-reactions.ts)
//! read: each holds every preset exactly as sceneuron-core defines it. Run with
//! SCENEURON_WRITE_REACTION_PRESETS=1 to rewrite both after a preset changes.

use std::fs;
use std::path::PathBuf;

use sceneuron_core::presets;

const WEB_HEADER: &str = "\
// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron's reaction presets: the emojis, banner line, color, and length of each overlay-wide flood, and the
   trigger phrases that play it, for the overlay and the editor. Written by
   crates/sceneuron-core/tests/reaction_copies.rs, which fails when it no longer matches the presets in
   sceneuron-core. */
(function () {
  \"use strict\";

  const PRESETS = Object.freeze(
    ";

const WEB_FOOTER: &str = "\
.map((preset) =>
      Object.freeze({ ...preset, emojis: Object.freeze(preset.emojis), phrases: Object.freeze(preset.phrases) }),
    ),
  );

  const api = { PRESETS };
  if (typeof module !== \"undefined\" && module.exports) {
    module.exports = api;
  } else {
    window.SceneuronReactionPresets = api;
  }
})();
";

const PLUGIN_HEADER: &str = "\
// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Sceneuron's reaction presets as sceneuron-core defines them, so Reaction keys show their preset's
 * emoji, short name, and color before the plugin is paired, and light up for as long as it plays.
 * Written by crates/sceneuron-core/tests/reaction_copies.rs, which fails when it no longer matches
 * the presets.
 */

export interface BuiltInReaction {
  readonly id: string;
  readonly name: string;
  readonly short: string;
  readonly emojis: readonly string[];
  readonly banner: string | null;
  readonly color: string;
  readonly duration_ms: number;
  readonly camera: \"look\" | \"facepalm\" | null;
  readonly phrases: readonly string[];
}

export const BUILT_IN_REACTIONS: readonly BuiltInReaction[] = ";

fn repo_file(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
}

/// The presets as pretty JSON, each line after the first indented by `indent` spaces.
fn presets_json(indent: usize) -> String {
    let json = serde_json::to_string_pretty(presets()).unwrap();
    json.replace('\n', &format!("\n{}", " ".repeat(indent)))
}

/// Rewrites `path` to `expected` when asked to, then checks that it holds exactly that.
fn check_copy(path: &str, expected: &str) {
    let path = repo_file(path);
    if std::env::var_os("SCENEURON_WRITE_REACTION_PRESETS").is_some() {
        fs::write(&path, expected).unwrap();
    }
    let actual = fs::read_to_string(&path).unwrap();
    assert!(
        actual == expected,
        "{} differs from the reaction presets; rerun this test with SCENEURON_WRITE_REACTION_PRESETS=1",
        path.display()
    );
}

#[test]
fn the_overlay_and_the_editor_read_the_presets_as_the_core_defines_them() {
    let expected = format!("{WEB_HEADER}{}{WEB_FOOTER}", presets_json(4));
    check_copy("web/reaction-presets.js", &expected);
}

#[test]
fn the_stream_deck_plugin_reads_the_presets_as_the_core_defines_them() {
    let expected = format!("{PLUGIN_HEADER}{};\n", presets_json(0));
    check_copy("streamdeck/src/lib/built-in-reactions.ts", &expected);
}

#[test]
fn the_copies_carry_every_field_a_page_or_a_key_reads() {
    let json = serde_json::to_value(presets()).unwrap();
    let email = &json[1];
    assert_eq!(
        email,
        &serde_json::json!({
            "id": "email",
            "name": "Could have been an email",
            "short": "Email",
            "emojis": ["📧", "✉️", "📨", "📬", "🤦"],
            "banner": "This meeting could have been an email",
            "color": "#1d4ed8",
            "duration_ms": 4000,
            "camera": "facepalm",
            "phrases": ["could have been an email", "should have been an email"]
        })
    );
    assert_eq!(json[2]["camera"], serde_json::Value::Null);
    assert_eq!(json[0]["camera"], "look");
    assert_eq!(json[0]["phrases"], serde_json::json!([]));
}
