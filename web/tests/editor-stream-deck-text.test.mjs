// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/stream-deck-text.js: the deep link that sends a pairing code to the Stream Deck app, telling
// OBS's dock browser, which cannot open that link, from other browsers, how the editor tells from the server's
// count of traded codes that a Stream Deck paired, and what it says when a code expired.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const T = require("../editor/stream-deck-text.js");

describe("the pairing deep link", () => {
  test("goes to the Sceneuron plugin's pair path, with the port, the code, and a passive window", () => {
    const link = T.pairingLink("4466", "092156");
    assert.equal(
      link,
      "streamdeck://plugins/message/ai.chaski.sceneuron/pair?port=4466&code=092156&streamdeck=hidden",
    );
    const url = new URL(link);
    const route = [url.protocol, url.host, url.pathname];
    assert.deepEqual(route, ["streamdeck:", "plugins", "/message/ai.chaski.sceneuron/pair"]);
    assert.deepEqual([...url.searchParams], [["port", "4466"], ["code", "092156"], ["streamdeck", "hidden"]]);
  });

  test("takes the port as the page address gives it, as a string or a number", () => {
    assert.match(T.pairingLink(80, "000000"), /\?port=80&code=000000&/);
    assert.match(T.pairingLink("65535", "999999"), /\?port=65535&/);
    assert.match(T.pairingLink("01", "123456"), /\?port=1&/);
  });

  test("is null for a port or code the plugin would refuse, so nothing half-valid is sent", () => {
    for (const port of ["0", "65536", "", "44 66", "4466&code=1", "-1", "1e3", null, undefined]) {
      assert.equal(T.pairingLink(port, "123456"), null, String(port));
    }
    for (const code of ["12345", "1234567", "123 456", "12345a", "", null, undefined, 123456.5]) {
      assert.equal(T.pairingLink("4466", code), null, String(code));
    }
  });
});

describe("telling OBS's dock browser", () => {
  const AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko)";

  test("is true for the user agent OBS gives its browser docks, on any OBS version", () => {
    assert.equal(T.inObs(`${AGENT} Chrome/127.0.6533.120 OBS/31.1.2 Safari/537.36`), true);
    assert.equal(T.inObs(`${AGENT} Chrome/103.0.5060.134 OBS/28.0.0 Safari/537.36`), true);
    assert.equal(T.inObs(`${AGENT} Chrome/127.0.6533.120 OBS/32.0.1 Safari/537.36`), true);
  });

  test("is false for browsers, and for names that only contain OBS", () => {
    assert.equal(T.inObs(`${AGENT} Chrome/140.0.0.0 Safari/537.36`), false);
    const firefox = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:143.0) Gecko/20100101 Firefox/143.0";
    assert.equal(T.inObs(firefox), false);
    assert.equal(T.inObs(`${AGENT} Chrome/140.0.0.0 Safari/537.36 JOBS/2.0`), false);
    assert.equal(T.inObs(`${AGENT} Chrome/140.0.0.0 Safari/537.36 OBS/`), false);
    assert.equal(T.inObs(undefined), false);
    assert.equal(T.inObs(""), false);
  });

  test("names the dock's way to pair, and the browser's one-click way", () => {
    assert.match(T.WORDS.inObs, /choose Connect a Stream Deck and enter the code in a key's settings/);
    assert.match(T.WORDS.inObs, /Open the editor in a browser to pair the Stream Deck on this computer/);
  });
});

describe("telling that a Stream Deck paired", () => {
  test("is true once the server counts more traded codes than when the code was issued", () => {
    assert.equal(T.pairedSince(0, { pairings: 1 }), true);
    assert.equal(T.pairedSince(3, { pairings: 5 }), true);
    assert.equal(T.pairedSince(3, { pairings: 3 }), false);
  });

  test("is false whenever either count is missing or not a whole number", () => {
    assert.equal(T.pairedSince(undefined, { pairings: 1 }), false, "an older reply without a count");
    assert.equal(T.pairedSince(0, {}), false);
    assert.equal(T.pairedSince(0, null), false);
    assert.equal(T.pairedSince(0, { pairings: "1" }), false);
    assert.equal(T.pairedSince(0.5, { pairings: 1 }), false);
  });
});

test("an expired code says how to try again, and after a deep link what to check first", () => {
  assert.equal(T.expiredNote(false), "This code expired. Choose Connect a Stream Deck for a new one.");
  const sent = T.expiredNote(true);
  assert.match(sent, /Stream Deck app, 7\.1 or newer, is running with the Sceneuron plugin/);
  assert.match(sent, /Pair the Stream Deck on this computer again/);
  assert.match(sent, /Connect a Stream Deck/);
});
