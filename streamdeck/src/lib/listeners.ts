// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/** Callbacks told of every change, such as the link's state and view. */

export class Listeners {
  readonly #listeners = new Set<() => void>();
  readonly #onError: (error: unknown) => void;

  /** `onError` hears what a listener throws; the listeners after it are still called. */
  constructor(onError: (error: unknown) => void) {
    this.#onError = onError;
  }

  /** Calls `listener` on every `notify`; returns a function that stops it. */
  add(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  notify(): void {
    for (const listener of this.#listeners) {
      try {
        listener();
      } catch (error) {
        this.#onError(error);
      }
    }
  }
}
