// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the AI settings from /api/ai: which AI controls the build offers, the provider form with
   its write-only API key, consent to send camera images, and the switch that turns watching on. The key is
   sent once and never read back; the field is emptied as soon as the server has it. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { keyRemovable, keyStatus, settingsUpdate } = Editor.aiText;
  const { consentPoints, consentUpdate, watchSwitch } = Editor.consentText;
  const { h } = Editor.dom;

  /** Wires the AI settings; returns the view with load and render. */
  Editor.aiSettings = function setupAiSettings(editor) {
    const { app, byId, api, views } = editor;
    const form = byId("ai-settings-form");
    const inputs = Array.from(form.querySelectorAll("[data-setting]"));
    const key = byId("ai-api-key");
    const removeKey = byId("ai-remove-key");
    const jevKey = byId("ai-jev-key");
    const removeJev = byId("ai-remove-jev-key");
    const consent = byId("ai-consent");
    const enabled = byId("ai-enabled");
    const onlyWithAi = ["ai-watch", "ai-provider-details", "ai-policy-details"].map(byId);

    /** Copies the server's settings into the provider form. */
    function fillForm() {
      for (const input of inputs) {
        const value = app.ai[input.getAttribute("data-setting")];
        input.value = value === undefined || value === null ? "" : String(value);
      }
    }

    function readForm() {
      const values = {};
      for (const input of inputs) {
        const name = input.getAttribute("data-setting");
        const text = input.value.trim();
        // An emptied number field keeps the saved value instead of sending zero.
        values[name] = input.type !== "number" ? text : text === "" ? undefined : Number(text);
      }
      values.api_key = key.value;
      values.jev_api_key = jevKey.value;
      return values;
    }

    function render() {
      const settings = app.ai;
      const available = Boolean(settings && settings.available);
      byId("ai-unavailable").hidden = !settings || available;
      for (const node of onlyWithAi) {
        node.hidden = !available;
      }
      if (available) {
        byId("ai-simulated").hidden = !settings.simulated;
        byId("ai-key-status").textContent = keyStatus(settings);
        removeKey.disabled = !keyRemovable(settings);
        byId("ai-jev-key-status").textContent = keyStatus(settings, "jev");
        removeJev.disabled = !keyRemovable(settings, "jev");
        byId("ai-price-note").hidden = settings.price_known !== false;
        const watch = watchSwitch(settings);
        consent.checked = Boolean(settings.consent);
        enabled.checked = watch.checked;
        enabled.disabled = watch.disabled;
        byId("ai-enabled-note").textContent = watch.note;
        const points = consentPoints(settings, app.saved ? app.saved.ai : null);
        byId("ai-consent-text").replaceChildren(
          ...points.map(([label, text]) => h("li", {}, h("strong", {}, `${label}: `), text)),
        );
      }
      views.aiStatus.render();
      views.voice.render();
      views.subtitles.render();
      views.assistant.render();
      views.setupWizard.render();
    }

    /** Stores what the server answered and redraws; `refill` also copies it into the provider form. */
    function accept(settings, refill) {
      app.ai = settings && typeof settings === "object" ? settings : { available: false };
      if (refill && app.ai.available) {
        fillForm();
      }
      render();
    }

    async function load() {
      try {
        accept(await api.aiSettings(), true);
        const details = byId("ai-provider-details");
        details.open = Boolean(app.ai.available && !app.ai.simulated && (!app.ai.has_key || !app.ai.consent));
      } catch (error) {
        editor.report(new Error(`Could not read the AI settings: ${error.message}`));
      }
    }

    async function update(change, success, refill) {
      try {
        accept(await api.updateAiSettings(change), refill);
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

    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      const change = settingsUpdate(app.ai, readForm());
      if (!change) {
        editor.toast("No AI settings changed.", "info");
        return;
      }
      // The form is refilled from the reply: switching to a provider with default models also switches each model
      // the update leaves out, the Subtitles section's translation model included, to that provider's default.
      if (await update(change, null, true)) {
        key.value = "";
        jevKey.value = "";
        editor.toast("AI settings saved.", "info");
      }
    });

    removeKey.addEventListener("click", async () => {
      if (await update({ api_key: "" }, "Removed the API key.", false)) {
        key.focus();
      }
    });

    removeJev.addEventListener("click", async () => {
      if (await update({ jev_api_key: "" }, "Removed the Jev key.", false)) {
        jevKey.focus();
      }
    });

    consent.addEventListener("change", () => {
      const message = consent.checked ? "Consent saved." : "Consent withdrawn; watching is off.";
      update(consentUpdate(consent.checked), message, false);
    });

    enabled.addEventListener("change", () => {
      const on = enabled.checked;
      update({ enabled: on }, on ? "The AI is watching the camera." : "The AI stopped watching.", false);
    });

    byId("ai-provider").addEventListener("change", () => {
      const defaults = {
        gemini: "https://generativelanguage.googleapis.com",
        "openai-compatible": "http://localhost:11434/v1",
      };
      const url = byId("ai-base-url");
      if (!url.value.trim() || Object.values(defaults).includes(url.value.trim())) {
        url.value = defaults[byId("ai-provider").value] || "";
      }
    });

    return { load, render };
  };
})();
