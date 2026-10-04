// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import { type KeyDownEvent, action } from "@elgato/streamdeck";

import type { Board } from "../board.ts";
import { ApiError } from "../lib/api.ts";
import type { RoomKind } from "../lib/faces.ts";
import { Control } from "./control.ts";

/**
 * What the room answers when this Sceneuron is in no room: 409 to join one first, and 400 when the
 * room module refuses a caller outside a room or the build has no shared room.
 */
const NO_ROOM = new Set([400, 409]);

/**
 * How long a press waits for the room. Sceneuron waits up to 10 s for the shared room's server on
 * each action, so a press that gave up sooner could fail on the key and still take effect.
 */
const ROOM_TIMEOUT_MS = 12_000;

/**
 * A key that acts on the shared planning poker room this Sceneuron joined in the editor. A press
 * with no room joined changes nothing: when Sceneuron says there is no room it sends nothing, and
 * otherwise the room refuses it. Either way the key shows the alert and says "No room", for a few
 * seconds when only the refusal said so; see `NoRoomNote`.
 */
abstract class RoomKey extends Control {
  readonly #action: "reveal" | "next";

  constructor(board: Board, kind: RoomKind, roomAction: "reveal" | "next") {
    super(board, kind);
    this.#action = roomAction;
  }

  override async onKeyDown(ev: KeyDownEvent): Promise<void> {
    const room = this.board.link.view?.room;
    await this.board.press(ev.action, async () => {
      if (room && !room.joined) {
        throw new Error("this Sceneuron is not in a room");
      }
      try {
        await this.board.link.send("/api/room", { action: this.#action }, ROOM_TIMEOUT_MS);
      } catch (error) {
        if (error instanceof ApiError && NO_ROOM.has(error.status)) {
          this.board.noteRoom(false);
        }
        throw error;
      }
      this.board.noteRoom(true);
    });
  }
}

/** "Reveal votes": shows everyone's votes in the shared room at once. */
@action({ UUID: "ai.chaski.sceneuron.reveal" })
export class RevealKey extends RoomKey {
  constructor(board: Board) {
    super(board, "reveal", "reveal");
  }
}

/** "Next round": lowers every card in the shared room and clears the votes for the next round. */
@action({ UUID: "ai.chaski.sceneuron.next-round" })
export class NextRoundKey extends RoomKey {
  constructor(board: Board) {
    super(board, "next-round", "next");
  }
}
