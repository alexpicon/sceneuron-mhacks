// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/api.js: override payloads and the HTTP client against a fake fetch.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const A = require("../editor/api.js");

describe("overridePayload", () => {
  test("builds the override body", () => {
    assert.deepEqual(A.overridePayload("brb", { visible: true }), { element: "brb", props: { visible: true } });
    assert.deepEqual(A.overridePayload("brb", { text: "", x: undefined }), { element: "brb", props: { text: "" } });
  });

  test("rejects unknown fields and empty patches like the server", () => {
    assert.throws(() => A.overridePayload("brb", { shadow: 1 }), /Unknown property shadow/);
    assert.throws(() => A.overridePayload("brb", {}), /sets no property/);
    assert.throws(() => A.overridePayload("brb", { x: undefined }), /sets no property/);
  });
});

describe("createApi", () => {
  function fakeFetch(...responses) {
    const calls = [];
    const fetch = (path, init) => {
      calls.push({ path, init });
      const next = responses.shift();
      if (next instanceof Error) {
        return Promise.reject(next);
      }
      const ok = next.status >= 200 && next.status < 300;
      return Promise.resolve({ ok, status: next.status, text: () => Promise.resolve(next.body) });
    };
    return { calls, api: A.createApi("tok123", fetch) };
  }

  test("sends the token header and JSON bodies", async () => {
    const { calls, api } = fakeFetch({ status: 200, body: '{"revision":3,"elements":[]}' });
    const result = await api.override("brb", { visible: false });
    assert.deepEqual(result, { revision: 3, elements: [] });
    assert.equal(calls[0].path, "/api/override");
    assert.equal(calls[0].init.method, "POST");
    assert.equal(calls[0].init.headers["X-Sceneuron-Token"], "tok123");
    assert.equal(calls[0].init.headers["Content-Type"], "application/json");
    assert.deepEqual(JSON.parse(calls[0].init.body), { element: "brb", props: { visible: false } });
  });

  test("every endpoint uses the documented path, method, and body", async () => {
    const ok = () => ({ status: 200, body: "{}" });
    const { calls, api } = fakeFetch(...Array.from({ length: 11 }, ok));
    await api.getProject();
    await api.saveProject({ version: 1 });
    await api.release("brb");
    await api.release(null);
    await api.release();
    await api.effect("brb", "shake");
    await api.automation(0);
    await api.simulate("away");
    await api.sources();
    await api.attachSource("uuid-1");
    await api.addOverlay();
    const seen = calls.map((
      { path, init },
    ) => [init.method, path, init.body === undefined ? null : JSON.parse(init.body)]);
    assert.deepEqual(seen, [
      ["GET", "/api/project", null],
      ["PUT", "/api/project", { version: 1 }],
      ["POST", "/api/release", { element: "brb" }],
      ["POST", "/api/release", { element: null }],
      ["POST", "/api/release", { element: null }],
      ["POST", "/api/effect", { element: "brb", effect: "shake" }],
      ["POST", "/api/automation", { enabled: false }],
      ["POST", "/api/simulate", { presence: "away" }],
      ["GET", "/api/sources", null],
      ["POST", "/api/sources/attach", { uuid: "uuid-1" }],
      ["POST", "/api/overlay", null],
    ]);
    assert.equal(calls[0].init.headers["Content-Type"], undefined);
  });

  test("saving with a revision sends If-Match, and a 409 keeps the server's revision", async () => {
    const conflict = '{"error":"the project changed","project_revision":7}';
    const { calls, api } = fakeFetch({ status: 200, body: "{}" }, { status: 409, body: conflict });
    await api.saveProject({ version: 2 }, 6);
    assert.equal(calls[0].init.headers["If-Match"], "6");
    await assert.rejects(api.saveProject({ version: 2 }, 6), (error) => {
      assert.ok(A.isConflict(error));
      assert.equal(error.data.project_revision, 7);
      assert.equal(error.message, "the project changed");
      return true;
    });
    assert.equal(A.isConflict(new A.ApiError("bad", 400)), false);
    assert.equal(A.isConflict(null), false);
  });

  test("a save without a known revision sends no If-Match", async () => {
    const { calls, api } = fakeFetch({ status: 200, body: "{}" }, { status: 200, body: "{}" });
    await api.saveProject({ version: 2 }, null);
    await api.saveProject({ version: 2 });
    assert.deepEqual(calls.map((call) => call.init.headers["If-Match"]), [undefined, undefined]);
  });

  test("the AI endpoints use their documented paths and bodies", async () => {
    const ok = () => ({ status: 200, body: "{}" });
    const { calls, api } = fakeFetch(...Array.from({ length: 6 }, ok));
    await api.getState();
    await api.aiSettings();
    await api.updateAiSettings({ consent: true });
    await api.plan("Say hi when I wave", "edit", 1);
    await api.plan("I paint miniatures", "setup", false);
    await api.analyze();
    const body = (init) => (init.body === undefined ? null : JSON.parse(init.body));
    const seen = calls.map(({ path, init }) => [init.method, path, body(init)]);
    assert.deepEqual(seen, [
      ["GET", "/api/state", null],
      ["GET", "/api/ai", null],
      ["PUT", "/api/ai", { consent: true }],
      ["POST", "/api/ai/plan", { request: "Say hi when I wave", mode: "edit", use_camera: true }],
      ["POST", "/api/ai/plan", { request: "I paint miniatures", mode: "setup", use_camera: false }],
      ["POST", "/api/ai/analyze", null],
    ]);
  });

  test("the voice endpoints use their documented paths and bodies", async () => {
    const ok = () => ({ status: 200, body: "{}" });
    const { calls, api } = fakeFetch(...Array.from({ length: 3 }, ok));
    await api.talk(1);
    await api.voiceGuide();
    await api.voiceCommand("hacker theme");
    const body = (init) => (init.body === undefined ? null : JSON.parse(init.body));
    assert.deepEqual(calls.map(({ path, init }) => [init.method, path, body(init)]), [
      ["POST", "/api/voice/talk", { down: true }],
      ["GET", "/api/voice/guide", null],
      ["POST", "/api/voice/command", { text: "hacker theme" }],
    ]);
  });

  test("the card and Stream Deck endpoints use their documented paths and bodies", async () => {
    const ok = () => ({ status: 200, body: "{}" });
    const { calls, api } = fakeFetch(...Array.from({ length: 7 }, ok));
    await api.packs();
    await api.card("poker", "p5", "toggle");
    await api.card("poker", null, "lower");
    await api.card("poker", undefined, "next");
    await api.pairing();
    await api.pairings();
    await api.revokeControl();
    const body = (init) => (init.body === undefined ? null : JSON.parse(init.body));
    assert.deepEqual(calls.map(({ path, init }) => [init.method, path, body(init)]), [
      ["GET", "/api/packs", null],
      ["POST", "/api/card", { deck: "poker", card: "p5", action: "toggle" }],
      ["POST", "/api/card", { deck: "poker", action: "lower" }],
      ["POST", "/api/card", { deck: "poker", action: "next" }],
      ["POST", "/api/control/pairing", null],
      ["GET", "/api/control/pairing", null],
      ["POST", "/api/control/revoke", null],
    ]);
  });

  test("the room endpoints use their documented paths and bodies", async () => {
    const ok = () => ({ status: 200, body: "{}" });
    const { calls, api } = fakeFetch(...Array.from({ length: 3 }, ok));
    await api.room();
    await api.roomAction("join", { code: "K7M2QX", name: "Ana", deck: "poker" });
    await api.roomAction("reveal");
    const body = (init) => (init.body === undefined ? null : JSON.parse(init.body));
    assert.deepEqual(calls.map(({ path, init }) => [init.method, path, body(init)]), [
      ["GET", "/api/room", null],
      ["POST", "/api/room", { action: "join", code: "K7M2QX", name: "Ana", deck: "poker" }],
      ["POST", "/api/room", { action: "reveal" }],
    ]);
  });

  test("server errors surface the error body", async () => {
    const { api } = fakeFetch({ status: 404, body: '{"error":"unknown element \\"x\\""}' });
    await assert.rejects(api.effect("x", "pop"), (error) => {
      assert.ok(error instanceof A.ApiError);
      assert.equal(error.message, 'unknown element "x"');
      assert.equal(error.status, 404);
      return true;
    });
  });

  test("errors without a JSON body still explain themselves", async () => {
    const { api } = fakeFetch({ status: 500, body: "oops" }, { status: 413, body: "" });
    await assert.rejects(api.sources(), /Request failed with HTTP 500/);
    await assert.rejects(api.saveProject({}), /Request failed with HTTP 413/);
  });

  test("network failures become a reachability error", async () => {
    const { api } = fakeFetch(new TypeError("Failed to fetch"));
    await assert.rejects(
      api.getProject(),
      (error) => error.status === 0 && /Cannot reach Sceneuron/.test(error.message),
    );
  });

  test("invalid overrides are rejected before any request", async () => {
    const { calls, api } = fakeFetch();
    await assert.rejects(api.override("brb", { glow: true }), /Unknown property glow/);
    assert.equal(calls.length, 0);
  });

  test("an empty success body yields null", async () => {
    const { api } = fakeFetch({ status: 200, body: "" });
    assert.equal(await api.getProject(), null);
  });
});
