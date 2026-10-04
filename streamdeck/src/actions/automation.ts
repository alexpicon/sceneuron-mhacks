// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import { type KeyDownEvent, action } from "@elgato/streamdeck";

import type { Board } from "../board.ts";
import { Control } from "./control.ts";

/** "Pause automation": state 0 while rules and camera signals run, state 1 while they are paused. */
@action({ UUID: "ai.chaski.sceneuron.automation" })
export class AutomationKey extends Control {
  constructor(board: Board) {
    super(board, "automation");
  }

  override async onKeyDown(ev: KeyDownEvent): Promise<void> {
    const view = this.board.link.view;
    await this.board.press(ev.action, async () => {
      if (!view) {
        throw new Error("the automation state is not known yet");
      }
      await this.board.link.send("/api/automation", { enabled: !view.automation_enabled });
    });
  }
}
