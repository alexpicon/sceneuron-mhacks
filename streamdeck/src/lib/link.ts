// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * The plugin's one connection to Sceneuron. It proves the server with a fresh hello nonce before
 * every (re)connect, keeps a single event stream open for the control view, and reconnects with
 * backoff. Requests that carry the token go out only while that proven stream is open, so a
 * program that takes the port after OBS quits never receives the token.
 */

import { type Endpoint, type Fetch, getJson, isRefusal, openStream, postJson, verifyServer } from "./api.ts";
import { type Backoff, RECONNECT, backoffDelay, sleep } from "./backoff.ts";
import { Listeners } from "./listeners.ts";
import { Session } from "./session.ts";
import { DEFAULT_PORT } from "./settings.ts";
import { readEvents } from "./sse.ts";
import { type ControlView, parseControlView, readControlView } from "./view.ts";

/**
 * - `unpaired`: no token yet.
 * - `connecting`: proving the server or opening the stream.
 * - `connected`: the stream is open; requests may go out.
 * - `refused`: the server on the port refused the token or did not prove it holds it (the token
 *   was revoked, or the program there is not Sceneuron).
 * - `offline`: nothing answered, or the stream dropped.
 */
export type LinkState = "unpaired" | "connecting" | "connected" | "refused" | "offline";

export interface LinkOptions {
  readonly fetch?: Fetch;
  readonly random?: () => number;
  readonly backoff?: Backoff;
  /** Silence after which a stream counts as dead; the server pings every 15 s. */
  readonly idleMs?: number;
  readonly log?: (message: string) => void;
}

/** Thrown by `send` while no proven stream is open. */
export class NotConnectedError extends Error {
  constructor() {
    super("not connected to Sceneuron");
    this.name = "NotConnectedError";
  }
}

const EVENTS_PATH = "/api/control/events";
const STATE_PATH = "/api/control/state";

export class Link {
  readonly #fetch: Fetch;
  readonly #random: () => number;
  readonly #backoff: Backoff;
  readonly #idleMs: number;
  readonly #log: (message: string) => void;
  readonly #listeners: Listeners;
  #port = DEFAULT_PORT;
  #endpoint: Endpoint | null = null;
  #state: LinkState = "unpaired";
  /** Whether `configure` has run once, so the link knows whether it is paired. */
  #settled = false;
  #view: ControlView | null = null;
  /** The newest proven stream; requests may use it while the state is `connected`. */
  #session: Session | null = null;
  #run: AbortController | null = null;

  constructor(options: LinkOptions = {}) {
    this.#fetch = options.fetch ?? fetch;
    this.#random = options.random ?? Math.random;
    this.#backoff = options.backoff ?? RECONNECT;
    this.#idleMs = options.idleMs ?? 45_000;
    this.#log = options.log ?? (() => undefined);
    this.#listeners = new Listeners((error) => this.#log(`a link listener failed: ${describe(error)}`));
  }

  get state(): LinkState {
    return this.#state;
  }

  /** Whether the saved pairing has been read; until then the state says nothing about pairing. */
  get settled(): boolean {
    return this.#settled;
  }

  get port(): number {
    return this.#port;
  }

  /** The newest control view; kept while offline so keys can still show what they control. */
  get view(): ControlView | null {
    return this.#view;
  }

  /** Calls `listener` whenever the state or the view changes; returns a function that stops it. */
  subscribe(listener: () => void): () => void {
    return this.#listeners.add(listener);
  }

