// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: "Set up my stream", a short guided flow in a modal dialog: describe the stream, choose
   whether the assistant may see the camera, then review the proposed setup. It opens by itself once per
   browser session when the AI is available and the saved project watches for nothing yet. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_REQUEST_CHARS } = Editor.constants;
  const { cameraChoice } = Editor.consentText;
  const { h } = Editor.dom;

  const STEPS = [["describe", "Describe"], ["camera", "Camera"], ["review", "Review"]];
  const EXAMPLES = [
    "I review keyboards and my cat jumps on the desk",
    "I play cozy games and wave hello to new followers",
    "I paint miniatures and hold each one up to the camera",
    "I run sprint planning on Zoom and the team votes on story points",
  ];
  const OFFERED_KEY = "sceneuron-setup-offered";

  const button = (attrs, label) => h("button", Object.assign({ type: "button" }, attrs), label);
  const step = (name, ...children) => h("section", { "data-setup-step": name, hidden: true }, ...children);
  const actions = (...buttons) => h("div", { class: "button-row end" }, ...buttons);

  function markup() {
    const request = h("textarea", { id: "setup-request", rows: 3, maxlength: MAX_REQUEST_CHARS });
    const camera = h("input", { id: "setup-camera", type: "checkbox", "aria-describedby": "setup-camera-note" });
    return [
      h(
        "div",
        { class: "setup-head" },
        h("h2", { id: "setup-title" }, "Set up my stream"),
        button({ class: "icon", "data-setup": "close", "aria-label": "Close" }, "×"),
      ),
      h(
        "ol",
        { class: "steps", "aria-label": "Steps" },
        ...STEPS.map(([name, label], index) => h("li", { "data-step-marker": name }, `${index + 1}. ${label}`)),
      ),
      step(
        "describe",
        h(
          "p",
          { class: "hint" },
          "Tell the assistant what happens on your stream. It proposes what the AI should watch for, graphics for ",
          "those moments, and the rules that connect them. Nothing changes until you apply the plan.",
        ),
        h("label", { for: "setup-request" }, "Describe your stream"),
        request,
        h("p", { id: "setup-error", class: "field-error", role: "alert", hidden: true }),
        h("p", { id: "setup-examples-label", class: "hint" }, "Or start from an example:"),
        h(
          "div",
          { class: "example-chips", role: "group", "aria-labelledby": "setup-examples-label" },
          ...EXAMPLES.map((text) => button({ class: "example-chip", "data-example": text }, text)),
        ),
        actions(button({ class: "primary", "data-setup": "next" }, "Next")),
      ),
      step(
        "camera",
        h("p", {}, "The assistant can look at one image from your camera to fit the graphics around you."),
        h("div", { class: "check" }, camera, h("label", { for: "setup-camera" }, "Use the camera image")),
        h("p", { id: "setup-camera-note", class: "hint" }),
        actions(
          button({ "data-setup": "back" }, "Back"),
          button({ class: "primary", "data-setup": "propose" }, "Propose a setup"),
        ),
      ),
      step(
        "review",
        h("div", { id: "setup-result", class: "assistant-result" }),
        actions(button({ "data-setup": "restart" }, "Start over")),
      ),
    ];
  }

  /** Builds and wires the wizard; returns the view with open and render. */
  Editor.setupWizard = function setupSetupWizard(editor) {
    const { app, byId } = editor;
    const dialog = byId("setup-dialog");
    dialog.replaceChildren(...markup());
    const request = byId("setup-request");
    const camera = byId("setup-camera");
    let opener = null;
    const session = Editor.planSession(editor, {
      container: byId("setup-result"),
      mode: "setup",
      onSettled(outcome) {
        if (outcome === "applied") {
          dialog.close();
          editor.announce("Your stream is set up, and the setup is saved.");
        } else if (outcome === "discarded") {
          go("describe");
        }
      },
    });

    function go(name) {
      for (const section of dialog.querySelectorAll("[data-setup-step]")) {
        section.hidden = section.getAttribute("data-setup-step") !== name;
      }
      for (const marker of dialog.querySelectorAll("[data-step-marker]")) {
        const here = marker.getAttribute("data-step-marker") === name;
        marker.setAttribute("aria-current", here ? "step" : "false");
      }
      const focus = { describe: request, camera, review: null }[name];
      if (focus) {
        focus.focus();
      }
    }

    function showError(message) {
      const error = byId("setup-error");
      error.textContent = message;
      error.hidden = !message;
    }

    function open() {
      if (dialog.open) {
        return;
      }
      opener = document.activeElement;
      showError("");
      dialog.showModal();
      go(session.busy() ? "review" : "describe");
    }

    function render() {
      const choice = cameraChoice(app.ai);
      camera.disabled = choice.disabled;
      if (camera.disabled) {
        camera.checked = false;
      }
      byId("setup-camera-note").textContent = choice.note;
      offer();
    }

    /** Opens the wizard once per browser session for a project that watches for nothing yet. */
    function offer() {
      const ready = app.ai && app.ai.available && app.saved && (app.saved.conditions || []).length === 0;
      if (!ready) {
        return;
      }
      try {
        if (window.sessionStorage.getItem(OFFERED_KEY)) {
          return;
        }
        window.sessionStorage.setItem(OFFERED_KEY, "1");
      } catch {
        // Without session storage the wizard is offered on every load, which is still correct.
      }
      open();
    }

    dialog.addEventListener("click", (event) => {
      const example = event.target.closest("[data-example]");
      if (example) {
        request.value = example.getAttribute("data-example");
        showError("");
        request.focus();
        return;
      }
      const target = event.target.closest("[data-setup]");
      const name = target ? target.getAttribute("data-setup") : null;
      if (name === "close") {
        dialog.close();
      } else if (name === "next") {
        const text = request.value.trim();
        if (!text) {
          showError("Describe your stream in a few words, or pick an example.");
          request.focus();
          return;
        }
        showError("");
        go("camera");
      } else if (name === "back") {
        go("describe");
      } else if (name === "propose") {
        go("review");
        session.propose(request.value.trim(), camera.checked && !camera.disabled);
      } else if (name === "restart") {
        session.cancel();
        go("describe");
      }
    });
    dialog.addEventListener("close", () => {
      session.cancel();
      if (opener && opener.isConnected && typeof opener.focus === "function") {
        opener.focus();
      }
    });

    return { open, render };
  };
})();
