// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: what the vision model is doing: the AI chip in the status bar, and the AI tab's
   status card with usage, spend against the hourly budget, the latest scene, and "Look now". */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { STATES, aiChipLabel, aiState, formatAge, formatCost, formatCount, quickLine, sceneText } = Editor.aiText;

  /** States that alternate on every look; changes between them are not announced. */
  const ROUTINE = new Set(["watching", "thinking"]);

  /** Wires the AI status; returns the view with render. */
  Editor.aiStatus = function setupAiStatus(editor) {
    const { app, byId, api } = editor;
    const chip = byId("ai-chip");
    const parts = {
      state: byId("ai-state"),
      message: byId("ai-message"),
      calls: byId("ai-calls"),
      tokens: byId("ai-tokens"),
      cost: byId("ai-cost"),
      latency: byId("ai-latency"),
      quick: byId("ai-quick"),
      hour: byId("ai-hour"),
      meter: byId("ai-budget-meter"),
      caption: byId("ai-caption"),
      captionAge: byId("ai-caption-age"),
      lookNow: byId("ai-look-now"),
    };
    let announced = null;

    function set(node, text) {
      if (node.textContent !== text) {
        node.textContent = text;
      }
    }

    function announceChange(state, ai) {
      if (announced !== null && state !== announced && !(ROUTINE.has(state) && ROUTINE.has(announced))) {
        const message = ai && ai.message ? ` ${ai.message}` : "";
        editor.announce(`AI is now ${STATES[state].toLowerCase()}.${message}`);
      }
      announced = state;
    }

    function render() {
      const available = Boolean(app.ai && app.ai.available);
      const ai = app.live ? app.live.ai : null;
      const state = aiState(ai);
      chip.hidden = !available;
      chip.dataset.state = state;
      set(chip, aiChipLabel(ai));
      chip.title = ai && ai.message ? String(ai.message) : "";
      if (!available) {
        return;
      }
      announceChange(state, ai);
      parts.state.dataset.state = state;
      set(parts.state, STATES[state]);
      set(parts.message, ai && ai.message ? String(ai.message) : "");
      set(parts.calls, formatCount(ai ? ai.calls : 0));
      set(parts.tokens, `${formatCount(ai ? ai.input_tokens : 0)} in, ${formatCount(ai ? ai.output_tokens : 0)} out`);
      set(parts.cost, formatCost(ai ? ai.cost_usd : 0));
      const latency = ai && Number.isFinite(ai.last_latency_ms) ? `${Math.round(ai.last_latency_ms)} ms` : "None yet";
      set(parts.latency, latency);
      set(parts.quick, quickLine(ai));
      const budget = app.saved && app.saved.ai ? Number(app.saved.ai.max_cost_per_hour) : 0;
      const spent = ai ? Number(ai.hour_cost_usd) || 0 : 0;
      set(parts.hour, budget > 0 ? `${formatCost(spent)} of ${formatCost(budget)} budget` : formatCost(spent));
      parts.meter.max = budget > 0 ? budget : 1;
      parts.meter.value = Math.min(spent, parts.meter.max);
      parts.meter.high = parts.meter.max * 0.8;
      const scene = app.live ? app.live.scene : null;
      const caption = scene && scene.caption ? String(scene.caption) : "";
      set(parts.caption, sceneText(caption, ai));
      const elapsed = Math.max(0, Date.now() - (app.liveReceivedAt || Date.now()));
      const age = caption && scene.at_ms ? `Seen ${formatAge(app.live.now_ms - scene.at_ms + elapsed)}.` : "";
      set(parts.captionAge, age);
      parts.lookNow.disabled = !app.ai.enabled || !app.live;
    }

    parts.lookNow.addEventListener("click", async () => {
      try {
        await api.analyze();
        editor.announce("Asked the AI to look now.");
      } catch (error) {
        editor.report(error);
      }
    });

    return { render };
  };
})();
