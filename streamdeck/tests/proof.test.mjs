// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the hello proof: HMAC-SHA256 against a published vector, the "nonce:port" message
// the server signs, fresh nonces in the server's accepted form, and the exact-match comparison.

import assert from "node:assert/strict";
import { test } from "node:test";

import { helloProof, hmacHex, newNonce, proofMatches } from "../src/lib/proof.ts";

test("hmacHex matches RFC 4231 test case 2", () => {
  const proof = hmacHex("Jefe", "what do ya want for nothing?");
  assert.equal(proof, "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
});

test("the proof signs the nonce and the port together", () => {
  const token = "control-token-0123456789abcdef";
  const nonce = "00112233445566778899aabbccddeeff";
  assert.equal(helloProof(token, nonce, 4466), hmacHex(token, `${nonce}:4466`));
  assert.notEqual(helloProof(token, "aa".repeat(16), 4466), helloProof(token, "aa".repeat(16), 4467));
  assert.notEqual(helloProof(token, "aa".repeat(16), 4466), helloProof(`${token}x`, "aa".repeat(16), 4466));
});

test("nonces are fresh 32-character hexadecimal strings", () => {
  const nonces = new Set(Array.from({ length: 50 }, newNonce));
  assert.equal(nonces.size, 50);
  for (const nonce of nonces) {
    assert.match(nonce, /^[0-9a-f]{32}$/);
  }
});

test("only the exact proof matches", () => {
  const expected = hmacHex("key", "message");
  assert.equal(proofMatches(expected, expected), true);
  assert.equal(proofMatches(expected, expected.toUpperCase()), false);
  assert.equal(proofMatches(expected, expected.slice(1)), false);
  assert.equal(proofMatches(expected, `${expected.slice(1)}é`), false);
  assert.equal(proofMatches(expected, undefined), false);
  assert.equal(proofMatches(expected, 42), false);
});
