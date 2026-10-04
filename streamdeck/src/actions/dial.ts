// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import {
  type DialDownEvent,
  type DialRotateEvent,
  type TouchTapEvent,
  action,
} from "@elgato/streamdeck";

import type { Board, Pressable, Settings } from "../board.ts";
import { dialCard } from "../lib/faces.ts";
import { readDialSettings } from "../lib/settings.ts";
import { findDeck } from "../lib/view.ts";
import { Control } from "./control.ts";

/**
 * "Card dial": each tick of a turn steps the deck to its next or previous card, which shows at
 * once; pressing lowers the card that is up, or raises again the one the strip shows. A long
 * touch lowers the deck.
 *
 * A dial's turns and presses run one after another, so each starts from what the one before it
 * left on the strip, however fast Stream Deck sends them.
 */
@action({ UUID: "ai.chaski.sceneuron.dial" })
export class CardDial extends Control {
  /** The newest turn or press of each dial that is still running; the next one waits for it. */
  readonly #queues = new Map<string, Promise<void>>();

  constructor(board: Board) {
    super(board, "dial");
  }

  override onDialRotate(ev: DialRotateEvent): Promise<void> {
    const { deck } = readDialSettings(ev.payload.settings);
    const { ticks } = ev.payload;
    return this.#queue(ev.action, async () => {
      if (!deck) {
        throw new Error("no deck is chosen");
      }
      for (const body of this.#steps(ev.action.id, deck, ticks)) {
        await this.board.link.send("/api/card", body);
      }
    });
  }

  override onDialDown(ev: DialDownEvent): Promise<void> {
    return this.#toggle(ev.action, ev.payload.settings);
  }

  override onTouchTap(ev: TouchTapEvent): Promise<void> {
    if (!ev.payload.hold) {
      return this.#toggle(ev.action, ev.payload.settings);
    }
    const { deck } = readDialSettings(ev.payload.settings);
    return this.#queue(ev.action, async () => {
      if (!deck) {
        throw new Error("no deck is chosen");
      }
      await this.board.link.send("/api/card", { deck, action: "lower" });
    });
  }

  /**
   * The requests for a turn of `ticks`. While the deck is lowered the strip still shows a card,
   * and Sceneuron would step from nothing raised (next raises the first card, previous the
   * last), so the first tick raises the strip card's neighbor instead.
   */
  #steps(dial: string, deck: string, ticks: number): object[] {
    const step = ticks > 0 ? "next" : "previous";
    const steps: object[] = Array.from({ length: Math.abs(ticks) }, () => ({ deck, action: step }));
    const shown = findDeck(this.board.link.view, deck);
    const card = dialCard(shown, this.board.lastCard(dial));
    if (shown && shown.raised === null && card !== null && steps.length > 0) {
      const ids = shown.cards.map((entry) => entry.id);
      const neighbor = ids[(ids.indexOf(card) + Math.sign(ticks) + ids.length) % ids.length];
      steps[0] = { deck, card: neighbor, action: "raise" };
    }
    return steps;
  }

  #toggle(dial: Pressable, settings: Settings): Promise<void> {
    const deck = readDialSettings(settings).deck;
    return this.#queue(dial, async () => {
      const card = dialCard(findDeck(this.board.link.view, deck), this.board.lastCard(dial.id));
      if (!deck || card === null) {
        throw new Error("the dial shows no card");
      }
      await this.board.link.send("/api/card", { deck, card, action: "toggle" });
    });
  }

  /** Runs `request` as a press of `dial` once the dial's earlier turns and presses are done. */
  #queue(dial: Pressable, request: () => Promise<void>): Promise<void> {
    const run = (this.#queues.get(dial.id) ?? Promise.resolve()).then(() => this.board.press(dial, request));
    this.#queues.set(dial.id, run);
    return run.finally(() => {
      if (this.#queues.get(dial.id) === run) {
        this.#queues.delete(dial.id);
      }
    });
  }
}
