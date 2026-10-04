// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Every Sceneuron control on the Stream Deck, drawn from the link's latest view. Each control is
 * drawn again whenever the view, the connection, or its settings change; an unchanged drawing is
 * not sent again. While the link is connecting, the controls keep the reach they showed for up to
 * 1.5 s; see `ShownReach`.
 */

import streamDeck, { type DialDownEvent, type KeyDownEvent, type WillAppearEvent } from "@elgato/streamdeck";

import {
  automationFace,
  cardFace,
  dialCard,
  dialFace,
  elementFace,
  lowerFace,
  type Reach,
  roomFace,
  talkFace,
} from "./lib/faces.ts";
import type { Link } from "./lib/link.ts";
import { PlayingReaction } from "./lib/playing.ts";
import { reactionFace } from "./lib/reaction-face.ts";
import { ShownReach } from "./lib/reach.ts";
import { NoRoomNote } from "./lib/room-note.ts";
import {
  readCardSettings,
  readDialSettings,
  readElementSettings,
  readLowerSettings,
  readReactionSettings,
} from "./lib/settings.ts";
import { keySvg, stripSvg, svgDataUrl } from "./lib/svg.ts";
import { PushToTalk } from "./lib/talk.ts";
import { findDeck } from "./lib/view.ts";

export type Kind =
  | "card"
  | "lower"
  | "element"
  | "automation"
  | "dial"
  | "talk"
  | "reveal"
  | "next-round"
  | "reaction";

/** An action and its settings as Stream Deck reports them. */
export type AnyAction = WillAppearEvent["action"];
export type Settings = WillAppearEvent["payload"]["settings"];

/** A control that can show the alert sign: a key or a dial. */
export type Pressable = KeyDownEvent["action"] | DialDownEvent["action"];

interface Entry {
  readonly action: AnyAction;
  readonly kind: Kind;
  readonly settings: Settings;
  drawn: string;
}

export class Board {
  readonly link: Link;
  readonly talk: PushToTalk;
  readonly #entries = new Map<string, Entry>();
  /** The card each dial stood on when it was last drawn; see `dialCard`. */
  readonly #lastCards = new Map<string, string>();
  readonly #shown: ShownReach;
  /** Whether a press of a room key just found no room; see `roomFace`. */
  readonly #noRoom: NoRoomNote;
  /** The reaction playing on the overlay, which lights its keys. */
  readonly #playing: PlayingReaction;

  constructor(link: Link) {
    this.link = link;
    this.talk = new PushToTalk(link, () => this.#drawAll());
    this.#shown = new ShownReach(link.state, () => this.#drawAll());
    this.#noRoom = new NoRoomNote(link.state, () => this.#drawAll());
    this.#playing = new PlayingReaction(() => this.#drawAll());
    link.subscribe(() => {
      this.#shown.follow(link.state);
      this.#noRoom.follow(link.state);
      this.#playing.follow(link.view);
      this.#drawAll();
    });
  }

  get reach(): Reach {
    return this.#shown.reach;
  }

  /** Records what a press of a room key found: no room, or one. */
  noteRoom(found: boolean): void {
    if (this.#noRoom.note(found)) {
      this.#drawAll();
    }
  }

  /** Draws `action` now and whenever anything it shows changes. */
  show(action: AnyAction, kind: Kind, settings: Settings): void {
    const entry = { action, kind, settings, drawn: this.#entries.get(action.id)?.drawn ?? "" };
    this.#entries.set(action.id, entry);
    void this.#draw(entry);
  }

  /** Stops drawing an action that left the Stream Deck. */
  forget(id: string): void {
    this.#entries.delete(id);
    this.#lastCards.delete(id);
  }

  lastCard(id: string): string | undefined {
    return this.#lastCards.get(id);
  }

  /**
   * Runs a press; a failure shows the alert sign on the control. Never rejects: the SDK does not
   * await event handlers, so a rejection would be unhandled and can stop the plugin.
   */
  async press(action: Pressable, request: () => Promise<void>): Promise<void> {
    try {
      await request();
    } catch (error) {
      streamDeck.logger.warn(`a press failed: ${error instanceof Error ? error.message : String(error)}`);
      await action.showAlert().catch(() => undefined);
    }
  }

  /**
   * Lets go of push to talk for a key that left the Stream Deck or a device that was unplugged.
   * A failure is only logged: no key is left to show the alert, and the let-go is sent again once
   * the link connects.
   */
  letGo(release: () => Promise<void>): void {
    release().catch((error: unknown) => {
      const reason = error instanceof Error ? error.message : String(error);
      streamDeck.logger.warn(`letting go of push to talk failed: ${reason}`);
    });
  }

  #drawAll(): void {
    for (const entry of this.#entries.values()) {
      void this.#draw(entry);
    }
  }

  async #draw(entry: Entry): Promise<void> {
    // Before the saved pairing is read, a paired plugin would briefly say "Not paired"; the key
    // keeps its action image until the link knows.
    if (!this.link.settled) {
      return;
    }
    const view = this.link.view;
    const reach = this.reach;
    const { action } = entry;
    try {
      switch (entry.kind) {
        case "card":
          return await this.#key(entry, keySvg(cardFace(view, reach, readCardSettings(entry.settings))));
        case "lower":
          return await this.#key(entry, keySvg(lowerFace(view, reach, readLowerSettings(entry.settings))));
        case "element":
          return await this.#key(entry, keySvg(elementFace(view, reach, readElementSettings(entry.settings))));
        case "automation": {
          const { face, state } = automationFace(view, reach);
          return await this.#key(entry, keySvg(face), state);
        }
        case "talk":
          return await this.#key(entry, keySvg(talkFace(view, reach, this.talk.holding)));
        case "reveal":
        case "next-round":
          return await this.#key(entry, keySvg(roomFace(entry.kind, view, reach, this.#noRoom.active)));
        case "reaction": {
          const settings = readReactionSettings(entry.settings);
          return await this.#key(entry, keySvg(reactionFace(reach, settings, this.#playing.flood)));
        }
        case "dial": {
          const settings = readDialSettings(entry.settings);
          const card = dialCard(findDeck(view, settings.deck), this.#lastCards.get(action.id));
          if (card !== null) {
            this.#lastCards.set(action.id, card);
          }
          const svg = stripSvg(dialFace(view, reach, settings, card));
          if (action.isDial() && svg !== entry.drawn) {
            entry.drawn = svg;
            await action.setFeedback({ card: svgDataUrl(svg) });
          }
          return;
        }
      }
    } catch (error) {
      streamDeck.logger.warn(`drawing a ${entry.kind} control failed: ${String(error)}`);
    }
  }

  async #key(entry: Entry, svg: string, state?: 0 | 1): Promise<void> {
    const drawn = `${state ?? ""}${svg}`;
    if (!entry.action.isKey() || drawn === entry.drawn) {
      return;
    }
    entry.drawn = drawn;
    if (state !== undefined) {
      await entry.action.setState(state);
    }
    await entry.action.setImage(svgDataUrl(svg));
  }
}