  /**
   * Starts over for `port` and `token`; a null token leaves the link unpaired. A new token or port
   * goes straight to connecting and keeps the last view until the new stream sends one, so the
   * controls keep their faces meanwhile.
   */
  configure(port: number, token: string | null): void {
    const first = !this.#settled;
    this.#settled = true;
    const same = port === this.#port && token === (this.#endpoint?.token ?? null);
    if (same && (this.#run || !token)) {
      if (first) {
        this.#listeners.notify();
      }
      return;
    }
    this.#run?.abort();
    this.#run = null;
    this.#port = port;
    this.#endpoint = token ? { port, token } : null;
    if (!this.#endpoint) {
      this.#view = null;
    }
    // Notifies even when the state stays the same, since the port may have changed.
    this.#state = this.#endpoint ? "connecting" : "unpaired";
    this.#listeners.notify();
    if (this.#endpoint) {
      const run = new AbortController();
      this.#run = run;
      this.#loop(this.#endpoint, run.signal).catch((error: unknown) => {
        this.#log(`the link stopped: ${describe(error)}`);
      });
    }
  }

  /** Closes the stream and stops reconnecting; nothing more is sent until the next `configure`. */
  stop(): void {
    if (this.#run) {
      this.#run.abort();
      this.#run = null;
      this.#setState("offline");
    }
  }

  /**
   * Posts `body` to `path` with the token, waiting `timeoutMs` for an answer when given, else the
   * usual 5 s. A control view in the reply is taken at once only when it is newer than the one on
   * screen, since the stream sends every view anyway. A refused token ends the connection, so the
   * server must prove itself again before the next request.
   */
  async send(path: string, body: object, timeoutMs?: number): Promise<void> {
    const endpoint = this.#endpoint;
    const session = this.#session;
    if (this.#state !== "connected" || !endpoint || !session) {
      throw new NotConnectedError();
    }
    let reply: unknown;
    try {
      reply = await postJson(endpoint, path, body, this.#fetch, timeoutMs);
    } catch (error) {
      this.#endIfRefused(error, session);
      throw error;
    }
    this.#take(parseControlView(reply), session, true);
  }

  async #loop(endpoint: Endpoint, signal: AbortSignal): Promise<void> {
    let attempt = 0;
    while (!signal.aborted) {
      const { state, delivered } = await this.#connect(endpoint, signal);
      if (signal.aborted) {
        return;
      }
      attempt = delivered ? 0 : attempt + 1;
      this.#setState(state);
      await sleep(backoffDelay(attempt, this.#random, this.#backoff), signal);
    }
  }

  /**
   * One attempt: prove the server, then read the stream until it ends. `delivered` tells whether
   * the stream sent a view, which resets the backoff; a server that keeps accepting and closing
   * the stream at once is retried ever more slowly. Sceneuron ends the streams of a token it
   * revokes, so a stream that ends is followed by a proof at once, and a revoked token shows as
   * refused right away rather than offline until the next attempt.
   */
  async #connect(endpoint: Endpoint, signal: AbortSignal): Promise<{ state: LinkState; delivered: boolean }> {
    let delivered = false;
    const session = new Session(signal);
    try {
      if (!(await verifyServer(endpoint.port, endpoint.token, this.#fetch, signal))) {
        this.#log(`the server on port ${endpoint.port} did not prove it holds this plugin's token`);
        return { state: "refused", delivered };
      }
      const body = await openStream(endpoint, EVENTS_PATH, this.#fetch, session.signal);
      if (signal.aborted) {
        // Reconfigured while the stream opened: a newer run owns the state now.
        await body.cancel();
        return { state: "offline", delivered };
      }
      this.#session = session;
      this.#setState("connected");
      void this.#seed(endpoint, session);
      for await (const event of readEvents(body, this.#idleMs)) {
        if (event.event === "control") {
          delivered = this.#take(readControlView(event.data), session, false) || delivered;
        }
      }
      this.#setState("connecting");
      if (!(await verifyServer(endpoint.port, endpoint.token, this.#fetch, signal))) {
        this.#log(`the server on port ${endpoint.port} ended the stream and no longer holds this plugin's token`);
        return { state: "refused", delivered };
      }
      return { state: "offline", delivered };
    } catch (error) {
      // A session ends only when its token was refused, which is already logged.
      const refused = session.ended || isRefusal(error);
      if (!signal.aborted && !session.ended) {
        this.#log(`connection to port ${endpoint.port} failed: ${describe(error)}`);
      }
      return { state: refused ? "refused" : "offline", delivered };
    }
  }

  /** Asks for the view once, in case the stream only sends changes. */
  async #seed(endpoint: Endpoint, session: Session): Promise<void> {
    try {
      this.#take(parseControlView(await getJson(endpoint, STATE_PATH, this.#fetch, session.signal)), session, false);
    } catch (error) {
      if (!session.signal.aborted) {
        this.#log(`reading the control state failed: ${describe(error)}`);
        this.#endIfRefused(error, session);
      }
    }
  }

  /**
   * Ends `session` when the server refused its token. Sceneuron ends a revoked token's streams,
   * but a server that keeps one open leaves a refused request as the only sign; the next attempt
   * proves the server again.
   */
  #endIfRefused(error: unknown, session: Session): void {
    if (isRefusal(error) && session === this.#session && this.#state === "connected") {
      this.#log(`the server on port ${this.#port} refused this plugin's token`);
      this.#setState("refused");
      session.end();
    }
  }

  /** Takes `view` if it belongs to the open session and the session admits it; true if taken. */
  #take(view: ControlView | null, session: Session, strictly: boolean): boolean {
    const current = this.#state === "connected" && session === this.#session;
    if (!view || !current || !session.admit(view, strictly)) {
      return false;
    }
    this.#view = view;
    this.#listeners.notify();
    return true;
  }

  #setState(state: LinkState): void {
    if (state !== this.#state) {
      this.#state = state;
      this.#listeners.notify();
    }
  }
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
