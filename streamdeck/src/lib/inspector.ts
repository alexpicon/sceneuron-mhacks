// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Messages between the plugin and its property inspector. The inspector never sees the token:
 * it gets the connection state and the names it can offer, and sends back the port and code.
 */

import { BUILT_IN_REACTIONS } from "./built-in-reactions.ts";
import type { LinkState } from "./link.ts";
import type { ControlView } from "./view.ts";

/** What the inspector shows: the connection, and the decks, elements, and reactions to choose from. */
export interface InspectorStatus {
  readonly type: "status";
  readonly state: LinkState;
  readonly port: number;
  readonly message: string;
  /** Empty until the plugin has seen a control view. */
  readonly decks: readonly { id: string; name: string; cards: { id: string; name: string }[] }[];
  readonly elements: readonly { id: string; name: string }[];
  /** Every reaction preset, which the plugin knows before it is paired. */
  readonly reactions: readonly { id: string; name: string }[];
}

/** The answer to a pairing request. */
export interface InspectorPairing {
  readonly type: "paired";
  readonly ok: boolean;
  readonly message: string;
}

/** What the inspector may ask for. */
export type InspectorRequest =
  | { readonly type: "refresh" }
  | { readonly type: "pair"; readonly port: unknown; readonly code: unknown }
  | { readonly type: "forget" };

/** The connection state in a sentence. */
export function statusMessage(state: LinkState, port: number): string {
  switch (state) {
    case "unpaired":
      return (
        "Not paired. In the Sceneuron editor's Cards tab, choose Pair the Stream Deck on this computer, or choose " +
        "Connect a Stream Deck and enter its port and code here."
      );
    case "connecting":
      return `Connecting to Sceneuron on port ${port}...`;
    case "connected":
      return `Connected to Sceneuron on port ${port}.`;
    case "refused":
      return (
        `The server on port ${port} does not recognize this Stream Deck. If you disconnected Stream Decks ` +
        "in the editor, or Sceneuron moved to another port, pair again."
      );
    case "offline":
      return `Waiting for Sceneuron on port ${port}. Start OBS (or sceneuron-dev); this reconnects by itself.`;
  }
}

export function inspectorStatus(state: LinkState, port: number, view: ControlView | null): InspectorStatus {
  return {
    type: "status",
    state,
    port,
    message: statusMessage(state, port),
    decks: (view?.decks ?? []).map((deck) => ({
      id: deck.id,
      name: deck.name,
      cards: deck.cards.map((card) => ({ id: card.id, name: card.name })),
    })),
    elements: (view?.elements ?? []).map((element) => ({ id: element.id, name: element.name })),
    reactions: BUILT_IN_REACTIONS.map((preset) => ({ id: preset.id, name: `${preset.emojis[0]} ${preset.name}` })),
  };
}

/** A request from the inspector, or null when `payload` is not one. */
export function readInspectorRequest(payload: unknown): InspectorRequest | null {
  if (typeof payload !== "object" || payload === null) {
    return null;
  }
  const message = payload as Record<string, unknown>;
  switch (message.type) {
    case "refresh":
      return { type: "refresh" };
    case "forget":
      return { type: "forget" };
    case "pair":
      return { type: "pair", port: message.port, code: message.code };
    default:
      return null;
  }
}
