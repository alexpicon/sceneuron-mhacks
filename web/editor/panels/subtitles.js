// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the AI tab's Subtitles section: consent, the switch, the language labels, stream captions and
   their language, the subtitle settings form, and the live preview from the snapshot. Toggles and the caption
   language apply at once; the form's fields apply when it is saved. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const {
    captionNote,
    finalAnnouncement,
    previewText,
    spokenOptions,
    spokenProblem,
    streamOptions,
    subtitleConsentPoints,
    subtitleState,
    subtitleSwitch,
    subtitleUpdate,
    targetsProblem,
  } = Editor.subtitlesText;
  const { targetPickers } = Editor.subtitlesFields;
  const { clearInvalid, h, option, setInvalid } = Editor.dom;

  /** Wires the Subtitles section; returns the view with render and renderLive. */
  Editor.subtitles = function setupSubtitles(editor) {
    const { app, byId, api, views } = editor;
    const section = byId("subtitles");
    const form = byId("subtitles-form");
    const consent = byId("subtitles-consent");
    const enabled = byId("subtitles-enabled");
    const stream = byId("subtitles-stream");
    const labels = byId("subtitles-labels");
    const streamLanguage = byId("subtitles-stream-language");
    const language = byId("subtitles-language");
    const targets = targetPickers(byId);
    let filled = false;
    // The saved translation model the field last showed; the server changes it when the AI provider changes.
    let shownModel = "";
    // The ids of the final segments last seen; undefined until the first live view.
    let finalsSeen;

    const subtitles = () => (app.ai && app.ai.available && app.ai.subtitles) || null;

    function set(node, text) {
      if (node.textContent !== text) {
        node.textContent = text;
      }
    }

    function fillForm() {
      const settings = subtitles();
      language.replaceChildren(...spokenOptions(settings.language).map(([value, label]) => option(value, label)));
      language.value = settings.language || "";
      targets.show(settings.targets || [], settings.latin_targets || []);
      byId("subtitles-latin-lines").checked = Boolean(settings.latin_lines);
      shownModel = settings.translation_model || "";
      byId("subtitles-model").value = shownModel;
      byId("subtitles-lines").value = String(settings.lines || 2);
      byId("subtitles-clear").value = String((settings.clear_after_ms || 4000) / 1000);
      byId("subtitles-hide-commands").checked = settings.hide_commands !== false;
      filled = true;
    }

    /** Shows a translation model the server saved, unless the field holds an edit that is not saved yet. */
    function followModel(settings) {
      const saved = settings.translation_model || "";
      const field = byId("subtitles-model");
      if (field.value.trim() === shownModel) {
        field.value = saved;
      }
      shownModel = saved;
    }

    /**
     * Draws the settings from /api/ai; the form is filled once, and again after it is saved. A translation model the
     * server changed, as it does when the AI provider changes, replaces the one shown unless that one was edited.
     */
    function render() {
      const settings = subtitles();
      section.hidden = !settings;
      if (!settings) {
        return;
      }
      byId("subtitles-simulated").hidden = !app.ai.simulated;
      consent.checked = Boolean(settings.consent);
      const toggle = subtitleSwitch(settings);
      enabled.checked = toggle.checked;
      enabled.disabled = toggle.disabled;
      set(byId("subtitles-enabled-note"), toggle.note);
      const points = subtitleConsentPoints(settings, app.ai);
      byId("subtitles-consent-text").replaceChildren(
        ...points.map(([label, text]) => h("li", {}, h("strong", {}, `${label}: `), text)),
      );
      labels.checked = settings.labels !== false;
      stream.checked = Boolean(settings.stream_captions);
      set(byId("subtitles-stream-note"), `${captionNote(settings)} Captions need OBS; the editor alone sends none.`);
      streamLanguage.replaceChildren(...streamOptions(settings).map(([value, label]) => option(value, label)));
      streamLanguage.value = settings.stream_language || "";
      streamLanguage.disabled = !settings.stream_captions;
      if (!filled) {
        fillForm();
      } else {
        followModel(settings);
      }
      renderLive();
    }

    function lineNodes(lines) {
      return lines.flatMap((line, index) => (index ? [h("br"), line] : [line]));
    }

    /** Draws what the snapshot says: the chip, the message, and the preview, announcing new final segments. */
    function renderLive() {
      if (section.hidden) {
        return;
      }
      const view = app.live ? app.live.subtitles : null;
      const { state, label } = subtitleState(view);
      const chip = byId("subtitles-state");
      chip.dataset.state = state;
      set(chip, label);
      set(byId("subtitles-message"), view && view.status && view.status.message ? String(view.status.message) : "");
      const shown = previewText(view);
      set(byId("subtitles-note"), shown.note);
      byId("subtitles-note").hidden = !shown.note;
      byId("subtitles-lines-shown").replaceChildren(...lineNodes(shown.lines));
      set(byId("subtitles-live"), shown.live);
      const translated = shown.translations.filter(({ lines }) => lines.length);
      byId("subtitles-translations").replaceChildren(
        ...translated.map(({ code, name, lines }) =>
          h("p", { class: "subtitle-lines", lang: code, "data-target": code },
            h("span", { class: "subtitle-language" }, `${name}: `), ...lineNodes(lines))),
      );
      const announcement = finalAnnouncement(finalsSeen, view);
      finalsSeen = announcement.seen;
      if (announcement.message) {
        editor.announce(announcement.message);
      }
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

    /** The form's values, after marking the fields the server would refuse; null when one is refused. */
    function formValues() {
      const codes = targets.codes();
      // Each target is checked with the ones before it, so a code listed twice is marked where it repeats.
      const targetProblems = codes.map((_, index) => {
        const upToHere = codes.slice(0, index + 1).filter(Boolean);
        return [targets.control(index), targetsProblem(upToHere)];
      });
      const culprit = targetProblems.find(([, problem]) => problem);
      const marked = (input) => (culprit && culprit[0] === input ? culprit[1] : null);
      const problems = [[language, spokenProblem(language.value)]].concat(
        targets.controls().map((input) => [input, marked(input)]),
      );
      for (const [input, problem] of problems) {
        if (problem) {
          setInvalid(input, problem);
        } else {
          clearInvalid(input);
        }
      }
      if (problems.some(([, problem]) => problem)) {
        problems.find(([, problem]) => problem)[0].focus();
        return null;
      }
      return {
        language: language.value,
        targets: codes,
        latin_targets: targets.latinTargets(),
        latin_lines: byId("subtitles-latin-lines").checked,
        translation_model: byId("subtitles-model").value,
        lines: byId("subtitles-lines").value,
        clear_after_s: byId("subtitles-clear").value,
        hide_commands: byId("subtitles-hide-commands").checked,
      };
    }

    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      const values = formValues();
      if (!values) {
        return;
      }
      const change = subtitleUpdate(subtitles(), values);
      if (!change) {
        editor.toast("No subtitle settings changed.", "info");
        return;
      }
      if (await update(change, null)) {
        fillForm();
        editor.toast("Subtitle settings saved.", "info");
      }
    });

    consent.addEventListener("change", () => {
      const change = consent.checked ? { consent: true } : { consent: false, enabled: false };
      const message = consent.checked ? "Consent saved." : "Consent withdrawn; subtitles are off.";
      update({ subtitles: change }, message);
    });

    enabled.addEventListener("change", () => {
      const on = enabled.checked;
      update({ subtitles: { enabled: on } }, on ? "Subtitles are on." : "Subtitles are off.");
    });

    labels.addEventListener("change", () => {
      const on = labels.checked;
      update({ subtitles: { labels: on } }, on ? "Language labels are on." : "Language labels are off.");
    });

    stream.addEventListener("change", () => {
      const on = stream.checked;
      update({ subtitles: { stream_captions: on } }, on ? "Stream captions are on." : "Stream captions are off.");
    });

    streamLanguage.addEventListener("change", () => {
      update({ subtitles: { stream_language: streamLanguage.value } }, "Caption language saved.");
    });

    return { render, renderLive };
  };
})();
