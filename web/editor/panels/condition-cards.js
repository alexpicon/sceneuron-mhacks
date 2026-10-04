// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of one AI condition card: what the AI last said about it, and the fields
   that describe it. Controls carry the condition's index and field, and a focus key by index, because an
   unsaved condition's id follows its name while the streamer types. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_DETAIL_CHARS, MAX_NAME_CHARS, MAX_PROMPT_CHARS } = Editor.constants;
  const { conditionState, formatAge } = Editor.aiText;
  const { check, field, h, hidden } = Editor.dom;

  function liveBlock(base) {
    const meter = h("meter", { min: 0, max: 1, value: 0, "data-live": "meter", "aria-labelledby": `${base}-sure` });
    return h(
      "div",
      { class: "condition-live" },
      h(
        "div",
        { class: "confidence" },
        h("span", { id: `${base}-sure`, class: "hint" }, "Confidence ", h("span", { "data-live": "confidence" })),
        meter,
      ),
      h("p", { class: "condition-detail", "data-live": "detail", hidden: true }),
      h("p", { class: "condition-reason", "data-live": "reason", hidden: true }),
      h("p", { class: "hint", "data-live": "age" }),
    );
  }

  function confirmBox(base, ci) {
    const command = (name, label, extra) =>
      h(
        "button",
        Object.assign({ type: "button", "data-condition-command": name, "data-condition-index": ci }, extra),
        label,
      );
    return h(
      "div",
      { class: "confirm-box", "data-confirm": "", hidden: true },
      h("p", { id: `${base}-confirm-text`, "data-confirm-text": "" }),
      h(
        "div",
        { class: "button-row" },
        command("remove-confirmed", "Remove condition and its rules", {
          class: "danger",
          "aria-describedby": `${base}-confirm-text`,
          "data-focus-key": `condition:${ci}:remove-confirmed`,
        }),
        command("cancel-remove", "Keep it"),
      ),
    );
  }

  /** The card for the condition at index `ci` of the draft; `saved` tells whether its id is final. */
  function conditionCard(condition, ci, saved) {
    const base = `condition-${ci}`;
    const data = (name, extra) =>
      Object.assign(
        {
          id: `${base}-${name}`,
          "data-condition-index": ci,
          "data-condition-field": name,
          "data-focus-key": `condition:${ci}:${name}`,
        },
        extra || {},
      );
    const number = (name, label, min, max, value) =>
      field(`${base}-${name}`, label, h("input", data(name, { type: "number", min, max, step: 1, value })));
    const token = `{{${condition.id}}}`;
    const placeholder = saved
      ? h("p", { class: "hint" }, "Overlay text shows what it reports with ", h("code", {}, token), ".")
      : h(
        "p",
        { class: "hint" },
        "Overlay text shows what it reports with ",
        h("code", {}, token),
        ". The placeholder follows the name until you save.",
      );
    const prompt = h(
      "textarea",
      data("prompt", {
        rows: 3,
        maxlength: MAX_PROMPT_CHARS,
        value: condition.prompt,
        placeholder: "The person holds an object up to the camera.",
      }),
    );
    const detail = h(
      "input",
      data("detail", {
        type: "text",
        maxlength: MAX_DETAIL_CHARS,
        value: condition.detail || "",
        placeholder: "Such as the object's name",
        autocomplete: "off",
      }),
    );
    const name = h("input", data("name", { type: "text", maxlength: MAX_NAME_CHARS, value: condition.name }));
    const enabled = h("input", data("enabled", { type: "checkbox", checked: condition.enabled !== false }));
    const remove = h(
      "button",
      {
        type: "button",
        class: "danger",
        "data-condition-command": "remove",
        "data-condition-index": ci,
        "data-focus-key": `condition:${ci}:remove`,
      },
      "Remove condition",
      hidden(` ${condition.name}`),
    );
    return h(
      "article",
      { class: "condition", "data-condition-index": ci, "aria-labelledby": `${base}-heading` },
      h(
        "div",
        { class: "condition-head" },
        h("h4", { id: `${base}-heading`, "data-heading": "" }, condition.name),
        h("span", { class: "state-badge", "data-live": "badge" }),
      ),
      liveBlock(base),
      placeholder,
      field(`${base}-name`, "Name", name),
      field(`${base}-prompt`, "What should the AI watch for?", prompt),
      field(`${base}-detail`, "What should it report? (optional)", detail),
      check(`${base}-enabled`, "Watch for this", enabled),
      h(
        "div",
        { class: "grid-3" },
        number("min_confidence", "Confidence (%)", 0, 100, Math.round((condition.min_confidence ?? 0.6) * 100)),
        number("confirm", "Turn on after", 1, 10, condition.confirm ?? 1),
        number("release", "Turn off after", 1, 10, condition.release ?? 2),
      ),
      h(
        "p",
        { class: "hint" },
        "It counts when the AI is at least this sure. Turning on and off waits for this many looks in a row.",
      ),
      h("div", { class: "button-row" }, remove),
      confirmBox(base, ci),
    );
  }

  function setText(node, text) {
    if (node.textContent !== text) {
      node.textContent = text;
    }
  }

  /** Shows what the AI last said about the condition; `ageMs` is how long ago it last turned on or off. */
  function fillLive(card, view, condition, ageMs) {
    const part = (name) => card.querySelector(`[data-live="${name}"]`);
    const { state, label } = conditionState(view, condition);
    const badge = part("badge");
    badge.setAttribute("data-state", state);
    setText(badge, label);
    const judged = Boolean(view && view.judged);
    part("meter").value = judged ? view.confidence : 0;
    setText(part("confidence"), judged ? `${Math.round(view.confidence * 100)}%` : "unknown");
    const detail = part("detail");
    detail.hidden = !(view && view.detail);
    setText(detail, view && view.detail ? `Reports: ${view.detail}` : "");
    const reason = part("reason");
    reason.hidden = !(judged && view.reason);
    setText(reason, judged && view.reason ? `Why: ${view.reason}` : "");
    let age = "";
    if (state === "unsaved") {
      age = "Save the project to start watching for it.";
    } else if (state === "waiting") {
      age = "Waiting for the AI to look.";
    } else if (state !== "disabled" && view.since_ms > 0) {
      age = `Turned ${view.on ? "on" : "off"} ${formatAge(ageMs)}.`;
    }
    setText(part("age"), age);
  }

  Editor.conditionCards = { conditionCard, fillLive };
})();
