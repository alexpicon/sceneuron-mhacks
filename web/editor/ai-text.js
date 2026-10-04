// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: AI words and numbers: status labels, costs, ages, the write-only key's state,
   which settings changed, condition badges, and plain explanations of assistant errors. The consent
   wording is in consent-text.js. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);

  const STATES = {
    off: "Off",
    needs_setup: "Needs setup",
    watching: "Watching",
    thinking: "Thinking",
    paused: "Paused",
    failed: "Failed",
  };
  /** Settings the editor's form edits; the key is handled apart because it is write-only. */
  const SETTING_FIELDS = [
    "provider",
    "base_url",
    "vision_model",
    "planner_model",
    "vision_effort",
    "planner_effort",
    "image_width",
    "input_price_per_mtok",
    "output_price_per_mtok",
  ];

  function shorten(text, max) {
    const value = String(text || "");
    return value.length > max ? `${value.slice(0, max - 3)}...` : value;
  }

  /** The AI state, known or "off". */
  function aiState(ai) {
    return ai && Object.prototype.hasOwnProperty.call(STATES, ai.state) ? ai.state : "off";
  }

  /** The status chip's words, such as "AI watching" or "AI failed: the key was rejected". */
  function aiChipLabel(ai) {
    const state = aiState(ai);
    const label = `AI ${STATES[state].toLowerCase()}`;
    const message = ai && ai.message ? shorten(ai.message, 60) : "";
    return state === "failed" && message ? `${label}: ${message}` : label;
  }

  /** US dollars: two decimals, or three below one cent so small spends still show. */
  function formatCost(usd) {
    const value = Number(usd) || 0;
    return `$${value > 0 && value < 0.01 ? value.toFixed(3) : value.toFixed(2)}`;
  }

  function formatCount(count) {
    const value = Number(count) || 0;
    if (value >= 1e6) {
      return `${(value / 1e6).toFixed(1)}M`;
    }
    return value >= 1e4 ? `${(value / 1e3).toFixed(1)}k` : String(Math.round(value));
  }

  /** How long ago something happened, for an age in milliseconds. */
  function formatAge(ms) {
    const seconds = Math.max(0, Math.round((Number(ms) || 0) / 1000));
    if (seconds < 5) {
      return "just now";
    }
    if (seconds < 90) {
      return `${seconds} s ago`;
    }
    const minutes = Math.round(seconds / 60);
    return minutes < 90 ? `${minutes} min ago` : `${Math.round(minutes / 60)} h ago`;
  }

  /**
   * What the AI tab says as the latest scene: the caption, or why there is none. A failure clears the caption, so
   * then it says that scene understanding is not working rather than that nothing was seen.
   */
  function sceneText(caption, ai) {
    if (caption) {
      return String(caption);
    }
    return aiState(ai) === "failed"
      ? "Scene understanding is not working right now, so there is no scene description."
      : "Nothing seen yet.";
  }

  /** Seconds for a duration in milliseconds, without trailing zeros. */
  function formatSeconds(ms) {
    return `${Number(((Number(ms) || 0) / 1000).toFixed(1))} s`;
  }

  /**
   * What a key field says about its key; the key itself is never sent back. `which` is "" for the
   * vision and assistant key, or "jev" for the optional Jev key. A key from a .env file or the
   * environment is named as such, since the editor cannot remove it.
   */
  function keyStatus(settings, which = "") {
    const prefix = which ? `${which}_` : "";
    const missing = which ? "No Jev key; quick judgments stay off, and everything else works" : "No key saved";
    if (!settings || !settings[`has_${prefix}key`]) {
      return missing;
    }
    const hint = String(settings[`${prefix}key_hint`] || "").replace(/^\.+/, "");
    const shown = hint ? ` (...${hint})` : "";
    if (settings[`${prefix}key_source`] === "environment") {
      return `Key from .env or the environment${shown}; change it there`;
    }
    return `Key saved${shown}`;
  }

  /** Whether the editor may remove a key: only one it saved itself. */
  function keyRemovable(settings, which = "") {
    const prefix = which ? `${which}_` : "";
    return Boolean(settings && settings[`has_${prefix}key`] && settings[`${prefix}key_source`] !== "environment");
  }

  /** One line about quick judgments from on-device facts, for the status card. */
  function quickLine(ai) {
    if (!ai || !ai.quick_state || ai.quick_state === "off") {
      return ai && ai.quick_message ? `Off: ${ai.quick_message}` : "Off";
    }
    const latency = Number.isFinite(ai.quick_latency_ms) ? `, last in ${Math.round(ai.quick_latency_ms)} ms` : "";
    const head = ai.quick_state === "failed" ? "Failed" : `${formatCount(ai.quick_calls || 0)} judgments${latency}`;
    return ai.quick_message ? `${head}: ${ai.quick_message}` : head;
  }

  /**
   * The PUT /api/ai body for a settings form: the fields whose values differ from `current`, plus
   * the typed key when there is one. Returns null when nothing changed.
   */
  function settingsUpdate(current, values) {
    const update = {};
    for (const field of SETTING_FIELDS) {
      if (field in values && values[field] !== undefined && values[field] !== (current || {})[field]) {
        update[field] = values[field];
      }
    }
    for (const field of ["api_key", "jev_api_key"]) {
      const key = typeof values[field] === "string" ? values[field].trim() : "";
      if (key) {
        update[field] = key;
      }
    }
    return Object.keys(update).length ? update : null;
  }

  /** A condition's live state as `{ state, label }` for its badge; state is also a CSS hook. */
  function conditionState(view, condition) {
    if (condition && condition.enabled === false) {
      return { state: "disabled", label: "Turned off" };
    }
    if (!view) {
      return { state: "unsaved", label: "Not saved yet" };
    }
    if (!view.judged) {
      return { state: "waiting", label: "Not judged yet" };
    }
    return view.on ? { state: "on", label: "On" } : { state: "off", label: "Off" };
  }

  /** An assistant failure in plain words, keeping the server's own explanation. */
  function planErrorMessage(error) {
    const detail = error && error.message ? error.message : String(error);
    switch (error && error.status) {
      case 0:
        return detail;
      case 429:
        return `The AI provider is busy or the hourly budget is used up: ${detail}. Try again in a moment.`;
      case 502:
        return `The AI provider refused the request: ${detail}. Check the provider settings in the AI tab.`;
      case 503:
        return `The assistant is not ready: ${detail}. Check the AI tab.`;
      case 504:
        return `The assistant took too long to answer: ${detail}. Try a shorter request.`;
      default:
        return `The assistant could not make a plan: ${detail}`;
    }
  }

  const aiText = {
    STATES,
    aiChipLabel,
    aiState,
    conditionState,
    formatAge,
    formatCost,
    formatCount,
    formatSeconds,
    keyRemovable,
    keyStatus,
    planErrorMessage,
    quickLine,
    sceneText,
    settingsUpdate,
  };
  if (inNode) {
    module.exports = aiText;
  } else {
    window.SceneuronEditor.aiText = aiText;
  }
})();
