// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the link's requests against a fake Sceneuron: the token on each send, replies taken
// only when they are newer control views, and a refused token, from a send or from the state
// read, that ends the connection so the server must prove itself again.

import assert from "node:assert/strict";
import { test } from "node:test";

import { NotConnectedError } from "../../src/lib/link.ts";
import { CONTROL_TOKEN } from "../fake-sceneuron.mjs";
import { settled, startLink, startSceneuron, waitFor } from "../harness.mjs";

test("a send carries the token and its reply updates the view at once", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => subject.state === "connected", "the connection");
  await subject.send("/api/card", { deck: "meeting", card: "agree", action: "toggle" });
  assert.equal(subject.view.decks[1].raised, "agree");
  const post = server.requests.find((request) => request.method === "POST");
  assert.equal(post.headers["x-sceneuron-token"], CONTROL_TOKEN);
  assert.deepEqual(post.body, { deck: "meeting", card: "agree", action: "toggle" });
  await assert.rejects(subject.send("/api/card", { deck: "gone", action: "lower" }), { status: 404 });
  assert.equal(subject.state, "connected", "an unknown deck does not end the connection");
});

test("a token revoked while the stream stays open ends the connection at the next send", async (t) => {
  const server = await startSceneuron(t);
  const subject = startLink(t);
  subject.configure(server.port, CONTROL_TOKEN);
  await settled(server, subject);
  server.rotateToken("rotated-token-0123456789abcdef");
  const seen = server.requests.length;
  await assert.rejects(subject.send("/api/card", { deck: "poker", card: "p1", action: "raise" }), { status: 401 });
  await waitFor(() => server.streamCount === 0, "the stream to close");
  const after = () => server.requests.slice(seen + 1);
  await waitFor(() => after().filter((request) => request.path === "/api/hello").length >= 2, "proofs again");
  assert.equal(subject.state, "refused");
  assert.ok(after().every((request) => request.path === "/api/hello"));
  await assert.rejects(subject.send("/api/automation", { enabled: false }), NotConnectedError);
  assert.ok(subject.view !== null, "the last view stays for darkened keys");
});

test("a refused state read also ends the connection, and a server that proves itself is taken back", async (t) => {
  const server = await startSceneuron(t);
  let refusals = 1;
  const refuseOnce = (url, init) =>
    String(url).endsWith("/api/control/state") && refusals-- > 0
      ? Promise.resolve(Response.json({ error: "a valid token is required" }, { status: 403 }))
      : fetch(url, init);
  const subject = startLink(t, { fetch: refuseOnce });
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => server.requests.length >= 5 && subject.view !== null, "a second proven stream");
  assert.equal(subject.state, "connected");
  assert.deepEqual(
    server.requests.map((request) => request.path),
    ["/api/hello", "/api/control/events", "/api/hello", "/api/control/events", "/api/control/state"],
  );
});

test("a reply older than the view on screen is ignored", async (t) => {
  const server = await startSceneuron(t);
  const stale = async (url, init) => {
    const response = await fetch(url, init);
    if (init?.method !== "POST") {
      return response;
    }
    const view = await response.json();
    return Response.json({ ...view, revision: 0, automation_enabled: !view.automation_enabled });
  };
  const subject = startLink(t, { fetch: stale });
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => subject.view !== null, "the first view");
  const listened = [];
  subject.subscribe(() => listened.push(subject.view.revision));
  await subject.send("/api/automation", { enabled: false });
  await waitFor(() => subject.view.automation_enabled === false, "the streamed change");
  assert.ok(listened.every((revision) => revision > 0));
});

test("a reply that is not a control view leaves the view alone", async (t) => {
  const server = await startSceneuron(t);
  server.change((view) => {
    view.elements[1].held = true;
  });
  let subject;
  const snapshotReply = async (url, init) => {
    const response = await fetch(url, init);
    if (init?.method !== "POST") {
      return response;
    }
    // A snapshot newer than the view the stream delivered, so only its shape can keep it out.
    const { revision, automation_enabled, elements } = await response.json();
    await waitFor(() => subject.view.revision === revision, "the streamed view");
    const shown = elements.map(({ id, name }) => ({ id, name, kind: "text", props: { visible: true } }));
    const snapshot = { revision: revision + 1, project_revision: 1, automation_enabled, elements: shown, decks: [] };
    return Response.json(snapshot);
  };
  subject = startLink(t, { fetch: snapshotReply });
  subject.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => subject.view !== null, "the first view");
  await subject.send("/api/automation", { enabled: false });
  assert.deepEqual(subject.view, server.view);
  assert.equal(subject.view.elements[1].held, true, "a loosely read snapshot would drop the hold");
});
