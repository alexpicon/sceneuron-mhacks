// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the AI tab's Voice section: the write-only ElevenLabs key, consent, voice control and its
   mode, spoken cards, the voice settings form, a Hold to talk button, and the live heard line and voice spend
   from the snapshot. Toggles apply at once; the form's fields apply when it is saved. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { formatCost, formatCount, formatSeconds } = Editor.aiText;
  const {
    heardAnnouncement,
    heardLine,
    listenSwitch,
    voiceConsentPoints,
    voiceKeyRemovable,
    voiceKeyStatus,
    voiceState,
    voiceUpdate,
  } = Editor.voiceText;
  const { h } = Editor.dom;

  const TALK_KEYS = [" ", "Enter"];

  /** Wires the Voice section; returns the view with render and renderLive. */
  Editor.voice = function setupVoice(editor) {
    const { app, byId, api, views } = editor;
    const section = byId("voice");
    const form = byId("voice-form");
    const inputs = Array.from(form.querySelectorAll("[data-voice-setting]"));
    const key = byId("voice-key");
    const removeKey = byId("voice-remove-key");
    const consent = byId("voice-consent");
    const enabled = byId("voice-enabled");
    const mode = byId("voice-mode");
    const speak = byId("voice-speak");
    const talk = byId("voice-talk");
    const parts = { state: byId("voice-state"), message: byId("voice-message"), heard: byId("voice-heard") };
    let filled = false;
    let held = false;
    // The last final heard result announced; undefined until the first live view.
    let heardSeen;
    // Push-to-talk requests go out one at a time, so a quick press never arrives after its release.
    let talkQueue = Promise.resolve();

    const voice = () => (app.ai && app.ai.available && app.ai.voice) || null;

    function set(node, text) {
      if (node.textContent !== text) {
        node.textContent = text;
      }
    }

    function fillForm() {
      const settings = voice();
      for (const input of inputs) {
        const value = settings[input.getAttribute("data-voice-setting")];
        input.value = value === undefined || value === null ? "" : String(value);
      }
      filled = true;
    }

    /** Draws the settings from /api/ai; the form is filled once, and again after it is saved. */
    function render() {
      const settings = voice();
      section.hidden = !settings;
      if (!settings) {
        return;
      }
      byId("voice-simulated").hidden = !app.ai.simulated;
      set(byId("voice-key-status"), voiceKeyStatus(app.ai));
      removeKey.disabled = !voiceKeyRemovable(app.ai);
      consent.checked = Boolean(settings.consent);
      const listen = listenSwitch(settings);
      enabled.checked = listen.checked;
      enabled.disabled = listen.disabled;
      set(byId("voice-enabled-note"), listen.note);
      mode.value = settings.mode;
      speak.checked = Boolean(settings.speak);
      byId("voice-consent-text").replaceChildren(
        ...voiceConsentPoints(settings).map(([label, text]) => h("li", {}, h("strong", {}, `${label}: `), text)),
      );
      if (!filled) {
        fillForm();
      }
      renderLive();
    }

    /** Draws what the snapshot says: the voice state, the heard line, push to talk, and voice spend. */
    function renderLive() {
      if (section.hidden) {
        return;
      }
      const view = app.live ? app.live.voice : null;
      const { state, label } = voiceState(view);
      parts.state.dataset.state = state;
      set(parts.state, label);
      set(parts.message, view && view.message ? String(view.message) : "");
      set(parts.heard, heardLine(view));
      const announcement = heardAnnouncement(heardSeen, view);
      heardSeen = announcement.seen;
      if (announcement.message) {
        editor.announce(announcement.message);
      }
      talk.setAttribute("aria-pressed", String(Boolean(view && view.talking) || held));
      talk.disabled = !(view && view.enabled) && !held;
      const ai = app.live ? app.live.ai : null;
      set(byId("voice-seconds"), formatSeconds(ai ? ai.voice_seconds * 1000 : 0));
      set(byId("voice-characters"), formatCount(ai ? ai.voice_characters : 0));
      set(byId("voice-cost"), formatCost(ai ? ai.voice_cost_usd : 0));
      views.voiceGuide.render();
    }

    async function update(change, success) {
      try {
        app.ai = await api.updateAiSettings(change);
        views.aiSettings.render();
        if (success) {
          editor.announce(success);
        }
        return true;
      } catch (error) {
        editor.report(error);
        render();
        return false;
      }
    }

    function holdToTalk(down) {
      if (held === down) {
        return;
      }
      held = down;
      talk.setAttribute("aria-pressed", String(down));
      talkQueue = talkQueue.then(() => editor.act(api.talk(down)));
    }

    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      const values = { elevenlabs_api_key: key.value };
      for (const input of inputs) {
        values[input.getAttribute("data-voice-setting")] = input.value;
      }
      const change = voiceUpdate(voice(), values);
      if (!change) {
        editor.toast("No voice settings changed.", "info");
        return;
      }
      if (await update(change, null)) {
        key.value = "";
        fillForm();
        editor.toast("Voice settings saved.", "info");
      }
    });

    removeKey.addEventListener("click", async () => {
      if (await update({ elevenlabs_api_key: "" }, "Removed the ElevenLabs key.")) {
        key.focus();
      }
    });

    consent.addEventListener("change", () => {
      const change = consent.checked ? { consent: true } : { consent: false, enabled: false };
      const message = consent.checked ? "Consent saved." : "Consent withdrawn; voice control is off.";
      update({ voice: change }, message);
    });

    enabled.addEventListener("change", () => {
      const on = enabled.checked;
      update({ voice: { enabled: on } }, on ? "Voice control is on." : "Voice control is off.");
    });

    mode.addEventListener("change", () => {
      const always = mode.value === "always_on";
      update({ voice: { mode: mode.value } }, always ? "Listening all the time." : "Listening while you hold to talk.");
    });

    speak.addEventListener("change", () => {
      const on = speak.checked;
      update({ voice: { speak: on } }, on ? "Cards are spoken when raised." : "Cards are no longer spoken.");
    });

    talk.addEventListener("pointerdown", (event) => {
      if (event.button === 0) {
        holdToTalk(true);
      }
    });
    for (const type of ["pointerup", "pointercancel", "pointerleave", "blur"]) {
      talk.addEventListener(type, () => holdToTalk(false));
    }
    talk.addEventListener("keydown", (event) => {
      if (TALK_KEYS.includes(event.key)) {
        event.preventDefault();
        if (!event.repeat) {
          holdToTalk(true);
        }
      }
    });
    talk.addEventListener("keyup", (event) => {
      if (TALK_KEYS.includes(event.key)) {
        event.preventDefault();
        holdToTalk(false);
      }
    });

    return { render, renderLive };
  };
})();
