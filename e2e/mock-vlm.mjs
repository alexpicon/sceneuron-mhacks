// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// A scripted stand-in for a vision-language model with an OpenAI-compatible chat API, so the
// end-to-end proof can drive Sceneuron's AI path without a paid provider.
//
// POST /v1/chat/completions answers judge requests with the scripted scene and planner requests
// (response format names containing "plan") with the scripted plan. POST /v1/systemone answers like
// TypeSafe's Jev: every yes/no question gets the scripted probability ("quick", default 0.9).
// POST /control sets the script: {"verdicts": {"condition-id": {"holds": true, "detail": "coffee
// mug"}}, "caption": "...", "plan": {...}, "quick": 0.9}. GET /control returns the script and the
// requests seen so far.

import { createServer } from "node:http";

/** Builds the JSON answer for one chat request from the current script. */
export function answer(request, script) {
  const format = request.response_format?.json_schema ?? {};
  if (String(format.name ?? "").includes("plan")) {
    return script.plan ?? { summary: "No changes", operations: [] };
  }
  const conditions = format.schema?.properties?.verdicts?.items?.properties?.condition?.enum ?? [];
  const verdicts = conditions.map((condition) => {
    const scripted = script.verdicts?.[condition] ?? {};
    const holds = Boolean(scripted.holds);
    return {
      condition,
      reason: scripted.reason ?? (holds ? `mock: ${condition} is visible` : `mock: no ${condition}`),
      holds,
      confidence: scripted.confidence ?? 0.95,
      detail: holds ? (scripted.detail ?? "") : "",
    };
  });
  return { caption: script.caption ?? "A person at a desk", verdicts };
}

/** Jev's answer: the scripted probability for every question. */
export function quickAnswer(request, script) {
  const probability = typeof script.quick === "number" ? script.quick : 0.9;
  const answers = Object.fromEntries(Object.keys(request.questions ?? {}).map((id) => [id, { type: "noul", noul: probability }]));
  return { model: "jev-mock", answers, usage: { input_tokens: 150, output_tokens: 0 } };
}

/** Wraps an answer in a chat completion response. */
export function completion(json, model) {
  return {
    id: `mock-${Date.now()}`,
    object: "chat.completion",
    model,
    choices: [{ index: 0, message: { role: "assistant", content: JSON.stringify(json) }, finish_reason: "stop" }],
    usage: { prompt_tokens: 1200, completion_tokens: 80, total_tokens: 1280 },
  };
}

async function readJson(request) {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  const text = Buffer.concat(chunks).toString("utf8");
  return text ? JSON.parse(text) : {};
}

/** Starts the mock on 127.0.0.1; resolves with { port, script, requests, close }. */
export function startMockVlm({ port = 0 } = {}) {
  const state = { script: {}, requests: [] };
  const server = createServer(async (request, response) => {
    const reply = (status, body) => {
      response.writeHead(status, { "content-type": "application/json" });
      response.end(JSON.stringify(body));
    };
    try {
      if (request.method === "POST" && request.url === "/v1/chat/completions") {
        const body = await readJson(request);
        const hasImage = (body.messages ?? []).some(
          (message) => Array.isArray(message.content) && message.content.some((part) => part.type === "image_url"),
        );
        state.requests.push({ at: Date.now(), model: body.model, format: body.response_format?.json_schema?.name, hasImage });
        return reply(200, completion(answer(body, state.script), body.model ?? "mock"));
      }
      if (request.method === "POST" && request.url === "/v1/systemone") {
        const body = await readJson(request);
        state.requests.push({ at: Date.now(), kind: "quick", facts: body.state?.camera_facts ?? [], auth: request.headers.authorization });
        return reply(200, quickAnswer(body, state.script));
      }
      if (request.url === "/control" && request.method === "POST") {
        state.script = { ...state.script, ...(await readJson(request)) };
        return reply(200, state.script);
      }
      if (request.url === "/control" && request.method === "GET") {
        return reply(200, { script: state.script, requests: state.requests });
      }
      return reply(404, { error: "not found" });
    } catch (error) {
      return reply(400, { error: String(error) });
    }
  });
  return new Promise((resolve) => {
    server.listen(port, "127.0.0.1", () =>
      resolve({
        port: server.address().port,
        state,
        close: () => new Promise((done) => server.close(done)),
      }),
    );
  });
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const port = Number(process.argv[2] ?? 4499);
  const mock = await startMockVlm({ port });
  console.log(`mock vision model on http://127.0.0.1:${mock.port}/v1`);
}
