// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the overlay page helpers in overlay.js: the event stream URL, reconnect backoff,
// and snapshot parsing.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const O = require("../overlay.js");

describe("streamUrl", () => {
  test("passes the page token to the event stream", () => {
    assert.equal(O.streamUrl("abc123"), "/api/events?token=abc123");
    assert.equal(O.streamUrl("a b&c"), "/api/events?token=a%20b%26c");
    assert.equal(O.streamUrl(""), "/api/events?token=");
  });
});

describe("nextDelay", () => {
  test("doubles from one second up to ten", () => {
    const delays = [];
    let delay = 0;
    for (let attempt = 0; attempt < 6; attempt += 1) {
      delay = O.nextDelay(delay);
      delays.push(delay);
    }
    assert.deepEqual(delays, [1000, 2000, 4000, 8000, 10000, 10000]);
  });
});

describe("parseSnapshot", () => {
  test("accepts snapshot objects only", () => {
    assert.deepEqual(O.parseSnapshot('{"revision":1,"elements":[]}'), { revision: 1, elements: [] });
    for (const data of ["", "nope", "null", "[]", '{"elements":{}}', "42"]) {
      assert.equal(O.parseSnapshot(data), null, data);
    }
  });
});
