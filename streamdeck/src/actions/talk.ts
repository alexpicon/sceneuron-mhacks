// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import { type KeyDownEvent, type KeyUpEvent, type WillDisappearEvent, action } from "@elgato/streamdeck";

import type { Board } from "../board.ts";
import { Control } from "./control.ts";

/**
 * "Push to talk": Sceneuron's voice control hears while the key is held. A key that leaves the
 * Stream Deck while held, as when its page changes or its device is unplugged, lets go.
 */
@action({ UUID: "ai.chaski.sceneuron.talk" })
export class TalkKey extends Control {
  constructor(board: Board) {
    super(board, "talk");
  }

  override async onKeyDown(ev: KeyDownEvent): Promise<void> {
    const voice = this.board.link.view?.voice;
    await this.board.press(ev.action, async () => {
      if (voice && !voice.enabled) {
        throw new Error("voice control is off in the Sceneuron editor");
      }
      await this.board.talk.hold(ev.action.id, ev.action.device.id);
    });
  }

  override async onKeyUp(ev: KeyUpEvent): Promise<void> {
    await this.board.press(ev.action, () => this.board.talk.release(ev.action.id));
  }

  override onWillDisappear(ev: WillDisappearEvent): void {
    super.onWillDisappear(ev);
    this.board.letGo(() => this.board.talk.release(ev.action.id));
  }
}
