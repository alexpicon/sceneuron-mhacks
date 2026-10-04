// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the Settings tab's fun reactions: what they do now, and every preset with a button that plays it
   at once and, for a preset the camera plays, a switch for its camera and what the camera does for it now, and for a
   preset with trigger phrases, a switch for its voice with the phrase to say. The reactions' switch and cooldown are
   project settings, which settings.js edits with the other settings; the camera and voice switches edit the draft's
   reactions.camera_off and reactions.voice_off. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { PRESETS } = window.SceneuronReactionPresets;
  const { cameraLabel, cameraStatus, reactionsStatus, voiceLabel, voiceStatus } = Editor.reactionsText;
  const { check, h } = Editor.dom;
  const SWITCHES = { camera: "camera_off", voice: "voice_off" };

  /** A switch for the preset's camera or voice, and the line that says what it does now. */
  function switchRow(preset, kind, label) {
    const id = `reaction-${kind}-${preset.id}`;
    return [
      check(id, label, h("input", { id, type: "checkbox", [`data-reaction-${kind}`]: preset.id })),
      h("p", { id: `reaction-${kind}-status-${preset.id}`, class: "hint", "aria-live": "polite" }),
    ];
  }

  /** One preset's row: its first emoji, name, and banner line, its Try it button, and its camera and voice switches. */
  function presetItem(preset) {
    const camera = preset.camera ? switchRow(preset, "camera", cameraLabel(preset)) : [];
    const voice = preset.phrases.length ? switchRow(preset, "voice", voiceLabel(preset)) : [];
    const line = preset.banner ? `"${preset.banner}"` : preset.emojis.join(" ");
    return h(
      "li",
      { class: "reaction-preset", "data-preset": preset.id },
      h(
        "div",
        { class: "reaction-head" },
        h("span", { class: "reaction-emoji", "aria-hidden": "true" }, preset.emojis[0]),
        h("span", { class: "reaction-name" }, preset.name, h("span", { class: "hint" }, line)),
        h(
          "button",
          { id: `reaction-try-${preset.id}`, type: "button", "data-reaction-try": preset.id },
          "Try it",
          h("span", { class: "visually-hidden" }, `: ${preset.name}`),
        ),
      ),
      ...camera,
      ...voice,
    );
  }

  /** Wires the fun reactions section; returns the view with render. */
  Editor.reactions = function setupReactions(editor) {
    const { app, byId, api } = editor;
    const status = byId("reactions-status");
    const list = byId("reactions-presets");
    list.replaceChildren(...PRESETS.map(presetItem));

    function setText(node, text) {
      if (node.textContent !== text) {
        node.textContent = text;
      }
    }

    /** Shows the camera or voice switch of `preset` and what it does now, from `shown`, null when it has none. */
    function renderSwitch(preset, kind, shown) {
      if (!shown) {
        return;
      }
      const reactions = (app.draft && app.draft.reactions) || null;
      const toggle = byId(`reaction-${kind}-${preset.id}`);
      toggle.checked = !((reactions && reactions[SWITCHES[kind]]) || []).includes(preset.id);
      toggle.disabled = !reactions || reactions.enabled === false;
      const note = byId(`reaction-${kind}-status-${preset.id}`);
      setText(note, shown.text);
      note.setAttribute("data-state", shown.state);
    }

    function render() {
      const draft = app.draft || {};
      const overall = reactionsStatus(draft.reactions, app.live);
      setText(status, overall.text);
      status.setAttribute("data-state", overall.state);
      for (const preset of PRESETS) {
        byId(`reaction-try-${preset.id}`).disabled = !app.live;
        renderSwitch(preset, "camera", cameraStatus(preset, draft.reactions, draft.ai, app.live, app.ai));
        renderSwitch(preset, "voice", voiceStatus(preset, draft.reactions, app.live));
      }
    }

    /** Switches the camera or voice of `id` on or off in the draft, keeping the presets' order. */
    function setSwitch(kind, id, on) {
      const reactions = app.draft && app.draft.reactions;
      if (!reactions) {
        return;
      }
      const field = SWITCHES[kind];
      const off = new Set(reactions[field] || []);
      if (on) {
        off.delete(id);
      } else {
        off.add(id);
      }
      reactions[field] = PRESETS.map((preset) => preset.id).filter((preset) => off.has(preset));
      editor.draftChanged({});
      render();
    }

    // On input, which reaches the list before the form, whose listener below renders from the draft.
    list.addEventListener("input", (event) => {
      for (const kind of Object.keys(SWITCHES)) {
        const id = event.target.getAttribute(`data-reaction-${kind}`);
        if (id) {
          setSwitch(kind, id, event.target.checked);
        }
      }
    });
    list.addEventListener("click", (event) => {
      const button = event.target.closest("[data-reaction-try]");
      const preset = button && PRESETS.find((entry) => entry.id === button.getAttribute("data-reaction-try"));
      if (preset) {
        editor.act(api.flood(preset.id), `Played ${preset.emojis[0]} ${preset.name}.`);
      }
    });
    // settings.js listens first, so the draft already holds the new value.
    byId("settings-form").addEventListener("input", render);
    // The AI settings load after the first render, so the section is brought up to date whenever it shows.
    new MutationObserver(render).observe(byId("panel-settings"), { attributes: true, attributeFilter: ["hidden"] });

    return { render };
  };
})();
