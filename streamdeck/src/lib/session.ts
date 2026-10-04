// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * One proven event stream. Each stream gets a new session, so a reply from before a reconnect
 * cannot land on the new one, and ending a session ends only its own stream.
 */

import type { ControlView } from "./view.ts";

export class Session {
  /** Aborts when the link stops or this session ends; the stream and its requests listen to it. */
  readonly signal: AbortSignal;
  readonly #end = new AbortController();
  #revision = -1;

  constructor(run: AbortSignal) {
    this.signal = AbortSignal.any([run, this.#end.signal]);
  }

  /** True once `end` was called. */
  get ended(): boolean {
    return this.#end.signal.aborted;
  }

  /** Closes the stream; the link proves the server again before it opens another. */
  end(): void {
    this.#end.abort();
  }

  /**
   * True when `view` may replace the one on screen, and it then counts as the newest: never when
   * it is older than the newest taken, and with `strictly`, not when it is as old either.
   */
  admit(view: ControlView, strictly: boolean): boolean {
    if (view.revision < this.#revision || (strictly && view.revision === this.#revision)) {
      return false;
    }
    this.#revision = view.revision;
    return true;
  }
}
