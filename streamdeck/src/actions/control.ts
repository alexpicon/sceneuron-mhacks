// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import {
  type DidReceiveSettingsEvent,
  SingletonAction,
  type WillAppearEvent,
  type WillDisappearEvent,
} from "@elgato/streamdeck";

import type { Board, Kind } from "../board.ts";

/** What every Sceneuron action shares: it is drawn by the board while it is on the Stream Deck. */
export abstract class Control extends SingletonAction {
  protected readonly board: Board;
  readonly #kind: Kind;

  constructor(board: Board, kind: Kind) {
    super();
    this.board = board;
    this.#kind = kind;
  }

  override onWillAppear(ev: WillAppearEvent): void {
    this.board.show(ev.action, this.#kind, ev.payload.settings);
  }

  override onDidReceiveSettings(ev: DidReceiveSettingsEvent): void {
    this.board.show(ev.action, this.#kind, ev.payload.settings);
  }

  override onWillDisappear(ev: WillDisappearEvent): void {
    this.board.forget(ev.action.id);
  }
}
