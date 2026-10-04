// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Helpers the tests share: waiting for a condition, and a link or the built plugin started against
// the fakes and stopped when its test ends.

import assert from "node:assert/strict";

import { Link } from "../src/lib/link.ts";
import { startFakeSceneuron } from "./fake-sceneuron.mjs";
import { startStreamDeck } from "./fake-stream-deck.mjs";

/** Polls until `check()` holds; fails naming `what` after `ms` milliseconds, three seconds unless given. */
export async function waitFor(check, what, ms = 3000) {
  const deadline = Date.now() + ms;
  while (!check()) {
    if (Date.now() > deadline) {
      assert.fail(`timed out waiting for ${what}`);
    }
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
}

/** A fake Sceneuron that closes when test `t` ends. */
export async function startSceneuron(t, options) {
  const server = await startFakeSceneuron(options);
  t.after(() => server.close());
  return server;
}

/** A link that retries within milliseconds and stops when test `t` ends. */
export function startLink(t, options = {}) {
  const link = new Link({ backoff: { baseMs: 10, maxMs: 40 }, ...options });
  t.after(() => link.stop());
  return link;
}

/**
 * Waits until `link` is connected and its one state read has reached `server`, which answers it
 * at once, so a later change of token cannot refuse that read.
 */
export async function settled(server, link) {
  const read = () => server.requests.some((request) => request.path === "/api/control/state");
  await waitFor(() => link.state === "connected" && read(), "the connection and its state read");
}

/**
 * The built plugin, started by a fake Stream Deck that holds the global settings
 * `globals(sceneuron)`, next to a fake Sceneuron; the plugin stops first when test `t` ends.
 */
export async function startPlugin(t, sceneuronOptions, globals) {
  const sceneuron = await startFakeSceneuron(sceneuronOptions);
  let deck = null;
  t.after(async () => {
    await deck?.stop();
    await sceneuron.close();
  });
  deck = await startStreamDeck(globals(sceneuron));
  return { sceneuron, deck };
}
