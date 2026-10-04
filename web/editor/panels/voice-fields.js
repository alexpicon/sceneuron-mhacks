// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of the AI tab's Voice section: its status and live heard line, the microphone it hears,
   the ElevenLabs key, consent to send microphone audio, voice control and push to talk, a typed command and what can
   be said, spoken cards, and the voice settings. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { check, field, h, option } = Editor.dom;

  const SIMULATED = "Demo mode: voice is simulated. Letting go of Hold to talk hears a scripted command, spoken cards " +
    "play a chime, and nothing leaves this computer.";

  const stat = (id, term, text) => h("div", {}, h("dt", {}, term), h("dd", { id }, text));

  function text(id, label, setting, extra) {
    const input = h(
      "input",
      Object.assign({ id, type: "text", "data-voice-setting": setting, autocomplete: "off", spellcheck: "false" }, extra),
    );
    return field(id, label, input);
  }

  function switchRow(id, label, note) {
    const input = h("input", { id, type: "checkbox", role: "switch", "aria-describedby": `${id}-note` });
    return [
      h("div", { class: "check switch" }, input, h("label", { for: id }, label)),
      h("p", { id: `${id}-note`, class: "hint" }, note),
    ];
  }

  function settingsForm() {
    const key = h("input", {
      id: "voice-key",
      type: "password",
      autocomplete: "off",
      spellcheck: "false",
      "aria-describedby": "voice-key-status",
    });
    const remove = h("button", { id: "voice-remove-key", type: "button", class: "danger" }, "Remove key");
    return h(
      "form",
      { id: "voice-form", class: "form", novalidate: true },
      h(
        "div",
        { class: "field" },
        h("label", { for: "voice-key" }, "ElevenLabs key"),
        h("div", { class: "inline-field" }, key, remove),
        h("p", { id: "voice-key-status", class: "hint" }, "No ElevenLabs key"),
      ),
      h(
        "div",
        { class: "grid-2" },
        text("voice-wake-word", "Wake word", "wake_word", { maxlength: 30 }),
        text("voice-language", "Language code (empty to detect)", "language", { maxlength: 3 }),
      ),
      text("voice-voice-id", "Voice id", "voice_id", { maxlength: 64 }),
      h(
        "div",
        { class: "grid-2" },
        text("voice-tts-model", "Speech model", "tts_model", { maxlength: 100 }),
        text("voice-stt-model", "Listening model", "stt_model", { maxlength: 100 }),
      ),
      h(
        "p",
        { class: "hint" },
        "Any voice from your ElevenLabs voice library works; copy its id from the ElevenLabs website.",
      ),
      h(
        "div",
        { class: "button-row" },
        h("button", { id: "voice-save", type: "submit", class: "primary" }, "Save voice settings"),
      ),
    );
  }

  /** Typed words run as a command, and the list of what can be said, which voice-guide.js fills. */
  function commandParts() {
    const input = h("input", {
      id: "voice-command-text",
      type: "text",
      maxlength: 200,
      autocomplete: "off",
      spellcheck: "false",
      placeholder: "mind blown",
      "aria-describedby": "voice-command-hint",
    });
    return [
      h(
        "form",
        { id: "voice-command-form", class: "voice-command", novalidate: true },
        h("label", { for: "voice-command-text" }, "Type a command"),
        h("div", { class: "inline-field" }, input, h("button", { id: "voice-command-run", type: "submit" }, "Run")),
        h(
          "p",
          { id: "voice-command-hint", class: "hint" },
          "Runs as if said with push to talk held, even while voice control is off; the line above says what it did.",
        ),
        h("p", { id: "voice-command-error", class: "field-error", role: "alert", hidden: true }),
      ),
      h(
        "details",
        { id: "voice-guide", class: "ai-details" },
        h("summary", {}, "What you can say"),
        h("div", { id: "voice-guide-list", class: "voice-guide" }),
      ),
    ];
  }

  /** The Voice section, hidden until the AI settings say voice is available. */
  function buildVoiceSection() {
    const consent = h("input", { id: "voice-consent", type: "checkbox", "aria-describedby": "voice-consent-text" });
    const modes = [["push_to_talk", "Push to talk"], ["always_on", "Always on, after the wake word"]];
    const mode = h("select", { id: "voice-mode" }, ...modes.map(([value, label]) => option(value, label)));
    const talk = h(
      "button",
      { id: "voice-talk", type: "button", class: "talk", "aria-pressed": "false", "aria-describedby": "voice-talk-hint" },
      "Hold to talk",
    );
    return h(
      "section",
      { id: "voice", class: "ai-card", "aria-labelledby": "voice-heading", hidden: true },
      h(
        "div",
        { class: "section-head" },
        h("h3", { id: "voice-heading" }, "Voice"),
        h("span", { id: "voice-state", class: "chip", "data-state": "off" }, "Off"),
      ),
      h("p", { id: "voice-message", class: "hint" }),
      h("p", { id: "voice-simulated", class: "note", hidden: true }, SIMULATED),
      h(
        "div",
        { class: "mic-note" },
        h("p", { id: "voice-mic", class: "hint" }),
        h("button", { id: "voice-mic-choose", type: "button", hidden: true }, "Choose microphone"),
      ),
      check("voice-consent", "I agree to send microphone audio to ElevenLabs", consent),
      h("ul", { id: "voice-consent-text", class: "consent-points" }),
      ...switchRow("voice-enabled", "Listen for voice commands", ""),
      field("voice-mode", "When to listen", mode),
      h("div", { class: "button-row" }, talk),
      h(
        "p",
        { id: "voice-talk-hint", class: "hint" },
        "Hold the button, or Space while it has focus, and say a card, a reaction such as \"mind blown\", a " +
          "theme, \"lower\", \"pause\", or anything else for the AI. Control surfaces can hold it through the " +
          "control API.",
      ),
      // Partial words rewrite this line several times a second, so it is no live region; voice.js announces each
      // final result instead.
      h("p", { id: "voice-heard", class: "heard" }, "Nothing heard yet."),
      ...commandParts(),
      ...switchRow(
        "voice-speak",
        "Speak cards when raised",
        "Each deck chooses in the Cards tab. The overlay plays the voice, so it reaches OBS's audio.",
      ),
      h(
        "dl",
        { class: "ai-stats" },
        stat("voice-seconds", "Audio transcribed", "0 s"),
        stat("voice-characters", "Characters spoken", "0"),
        stat("voice-cost", "Voice spend", "$0.00"),
      ),
      h("details", { id: "voice-details", class: "ai-details" }, h("summary", {}, "ElevenLabs key and voice"), settingsForm()),
    );
  }

  Editor.voiceFields = { buildVoiceSection };
})();
