// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the event stream reader: fields, comments, multi-line data, every line ending even
// when split across chunks, UTF-8 split across chunks, and the idle limit that ends a dead stream.

import assert from "node:assert/strict";
import { test } from "node:test";

import { SseParser, StreamIdleError, readEvents } from "../src/lib/sse.ts";

test("named events, default names, comments, and multi-line data", () => {
  const parser = new SseParser();
  const events = parser.push(": ping\n\nevent: control\ndata: {\"a\":1}\n\ndata: one\ndata:two\nid: 7\n\n");
  assert.deepEqual(events, [
    { event: "control", data: '{"a":1}' },
    { event: "message", data: "one\ntwo" },
  ]);
});

test("an event without data is not sent, and its name does not leak into the next", () => {
  const parser = new SseParser();
  assert.deepEqual(parser.push("event: control\n\ndata: x\n\n"), [{ event: "message", data: "x" }]);
});

test("events split anywhere across chunks come out whole", () => {
  const text = "event: control\r\ndata: {\"revision\":2}\r\n\r\nevent: control\rdata: 3\r\r";
  for (let size = 1; size <= text.length; size++) {
    const parser = new SseParser();
    const events = [];
    for (let at = 0; at < text.length; at += size) {
      events.push(...parser.push(text.slice(at, at + size)));
    }
    assert.deepEqual(
      events,
      [
        { event: "control", data: '{"revision":2}' },
        { event: "control", data: "3" },
      ],
      `chunks of ${size}`,
    );
  }
});

test("a CR at the end of a chunk followed by an empty chunk keeps its LF pairing", () => {
  const parser = new SseParser();
  assert.deepEqual(parser.push("data: x\r"), []);
  assert.deepEqual(parser.push(""), []);
  assert.deepEqual(parser.push("\n"), []);
  assert.deepEqual(parser.push("\r\n"), [{ event: "message", data: "x" }]);
});

/** A byte stream that sends `chunks`, then stays open unless `end` is set. */
function body(chunks, { end = true } = {}) {
  return new ReadableStream({
    start(controller) {
      for (const chunk of chunks) {
        controller.enqueue(chunk);
      }
      if (end) {
        controller.close();
      }
    },
  });
}

test("readEvents decodes UTF-8 split between chunks", async () => {
  const bytes = new TextEncoder().encode("event: control\ndata: ☕ café\n\n");
  const split = bytes.indexOf(0xe2) + 1;
  const events = [];
  for await (const event of readEvents(body([bytes.slice(0, split), bytes.slice(split)]), 1000)) {
    events.push(event);
  }
  assert.deepEqual(events, [{ event: "control", data: "☕ café" }]);
});

test("a stream that goes silent ends with StreamIdleError", async () => {
  const events = [];
  const stream = body([new TextEncoder().encode("data: first\n\n")], { end: false });
  await assert.rejects(async () => {
    for await (const event of readEvents(stream, 50)) {
      events.push(event);
    }
  }, StreamIdleError);
  assert.deepEqual(events, [{ event: "message", data: "first" }]);
});
