// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import { type KeyDownEvent, action } from "@elgato/streamdeck";

import type { Board } from "../board.ts";
import { readElementSettings } from "../lib/settings.ts";
import { Control } from "./control.ts";

/** "Show or hide": framed while the element shows; a press flips it and holds the change. */
@action({ UUID: "ai.chaski.sceneuron.element" })
export class ElementKey extends Control {
  constructor(board: Board) {
    super(board, "element");
  }

  override async onKeyDown(ev: KeyDownEvent): Promise<void> {
    const { element } = readElementSettings(ev.payload.settings);
    await this.board.press(ev.action, async () => {
      if (!element) {
        throw new Error("no element is chosen");
      }
      await this.board.link.send("/api/control/element", { element, action: "toggle" });
    });
  }
}
