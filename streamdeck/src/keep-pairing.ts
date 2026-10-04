// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Pairing that every key shares: a code the server takes, with a token it proves, goes into the
 * plugin's global settings and onto the link, so every Sceneuron key goes live at once. The property
 * inspector and the editor's deep link both pair through here; a pairing that fails changes nothing.
 */

import streamDeck from "@elgato/streamdeck";

import type { Link } from "./lib/link.ts";
import { type PairingResult, pairWithCode } from "./lib/pairing.ts";

/** Logs instead of rejecting: the SDK does not await these handlers. */
export function guarded(what: string, work: () => Promise<void>): void {
  work().catch((error: unknown) => {
    streamDeck.logger.warn(`${what} failed: ${error instanceof Error ? error.message : String(error)}`);
  });
}

/** Pairs with `code` on `port`, and keeps the token only once the server proved it holds it. */
export async function pairAndKeep(link: Link, port: unknown, code: unknown): Promise<PairingResult> {
  const result = await pairWithCode(port, code);
  if (result.ok) {
    await streamDeck.settings.setGlobalSettings({ port: result.port, token: result.token });
    link.configure(result.port, result.token);
  }
  return result;
}
