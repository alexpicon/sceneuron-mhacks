// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Pairs from the deep link that the editor's "Pair the Stream Deck on this computer" opens,
 * `streamdeck://plugins/message/ai.chaski.sceneuron/pair?port=4466&code=123456&streamdeck=hidden`,
 * exactly as a typed code pairs: the same exchange, and the same hello proof before the token is
 * kept. Any program on this computer can open such a link, so one that is not a pairing link, or
 * whose code Sceneuron refuses, changes nothing; neither the code nor the token is logged.
 */

import streamDeck from "@elgato/streamdeck";

import { guarded, pairAndKeep } from "./keep-pairing.ts";
import type { InspectorPairing } from "./lib/inspector.ts";
import type { Link } from "./lib/link.ts";
import { readPairingLink } from "./lib/pairing.ts";

/**
 * Listens for pairing links. Each waits for `saved`, which settles once the saved pairing is on the
 * link, so a link that arrives while the plugin starts is not undone by the older pairing.
 */
export function pairFromDeepLinks(link: Link, saved: Promise<void>): void {
  streamDeck.system.onDidReceiveDeepLink((ev) => {
    const asked = readPairingLink(ev.url.path, ev.url.queryParameters);
    if (!asked) {
      streamDeck.logger.warn("ignored a deep link that is not a pairing link");
      return;
    }
    guarded("pairing from a link", async () => {
      await saved;
      const result = await pairAndKeep(link, asked.port, asked.code);
      // A server's own words can repeat the code it was sent.
      const message = result.message.replaceAll(asked.code, "the code");
      if (result.ok) {
        streamDeck.logger.info(`a pairing link: ${message}`);
      } else {
        streamDeck.logger.warn(`a pairing link paired nothing: ${message}`);
      }
      const reply: InspectorPairing = { type: "paired", ok: result.ok, message };
      await streamDeck.ui.sendToPropertyInspector({ ...reply });
    });
  });
}
