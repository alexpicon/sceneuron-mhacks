// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of an assistant request's progress, its failure, and the plan to review. Buttons
   carry data-plan-action, which plan-session.js handles. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { formatCost } = Editor.aiText;
  const { h } = Editor.dom;

  const action = (name, label, attrs) =>
    h("button", Object.assign({ type: "button", "data-plan-action": name }, attrs || {}), label);
  const row = (...buttons) => h("div", { class: "button-row" }, ...buttons);

  /** While the assistant thinks: an indeterminate bar, the elapsed time, and a way to stop waiting. */
  function progressView(mode) {
    const what = mode === "setup" ? "Designing your setup" : "Working on your request";
    return h(
      "div",
      { class: "plan-progress" },
      h("div", { class: "progress-bar", "aria-hidden": "true" }, h("span", {})),
      h("p", {}, h("strong", {}, what), " ", h("span", { class: "hint", "data-elapsed": "" }, "0 s")),
      h("p", { class: "hint" }, "This can take up to two minutes. You can keep editing meanwhile."),
      row(action("cancel", "Stop waiting")),
    );
  }

  function errorView(message) {
    return h(
      "div",
      { class: "plan-error", role: "alert" },
      h("p", {}, message),
      row(action("again", "Try again"), action("discard", "Dismiss")),
    );
  }

  function stoppedView() {
    return h(
      "p",
      { class: "hint plan-stopped" },
      "Stopped waiting. The assistant may still finish in the background, and its cost still counts, ",
      "but its plan will be ignored.",
    );
  }

  function count(n, word) {
    return `${n} ${word}${n === 1 ? "" : "s"}`;
  }

  /**
   * The plan with its changes in plain words. `state` may hold `dirty` (the draft has unsaved
   * edits), `confirm` (asking to replace them), `conflict` (the save was refused), `error`, and `busy`.
   */
  function planView(plan, lines, mode, state) {
    const id = `plan-${mode}-heading`;
    const operations = lines.length
      ? h(
        "ol",
        { class: "plan-ops", "aria-label": "Changes" },
        ...lines.map((line) =>
          h(
            "li",
            { class: "plan-op" },
            h("span", { class: "op-verb", "data-kind": line.kind }, line.verb),
            " ",
            h("span", { class: "op-text" }, line.text),
          )
        ),
      )
      : h("p", { class: "hint" }, "Nothing needs to change.");
    const meta = `${count(lines.length, "change")} · made by ${plan.model || "the assistant"} · cost ` +
      formatCost(plan.cost_usd);
    const parts = [
      h("h3", { id, tabindex: "-1" }, mode === "setup" ? "Proposed setup" : "Proposed plan"),
      h("p", { class: "plan-summary" }, plan.summary || "The assistant gave no summary."),
      operations,
      h("p", { class: "hint plan-meta" }, meta),
    ];
    if (state.conflict) {
      parts.push(
        h(
          "div",
          { class: "confirm-box", role: "alert" },
          h(
            "p",
            {},
            "Not applied: the project changed after the assistant made this plan, in another editor or by a save ",
            "here. Propose again to get a plan for the current project.",
          ),
          row(action("again", "Propose again", { class: "primary" }), action("discard", "Discard plan")),
        ),
      );
      return h("section", { class: "plan", "aria-labelledby": id }, ...parts);
    }
    if (state.confirm) {
      parts.push(
        h(
          "div",
          { class: "confirm-box", role: "alert" },
          h("p", {}, "Applying replaces your unsaved edits with this plan. Apply it anyway?"),
          row(action("confirm", "Replace my edits and apply", { class: "primary" }), action("keep", "Keep my edits")),
        ),
      );
      return h("section", { class: "plan", "aria-labelledby": id }, ...parts);
    }
    if (state.dirty) {
      parts.push(
        h(
          "p",
          { class: "plan-note" },
          "You have unsaved edits. This plan was made from the saved project, so applying it replaces them.",
        ),
      );
    }
    if (state.error) {
      parts.push(h("p", { class: "field-error", role: "alert" }, `Not applied: ${state.error}`));
    }
    const apply = action("apply", state.busy ? "Applying" : "Apply plan", {
      class: "primary",
      disabled: state.busy || lines.length === 0,
      "aria-busy": state.busy ? "true" : null,
    });
    parts.push(row(apply, action("discard", lines.length ? "Discard" : "Dismiss", { disabled: state.busy })));
    return h("section", { class: "plan", "aria-labelledby": id }, ...parts);
  }

  Editor.planView = { errorView, planView, progressView, stoppedView };
})();
