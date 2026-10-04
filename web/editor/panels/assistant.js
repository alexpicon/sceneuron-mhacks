// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the assistant box above the preview, where the streamer describes a change in words and
   reviews the plan the assistant proposes. It also invites a new project to run the setup wizard. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_REQUEST_CHARS } = Editor.constants;
  const { cameraChoice } = Editor.consentText;
  const { h } = Editor.dom;

  const EXAMPLE = "When I hold up a product, show its name in a pink lower third";

  function markup() {
    const request = h("textarea", {
      id: "assistant-request",
      rows: 2,
      maxlength: MAX_REQUEST_CHARS,
      placeholder: EXAMPLE,
      "aria-describedby": "assistant-help",
    });
    const note = { "aria-describedby": "assistant-camera-note" };
    const camera = h("input", Object.assign({ id: "assistant-camera", type: "checkbox" }, note));
    return [
      h(
        "div",
        { class: "section-head" },
        h("h2", { id: "assistant-heading" }, "Assistant"),
        h("button", { id: "setup-open", type: "button", "data-open-setup": "" }, "Set up with AI"),
      ),
      h(
        "div",
        { id: "setup-invite", class: "invite", hidden: true },
        h(
          "p",
          {},
          h("strong", {}, "Start here. "),
          "Describe your stream, and the assistant proposes what the AI should watch for, graphics for those ",
          "moments, and the rules that connect them.",
        ),
        h("button", { type: "button", class: "primary", "data-open-setup": "" }, "Set up my stream"),
      ),
      h(
        "form",
        { id: "assistant-form", class: "assistant-form", novalidate: true },
        h("label", { for: "assistant-request" }, "Describe a change"),
        h(
          "div",
          { class: "assistant-input" },
          request,
          h("button", { id: "assistant-propose", type: "submit", class: "primary" }, "Propose"),
        ),
        h(
          "div",
          { class: "assistant-options" },
          h("div", { class: "check" }, camera, h("label", { for: "assistant-camera" }, "Use the camera image")),
          h("span", { id: "assistant-camera-note", class: "hint" }),
        ),
        h(
          "p",
          { id: "assistant-help", class: "hint" },
          "Press Enter to propose, Shift+Enter for a new line. Nothing changes until you apply the plan.",
        ),
        h("p", { id: "assistant-error", class: "field-error", role: "alert", hidden: true }),
      ),
      h("div", { id: "assistant-result", class: "assistant-result" }),
    ];
  }

  /** Builds and wires the assistant box; returns the view with render. */
  Editor.assistant = function setupAssistant(editor) {
    const { app, byId, views } = editor;
    const section = byId("assistant");
    section.replaceChildren(...markup());
    const request = byId("assistant-request");
    const camera = byId("assistant-camera");
    const propose = byId("assistant-propose");
    const session = Editor.planSession(editor, {
      container: byId("assistant-result"),
      mode: "edit",
      onSettled(outcome) {
        propose.disabled = outcome === "busy";
        propose.textContent = outcome === "busy" ? "Thinking" : "Propose";
        if (outcome === "applied") {
          request.value = "";
          request.focus();
        }
      },
    });

    function render() {
      const settings = app.ai;
      section.hidden = !(settings && settings.available);
      if (section.hidden) {
        return;
      }
      const choice = cameraChoice(settings);
      camera.disabled = choice.disabled;
      if (camera.disabled) {
        camera.checked = false;
      }
      byId("assistant-camera-note").textContent = choice.note;
      const conditions = app.draft ? app.draft.conditions || [] : null;
      byId("setup-invite").hidden = !(conditions && conditions.length === 0);
    }

    // Not marked aria-invalid: saving the project refuses while any field on the page is invalid.
    function showError(message) {
      const error = byId("assistant-error");
      error.textContent = message;
      error.hidden = !message;
    }

    function submit() {
      const text = request.value.trim();
      if (!text || Array.from(text).length > MAX_REQUEST_CHARS) {
        showError(`Describe the change in 1 to ${MAX_REQUEST_CHARS} characters.`);
        request.focus();
        return;
      }
      showError("");
      session.propose(text, camera.checked && !camera.disabled);
    }

    byId("assistant-form").addEventListener("submit", (event) => {
      event.preventDefault();
      if (!session.busy()) {
        submit();
      }
    });
    request.addEventListener("keydown", (event) => {
      if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
        event.preventDefault();
        if (!session.busy()) {
          submit();
        }
      }
    });
    request.addEventListener("input", () => showError(""));
    section.addEventListener("click", (event) => {
      if (event.target.closest("[data-open-setup]")) {
        views.setupWizard.open();
      }
    });

    return { render };
  };
})();
