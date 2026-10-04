// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * The reach the controls show for the link's state. Unpaired, refused, and offline show at once. A
 * link that is connecting holds a token, so where it was unpaired, as when the plugin starts or
 * right after a first pairing, it shows offline. Otherwise it keeps the reach shown before it, so
 * the controls keep their faces: it connects again right after a stream ends and after pairing
 * again, and the proof that follows usually settles within milliseconds, so a revoke goes from
 * live straight to "Pair again". A link still connecting after 1.5 s is shown as offline.
 */

import type { Reach } from "./faces.ts";
import type { LinkState } from "./link.ts";

/** How long a connecting link keeps the reach shown before it. */
const CONNECTING_GRACE_MS = 1_500;

export class ShownReach {
  readonly #onLapse: () => void;
  /** Unpaired, as a new link is, until the constructor follows the link's state. */
  #reach: Reach = "unpaired";
  #grace: ReturnType<typeof setTimeout> | null = null;

  /**
   * Starts from link state `state`. `onLapse` is called once the reach has turned offline because
   * the link has been connecting for the whole grace period.
   */
  constructor(state: LinkState, onLapse: () => void) {
    this.#onLapse = onLapse;
    this.follow(state);
  }

  get reach(): Reach {
    return this.#reach;
  }

  /** Follows the link to `state`; a link still connecting keeps the grace it started with. */
  follow(state: LinkState): void {
    if (state === "connecting") {
      if (this.#reach === "unpaired") {
        this.#reach = "offline";
      }
      this.#grace ??= setTimeout(() => {
        this.#grace = null;
        this.#reach = "offline";
        this.#onLapse();
      }, CONNECTING_GRACE_MS);
      return;
    }
    if (this.#grace !== null) {
      clearTimeout(this.#grace);
      this.#grace = null;
    }
    this.#reach = state === "connected" ? "live" : state;
  }
}
