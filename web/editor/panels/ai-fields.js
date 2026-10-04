// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of the AI tab: scene understanding status and controls, the conditions
   list, the provider settings, and the project's pace and budget fields. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { check, field, h, option } = Editor.dom;

  const SIMULATED = "Demo mode: this AI is simulated. It watches a scripted scene, and nothing leaves this computer.";
  const UNKNOWN_PRICE =
    "Sceneuron does not know this model's prices. Enter them so spending estimates and the hourly budget work.";
  const NO_AI =
    "This build of Sceneuron has no AI. You can still write conditions and rules here; they run in a build with AI.";

  const note = (id, text) => h("p", { id, class: "note", hidden: true }, text);
  const head = (id, text, ...extra) => h("div", { class: "section-head" }, h("h3", { id }, text), ...extra);
  const stat = (id, term, text) => h("div", {}, h("dt", {}, term), h("dd", { id }, text));

  function number(id, label, attrs) {
    const input = h("input", Object.assign({ id, type: "number" }, attrs));
    return field(id, label, input);
  }

  function select(id, label, attrs, choices) {
    const control = h("select", Object.assign({ id }, attrs), ...choices.map(([value, name]) => option(value, name)));
    return field(id, label, control);
  }

  function statusCard() {
    const consent = h("input", { id: "ai-consent", type: "checkbox", "aria-describedby": "ai-consent-text" });
    const enabled = h("input", {
      id: "ai-enabled",
      type: "checkbox",
      role: "switch",
      "aria-describedby": "ai-enabled-note",
    });
    const meter = { id: "ai-budget-meter", min: 0, max: 1, value: 0, "aria-labelledby": "ai-hour-label" };
    const budget = h("meter", meter);
    return h(
      "section",
      { id: "ai-watch", class: "ai-card", "aria-labelledby": "ai-watch-heading", hidden: true },
      head("ai-watch-heading", "Scene understanding", h("span", { id: "ai-state", class: "chip" }, "Off")),
      h("p", { id: "ai-message", class: "hint" }),
      note("ai-simulated", SIMULATED),
      check("ai-consent", "I agree to send camera images to the AI provider", consent),
      h("ul", { id: "ai-consent-text", class: "consent-points" }),
      h("div", { class: "check switch" }, enabled, h("label", { for: "ai-enabled" }, "Watch the camera with AI")),
      h("p", { id: "ai-enabled-note", class: "hint" }),
      h(
        "dl",
        { class: "ai-stats" },
        stat("ai-calls", "Looks", "0"),
        stat("ai-tokens", "Tokens", "0 in, 0 out"),
        stat("ai-cost", "Spent", "$0.00"),
        stat("ai-latency", "Last answer", "None yet"),
        stat("ai-quick", "Quick judgments", "Off"),
      ),
      h(
        "div",
        { class: "budget" },
        h("span", { id: "ai-hour-label" }, "This hour: ", h("span", { id: "ai-hour" }, "$0.00")),
        budget,
      ),
      h(
        "figure",
        { class: "scene" },
        h("figcaption", {}, "Latest scene"),
        h("blockquote", { id: "ai-caption" }, "Nothing seen yet."),
        h("p", { id: "ai-caption-age", class: "hint" }),
      ),
      h("button", { id: "ai-look-now", type: "button" }, "Look now"),
    );
  }

  function conditionsSection() {
    return h(
      "section",
      { class: "ai-section", "aria-labelledby": "conditions-heading" },
      head("conditions-heading", "What the AI watches for"),
      h(
        "p",
        { class: "hint" },
        "Describe something the camera might see. Rules can react when it turns on or off, and overlay text ",
        "can show what the AI reports through its placeholder.",
      ),
      h("div", { id: "conditions-list", class: "conditions" }),
      h("p", { id: "conditions-empty", class: "hint", hidden: true }, "No conditions yet."),
      h("button", { id: "add-condition", type: "button" }, "Add condition"),
    );
  }

  function providerForm() {
    const providers = [
      ["gemini", "Google Gemini"],
      ["openai-compatible", "OpenAI-compatible (NVIDIA NIM, Ollama, llama.cpp)"],
    ];
    const efforts = [["low", "Low"], ["medium", "Medium"], ["high", "High"]];
    const text = (id, label, setting) => {
      const input = h("input", { id, type: "text", "data-setting": setting, autocomplete: "off", spellcheck: "false" });
      return field(id, label, input);
    };
    const key = h("input", {
      id: "ai-api-key",
      type: "password",
      autocomplete: "off",
      spellcheck: "false",
      "aria-describedby": "ai-key-status",
    });
    const remove = h("button", { id: "ai-remove-key", type: "button", class: "danger" }, "Remove key");
    const jevKey = h("input", {
      id: "ai-jev-key",
      type: "password",
      autocomplete: "off",
      spellcheck: "false",
      "aria-describedby": "ai-jev-key-status",
    });
    const removeJev = h("button", { id: "ai-remove-jev-key", type: "button", class: "danger" }, "Remove Jev key");
    return h(
      "form",
      { id: "ai-settings-form", class: "form", novalidate: true },
      select("ai-provider", "Provider", { "data-setting": "provider" }, providers),
      text("ai-base-url", "Address (base URL)", "base_url"),
      h(
        "div",
        { class: "grid-2" },
        text("ai-vision-model", "Vision model", "vision_model"),
        text("ai-planner-model", "Assistant model", "planner_model"),
        select("ai-vision-effort", "Vision effort", { "data-setting": "vision_effort" }, efforts),
        select("ai-planner-effort", "Assistant effort", { "data-setting": "planner_effort" }, efforts),
      ),
      h(
        "div",
        { class: "field" },
        h("label", { for: "ai-api-key" }, "API key"),
        h("div", { class: "inline-field" }, key, remove),
        h("p", { id: "ai-key-status", class: "hint" }, "No key saved"),
      ),
      h(
        "div",
        { class: "field" },
        h("label", { for: "ai-jev-key" }, "Jev key (optional)"),
        h("div", { class: "inline-field" }, jevKey, removeJev),
        h("p", { id: "ai-jev-key-status", class: "hint" }, "No Jev key"),
        h(
          "p",
          { class: "hint" },
          "TypeSafe's Jev judges each condition from on-device facts in under a second, so overlays react " +
            "faster and the vision model is asked less often. Only text facts are sent, never images.",
        ),
      ),
      number("ai-image-width", "Image width sent (px)", {
        min: 256,
        max: 2048,
        step: 1,
        "data-setting": "image_width",
      }),
      h(
        "div",
        { class: "grid-2" },
        number("ai-input-price", "Input price (US$ per million tokens)", {
          min: 0,
          max: 1000,
          step: 0.01,
          "data-setting": "input_price_per_mtok",
        }),
        number("ai-output-price", "Output price (US$ per million tokens)", {
          min: 0,
          max: 1000,
          step: 0.01,
          "data-setting": "output_price_per_mtok",
        }),
      ),
      note("ai-price-note", UNKNOWN_PRICE),
      h(
        "div",
        { class: "button-row" },
        h("button", { id: "ai-save-settings", type: "submit", class: "primary" }, "Save AI settings"),
      ),
      h("p", { class: "hint" }, "These settings apply at once and stay on this computer, outside the project."),
    );
  }

  function policyForm() {
    const policy = (id, label, attrs) => number(id, label, Object.assign({ "data-ai": id.slice(3) }, attrs));
    return h(
      "form",
      { id: "ai-policy-form", class: "form", novalidate: true },
      policy("ai-min_interval_ms", "Shortest time between looks (seconds)", {
        min: 1,
        max: 600,
        step: 0.5,
        "data-scale": 1000,
        "data-integer-scaled": "true",
      }),
      policy("ai-refresh_ms", "Look again after this long anyway (seconds)", {
        min: 1,
        max: 3600,
        step: 1,
        "data-scale": 1000,
        "data-integer-scaled": "true",
      }),
      policy("ai-change_threshold", "Picture change that earns an early look (%)", {
        min: 0,
        max: 100,
        step: 1,
        "data-scale": 0.01,
      }),
      policy("ai-max_calls_per_minute", "Most looks per minute", { min: 1, max: 60, step: 1, "data-integer": "true" }),
      policy("ai-max_cost_per_hour", "Hourly budget (US$)", { min: 0, max: 100, step: 0.5 }),
      h("p", { class: "hint" }, "Saved with the project. Scene understanding pauses when the hourly budget is spent."),
    );
  }

  /** Fills the empty AI tab panel. */
  function buildAiPanel(panel) {
    const details = (id, summary, form) =>
      h("details", { id, class: "ai-details", hidden: true }, h("summary", {}, summary), form);
    panel.replaceChildren(
      note("ai-unavailable", NO_AI),
      statusCard(),
      conditionsSection(),
      details("ai-provider-details", "AI provider and key", providerForm()),
      details("ai-policy-details", "Pace and budget", policyForm()),
    );
  }

  Editor.aiFields = { buildAiPanel };
})();
