// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the scripted mock vision model: judge answers follow the condition ids in the
// request's schema and the script, planner requests get the scripted plan, Jev questions get the
// scripted probability, and the HTTP server records requests and accepts script changes.

import { test } from "node:test";
import assert from "node:assert/strict";

import { answer, completion, quickAnswer, startMockVlm } from "../mock-vlm.mjs";

const judgeRequest = (ids) => ({
  model: "mock",
  response_format: {
    type: "json_schema",
    json_schema: {
      name: "scene_observation",
      schema: { properties: { verdicts: { items: { properties: { condition: { enum: ids } } } } } },
    },
  },
  messages: [{ role: "user", content: [{ type: "text", text: "judge" }, { type: "image_url", image_url: { url: "data:," } }] }],
});

test("judge answers cover every condition and follow the script", () => {
  const script = { caption: "Holding a mug", verdicts: { "showing-item": { holds: true, detail: "coffee mug" } } };
  const result = answer(judgeRequest(["showing-item", "pet-in-view"]), script);
  assert.equal(result.caption, "Holding a mug");
  assert.deepEqual(
    result.verdicts.map((v) => [v.condition, v.holds, v.detail]),
    [["showing-item", true, "coffee mug"], ["pet-in-view", false, ""]],
  );
});

test("planner requests get the scripted plan", () => {
  const plan = { summary: "Add waving", operations: [] };
  const request = { response_format: { json_schema: { name: "edit_plan", schema: {} } } };
  assert.deepEqual(answer(request, { plan }), plan);
  assert.equal(answer(request, {}).summary, "No changes");
});

test("completions carry the answer as message content", () => {
  const body = completion({ a: 1 }, "m");
  assert.equal(JSON.parse(body.choices[0].message.content).a, 1);
  assert.equal(body.choices[0].finish_reason, "stop");
});

test("the server answers, records requests, and takes script changes", async () => {
  const mock = await startMockVlm();
  try {
    const base = `http://127.0.0.1:${mock.port}`;
    await fetch(`${base}/control`, { method: "POST", body: JSON.stringify({ caption: "scripted" }) });
    const reply = await fetch(`${base}/v1/chat/completions`, { method: "POST", body: JSON.stringify(judgeRequest(["x"])) });
    const content = JSON.parse((await reply.json()).choices[0].message.content);
    assert.equal(content.caption, "scripted");
    const control = await (await fetch(`${base}/control`)).json();
    assert.equal(control.requests.length, 1);
    assert.equal(control.requests[0].hasImage, true);
  } finally {
    await mock.close();
  }
});

test("Jev questions get the scripted probability", () => {
  const request = { questions: { q0: { type: "noul" }, q1: { type: "noul" } } };
  assert.deepEqual(quickAnswer(request, { quick: 0.2 }).answers, { q0: { type: "noul", noul: 0.2 }, q1: { type: "noul", noul: 0.2 } });
  assert.equal(quickAnswer(request, {}).answers.q0.noul, 0.9);
});
