// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Push to talk for every talk key: Sceneuron hears while any of them is held. The requests go out
 * one at a time, so a quick press never arrives after its release. Sceneuron lets go by itself a
 * minute after the last hold, so while a key is held the hold goes out again every 30 s.
 *
 * Once no key is held, a let-go is owed while a hold may have reached Sceneuron with no let-go
 * reaching it since, which includes a hold that failed after Sceneuron may have taken it. An owed
 * let-go that fails is tried again after 1, 2, and 4 s while the link stays connected, and again
 * each time the link connects after a drop. Holding never turns voice control on: audio goes
 * anywhere only when the owner turned it on and consented in the editor.
 */

import { ApiError } from "./api.ts";
import { type Link, NotConnectedError } from "./link.ts";

const TALK_PATH = "/api/voice/talk";

/** The waits push to talk keeps. */
export interface TalkTimes {
  /** How often a held key sends the hold again. */
  readonly refreshMs: number;
  /** The waits before each new try of an owed let-go that failed while the link stayed connected. */
  readonly retryMs: readonly number[];
}

const TIMES: TalkTimes = { refreshMs: 30_000, retryMs: [1_000, 2_000, 4_000] };

/**
 * Whether a request that failed with `error` may still have reached Sceneuron and been acted on.
 * Only one that never went out, or that Sceneuron refused, surely was not.
 */
function mayHaveLanded(error: unknown): boolean {
  if (error instanceof NotConnectedError) {
    return false;
  }
  return !(error instanceof ApiError && error.status >= 400 && error.status < 500);
}

export class PushToTalk {
  readonly #link: Link;
  readonly #onChange: () => void;
  readonly #times: TalkTimes;
  /** Each held key's action id and the device it is on. */
  readonly #held = new Map<string, string>();
  #queue: Promise<void> = Promise.resolve();
  /** A hold may have reached Sceneuron with no let-go reaching it since. */
  #mayHold = false;
  #refresh: ReturnType<typeof setInterval> | null = null;
  #retry: ReturnType<typeof setTimeout> | null = null;
  /** How many tries again of an owed let-go were set off since a hold or let-go last reached Sceneuron. */
  #retries = 0;

  /** `onChange` is called whenever a key is held or let go. */
  constructor(link: Link, onChange: () => void, times: TalkTimes = TIMES) {
    this.#link = link;
    this.#onChange = onChange;
    this.#times = times;
    let connected = link.state === "connected";
    link.subscribe(() => {
      const reconnected = !connected && link.state === "connected";
      connected = link.state === "connected";
      if (reconnected && this.#owed()) {
        this.#cancelRetry();
        this.#letGo().catch(() => undefined);
      }
    });
  }

  /** True while any talk key is held. */
  get holding(): boolean {
    return this.#held.size > 0;
  }

  /**
   * Holds push to talk for key `id` on `device`; rejects, and lets the key go, when Sceneuron did
   * not take it. A hold that failed after Sceneuron may have taken it is let go once no key holds.
   */
  async hold(id: string, device: string): Promise<void> {
    this.#held.set(id, device);
    this.#onChange();
    this.#startRefresh();
    try {
      await this.#enqueue(() => this.#sendHold());
    } catch (error) {
      if (this.#held.delete(id)) {
        this.#onChange();
        if (this.#held.size === 0) {
          this.#stopRefresh();
          if (this.#owed()) {
            this.#letGo().catch(() => undefined);
          }
        }
      }
      throw error;
    }
  }

  /** Lets key `id` go; push to talk is let go once no key holds it. A key that was not held sends nothing. */
  async release(id: string): Promise<void> {
    if (!this.#held.delete(id)) {
      return;
    }
    this.#onChange();
    if (this.#held.size === 0) {
      this.#stopRefresh();
      await this.#letGo();
    }
  }

  /** Lets go of every key on `device`, as when the device is unplugged. */
  async releaseDevice(device: string): Promise<void> {
    const ids = [...this.#held].filter(([, on]) => on === device).map(([id]) => id);
    await Promise.all(ids.map((id) => this.release(id)));
  }

  /** True when no key is held but Sceneuron may still be holding push to talk. */
  #owed(): boolean {
    return this.#mayHold && this.#held.size === 0;
  }

  async #sendHold(): Promise<void> {
    try {
      await this.#link.send(TALK_PATH, { down: true });
    } catch (error) {
      this.#mayHold ||= mayHaveLanded(error);
      throw error;
    }
    this.#mayHold = true;
    this.#cancelRetry();
  }

  /** Sends the let-go; one that fails stays owed while a hold may have reached Sceneuron. */
  #letGo(): Promise<void> {
    return this.#enqueue(async () => {
      try {
        await this.#link.send(TALK_PATH, { down: false });
      } catch (error) {
        this.#scheduleRetry();
        throw error;
      }
      this.#mayHold = false;
      this.#cancelRetry();
    });
  }

  /**
   * Tries an owed let-go again after the next wait while the link stays connected; once the waits
   * run out Sceneuron's own minute ends the hold, and a link that drops sends it when it connects.
   */
  #scheduleRetry(): void {
    const wait = this.#times.retryMs[this.#retries];
    if (wait === undefined || this.#retry !== null || !this.#owed() || this.#link.state !== "connected") {
      return;
    }
    this.#retries += 1;
    this.#retry = setTimeout(() => {
      this.#retry = null;
      if (this.#owed() && this.#link.state === "connected") {
        this.#letGo().catch(() => undefined);
      }
    }, wait);
    this.#retry.unref();
  }

  #cancelRetry(): void {
    if (this.#retry !== null) {
      clearTimeout(this.#retry);
      this.#retry = null;
    }
    this.#retries = 0;
  }

  /** Sends the hold again every `refreshMs` while a key is held, so Sceneuron does not let go by itself. */
  #startRefresh(): void {
    if (this.#refresh !== null) {
      return;
    }
    this.#refresh = setInterval(() => {
      if (this.#held.size > 0 && this.#link.state === "connected") {
        this.#enqueue(() => this.#sendHold()).catch(() => undefined);
      }
    }, this.#times.refreshMs);
    this.#refresh.unref();
  }

  #stopRefresh(): void {
    if (this.#refresh !== null) {
      clearInterval(this.#refresh);
      this.#refresh = null;
    }
  }

  #enqueue(request: () => Promise<void>): Promise<void> {
    const run = this.#queue.then(request);
    this.#queue = run.catch(() => undefined);
    return run;
  }
}
