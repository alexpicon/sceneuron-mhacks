// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import { type KeyDownEvent, action } from "@elgato/streamdeck";

import type { Board } from "../board.ts";
import { readCardSettings } from "../lib/settings.ts";
import { Control } from "./control.ts";

/** "Raise a card": the key is drawn as the card, framed while it is up; a press raises or lowers it. */
@action({ UUID: "ai.chaski.sceneuron.card" })
export class CardKey extends Control {
  constructor(board: Board) {
    super(board, "card");
  }

  override async onKeyDown(ev: KeyDownEvent): Promise<void> {
    const { deck, card } = readCardSettings(ev.payload.settings);
    await this.board.press(ev.action, async () => {
      if (!deck || !card) {
        throw new Error("no card is chosen");
      }
      await this.board.link.send("/api/card", { deck, card, action: "toggle" });
    });
  }
}
