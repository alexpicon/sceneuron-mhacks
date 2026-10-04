// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the end-to-end harness: recorded checks, polling until a probe answers or times out,
// the editor API client's token, body, and error replies, and where an element is on the overlay.

import assert from "node:assert/strict";
import { createServer } from "node:http";
import { test } from "node:test";

import { createChecks, editorApi, element, elementBox, waitFor } from "../harness.mjs";

test("checks are recorded and printed as they happen", () => {
  const lines = [];
  const { results, check } = createChecks((line) => lines.push(line));
  check("R1", "the module loaded", "yes");
  check("Cards", "a card rose", undefined, "still lowered");
  assert.deepEqual(results, [
    { requirement: "R1", name: "the module loaded", ok: true, detail: "" },
    { requirement: "Cards", name: "a card rose", ok: false, detail: "still lowered" },
  ]);
  assert.deepEqual(lines, ["PASS [R1] the module loaded", "FAIL [Cards] a card rose - still lowered"]);
});

test("waiting returns the first answer and retries probes that throw", async () => {
  let calls = 0;
  const answer = await waitFor("an answer", async () => {
    calls += 1;
    if (calls === 1) throw new Error("not yet");
    return calls === 3 ? { ready: true } : null;
  }, 1_000, 5);
  assert.deepEqual(answer, { ready: true });
  assert.equal(calls, 3);
});

test("waiting times out naming what it waited for and the last answer", async () => {
  await assert.rejects(waitFor("the overlay", async () => 0, 30, 5), /timed out waiting for the overlay \(last: 0\)/);
  const failing = async () => {
    throw new Error("connection refused");
  };
  await assert.rejects(waitFor("OBS", failing, 30, 5), /last: connection refused/);
});

test("the editor API sends the token and JSON, and error replies throw", async (t) => {
  const seen = [];
  const server = createServer((request, response) => {
    let body = "";
    request.on("data", (chunk) => (body += chunk));
    request.on("end", () => {
      seen.push({ method: request.method, url: request.url, token: request.headers["x-sceneuron-token"], body });
      const missing = request.url === "/api/missing";
      response.writeHead(missing ? 404 : 200, { "Content-Type": "application/json" });
      response.end(JSON.stringify(missing ? { error: "no such route" } : { ok: true }));
    });
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  t.after(() => server.close());
  const api = editorApi(server.address().port, "test-token-0123456789");

  assert.deepEqual(await api("PUT", "/api/project", { decks: [] }), { ok: true });
  assert.deepEqual(await api("GET", "/api/state"), { ok: true });
  await assert.rejects(api("GET", "/api/missing"), /GET \/api\/missing: 404 no such route/);
  assert.deepEqual(seen[0], { method: "PUT", url: "/api/project", token: "test-token-0123456789", body: '{"decks":[]}' });
  assert.equal(seen[1].body, "", "a request without a body sends none");
});

test("elements are found by id", () => {
  const state = { elements: [{ id: "brb" }, { id: "name-label" }] };
  assert.equal(element(state, "name-label"), state.elements[1]);
  assert.equal(element(state, "gone"), undefined);
});

test("an element's box is its middle, in fractions of the canvas", () => {
  const props = { x: 60, y: 900, width: 640, height: 110 };
  const state = { canvas: { width: 1920, height: 1080 }, elements: [{ id: "name-label", props }] };
  const box = elementBox(state, "name-label");
  const rounded = Object.fromEntries(Object.entries(box).map(([key, value]) => [key, Math.round(value * 1000) / 1000]));
  assert.deepEqual(rounded, { x0: 0.081, y0: 0.849, x1: 0.315, y1: 0.92 });
  assert.deepEqual(elementBox(state, "name-label", 0), { x0: 60 / 1920, y0: 900 / 1080, x1: 700 / 1920, y1: 1010 / 1080 });
});
