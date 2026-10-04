// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Whether a press of a room key just found no room, for when Sceneuron's control view does not say.
 * Nothing tells the plugin when a room is joined in the editor, so the finding lasts 4 s, and ends
 * sooner when the link's state changes or a press finds a room: room keys that went on saying "No
 * room", darkened, after a join would look as if they could not act.
 */

import type { LinkState } from "./link.ts";

/** How long the room keys say "No room" after a press found none. */
const NO_ROOM_MS = 4_000;

export class NoRoomNote {
  readonly #onLapse: () => void;
  #state: LinkState;
  #lapse: ReturnType<typeof setTimeout> | null = null;

  /** Starts from link state `state`; `onLapse` is called when a finding ends by itself. */
  constructor(state: LinkState, onLapse: () => void) {
    this.#state = state;
    this.#onLapse = onLapse;
  }

  /** True while the last press's finding of no room holds. */
  get active(): boolean {
    return this.#lapse !== null;
  }

  /**
   * Records what a press found: no room starts the 4 s over, and a room ends them. Returns
   * whether `active` changed.
   */
  note(found: boolean): boolean {
    const before = this.active;
    this.#end();
    if (!found) {
      this.#lapse = setTimeout(() => {
        this.#lapse = null;
        this.#onLapse();
      }, NO_ROOM_MS);
    }
    return this.active !== before;
  }

  /** Follows the link to `state`; a change of state ends the finding. */
  follow(state: LinkState): void {
    if (state !== this.#state) {
      this.#state = state;
      this.#end();
    }
  }

  #end(): void {
    if (this.#lapse !== null) {
      clearTimeout(this.#lapse);
      this.#lapse = null;
    }
  }
}
