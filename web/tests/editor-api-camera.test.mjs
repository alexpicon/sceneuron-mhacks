// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/api.js's Camera & mic calls against a fake fetch: the camera preview as a picture or a reason, the
// analysis, microphone, and setup endpoints, and that nothing goes out before the server proves itself.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";

const require = createRequire(import.meta.url);
const A = require("../editor/api.js");

/** A fetch whose responses carry headers and can be read as a Blob, as the preview needs. */
function fakeFetch(...responses) {
  const calls = [];
  const fetch = (path, init) => {
    calls.push({ path, init });
    const next = responses.shift();
    const headers = new Map(Object.entries(next.headers || {}).map(([name, value]) => [name.toLowerCase(), value]));
    return Promise.resolve({
      ok: next.status >= 200 && next.status < 300,
      status: next.status,
      headers: { get: (name) => headers.get(name.toLowerCase()) ?? null },
      text: () => Promise.resolve(next.body),
      blob: () => Promise.resolve({ blob: next.body }),
    });
  };
  return { calls, api: A.createApi("tok123", fetch) };
}

test("the preview is a picture, marked when simulated, and never cached", async () => {
  const { calls, api } = fakeFetch(
    { status: 200, body: "jpeg", headers: { "Content-Type": "image/jpeg" } },
    { status: 200, body: "drawn", headers: { "Content-Type": "image/jpeg", "X-Sceneuron-Simulated": "true" } },
  );
  assert.deepEqual(await api.cameraPreview(), { image: { blob: "jpeg" }, simulated: false });
  assert.deepEqual(await api.cameraPreview(), { image: { blob: "drawn" }, simulated: true });
  const sent = calls.map(({ path, init }) => [init.method, path, init.cache, init.headers["X-Sceneuron-Token"]]);
  assert.deepEqual(sent, [
    ["GET", "/api/camera/preview", "no-store", "tok123"],
    ["GET", "/api/camera/preview", "no-store", "tok123"],
  ]);
});

test("without a picture the preview gives the reason, and a refusal is an error", async () => {
  const json = { "Content-Type": "application/json" };
  const { api } = fakeFetch(
    { status: 200, body: '{"reason":"no_source"}', headers: json },
    { status: 200, body: "", headers: json },
    { status: 403, body: '{"error":"the overlay token can only read the overlay state"}', headers: json },
  );
  assert.deepEqual(await api.cameraPreview(), { reason: "no_source" });
  assert.deepEqual(await api.cameraPreview(), { reason: "unknown" });
  await assert.rejects(api.cameraPreview(), (error) => error.status === 403 && /overlay token/.test(error.message));
});

test("the analysis, microphone, and setup endpoints use their documented paths and bodies", async () => {
  const ok = () => ({ status: 200, body: "{}" });
  const { calls, api } = fakeFetch(ok(), ok(), ok(), ok());
  await api.cameraAnalysis();
  await api.audioSources();
  await api.attachVoice("mic-1");
  await api.setUp();
  const body = (init) => (init.body === undefined ? null : JSON.parse(init.body));
  assert.deepEqual(calls.map(({ path, init }) => [init.method, path, body(init)]), [
    ["GET", "/api/camera/analysis", null],
    ["GET", "/api/audio-sources", null],
    ["POST", "/api/audio-sources/attach", { uuid: "mic-1" }],
    ["POST", "/api/setup", null],
  ]);
});

test("nothing is sent before the server proves itself", async () => {
  const calls = [];
  const api = A.createApi("tok123", (path) => calls.push(path), async () => false);
  await assert.rejects(api.cameraPreview(), (error) => error.status === 0 && /not answering/.test(error.message));
  assert.equal(calls.length, 0);
});
