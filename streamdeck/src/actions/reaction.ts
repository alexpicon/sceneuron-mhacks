// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import { type KeyDownEvent, action } from "@elgato/streamdeck";

import type { Board } from "../board.ts";
import { readReactionSettings } from "../lib/settings.ts";
import { Control } from "./control.ts";

/**
 * "Reaction": the key shows its preset, such as "This meeting could have been an email", and a press
 * plays it on the overlay at once, whatever the cooldown that rules and the camera wait for. The
 * key lights up while the preset plays.
 */
@action({ UUID: "ai.chaski.sceneuron.reaction" })
export class ReactionKey extends Control {
  constructor(board: Board) {
    super(board, "reaction");
  }

  override async onKeyDown(ev: KeyDownEvent): Promise<void> {
    const { reaction } = readReactionSettings(ev.payload.settings);
    await this.board.press(ev.action, async () => {
      if (!reaction) {
        throw new Error("no reaction is chosen");
      }
      await this.board.link.send("/api/flood", { flood: reaction });
    });
  }
}
