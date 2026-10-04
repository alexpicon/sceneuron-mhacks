// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the HTTP client against a fake Sceneuron on 127.0.0.1: the hello proof without the
// token, pairing without Origin headers, the token only in its header, error statuses, refused
// redirects, nothing answering on a port, and how long a request waits for an answer.

import assert from "node:assert/strict";
import http from "node:http";
import { after, before, test } from "node:test";

import { ApiError, claimToken, getJson, postJson, verifyServer } from "../src/lib/api.ts";
import { CONTROL_TOKEN, startFakeSceneuron } from "./fake-sceneuron.mjs";

let server;
before(async () => {
  server = await startFakeSceneuron();
});
after(() => server.close());

/** A port with nothing listening on it. */
async function closedPort() {
  const probe = http.createServer();
  await new Promise((resolve) => probe.listen(0, "127.0.0.1", resolve));
  const { port } = probe.address();
  await new Promise((resolve) => probe.close(resolve));
  return port;
}

test("the server proves it holds the token, and the token is never sent", async () => {
  const before = server.requests.length;
  assert.equal(await verifyServer(server.port, CONTROL_TOKEN, fetch), true);
  assert.equal(await verifyServer(server.port, `${CONTROL_TOKEN}x`, fetch), false);
  for (const request of server.requests.slice(before)) {
    assert.equal(request.path, "/api/hello");
    assert.match(request.query, /^\?nonce=[0-9a-f]{32}$/);
    assert.equal(request.headers["x-sceneuron-token"], undefined);
    assert.equal(request.headers.host, `127.0.0.1:${server.port}`);
  }
});

test("hello on a closed port throws ApiError 0, so offline differs from refused", async () => {
  const port = await closedPort();
  const offline = (error) => error instanceof ApiError && error.status === 0;
  await assert.rejects(verifyServer(port, CONTROL_TOKEN, fetch), offline);
});

test("a pairing code buys the token once, and the request carries no Origin", async () => {
  server.issueCode("424242");
  assert.equal(await claimToken(server.port, "424242", fetch), CONTROL_TOKEN);
  const request = server.requests.at(-1);
  assert.equal(request.path, "/api/control/pair");
  assert.equal(request.headers.origin, undefined);
  assert.equal(request.headers["sec-fetch-site"], undefined);
  await assert.rejects(claimToken(server.port, "424242", fetch), { name: "ApiError", status: 403 });
});

test("authenticated requests carry the token in the header and never in the address", async () => {
  const state = await getJson({ port: server.port, token: CONTROL_TOKEN }, "/api/control/state", fetch);
  assert.equal(state.decks[0].id, "poker");
  const raise = { deck: "poker", card: "p2", action: "raise" };
  const view = await postJson({ port: server.port, token: CONTROL_TOKEN }, "/api/card", raise, fetch);
  assert.equal(view.decks[0].raised, "p2");
  for (const request of server.requests.filter((entry) => entry.path !== "/api/hello")) {
    assert.ok(!request.query.includes(CONTROL_TOKEN));
  }
  assert.equal(server.requests.at(-1).headers["x-sceneuron-token"], CONTROL_TOKEN);
  assert.equal(server.requests.at(-1).headers["content-type"], "application/json");
});

test("error replies become ApiError with the server's message", async () => {
  const endpoint = { port: server.port, token: CONTROL_TOKEN };
  await assert.rejects(postJson(endpoint, "/api/card", { deck: "nope", action: "lower" }, fetch), {
    status: 404,
    message: "no deck nope",
  });
  await assert.rejects(postJson(endpoint, "/api/card", { deck: "poker", action: "raise" }, fetch), { status: 400 });
  await assert.rejects(getJson({ ...endpoint, token: "wrong-token-0123456789" }, "/api/control/state", fetch), {
    status: 401,
  });
});

test("redirects are refused, so the token cannot follow one", async () => {
  const redirector = http.createServer((req, res) => {
    res.writeHead(302, { Location: `http://127.0.0.1:${server.port}/api/control/state`, Connection: "close" });
    res.end();
  });
  await new Promise((resolve) => redirector.listen(0, "127.0.0.1", resolve));
  try {
    const endpoint = { port: redirector.address().port, token: CONTROL_TOKEN };
    await assert.rejects(getJson(endpoint, "/api/control/state", fetch), { status: 0 });
  } finally {
    await new Promise((resolve) => redirector.close(resolve));
  }
});

test("a request waits 5 s for an answer, or as long as its caller allows", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const silent = (url, init) => new Promise((resolve, reject) => init.signal.addEventListener("abort", reject));
  const outcome = (promise) => {
    const seen = { error: null };
    promise.catch((error) => (seen.error = error));
    return seen;
  };
  const settle = () => new Promise((resolve) => setImmediate(resolve));
  const endpoint = { port: server.port, token: CONTROL_TOKEN };
  const usual = outcome(postJson(endpoint, "/api/card", {}, silent));
  const longer = outcome(postJson(endpoint, "/api/room", {}, silent, 12_000));
  t.mock.timers.tick(4_999);
  await settle();
  assert.deepEqual([usual.error, longer.error], [null, null]);
  t.mock.timers.tick(1);
  await settle();
  assert.equal(usual.error?.status, 0, "the usual request gave up");
  assert.equal(longer.error, null);
  t.mock.timers.tick(6_999);
  await settle();
  assert.equal(longer.error, null);
  t.mock.timers.tick(1);
  await settle();
  assert.equal(longer.error?.status, 0, "the longer request gave up");
});
