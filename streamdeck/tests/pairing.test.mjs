// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises pairing against a fake Sceneuron: checking what the user typed, reading the editor's
// pairing deep link strictly, trading a code for the token in any characters the server allows,
// burning the code after five wrong tries, refusing a server that cannot prove the token, and the
// messages the property inspector shows for each failure.

import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";

import { ApiError } from "../src/lib/api.ts";
import { pairWithCode, pairingMessage, readPairingInput, readPairingLink } from "../src/lib/pairing.ts";
import { CONTROL_TOKEN, startFakeSceneuron } from "./fake-sceneuron.mjs";

let server;
beforeEach(async () => {
  server = await startFakeSceneuron({ code: "135790" });
});
afterEach(() => server.close());

test("typed ports and codes are read leniently and checked strictly", () => {
  assert.deepEqual(readPairingInput(" 4466 ", "135 790"), { port: 4466, code: "135790" });
  assert.deepEqual(readPairingInput(5000, "000000"), { port: 5000, code: "000000" });
  assert.match(readPairingInput("0", "135790").message, /port/);
  assert.match(readPairingInput("http://x", "135790").message, /port/);
  assert.match(readPairingInput("4466", "1357").message, /six digits/);
  assert.match(readPairingInput("4466", undefined).message, /six digits/);
});

test("a pairing link gives its port and code only when it is exactly what the editor writes", () => {
  const read = (url) => {
    const parsed = new URL(`streamdeck://${url}`);
    return readPairingLink(parsed.pathname, parsed.searchParams);
  };
  assert.deepEqual(read("/pair?port=4466&code=092156&streamdeck=hidden"), { port: 4466, code: "092156" });
  assert.deepEqual(read("/pair?code=000000&port=1"), { port: 1, code: "000000" });
  const refused = [
    "/unpair?port=4466&code=092156",
    "/pair/?port=4466&code=092156",
    "/pair?port=4466",
    "/pair?code=092156",
    "/pair?port=0&code=092156",
    "/pair?port=65536&code=092156",
    "/pair?port=%204466&code=092156",
    "/pair?port=4466.0&code=092156",
    "/pair?port=0x1172&code=092156",
    "/pair?port=4466&code=092%20156",
    "/pair?port=4466&code=09215",
    "/pair?port=4466&code=0921567",
    "/pair?port=4466&port=4467&code=092156",
    "/pair?port=4466&code=092156&code=135790",
  ];
  for (const url of refused) {
    assert.equal(read(url), null, url);
  }
  assert.deepEqual(readPairingLink("pair", new URLSearchParams("port=4466&code=092156")), {
    port: 4466,
    code: "092156",
  });
});

test("a good code returns the token once the server proves it", async () => {
  const result = await pairWithCode(String(server.port), "135790");
  assert.deepEqual(result, {
    ok: true,
    port: server.port,
    token: CONTROL_TOKEN,
    message: `Paired with Sceneuron on port ${server.port}.`,
  });
  assert.deepEqual(
    server.requests.map((request) => request.path),
    ["/api/control/pair", "/api/hello"],
  );
});

test("a token with dots and tildes, which the server allows, pairs too", async (t) => {
  const token = "my.control~token.1234";
  const dotted = await startFakeSceneuron({ token, code: "246802" });
  t.after(() => dotted.close());
  const result = await pairWithCode(dotted.port, "246802");
  assert.equal(result.ok, true, result.message);
  assert.equal(result.token, token);
});

test("a used code is refused", async () => {
  assert.equal((await pairWithCode(server.port, "135790")).ok, true);
  const again = await pairWithCode(server.port, "135790");
  assert.equal(again.ok, false);
  assert.match(again.message, /wrong, used, or expired/);
});

test("five wrong codes burn the real one", async () => {
  for (let attempt = 0; attempt < 5; attempt++) {
    assert.equal((await pairWithCode(server.port, "000000")).ok, false);
  }
  assert.equal((await pairWithCode(server.port, "135790")).ok, false);
});

test("a token the server cannot prove is not kept", async () => {
  server.stopProving();
  const result = await pairWithCode(server.port, "135790");
  assert.equal(result.ok, false);
  assert.match(result.message, /did not prove/);
  assert.equal("token" in result, false);
});

test("bad input never reaches the server", async () => {
  const result = await pairWithCode(server.port, "12-34");
  assert.equal(result.ok, false);
  assert.equal(server.requests.length, 0);
});

test("each failure has its own message", () => {
  assert.match(pairingMessage(new ApiError(0, "x"), 4466), /Nothing answered on port 4466/);
  assert.match(pairingMessage(new ApiError(401, "x"), 4466), /Update Sceneuron/);
  assert.match(pairingMessage(new ApiError(404, "x"), 4466), /Update Sceneuron/);
  assert.match(pairingMessage(new ApiError(500, "disk full"), 4466), /disk full/);
  assert.match(pairingMessage(new Error("boom"), 4466), /unexpectedly/);
});
