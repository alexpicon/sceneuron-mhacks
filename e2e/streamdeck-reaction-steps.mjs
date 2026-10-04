// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// The Stream Deck live test's Reaction key step (streamdeck-steps.mjs runs it while the plugin is
// paired): a press plays the email reaction on a real Sceneuron through POST /api/flood with the
// control token, the key lights up while it plays and goes dark once it ends, and Sceneuron logs it
// as a manual reaction with its banner line.

import { readRequests } from "./streamdeck-log.mjs";

/** The settings of the live test's Reaction key. */
export const REACTION = { reaction: "email" };
const LOGGED = 'flooded the overlay with 📧 ✉️ 📨 📬 🤦 and "This meeting could have been an email"';

/**
 * Runs the step; `t` holds the harness's step, same, and api, and `keys` the running plugin's fake
 * Stream Deck `deck`, the fake's helpers `fake`, the Reaction key's `context`, a `press` that makes
 * its keyDown, and `answered`, which tells how a request went out and was answered.
 */
export async function runStreamDeckReactionSteps(t, keys) {
  const { step, same, api } = t;
  const { deck, fake, context, press, answered } = keys;
  const state = () => api("GET", "/api/state");
  const shows = (check) => fake.image(context, (svg) => check(svg) && !fake.dim(svg));

  await step("a Reaction key plays the email on the overlay, and lights up while it plays", async () => {
    const before = (await state()).flood?.nonce ?? 0;
    const asked = readRequests(deck.output).length;
    const playing = shows((svg) => fake.lit(svg) && fake.words(svg) === "📧 Email Playing");
    const lit = await deck.after(press, playing, "the lit email key");
    const started = Date.now();
    const flood = (await state()).flood;
    same(flood.flood, "email", "the reaction Sceneuron plays");
    if (!(flood.nonce > before)) throw new Error(`no new reaction: nonce ${flood.nonce} after ${before}`);
    const posts = readRequests(deck.output)
      .slice(asked)
      .filter((entry) => entry.path === "/api/flood");
    same(posts.map(answered), [["POST", true, 200]], "the reaction request");
    const log = (await state()).events.map((event) => [event.source, event.detail]);
    if (!log.some(([source, detail]) => source === "manual" && detail === LOGGED)) {
      throw new Error(`no reaction in the event log: ${JSON.stringify(log.slice(0, 3))}`);
    }
    const dark = shows((svg) => !fake.lit(svg) && fake.words(svg) === "📧 Email");
    await deck.next(dark, "the email key once the reaction ended", deck.messages.indexOf(lit));
    return `lit for ${Date.now() - started} ms of the email's 4000`;
  });
}
