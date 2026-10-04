// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the card steps: adding a pack to a project, once, refusing a pack whose ids are taken,
// reading raised cards from exact vendor replies, and recording failures of the scenario and of
// the restart check as checks instead of throwing them.

import assert from "node:assert/strict";
import { test } from "node:test";

import { checkCardsRestored, exactState, raisedCard, runCardScenario, withPack, withPackOnce } from "../card-steps.mjs";

const pack = {
  id: "poker",
  name: "Planning poker",
  elements: [{ id: "poker-card", kind: "text" }],
  decks: [{ id: "poker", element: "poker-card", cards: [{ id: "p5", name: "5" }] }],
};
const project = { version: 3, elements: [{ id: "brb", kind: "text" }], decks: [], rules: [{ id: "away" }] };

test("a pack's elements and decks are added after the project's own", () => {
  const added = withPack(project, pack);
  assert.deepEqual(added.elements.map((e) => e.id), ["brb", "poker-card"]);
  assert.deepEqual(added.decks.map((d) => d.id), ["poker"]);
  assert.deepEqual(added.rules, project.rules);
  assert.deepEqual(project.elements.map((e) => e.id), ["brb"], "the original project is left alone");
});

test("a project saved without decks gets a deck list", () => {
  const { decks: _, ...older } = project;
  assert.deepEqual(withPack(older, pack).decks.map((d) => d.id), ["poker"]);
});

test("a pack whose element or deck id is taken is refused", () => {
  assert.throws(() => withPack(withPack(project, pack), pack), /Planning poker needs the id poker-card/);
  const deckOnly = { ...project, decks: [{ id: "poker", element: "brb", cards: [] }] };
  assert.throws(() => withPack(deckOnly, pack), /Planning poker needs the id poker,/);
});

test("a pack is added once: a project that has all of it, as the starter does, is kept as it is", () => {
  const added = withPackOnce(project, pack);
  assert.deepEqual(added.decks.map((d) => d.id), ["poker"]);
  assert.equal(withPackOnce(added, pack), added);
  const deckOnly = { ...project, decks: [{ id: "poker", element: "brb", cards: [] }] };
  assert.throws(() => withPackOnce(deckOnly, pack), /Planning poker needs the id poker,/);
  const subtitles = { id: "subtitles", name: "Subtitles", elements: [{ id: "brb", kind: "text" }], decks: [] };
  assert.equal(withPackOnce(project, subtitles), project, "a pack of elements only");
});

test("vendor replies give their exact state, and error replies throw", () => {
  const state = { revision: 4, decks: [] };
  assert.deepEqual(exactState({ state_json: JSON.stringify(state), state: { revision: 4 } }), state);
  assert.throws(() => exactState({ error: "unknown deck \"votes\"" }), /unknown deck "votes"/);
});

test("raised cards read as the card id, or null while the deck is lowered", () => {
  const state = { decks: [{ id: "poker", raised: "p5", raised_by: "manual" }, { id: "meeting", raised: null }] };
  assert.equal(raisedCard(state, "poker"), "p5");
  assert.equal(raisedCard(state, "meeting"), null);
  assert.equal(raisedCard({ decks: [{ id: "poker" }] }, "poker"), null);
  assert.throws(() => raisedCard(state, "status"), /no deck status/);
  assert.throws(() => raisedCard({}, "poker"), /no deck poker/);
});

