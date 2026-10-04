// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the Voice section's Type a command box, which runs words as if said with push to talk held, and
   its What you can say list, loaded from the server while it is open and again when the saved project, the mode, or
   the wake word changes. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { commandProblem, guideSections } = Editor.voiceGuideText;
  const { h } = Editor.dom;

  function section({ title, note, lines }) {
    const items = lines.map(({ say, does }) =>
      say.length ? h("li", {}, h("span", { class: "voice-say" }, say.join(" or ")), `: ${does}`) : h("li", {}, does),
    );
    return h(
      "section",
      { class: "voice-guide-group" },
      h("h4", {}, title),
      h("p", { class: "hint" }, note),
      h("ul", {}, ...items),
    );
  }

  /** Wires the box and the list; returns the view with render, which loads the list when it is open and stale. */
  Editor.voiceGuide = function setupVoiceGuide(editor) {
    const { app, byId, api } = editor;
    const details = byId("voice-guide");
    const list = byId("voice-guide-list");
    const form = byId("voice-command-form");
    const input = byId("voice-command-text");
    const error = byId("voice-command-error");
    let loadedFor = null;

    /** What the list depends on: the saved project, the mode, and the wake word. */
    function stamp() {
      const live = app.live || {};
      const voice = (app.ai && app.ai.voice) || {};
      return [live.project_revision, live.voice && live.voice.mode, voice.wake_word].join(" ");
    }

    async function render() {
      const wanted = stamp();
      if (!details.open || wanted === loadedFor) {
        return;
      }
      loadedFor = wanted;
      try {
        const guide = await api.voiceGuide();
        if (loadedFor === wanted) {
          list.replaceChildren(...guideSections(guide).map(section));
        }
      } catch (failure) {
        loadedFor = null;
        list.replaceChildren(h("p", { class: "field-error" }, `Could not load what you can say: ${failure.message}`));
      }
    }

    function showError(message) {
      error.textContent = message || "";
      error.hidden = !message;
    }

    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      const text = input.value.trim();
      const problem = commandProblem(text);
      showError(problem);
      if (problem) {
        input.focus();
        return;
      }
      try {
        await api.voiceCommand(text);
        input.value = "";
      } catch (failure) {
        editor.report(failure);
      }
    });
    details.addEventListener("toggle", render);

    return { render };
  };
})();
