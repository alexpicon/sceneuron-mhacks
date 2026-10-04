// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/** Waits between reconnection attempts. */

export interface Backoff {
  readonly baseMs: number;
  readonly maxMs: number;
}

/** From half a second up to 30 seconds, so a restarted OBS is found quickly and an absent one costs little. */
export const RECONNECT: Backoff = { baseMs: 500, maxMs: 30_000 };

/**
 * The wait before retry number `attempt`, counting from 0: doubling from `baseMs` up to `maxMs`,
 * less up to half at random so retries do not fall into step with anything periodic.
 */
export function backoffDelay(
  attempt: number,
  random: () => number = Math.random,
  backoff: Backoff = RECONNECT,
): number {
  const ceiling = Math.min(backoff.maxMs, backoff.baseMs * 2 ** Math.min(Math.max(attempt, 0), 30));
  return Math.round(ceiling * (1 - random() / 2));
}

/** Resolves after `ms`, or as soon as `signal` aborts. */
export function sleep(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal.aborted) {
      resolve();
      return;
    }
    const done = () => {
      clearTimeout(timer);
      signal.removeEventListener("abort", done);
      resolve();
    };
    const timer = setTimeout(done, ms);
    signal.addEventListener("abort", done, { once: true });
  });
}
