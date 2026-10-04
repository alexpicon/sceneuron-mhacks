// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the page link helpers: reading the token from the fragment or an older query, moving it
// to the fragment, the page's port, HMAC-SHA256 against a published vector, and proving the server
// before trusting it.

import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { webcrypto } from "node:crypto";

const require = createRequire(import.meta.url);
const { tokenFrom, fragmentAddress, pagePort, hmacHex, verifyServer } = require("../link.js");

test("the token comes from the fragment, then from an older query", () => {
  assert.equal(tokenFrom({ hash: "#token=abc", search: "?token=old" }), "abc");
  assert.equal(tokenFrom({ hash: "", search: "?x=1&token=old" }), "old");
  assert.equal(tokenFrom({ hash: "", search: "" }), "");
});

test("a query token moves to the fragment and other parameters stay", () => {
  assert.equal(fragmentAddress({ pathname: "/editor", search: "?token=abc&x=1" }), "/editor?x=1#token=abc");
  assert.equal(fragmentAddress({ pathname: "/overlay", search: "?token=abc" }), "/overlay#token=abc");
  assert.equal(fragmentAddress({ pathname: "/editor", search: "" }), null);
});

test("the page port is the address's own, or its scheme's default when the address leaves it out", () => {
  assert.equal(pagePort({ protocol: "http:", port: "4466" }), "4466");
  assert.equal(pagePort({ protocol: "http:", port: "" }), "80");
  assert.equal(pagePort({ protocol: "https:", port: "" }), "443");
  assert.equal(pagePort({ protocol: "https:", port: "8443" }), "8443");
});

test("hmacHex matches RFC 4231 test case 2", async () => {
  const proof = await hmacHex("Jefe", "what do ya want for nothing?", webcrypto.subtle);
  assert.equal(proof, "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
});

/** A fetch that answers hello like a server holding `tokens` on `port`. */
function server(tokens, port, { status = 200 } = {}) {
  const asked = [];
  const fetchImpl = async (url) => {
    asked.push(url);
    const nonce = new URL(url, "http://x").searchParams.get("nonce");
    const proof = (token) => hmacHex(token, `${nonce}:${port}`, webcrypto.subtle);
    const body = { editor: await proof(tokens.editor), overlay: await proof(tokens.overlay) };
    return { ok: status === 200, json: async () => body };
  };
  return { fetchImpl, asked };
}

test("the real server is trusted with either token", async () => {
  const real = server({ editor: "e".repeat(32), overlay: "o".repeat(32) }, 4466);
  assert.equal(await verifyServer("e".repeat(32), 4466, real.fetchImpl, webcrypto), true);
  assert.equal(await verifyServer("o".repeat(32), 4466, real.fetchImpl, webcrypto), true);
  assert.ok(real.asked.every((url) => !url.includes("e".repeat(32)) && !url.includes("o".repeat(32))));
});

test("another program, another port, refusals, and network errors are not trusted", async () => {
  const impostor = server({ editor: "x".repeat(32), overlay: "y".repeat(32) }, 4466);
  assert.equal(await verifyServer("e".repeat(32), 4466, impostor.fetchImpl, webcrypto), false);
  const moved = server({ editor: "e".repeat(32), overlay: "o".repeat(32) }, 50123);
  assert.equal(await verifyServer("e".repeat(32), 4466, moved.fetchImpl, webcrypto), false);
  const refusing = server({ editor: "e".repeat(32), overlay: "o".repeat(32) }, 4466, { status: 404 });
  assert.equal(await verifyServer("e".repeat(32), 4466, refusing.fetchImpl, webcrypto), false);
  const down = async () => {
    throw new TypeError("fetch failed");
  };
  assert.equal(await verifyServer("e".repeat(32), 4466, down, webcrypto), false);
});
