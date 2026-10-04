// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * The decks of Sceneuron's built-in card packs as the control view describes them, so card keys
 * look like their cards before the plugin is paired or while Sceneuron is out of reach. Written by
 * crates/sceneuron-server/tests/built_in_cards.rs, which fails when it no longer matches the packs.
 */

import type { ControlCard } from "./view.ts";

export interface BuiltInDeck {
  readonly id: string;
  readonly name: string;
  readonly cards: readonly ControlCard[];
}

export const BUILT_IN_DECKS: readonly BuiltInDeck[] = [
  {
    "cards": [
      {
        "background": "#f59e0bff",
        "color": "#111827ff",
        "id": "wrap-up",
        "name": "Wrap it up",
        "text": "⏱️ Wrap it up"
      },
      {
        "background": "#c2410cff",
        "color": "#ffffffff",
        "id": "off-track",
        "name": "We're off track",
        "text": "🧭 We're off track"
      },
      {
        "background": "#2563ebff",
        "color": "#ffffffff",
        "id": "question",
        "name": "I have a question",
        "text": "✋ I have a question"
      },
      {
        "background": "#15803dff",
        "color": "#ffffffff",
        "id": "agree",
        "name": "Agree",
        "text": "👍 Agree"
      },
      {
        "background": "#dc2626ff",
        "color": "#ffffffff",
        "id": "disagree",
        "name": "Disagree",
        "text": "👎 Disagree"
      },
      {
        "background": "#7c3aedff",
        "color": "#ffffffff",
        "id": "cant-hear",
        "name": "Can't hear you",
        "text": "🔇 Can't hear you"
      },
      {
        "background": "#0f766eff",
        "color": "#ffffffff",
        "id": "offline",
        "name": "Let's take this offline",
        "text": "🗂️ Let's take this offline"
      },
      {
        "background": "#db2777ff",
        "color": "#ffffffff",
        "id": "next-topic",
        "name": "Next topic",
        "text": "⏭️ Next topic"
      }
    ],
    "id": "meeting",
    "name": "Meeting signals"
  },
  {
    "cards": [
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "p0",
        "name": "0",
        "text": "0"
      },
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "p1",
        "name": "1",
        "text": "1"
      },
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "p2",
        "name": "2",
        "text": "2"
      },
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "p3",
        "name": "3",
        "text": "3"
      },
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "p5",
        "name": "5",
        "text": "5"
      },
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "p8",
        "name": "8",
        "text": "8"
      },
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "p13",
        "name": "13",
        "text": "13"
      },
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "p21",
        "name": "21",
        "text": "21"
      },
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "unsure",
        "name": "?",
        "text": "?"
      },
      {
        "background": "#f8fafcff",
        "color": "#111827ff",
        "id": "break",
        "name": "☕",
        "text": "☕"
      }
    ],
    "id": "poker",
    "name": "Planning poker"
  },
  {
    "cards": [
      {
        "background": "#facc15ff",
        "color": "#111827ff",
        "id": "hand",
        "name": "✋ Hand raised",
        "text": "✋ Hand raised"
      },
      {
        "background": "#2563ebff",
        "color": "#ffffffff",
        "id": "question",
        "name": "❓ Question",
        "text": "❓ Question"
      },
      {
        "background": "#7c3aedff",
        "color": "#ffffffff",
        "id": "repeat",
        "name": "🔁 Please repeat",
        "text": "🔁 Please repeat"
      },
      {
        "background": "#0f766eff",
        "color": "#ffffffff",
        "id": "slower",
        "name": "🐢 Slower, please",
        "text": "🐢 Slower, please"
      },
      {
        "background": "#15803dff",
        "color": "#ffffffff",
        "id": "got-it",
        "name": "✅ Got it",
        "text": "✅ Got it"
      },
      {
        "background": "#dc2626ff",
        "color": "#ffffffff",
        "id": "answer-a",
        "name": "A",
        "text": "A"
      },
      {
        "background": "#1d4ed8ff",
        "color": "#ffffffff",
        "id": "answer-b",
        "name": "B",
        "text": "B"
      },
      {
        "background": "#b45309ff",
        "color": "#ffffffff",
        "id": "answer-c",
        "name": "C",
        "text": "C"
      },
      {
        "background": "#166534ff",
        "color": "#ffffffff",
        "id": "answer-d",
        "name": "D",
        "text": "D"
      }
    ],
    "id": "classroom",
    "name": "Classroom"
  },
  {
    "cards": [
      {
        "background": "#dc2626ff",
        "color": "#ffffffff",
        "id": "muted",
        "name": "🔇 I'm on mute",
        "text": "🔇 I'm on mute"
      },
      {
        "background": "#b45309ff",
        "color": "#ffffffff",
        "id": "brb",
        "name": "☕ Back in 5",
        "text": "☕ Back in 5"
      },
      {
        "background": "#c2410cff",
        "color": "#ffffffff",
        "id": "lunch",
        "name": "🍽️ At lunch",
        "text": "🍽️ At lunch"
      },
      {
        "background": "#4f46e5ff",
        "color": "#ffffffff",
        "id": "focus",
        "name": "🎧 Focus time",
        "text": "🎧 Focus time"
      },
      {
        "background": "#2563ebff",
        "color": "#ffffffff",
        "id": "presenting",
        "name": "🖥️ Presenting",
        "text": "🖥️ Presenting"
      },
      {
        "background": "#7c3aedff",
        "color": "#ffffffff",
        "id": "on-call",
        "name": "📞 On another call",
        "text": "📞 On another call"
      }
    ],
    "id": "status",
    "name": "Status"
  }
];
