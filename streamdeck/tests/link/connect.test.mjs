// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how the link connects to a fake Sceneuron: the hello proof before any token is sent,
// the single event stream and live views, reconnecting with a fresh proof after drops and
// restarts, proving the server again as soon as a stream ends (offline, not refused, when nothing
// answers that proof), refusing a revoked token or a stranger on the port, idle streams, starting
// over for a new token or port without going offline or dropping the view, and stopping.

import assert from "node:assert/strict";
import { test } from "node:test";

import { NotConnectedError } from "../../src/lib/link.ts";
import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { settled, startLink, startSceneuron, waitFor } from "../harness.mjs";

/** True when no request carries the token unless a passing hello came before it. */
function tokenOnlyAfterHello(requests) {
  let proven = false;
  for (const request of requests) {
    if (request.path === "/api/hello") {
      proven = true;
    } else if (request.headers["x-sceneuron-token"] !== undefined && !proven) {
      return false;
    }
  }
  return true;
}

const paths = (server, path) => server.requests.filter((request) => request.path === path);

/** The states `link` goes through from now on, starting with the current one, each change once. */
function trail(link) {
  const states = [link.state];
  link.subscribe(() => states.at(-1) !== link.state && states.push(link.state));
  return states;
}

test("without a token the link stays unpaired and sends nothing", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, null);
  assert.equal(subject.state, "unpaired");
  await new Promise((resolve) => setTimeout(resolve, 30));
  assert.equal(server.requests.length, 0);
});

test("it proves the server, opens one stream, and follows the view", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => subject.state === "connected" && subject.view !== null, "the first view");
  assert.equal(server.requests[0].path, "/api/hello");
  assert.equal(server.requests[1].path, "/api/control/events");
  assert.equal(server.streamCount, 1);
  server.change((view) => {
    view.decks[0].raised = "p3";
  });
  await waitFor(() => subject.view.decks[0].raised === "p3", "the change");
  assert.equal(subject.view.revision, server.view.revision);
  assert.ok(tokenOnlyAfterHello(server.requests));
  assert.ok(server.requests.every((request) => !request.query.includes(CONTROL_TOKEN)));
});

test("nothing is sent while not connected", async (t) => {
  const server = await startSceneuron(t);
  server.stopProving();
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => subject.state === "refused", "the refusal");
  await assert.rejects(subject.send("/api/automation", { enabled: false }), NotConnectedError);
  assert.ok(server.requests.every((request) => request.headers["x-sceneuron-token"] === undefined));
});

test("after a dropped stream it proves the server again with a fresh nonce", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => subject.state === "connected", "the connection");
  server.dropStreams();
  await waitFor(() => paths(server, "/api/control/events").length === 2, "a new stream");
  const hellos = paths(server, "/api/hello");
  assert.equal(hellos.length, 2);
  assert.notEqual(hellos[0].query, hellos[1].query);
  assert.ok(tokenOnlyAfterHello(server.requests));
  await waitFor(() => subject.state === "connected", "the reconnection");
});

test("a revoked token stops at the hello and is not sent again", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await settled(server, subject);
  server.rotateToken("rotated-token-0123456789abcdef");
  server.dropStreams();
  await waitFor(() => subject.state === "refused", "the refusal");
  const seen = server.requests.length;
  await waitFor(() => server.requests.length > seen + 1, "more retries");
  const sinceRevoked = server.requests.slice(seen - 1);
  assert.ok(sinceRevoked.every((request) => request.path === "/api/hello"));
  assert.ok(subject.view !== null, "the last view stays for darkened keys");
});

test("a revoke that ends the stream, as Sceneuron does, is refused at once without going offline", async (t) => {
  const server = await startSceneuron(t);
  // No retry comes within the test, so only the proof right after the stream ends can refuse.
  const subject = startLink(t, { backoff: { baseMs: 60_000, maxMs: 60_000 } });
  subject.configure(server.port, CONTROL_TOKEN);
  await settled(server, subject);
  const states = trail(subject);
  server.rotateToken("rotated-token-0123456789abcdef");
  server.endStreams();
  await waitFor(() => subject.state === "refused", "the refusal");
  assert.deepEqual(states, ["connected", "connecting", "refused"]);
  assert.equal(paths(server, "/api/hello").length, 2);
  assert.ok(tokenOnlyAfterHello(server.requests));
});

