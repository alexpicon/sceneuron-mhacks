// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the settings readers: the global port and token with their defaults, action settings
// that keep only non-empty ids, and the exact checks for ports, pairing codes, and tokens.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  DEFAULT_PORT,
  isPairingCode,
  isPort,
  isToken,
  readCardSettings,
  readDialSettings,
  readElementSettings,
  readGlobalSettings,
  readLowerSettings,
} from "../src/lib/settings.ts";

test("global settings keep a valid port and token and default the rest", () => {
  const token = "abcdefghijklmnop_0123-XYZ";
  assert.deepEqual(readGlobalSettings({ port: 5000, token }), { port: 5000, token });
  assert.deepEqual(readGlobalSettings({}), { port: DEFAULT_PORT, token: null });
  assert.deepEqual(readGlobalSettings({ port: 70000, token: "short" }), { port: DEFAULT_PORT, token: null });
  assert.deepEqual(readGlobalSettings({ port: "4466", token: "has spaces in it, so no" }), {
    port: DEFAULT_PORT,
    token: null,
  });
  assert.deepEqual(readGlobalSettings(null), { port: DEFAULT_PORT, token: null });
});

test("action settings keep only non-empty strings", () => {
  assert.deepEqual(readCardSettings({ deck: "poker", card: "p5", other: 1 }), { deck: "poker", card: "p5" });
  assert.deepEqual(readCardSettings({ deck: "", card: 5 }), { deck: undefined, card: undefined });
  assert.deepEqual(readLowerSettings(undefined), { deck: undefined });
  assert.deepEqual(readElementSettings({ element: "brb" }), { element: "brb" });
  assert.deepEqual(readDialSettings([]), { deck: undefined });
});

test("ports, codes, and tokens are checked exactly", () => {
  assert.ok(isPort(1) && isPort(65535) && isPort(4466));
  assert.ok(!isPort(0) && !isPort(65536) && !isPort(44.5) && !isPort("4466"));
  assert.ok(isPairingCode("012345"));
  assert.ok(!isPairingCode("12345") && !isPairingCode("1234567") && !isPairingCode("12a456") && !isPairingCode(123456));
  assert.ok(isToken("a".repeat(16)) && isToken("A-_9".repeat(32)));
  assert.ok(!isToken("a".repeat(15)) && !isToken("a".repeat(129)) && !isToken(`${"a".repeat(16)}=`));
  for (const other of ["+", "/", "%", " ", "é"]) {
    assert.ok(!isToken(`${"a".repeat(16)}${other}`), other);
  }
});

test("tokens may use the server's whole URL-safe alphabet, dots and tildes included", () => {
  const token = "my.control~token.1234";
  assert.ok(isToken(token) && isToken(".~-_".repeat(4)));
  assert.deepEqual(readGlobalSettings({ port: 4466, token }), { port: 4466, token });
});