/** Harness helpers around fakes: `api` and `ws` answer as given and record every call. */
function fakeHarness(api, hotkeys = []) {
  const calls = [];
  const results = [];
  const state = { decks: [{ id: "poker", raised: null }], elements: [{ id: "poker-card", props: { visible: false } }] };
  const t = {
    api: async (method, path, body) => {
      calls.push(`${method} ${path}`);
      return api(method, path, body);
    },
    ws: {
      vendor: async (name) => {
        calls.push(`vendor ${name}`);
        return { state_json: JSON.stringify(state) };
      },
      request: async (name) => {
        calls.push(`request ${name}`);
        return name === "GetHotkeyList" ? { hotkeys } : {};
      },
    },
    check: (requirement, name, ok, detail = "") => results.push({ name, ok: Boolean(ok), detail }),
    waitFor: async (what, probe) => {
      const found = await probe();
      if (found) return found;
      throw new Error(`timed out waiting for ${what}`);
    },
    element: (current, id) => current.elements.find((e) => e.id === id),
  };
  return { t, calls, results };
}

test("a pack the editor API does not offer is a failed check, and nothing else runs", async () => {
  const missing = async () => {
    throw new Error("GET /api/packs: 404 not found");
  };
  for (const api of [missing, async () => [{ id: "meeting" }], async () => ({ error: "odd" })]) {
    const { t, calls, results } = fakeHarness(api);
    await runCardScenario(t);
    assert.deepEqual(calls, ["GET /api/packs"]);
    assert.deepEqual(results.map((r) => [r.name, r.ok]), [["the editor API offers the poker pack", false]]);
  }
  const { t, results } = fakeHarness(missing);
  await runCardScenario(t);
  assert.match(results[0].detail, /404/);
});

test("a step that throws is recorded as a failed check instead of ending the proof", async () => {
  const api = async (method, path) => {
    if (path === "/api/packs") return [pack];
    if (method === "GET") return project;
    throw new Error("PUT /api/project: 400 the deck needs a text element");
  };
  const { t, results } = fakeHarness(api);
  await runCardScenario(t);
  assert.deepEqual(results.map((r) => [r.name, r.ok]), [
    ["the editor API offers the poker pack", true],
    ["the card scenario ran to the end", false],
  ]);
  assert.match(results[1].detail, /400/);
});

test("hotkeys that never register are a failed check, and no hotkey is pressed", async () => {
  const { t, calls, results } = fakeHarness(async (method, path) => (path === "/api/packs" ? [pack] : project));
  await runCardScenario(t);
  assert.equal(results.find((r) => r.name.includes("registers its card"))?.ok, false);
  assert.ok(!calls.includes("request TriggerHotkeyByName"), calls.join(", "));
  assert.ok(!results.some((r) => r.name === "the card scenario ran to the end"), "nothing threw");
});

test("after a restart, the deck and its registered hotkeys pass as one check", async () => {
  const restored = { ...project, decks: pack.decks };
  const { t, results } = fakeHarness(async () => restored, ["sceneuron.card.poker.p8", "sceneuron.deck.poker.lower"]);
  await checkCardsRestored(t);
  assert.deepEqual(results.map((r) => [r.name, r.ok]), [["the deck and its hotkeys come back after a restart", true]]);
});

test("after a restart, a missing lower hotkey fails the check even with the deck back", async () => {
  const restored = { ...project, decks: pack.decks };
  const { t, results } = fakeHarness(async () => restored, ["sceneuron.card.poker.p8"]);
  await checkCardsRestored(t);
  assert.deepEqual(results.map((r) => r.ok), [false]);
});

test("after a restart, a missing deck fails the check even with both hotkeys registered", async () => {
  const { t, results } = fakeHarness(async () => project, ["sceneuron.card.poker.p8", "sceneuron.deck.poker.lower"]);
  await checkCardsRestored(t);
  assert.deepEqual(results.map((r) => r.ok), [false]);
});

test("a restart check whose project request fails is a failed check instead of a throw", async () => {
  const { t, calls, results } = fakeHarness(async () => {
    throw new Error("GET /api/project: 401 unauthorized");
  });
  await checkCardsRestored(t);
  assert.deepEqual(calls, ["GET /api/project"]);
  assert.deepEqual(results.map((r) => [r.name, r.ok]), [["the deck and its hotkeys come back after a restart", false]]);
  assert.match(results[0].detail, /401/);
});