test("a stream that ends while the token holds is proved again, then reopened after a wait", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await settled(server, subject);
  const states = trail(subject);
  server.endStreams();
  await waitFor(() => paths(server, "/api/control/events").length === 2, "a new stream");
  await waitFor(() => subject.state === "connected", "the reconnection");
  assert.deepEqual(states, ["connected", "connecting", "offline", "connected"]);
  assert.equal(paths(server, "/api/hello").length, 3);
});

test("a stream that ends cleanly, then a hello nothing answers, goes offline and never refused", async (t) => {
  const server = await startSceneuron(t);
  let gone = false;
  let unanswered = 0;
  // Like a server that ends its streams cleanly, then stops listening.
  const fetchUntilGone = (input, init) => {
    if (gone && new URL(String(input)).pathname === "/api/hello") {
      unanswered += 1;
      return Promise.reject(new TypeError("fetch failed"));
    }
    return fetch(input, init);
  };
  // No retry comes within the test, so only the proof right after the stream ends is seen.
  const subject = startLink(t, { fetch: fetchUntilGone, backoff: { baseMs: 60_000, maxMs: 60_000 } });
  subject.configure(server.port, CONTROL_TOKEN);
  await settled(server, subject);
  const states = trail(subject);
  gone = true;
  server.endStreams();
  await waitFor(() => unanswered === 1 && subject.state !== "connecting", "the proof after the end");
  assert.deepEqual(states, ["connected", "connecting", "offline"]);
});

test("a restarted server on the same port is found again", async (t) => {
  const first = await startSceneuron(t);
  const port = first.port;
  const subject = startLink(t);
  subject.configure(port, CONTROL_TOKEN);
  await waitFor(() => subject.state === "connected", "the connection");
  await first.close();
  await waitFor(() => subject.state === "offline", "going offline");
  const second = await startSceneuron(t, { port });
  second.change((view) => {
    view.automation_enabled = false;
  });
  await waitFor(() => subject.state === "connected" && subject.view.automation_enabled === false, "the new server");
});

test("a silent stream counts as dead and is replaced", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t, { idleMs: 60 });
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => paths(server, "/api/hello").length >= 2, "a second hello");
});

test("configuring the same settings again keeps the connection", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => subject.state === "connected", "the connection");
  const seen = paths(server, "/api/hello").length;
  subject.configure(server.port, CONTROL_TOKEN);
  await new Promise((resolve) => setTimeout(resolve, 50));
  assert.equal(paths(server, "/api/hello").length, seen);
  subject.configure(server.port, null);
  assert.equal(subject.state, "unpaired");
  assert.equal(subject.view, null);
});

test("a new token goes from connected straight to connecting and keeps the view until the next", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await settled(server, subject);
  const states = trail(subject);
  const view = subject.view;
  server.rotateToken("rotated-token-0123456789abcdef");
  subject.configure(server.port, "rotated-token-0123456789abcdef");
  assert.equal(subject.view, view, "the view stays while the new link connects");
  await waitFor(() => subject.state === "connected" && subject.view !== view, "the new stream's view");
  assert.deepEqual(states, ["connected", "connecting", "connected"]);
});

test("a new port is announced even while the link stays connecting", async (t) => {
  const first = await startSceneuron(t, { hung: true });
  const second = await startSceneuron(t, { hung: true });
  const subject = startLink(t);
  subject.configure(first.port, CONTROL_TOKEN);
  const heard = [];
  subject.subscribe(() => heard.push([subject.state, subject.port]));
  subject.configure(second.port, CONTROL_TOKEN);
  assert.deepEqual(heard, [["connecting", second.port]]);
});

test("stopping ends sending at once, and a new port is announced even while unpaired", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => subject.state === "connected", "the connection");
  subject.stop();
  assert.equal(subject.state, "offline");
  await assert.rejects(subject.send("/api/automation", { enabled: false }), NotConnectedError);
  let heard = 0;
  subject.subscribe(() => (heard += 1));
  subject.configure(5123, null);
  assert.equal(heard, 1);
  assert.equal(subject.port, 5123);
  assert.equal(subject.state, "unpaired");
});
