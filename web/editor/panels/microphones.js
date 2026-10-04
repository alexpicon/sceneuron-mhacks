// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the Camera & mic tab's Microphone section: the OBS sources that carry audio, which one voice
   control and subtitles hear, and adding the Sceneuron Voice filter to one in a click; and the same news in the AI
   tab's Voice and Subtitles sections, with a button that leads here. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { micKind, micNote, micSummary } = Editor.micText;
  const { h, hidden, withFocusRestore } = Editor.dom;

  /** The AI tab's notes about the microphone: each text and the button that leads to the list. */
  const NOTES = [
    ["voice-mic", "voice-mic-choose"],
    ["subtitles-mic", "subtitles-mic-choose"],
  ];

  /** Wires the Microphone section and the notes; returns the view with load, which fetches and lists the sources. */
  Editor.microphones = function setupMicrophones(editor) {
    const { app, byId, api, views } = editor;
    const list = byId("mic-list");
    const empty = byId("mics-empty");
    const status = byId("mic-status");
    const refresh = byId("refresh-microphones");

    function set(node, text) {
      if (node.textContent !== text) {
        node.textContent = text;
      }
    }

    function item(source, heard) {
      const name = String(source.name);
      const action = heard ? h("span", { class: "chip", "data-state": "running" }, "Listening") : h(
        "button",
        {
          type: "button",
          "data-mic-uuid": source.uuid,
          "data-mic-name": name,
          "data-focus-key": `mic:${source.uuid}`,
        },
        "Listen to this one",
        hidden(`: ${name}`),
      );
      const filter = source.voice && !heard ? " (has the Sceneuron Voice filter)" : "";
      return h(
        "li",
        {},
        h("div", {}, h("span", {}, name), h("span", { class: "source-kind" }, `${micKind(source)}${filter}`)),
        action,
      );
    }

    function render() {
      const reply = app.microphones;
      const summary = micSummary(reply);
      const sources = reply && reply.available && Array.isArray(reply.sources) ? reply.sources : [];
      withFocusRestore(list, () => {
        list.replaceChildren(...sources.map((source) => item(source, source.uuid === summary.uuid)));
      });
      empty.hidden = !reply || !reply.available || sources.length > 0;
      refresh.hidden = summary.state === "unavailable";
      set(status, micNote(reply, "tab"));
      const choose = summary.state === "one" ? "Change microphone" : "Choose microphone";
      for (const [noteId, buttonId] of NOTES) {
        set(byId(noteId), micNote(reply, "ai"));
        const button = byId(buttonId);
        set(button, choose);
        button.hidden = summary.state === "unavailable" || summary.state === "unknown";
      }
    }

    /** Fetches the sources; a failed request says why in the section, and in a notice when `loud`. */
    async function load(loud) {
      try {
        app.microphones = await api.audioSources();
        render();
      } catch (error) {
        set(status, `Could not list the microphones: ${error.message}`);
        if (loud) {
          editor.report(error);
        }
      }
    }

    refresh.addEventListener("click", () => load(true));

    list.addEventListener("click", async (event) => {
      const button = event.target.closest("[data-mic-uuid]");
      if (!button) {
        return;
      }
      const name = button.getAttribute("data-mic-name");
      try {
        await api.attachVoice(button.getAttribute("data-mic-uuid"));
        editor.toast(`Sceneuron now listens to ${name}.`, "info");
      } catch (error) {
        editor.report(error);
      }
      await load(false);
      if (!list.contains(document.activeElement)) {
        refresh.focus();
      }
    });

    // Leads from the AI tab to the list: to a microphone to choose, or to the refresh button when there is none.
    for (const [, buttonId] of NOTES) {
      byId(buttonId).addEventListener("click", () => {
        views.tabs.reveal(list.querySelector("button") || refresh);
      });
    }

    render();
    return { load };
  };
})();
