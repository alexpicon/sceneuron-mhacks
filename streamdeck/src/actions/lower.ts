// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import { type KeyDownEvent, action } from "@elgato/streamdeck";

import type { Board } from "../board.ts";
import { lowerScope } from "../lib/faces.ts";
import { NotConnectedError } from "../lib/link.ts";
import { readLowerSettings } from "../lib/settings.ts";
import { Control } from "./control.ts";

/** "Lower cards": lowers the chosen deck, or every deck with a card up. */
@action({ UUID: "ai.chaski.sceneuron.lower" })
export class LowerKey extends Control {
  constructor(board: Board) {
    super(board, "lower");
  }

  override async onKeyDown(ev: KeyDownEvent): Promise<void> {
    const settings = readLowerSettings(ev.payload.settings);
    const link = this.board.link;
    await this.board.press(ev.action, async () => {
      // Lowering every deck sends nothing while no card is up, so a press that cannot act must
      // fail here to show the alert.
      if (link.state !== "connected") {
        throw new NotConnectedError();
      }
      if (settings.deck) {
        await link.send("/api/card", { deck: settings.deck, action: "lower" });
        return;
      }
      if (!link.view) {
        throw new Error("the decks are not known yet");
      }
      for (const deck of lowerScope(link.view, settings).filter((entry) => entry.raised !== null)) {
        await link.send("/api/card", { deck: deck.id, action: "lower" });
      }
    });
  }
}
