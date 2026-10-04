// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of the AI tab's Subtitles section: its status, the microphone it hears, consent to send
   microphone audio and final lines, the switch, a preview of the lines, the language labels' switch, stream captions,
   and the subtitle settings, with language pickers that list languages by name and take any other code typed, and
   keep or drop the script of their own a picked language is written in. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_TARGETS, OTHER_LANGUAGE, spokenOptions } = Editor.subtitlesText;
  const { GROUPS, languageName, listedCode, scriptName } = Editor.languageText;
  const { check, field, h, option } = Editor.dom;

  const SIMULATED = "Demo mode: subtitles are simulated. A scripted speaker says a line every few seconds with " +
    "canned translations, and nothing leaves this computer.";

  function switchRow(id, label, note) {
    const input = h("input", { id, type: "checkbox", role: "switch", "aria-describedby": `${id}-note` });
    return [
      h("div", { class: "check switch" }, input, h("label", { for: id }, label)),
      h("p", { id: `${id}-note`, class: "hint" }, note),
    ];
  }

  function text(id, label, setting, extra) {
    const attrs = { id, type: "text", "data-subtitle-setting": setting, autocomplete: "off", spellcheck: "false" };
    const input = h("input", Object.assign(attrs, extra));
    return field(id, label, input);
  }

  /** A target's picker: languages by name in their groups, and a field for any code once "Another" is chosen. */
  function targetPicker(number) {
    const id = `subtitles-target-${number}`;
    const groups = GROUPS.map(({ label, languages }) =>
      h("optgroup", { label }, ...languages.map(([code, name]) => option(code, name))));
    const select = h(
      "select",
      { id, "data-subtitle-setting": "target", "aria-describedby": "subtitles-targets-hint" },
      option("", "None"),
      ...groups,
      option(OTHER_LANGUAGE, "Another language code"),
    );
    const code = h("input", {
      id: `${id}-code`,
      type: "text",
      autocomplete: "off",
      spellcheck: "false",
      maxlength: 35,
      "aria-describedby": "subtitles-targets-hint",
    });
    const script = h("input", { id: `${id}-script`, type: "checkbox", "aria-describedby": "subtitles-targets-hint" });
    return h(
      "div",
      { class: "target-pick" },
      field(id, `Translation ${number}`, select),
      h("div", { id: `${id}-code-field`, class: "field", hidden: true },
        h("label", { for: `${id}-code` }, `Code for translation ${number}`), code),
      h("div", { id: `${id}-script-field`, class: "check target-script", hidden: true },
        script, h("label", { id: `${id}-script-label`, for: `${id}-script` }, "")),
    );
  }

  function settingsForm() {
    const targets = Array.from({ length: MAX_TARGETS }, (_, index) => targetPicker(index + 1));
    const spoken = h(
      "select",
      { id: "subtitles-language", "data-subtitle-setting": "language", "aria-describedby": "subtitles-language-hint" },
      ...spokenOptions("").map(([value, label]) => option(value, label)),
    );
    const lines = h(
      "select",
      { id: "subtitles-lines", "data-subtitle-setting": "lines" },
      ...[1, 2, 3].map((count) => option(String(count), `${count} ${count === 1 ? "line" : "lines"}`)),
    );
    const clear = h("input", {
      id: "subtitles-clear",
      type: "number",
      min: 1,
      max: 60,
      step: 0.5,
      "data-subtitle-setting": "clear_after_s",
    });
    const hide = h("input", { id: "subtitles-hide-commands", type: "checkbox" });
    return h(
      "form",
      { id: "subtitles-form", class: "form", novalidate: true },
      field("subtitles-language", "Spoken language", spoken),
      h(
        "p",
        { id: "subtitles-language-hint", class: "hint" },
        "The languages ElevenLabs Scribe transcribes. Constructed and fictional languages can be translated into, " +
          "not heard.",
      ),
      h("fieldset", { class: "targets" }, h("legend", {}, "Translate into"), ...targets),
      h(
        "p",
        { id: "subtitles-targets-hint", class: "hint" },
        "Pick languages by name; constructed and fictional ones, such as Quenya or Klingon, are translated in Latin " +
          "script, and the overlay writes Quenya and Sindarin in Tengwar, Klingon in pIqaD, and Toki Pona in " +
          "sitelen pona unless you untick that under the language; captions keep Latin letters. For another, choose " +
          "Another language code and type its code, such as es-MX, or art-x- and up to 8 letters, such as " +
          "art-x-huttese, for a fictional language without one. Leave all three at None to show only what you say.",
      ),
      check("subtitles-latin-lines", "Show a small Latin line under each line written in its own script",
        h("input", { id: "subtitles-latin-lines", type: "checkbox" })),
      text("subtitles-model", "Translation model", "translation_model", { maxlength: 200 }),
      h("div", { class: "grid-2" }, field("subtitles-lines", "Lines per language", lines),
        field("subtitles-clear", "Clear after (seconds)", clear)),
      check("subtitles-hide-commands", "Leave out what you say to Sceneuron (with voice control on)", hide),
      h(
        "div",
        { class: "button-row" },
        h("button", { id: "subtitles-save", type: "submit", class: "primary" }, "Save subtitle settings"),
      ),
    );
  }

  /** The preview: the final lines, the words still being said, and each translation; it is no live region. */
  function preview() {
    return h(
      "div",
      { id: "subtitles-preview", class: "subtitle-preview", "aria-label": "Subtitles preview" },
      h("p", { id: "subtitles-note", class: "hint" }, "Subtitles are off."),
      h("p", { id: "subtitles-lines-shown", class: "subtitle-lines" }),
      h("p", { id: "subtitles-live", class: "subtitle-live" }),
      h("div", { id: "subtitles-translations" }),
    );
  }

  /** The Subtitles section, hidden until the AI settings say subtitles are available. */
  function buildSubtitlesSection() {
    const consent = h("input", {
      id: "subtitles-consent",
      type: "checkbox",
      "aria-describedby": "subtitles-consent-text",
    });
    const stream = h("select", { id: "subtitles-stream-language", "aria-describedby": "subtitles-stream-note" });
    return h(
      "section",
      { id: "subtitles", class: "ai-card", "aria-labelledby": "subtitles-heading", hidden: true },
      h(
        "div",
        { class: "section-head" },
        h("h3", { id: "subtitles-heading" }, "Subtitles"),
        h("span", { id: "subtitles-state", class: "chip", "data-state": "off" }, "Off"),
      ),
      h("p", { id: "subtitles-message", class: "hint" }),
      h("p", { id: "subtitles-simulated", class: "note", hidden: true }, SIMULATED),
      h(
        "div",
        { class: "mic-note" },
        h("p", { id: "subtitles-mic", class: "hint" }),
        h("button", { id: "subtitles-mic-choose", type: "button", hidden: true }, "Choose microphone"),
      ),
      h(
        "p",
        { class: "hint" },
        "Live subtitles of what you say, in your language and up to three others, on your own video in any call " +
          "or stream. Show them with the Subtitles pack in the Cards tab, or put {{subtitles}} or {{subtitles:es}} " +
          "in any text.",
      ),
      check("subtitles-consent", "I agree to send microphone audio to ElevenLabs, and final lines to my AI provider " +
        "when I choose languages to translate into", consent),
      h("ul", { id: "subtitles-consent-text", class: "consent-points" }),
      ...switchRow("subtitles-enabled", "Show subtitles", ""),
      preview(),
      ...switchRow("subtitles-labels", "Name each language on the overlay",
        "A small label beside each subtitle bar, such as English (detected) or Japanese; window themes title the bar " +
          "with it instead."),
      ...switchRow("subtitles-stream", "Send them to the stream as closed captions", ""),
      field("subtitles-stream-language", "Caption language", stream),
      h("p", { id: "subtitles-stream-note", class: "hint" }),
      h(
        "details",
        { id: "subtitles-details", class: "ai-details" },
        h("summary", {}, "Languages and lines"),
        settingsForm(),
      ),
    );
  }

  /**
   * Wires the target pickers, each a list, the code field "Another language code" shows, and, for a language the
   * overlay writes in a script of its own, a box to keep that script. Returns `show(codes, latin)`, which shows saved
   * codes, a listed one picked by name and any other typed, and ticks each script unless its code is in `latin`;
   * `codes()`, what is picked or typed in each; `latinTargets()`, the picked languages kept in Latin letters;
   * `control(index)`, the control holding a target's code, where a problem is marked; and `controls()`.
   */
  function targetPickers(byId) {
    const pickers = Array.from({ length: MAX_TARGETS }, (_, index) => {
      const id = `subtitles-target-${index + 1}`;
      const [script, scriptField, scriptLabel] = ["script", "script-field", "script-label"].map((part) =>
        byId(`${id}-${part}`));
      return { select: byId(id), code: byId(`${id}-code`), field: byId(`${id}-code-field`), script, scriptField,
        scriptLabel };
    });
    const typed = (picker) => picker.select.value === OTHER_LANGUAGE;
    const scripted = (picker) => (typed(picker) ? null : scriptName(picker.select.value));
    /** Shows the script box of a picked language that has a script, named for it. */
    const showScript = (picker) => {
      const script = scripted(picker);
      picker.scriptField.hidden = !script;
      picker.scriptLabel.textContent = script ? `Write ${languageName(picker.select.value)} in ${script}` : "";
    };
    // Arrow keys on a closed list fire change as they pass each option, so choosing only reveals the code field,
    // next after the list in tab order, and never moves the focus. A language picked anew starts in its script.
    for (const picker of pickers) {
      picker.select.addEventListener("change", () => {
        picker.field.hidden = !typed(picker);
        picker.script.checked = true;
        showScript(picker);
      });
    }
    return {
      show(codes, latin = []) {
        pickers.forEach((picker, index) => {
          const code = String(codes[index] || "").trim();
          const listed = listedCode(code);
          picker.select.value = code && !listed ? OTHER_LANGUAGE : listed || "";
          picker.code.value = code && !listed ? code : "";
          picker.field.hidden = !typed(picker);
          picker.script.checked = !latin.some((kept) => kept.toLowerCase() === code.toLowerCase());
          showScript(picker);
        });
      },
      codes: () => pickers.map((picker) => (typed(picker) ? picker.code.value : picker.select.value).trim()),
      latinTargets: () =>
        pickers.filter((picker) => scripted(picker) && !picker.script.checked).map((picker) => picker.select.value),
      control: (index) => (typed(pickers[index]) ? pickers[index].code : pickers[index].select),
      controls: () => pickers.flatMap((picker) => [picker.select, picker.code]),
    };
  }

  Editor.subtitlesFields = { buildSubtitlesSection, targetPickers };
})();
