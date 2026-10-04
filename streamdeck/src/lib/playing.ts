// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Which reaction plays on the overlay, for lighting its keys. A control view taken while one plays
 * names its preset and how long it still plays, so it counts as playing from when the plugin got
 * that view until then, whoever played it: a key here, the editor, a rule, or the camera. A later
 * view of the same reaction can only shorten that, and a view without one ends it.
 */

import type { ControlView } from "./view.ts";

export class PlayingReaction {
  readonly #onEnd: () => void;
  readonly #now: () => number;
  #nonce: number | null = null;
  #flood: string | null = null;
  #until = 0;
  #end: ReturnType<typeof setTimeout> | null = null;

  /** `onEnd` is called when a reaction stops playing by itself; `now` is the clock, in milliseconds. */
  constructor(onEnd: () => void, now: () => number = Date.now) {
    this.#onEnd = onEnd;
    this.#now = now;
  }

  /** The preset playing now, or null. */
  get flood(): string | null {
    return this.#end !== null ? this.#flood : null;
  }

  /** Follows the newest control view, or none; returns whether `flood` changed. */
  follow(view: ControlView | null): boolean {
    const before = this.flood;
    this.#stop();
    const playing = view?.flood;
    if (playing) {
      const now = this.#now();
      const ends = now + playing.left_ms;
      this.#until = playing.nonce === this.#nonce ? Math.min(this.#until, ends) : ends;
      this.#nonce = playing.nonce;
      this.#flood = playing.flood;
      if (this.#until > now) {
        this.#end = setTimeout(() => {
          this.#end = null;
          this.#onEnd();
        }, this.#until - now);
      }
    }
    return this.flood !== before;
  }

  #stop(): void {
    if (this.#end !== null) {
      clearTimeout(this.#end);
      this.#end = null;
    }
  }
}
