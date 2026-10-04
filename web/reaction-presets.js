// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron's reaction presets: the emojis, banner line, color, and length of each overlay-wide flood, and the
   trigger phrases that play it, for the overlay and the editor. Written by
   crates/sceneuron-core/tests/reaction_copies.rs, which fails when it no longer matches the presets in
   sceneuron-core. */
(function () {
  "use strict";

  const PRESETS = Object.freeze(
    [
      {
        "id": "tongue",
        "name": "Tongue out",
        "short": "Tongue",
        "emojis": [
          "😜",
          "😛",
          "🤪",
          "😝"
        ],
        "banner": null,
        "color": "#be185d",
        "duration_ms": 3000,
        "camera": "look",
        "phrases": []
      },
      {
        "id": "email",
        "name": "Could have been an email",
        "short": "Email",
        "emojis": [
          "📧",
          "✉️",
          "📨",
          "📬",
          "🤦"
        ],
        "banner": "This meeting could have been an email",
        "color": "#1d4ed8",
        "duration_ms": 4000,
        "camera": "facepalm",
        "phrases": [
          "could have been an email",
          "should have been an email"
        ]
      },
      {
        "id": "mind-blown",
        "name": "Mind blown",
        "short": "Mind blown",
        "emojis": [
          "🤯",
          "💥",
          "🧠",
          "✨"
        ],
        "banner": "Mind blown",
        "color": "#6d28d9",
        "duration_ms": 4000,
        "camera": null,
        "phrases": [
          "mind blown",
          "blew my mind",
          "my mind is blown"
        ]
      },
      {
        "id": "ship-it",
        "name": "Ship it",
        "short": "Ship it",
        "emojis": [
          "🚀",
          "🎉",
          "✅",
          "🙌"
        ],
        "banner": "Ship it!",
        "color": "#15803d",
        "duration_ms": 4000,
        "camera": null,
        "phrases": [
          "ship it"
        ]
      },
      {
        "id": "coffee",
        "name": "Coffee break",
        "short": "Coffee",
        "emojis": [
          "☕",
          "🍩",
          "🥐",
          "⏸️"
        ],
        "banner": "Coffee break?",
        "color": "#92400e",
        "duration_ms": 4000,
        "camera": null,
        "phrases": [
          "coffee break"
        ]
      },
      {
        "id": "absolutely-right",
        "name": "You're absolutely right",
        "short": "So right",
        "emojis": [
          "💯",
          "🙇",
          "✅",
          "👏"
        ],
        "banner": "You're absolutely right!",
        "color": "#0f766e",
        "duration_ms": 4000,
        "camera": null,
        "phrases": [
          "you're absolutely right",
          "you are absolutely right"
        ]
      },
      {
        "id": "more-tokens",
        "name": "We need more tokens",
        "short": "Tokens",
        "emojis": [
          "🪙",
          "💸",
          "🔥",
          "🤖"
        ],
        "banner": "We need more tokens!",
        "color": "#854d0e",
        "duration_ms": 4000,
        "camera": null,
        "phrases": [
          "need more tokens",
          "out of tokens",
          "ran out of tokens"
        ]
      },
      {
        "id": "works-on-my-gpu",
        "name": "Works on my GPU",
        "short": "My GPU",
        "emojis": [
          "🖥️",
          "🔥",
          "🤷",
          "⚙️"
        ],
        "banner": "Works on my GPU",
        "color": "#3f6212",
        "duration_ms": 4000,
        "camera": null,
        "phrases": [
          "works on my gpu",
          "works on my machine"
        ]
      },
      {
        "id": "meatproxy",
        "name": "Meatproxy alert",
        "short": "Meatproxy",
        "emojis": [
          "🥩",
          "🤖",
          "📋",
          "🚨"
        ],
        "banner": "Meatproxy alert!",
        "color": "#9f1239",
        "duration_ms": 4000,
        "camera": null,
        "phrases": [
          "meat proxy",
          "meat proxy alert"
        ]
      },
      {
        "id": "clanker",
        "name": "My clanker will do it",
        "short": "Clanker",
        "emojis": [
          "🤖",
          "🦾",
          "⚙️",
          "🔧"
        ],
        "banner": "My clanker will do it!",
        "color": "#334155",
        "duration_ms": 4000,
        "camera": null,
        "phrases": [
          "my clanker will do it",
          "my clanker will",
          "clanker will do it"
        ]
      }
    ].map((preset) =>
      Object.freeze({ ...preset, emojis: Object.freeze(preset.emojis), phrases: Object.freeze(preset.phrases) }),
    ),
  );

  const api = { PRESETS };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  } else {
    window.SceneuronReactionPresets = api;
  }
})();
