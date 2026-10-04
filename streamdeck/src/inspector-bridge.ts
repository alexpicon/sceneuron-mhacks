// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Answers the property inspector: keeps it told about the connection and the names it can offer,
 * and pairs when it sends a code. The token goes from the pairing reply straight into the global
 * settings and never back to the inspector.
 */

import streamDeck from "@elgato/streamdeck";

import { guarded, pairAndKeep } from "./keep-pairing.ts";
import {
  type InspectorPairing,
  type InspectorRequest,
  inspectorStatus,
  readInspectorRequest,
} from "./lib/inspector.ts";
import type { Link } from "./lib/link.ts";

export function bridgeInspector(link: Link): void {
  let sent = "";
  const push = async (force: boolean): Promise<void> => {
    const status = inspectorStatus(link.state, link.port, link.view);
    const text = JSON.stringify(status);
    if (force || text !== sent) {
      sent = text;
      await streamDeck.ui.sendToPropertyInspector(JSON.parse(text));
    }
  };

  const answer = async (request: InspectorRequest): Promise<void> => {
    switch (request.type) {
      case "refresh":
        return push(true);
      case "forget":
        await streamDeck.settings.setGlobalSettings({ port: link.port });
        link.configure(link.port, null);
        return;
      case "pair": {
        const result = await pairAndKeep(link, request.port, request.code);
        const reply: InspectorPairing = { type: "paired", ok: result.ok, message: result.message };
        await streamDeck.ui.sendToPropertyInspector({ ...reply });
        return;
      }
    }
  };

  link.subscribe(() => guarded("updating the inspector", () => push(false)));
  streamDeck.ui.onDidAppear(() => guarded("updating the inspector", () => push(true)));
  streamDeck.ui.onSendToPlugin((ev) => {
    const request = readInspectorRequest(ev.payload);
    if (request) {
      guarded(`the inspector's ${request.type} request`, () => answer(request));
    }
  });
}
