// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises what the Stream Deck live test reads from the plugin: request and answer lines written
// and read back among other output in the order the requests were sent, the link's own requests
// apart from pairing, the rule that each stream the link opens needs a proof of its own before the
// token goes out, and the connection states shown to the property inspector.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  answerLine,
  isProof,
  linkRequests,
  linkStates,
  readRequests,
  requestLine,
  tokenBeforeProof,
} from "../streamdeck-log.mjs";

const NONCE = "?nonce=0123456789abcdef0123456789abcdef";
const request = (path, fields = {}) => ({
  method: "GET",
  path,
  query: "",
  token: false,
  origin: false,
  status: 200,
  ...fields,
});
const hello = (fields = {}) => request("/api/hello", { query: NONCE, ...fields });
const pair = (fields = {}) => request("/api/control/pair", { method: "POST", ...fields });
const stream = (fields = {}) => request("/api/control/events", { token: true, ...fields });
const state = (fields = {}) => request("/api/control/state", { token: true, ...fields });

/**
 * Numbers `requests` in the order given, each answered before the next one is sent unless it
 * names its own `answeredAfter`.
 */
const sent = (...requests) =>
  requests.map((entry, index) => ({ seq: index + 1, answeredAfter: index + 1, ...entry }));

test("requests read back in the order sent, each with the answer that came for it", () => {
  const [first, second, third] = sent(hello(), stream({ answeredAfter: 3 }), state({ status: null }));
  const line = ({ status, answeredAfter, ...asked }) => requestLine(asked);
  const output = [
    "starting\n",
    line(first),
    answerLine(first),
    line(second),
    "a log line\r\n",
    line(third),
    answerLine(second),
  ].join("");
  assert.deepEqual(readRequests(output), [first, second, { ...third, answeredAfter: null }]);
  assert.deepEqual(readRequests("nothing to see\n"), []);
});

test("a request nothing answered reads back with status 0", () => {
  const [lost] = sent(hello({ status: 0 }));
  const { status, answeredAfter, ...asked } = lost;
  assert.deepEqual(readRequests(requestLine(asked) + answerLine(lost)), [lost]);
});

test("request and answer lines hold only what they are given to write, never a value", () => {
  const [entry] = sent(hello());
  for (const line of [requestLine({ ...entry, extra: "secret" }), answerLine({ ...entry, extra: "secret" })]) {
    assert.ok(!line.includes("secret"));
    assert.ok(line.endsWith("\n"));
  }
});

test("a proof is an answered hello with a 32-digit hexadecimal nonce", () => {
  assert.ok(isProof(hello()));
  assert.ok(!isProof(hello({ status: 0 })));
  assert.ok(!isProof(hello({ status: null })));
  assert.ok(!isProof(hello({ query: "?nonce=123" })));
  assert.ok(!isProof(hello({ query: `${NONCE}&token=x` })));
  assert.ok(!isProof(request("/api/control/state", { query: NONCE })));
});

test("the link's own requests start after the hello with which pairing checked its token", () => {
  const requests = sent(pair(), hello(), hello(), stream(), state());
  assert.deepEqual(linkRequests(requests), requests.slice(2));
  const unpaired = sent(hello(), stream());
  assert.deepEqual(linkRequests(unpaired), unpaired);
  const refused = sent(pair({ status: 403 }), hello(), stream());
  assert.deepEqual(linkRequests(refused), refused);
  assert.deepEqual(linkRequests(sent(pair())), []);
});

test("each stream the link opens is proved first, and other token requests follow a stream", () => {
  const reconnect = [pair(), hello(), hello(), stream(), state(), hello(), hello(), stream()];
  assert.equal(tokenBeforeProof(sent(...reconnect)), null);
  assert.equal(tokenBeforeProof(sent(pair({ status: 403 }), hello(), stream(), state())), null);
  assert.equal(tokenBeforeProof([]), null);
});

test("the hello pairing makes does not prove the server for the link's stream", () => {
  assert.match(
    tokenBeforeProof(sent(pair(), hello(), stream(), hello())),
    /GET \/api\/control\/events \(request 3\) carried the token before a proof of its own/,
  );
});

test("a proof counts once, and only when answered before the stream was asked for", () => {
  assert.match(tokenBeforeProof(sent(hello(), stream(), stream())), /request 3\) carried the token before a proof/);
  assert.match(tokenBeforeProof(sent(hello({ answeredAfter: 2 }), stream())), /request 2\) carried the token/);
  assert.match(tokenBeforeProof(sent(hello({ status: 0 }), stream())), /before a proof of its own/);
  assert.match(tokenBeforeProof(sent(pair(), stream())), /before a proof of its own/);
});

test("a token request before any proven stream, or a hello or pairing with the token, breaks the rule", () => {
  assert.match(tokenBeforeProof(sent(hello(), state())), /\/api\/control\/state \(request 2\) .* any proven stream/);
  assert.match(tokenBeforeProof(sent(hello({ token: true }))), /GET \/api\/hello \(request 1\) carried the token/);
  const pairing = tokenBeforeProof(sent(pair({ token: true })));
  assert.match(pairing, /POST \/api\/control\/pair \(request 1\) carried the token/);
});

test("after a stream ends, a token request needs a newly proven stream", () => {
  const card = request("/api/card", { method: "POST", token: true });
  const ended = sent(hello(), stream(), hello({ status: 403 }), card);
  assert.match(tokenBeforeProof(ended), /POST \/api\/card \(request 4\) carried the token before any proven stream/);
  assert.equal(tokenBeforeProof(sent(hello(), stream(), hello(), stream(), card)), null);
});

test("the inspector's connection states are listed once per change", () => {
  const status = (state) => ({ event: "sendToPropertyInspector", payload: { type: "status", state } });
  const messages = [
    status("unpaired"),
    { event: "setImage", payload: { image: "" } },
    { event: "sendToPropertyInspector", payload: { type: "paired", ok: true } },
    status("connecting"),
    status("connected"),
    status("connected"),
    status("refused"),
  ];
  assert.deepEqual(linkStates(messages), ["unpaired", "connecting", "connected", "refused"]);
});
