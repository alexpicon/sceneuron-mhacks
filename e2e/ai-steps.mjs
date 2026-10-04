// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// The AI part of the end-to-end proof: a scripted vision model judges conditions on real camera
// frames from OBS, rules react with explanations, the assistant's plan is reviewed and saved, and
// turning scene understanding off turns the conditions off.

import { elementBox } from "./harness.mjs";
import { startMockVlm } from "./mock-vlm.mjs";

const WAVING_PLAN = {
  summary: "Greet chat when you wave",
  operations: [
    { op: "add_condition", condition: { id: "waving", name: "Waving", prompt: "The person waves at the camera.", detail: "" } },
    {
      op: "add_element",
      element: { id: "wave-alert", name: "Wave alert", kind: "text", props: { text: "Hi chat!", visible: false, x: 1400, y: 200, width: 400, height: 100 } },
    },
    {
      op: "add_rule",
      rule: {
        id: "wave-on",
        name: "Wave hello",
        when: { condition: "waving", state: "on" },
        actions: [{ type: "set_props", element: "wave-alert", props: { visible: true } }],
      },
    },
  ],
};

/** Runs the AI scenario against a running OBS; `t` holds the harness helpers. */
export async function runAiScenario(t) {
  const { api, ws, check, waitFor, overlayAlpha, overlayName, element } = t;
  const mock = await startMockVlm();
  try {
    const project = await api("GET", "/api/project");
    project.ai = { ...project.ai, min_interval_ms: 1000, refresh_ms: 2000 };
    await api("PUT", "/api/project", project);
    mock.state.script = { caption: "A person holds up a mug", verdicts: { "showing-item": { holds: true, detail: "coffee mug" } } };
    await api("PUT", "/api/ai", {
      provider: "openai-compatible",
      base_url: `http://127.0.0.1:${mock.port}/v1`,
      vision_model: "mock-vision",
      planner_model: "mock-planner",
    });
    await new Promise((done) => setTimeout(done, 1500));
    check("AI", "no camera image leaves before consent", mock.state.requests.length === 0);
    await api("PUT", "/api/ai", { consent: true, enabled: true });

    const showing = await waitFor("the vision model names the item", async () => {
      const state = await api("GET", "/api/state");
      const props = element(state, "now-showing").props;
      return props.visible && props.text === "Now showing: coffee mug" ? state : null;
    }, 60_000);
    check("AI", "a natural-language condition turns on from real camera frames", true, `${showing.ai.calls} model calls`);
    check("AI", "the model only gets JPEG camera images", mock.state.requests.every((r) => r.hasImage));
    check("AI", "every AI change is explained in the event log",
      showing.events.some((e) => e.source === "ai" && e.detail.includes("mock: showing-item is visible")));
    const shownAlpha = await waitFor("the lower third on screen", async () => {
      const alpha = await overlayAlpha(ws, overlayName, elementBox(showing, "now-showing"));
      return alpha > 80 ? alpha : null;
    }, 30_000).catch(() => 0);
    check("AI", "the overlay draws what the model saw", shownAlpha > 80, `mean alpha ${shownAlpha.toFixed(1)}`);

    mock.state.script = { ...mock.state.script, plan: WAVING_PLAN };
    const proposed = await api("POST", "/api/ai/plan", { request: "Say hi to chat when I wave", mode: "edit" });
    check("AI", "the assistant proposes a validated plan without saving it",
      proposed.project.conditions.some((c) => c.id === "waving") &&
        !(await api("GET", "/api/project")).conditions.some((c) => c.id === "waving"));
    const response = await fetch(`http://127.0.0.1:${t.editorPort}/api/project`, {
      method: "PUT",
      headers: { "X-Sceneuron-Token": t.token, "Content-Type": "application/json", "If-Match": String(proposed.base_revision) },
      body: JSON.stringify(proposed.project),
    });
    check("AI", "applying the plan saves it", response.ok, `status ${response.status}`);

    mock.state.script = { caption: "A person waves", verdicts: { waving: { holds: true } } };
    const waved = await waitFor("the planned rule fires", async () => {
      const state = await api("GET", "/api/state");
      return element(state, "wave-alert")?.props.visible ? state : null;
    }, 60_000).catch(() => null);
    check("AI", "a rule written by the assistant reacts to the camera", waved);
    const putDown = await waitFor("the item is put down", async () => {
      const state = await api("GET", "/api/state");
      return !element(state, "now-showing").props.visible ? state : null;
    }, 60_000).catch(() => null);
    check("AI", "the lower third leaves when the item is gone", putDown);

    mock.state.script = { ...mock.state.script, quick: 0.95 };
    await api("PUT", "/api/ai", { jev_api_key: "jev-e2e-test-key-0123", jev_base_url: `http://127.0.0.1:${mock.port}` });
    const quick = await waitFor("quick judgments of on-device facts", async () => {
      const asked = mock.state.requests.filter((r) => r.kind === "quick" && r.facts.length > 0);
      const state = await api("GET", "/api/state");
      return asked.length > 0 && state.ai.quick_calls > 0 ? { asked, state } : null;
    }, 30_000).catch(() => null);
    check("AI", "Jev judges conditions from on-device facts, with the key sent only to it",
      quick && quick.asked.every((r) => r.auth === "Bearer jev-e2e-test-key-0123"),
      quick ? `facts: ${quick.asked[0].facts.slice(0, 3).join("; ")}` : "no quick judgment");
    check("AI", "only text facts reach the quick judge, never images",
      mock.state.requests.filter((r) => r.kind === "quick").every((r) => r.hasImage === undefined));
    await api("PUT", "/api/ai", { jev_api_key: "" });

    const looks = () => mock.state.requests.filter((r) => r.kind !== "quick").length;
    const before = looks();
    await ws.vendor("AnalyzeNow", {});
    await waitFor("a look on request", () => looks() > before, 15_000).catch(() => null);
    check("AI", "obs-websocket clients can ask the AI to look now", looks() > before);

    await api("PUT", "/api/ai", { enabled: false });
    const off = await waitFor("AI off", async () => {
      const state = await api("GET", "/api/state");
      return state.ai.state === "off" && !state.conditions.some((c) => c.on) ? state : null;
    }, 30_000).catch(() => null);
    check("AI", "turning scene understanding off turns every condition off", off);
  } finally {
    await mock.close();
  }
}
