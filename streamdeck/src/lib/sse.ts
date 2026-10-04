// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/** Server-sent events read from a fetch body, following the HTML event stream format. */

export interface SseEvent {
  /** The `event:` field, or "message" when the server sent none. */
  readonly event: string;
  readonly data: string;
}

/** Thrown when a stream stays silent past its idle limit. */
export class StreamIdleError extends Error {
  constructor(idleMs: number) {
    super(`no data for ${idleMs} ms`);
    this.name = "StreamIdleError";
  }
}

/** Turns chunks of text into events; lines may end in CRLF, LF, or CR and split across chunks. */
export class SseParser {
  #buffer = "";
  #skipLf = false;
  #event = "";
  #data: string[] = [];

  push(chunk: string): SseEvent[] {
    const events: SseEvent[] = [];
    let text = chunk;
    if (this.#skipLf && text.length > 0) {
      text = text.startsWith("\n") ? text.slice(1) : text;
      this.#skipLf = false;
    }
    this.#buffer += text;
    for (;;) {
      const end = this.#buffer.search(/[\r\n]/);
      if (end < 0) {
        break;
      }
      const line = this.#buffer.slice(0, end);
      const crlf = this.#buffer[end] === "\r" && this.#buffer[end + 1] === "\n";
      if (this.#buffer[end] === "\r" && end + 1 === this.#buffer.length) {
        this.#skipLf = true;
      }
      this.#buffer = this.#buffer.slice(end + (crlf ? 2 : 1));
      const event = this.#line(line);
      if (event) {
        events.push(event);
      }
    }
    return events;
  }

  #line(line: string): SseEvent | null {
    if (line === "") {
      const event = this.#data.length > 0 ? { event: this.#event || "message", data: this.#data.join("\n") } : null;
      this.#event = "";
      this.#data = [];
      return event;
    }
    if (line.startsWith(":")) {
      return null;
    }
    const colon = line.indexOf(":");
    const field = colon < 0 ? line : line.slice(0, colon);
    let value = colon < 0 ? "" : line.slice(colon + 1);
    if (value.startsWith(" ")) {
      value = value.slice(1);
    }
    if (field === "event") {
      this.#event = value;
    } else if (field === "data") {
      this.#data.push(value);
    }
    return null;
  }
}

/** `promise`, or a StreamIdleError once `ms` pass without it settling. */
function within<T>(promise: Promise<T>, ms: number): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new StreamIdleError(ms)), ms);
  });
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer));
}

/**
 * Events from `body` until it ends. A stream that sends nothing for `idleMs` (not even the
 * server's ping comment) is treated as dead, which catches connections lost while asleep.
 */
export async function* readEvents(body: ReadableStream<Uint8Array>, idleMs: number): AsyncGenerator<SseEvent> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  const parser = new SseParser();
  try {
    for (;;) {
      const { done, value } = await within(reader.read(), idleMs);
      if (done) {
        return;
      }
      yield* parser.push(decoder.decode(value, { stream: true }));
    }
  } finally {
    reader.cancel().catch(() => undefined);
  }
}
