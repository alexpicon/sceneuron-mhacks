// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: subtitle words and checks: the codes the server accepts, the spoken languages to choose from,
   what subtitles send and where, which settings a form changed, the subtitles chip, the preview, which final
   segments a screen reader hears, and what stream captions can carry. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const { dataUse, destination } = inNode ? require("./consent-text.js") : window.SceneuronEditor.consentText;
  const { constructed, hears, language, languageLabel, languageName, spokenLanguages, validTag } = inNode
    ? require("./language-text.js")
    : window.SceneuronEditor.languageText;

  const MAX_TARGETS = 3;
  const SPOKEN = /^[a-z]{2,3}$/;
  // The target pickers' choice that shows a field to type a code into; never a code, whose language has 2 or 3 letters.
  const OTHER_LANGUAGE = "other";

  /** Why the server would refuse these targets, or null when it accepts them. */
  function targetsProblem(targets) {
    if (targets.length > MAX_TARGETS) {
      return `Choose at most ${MAX_TARGETS} languages.`;
    }
    const bad = targets.find((code) => !validTag(code));
    if (bad !== undefined) {
      return `"${bad}" is not a language code such as es, pt-BR, zh-Hans, or art-x-dothraki.`;
    }
    const twice = targets.find((code, index) =>
      targets.slice(0, index).some((other) => other.toLowerCase() === code.toLowerCase()));
    return twice === undefined ? null : `${twice} is listed twice.`;
  }

  /** Why the server would refuse this spoken language, or null; empty means it is detected. */
  function spokenProblem(code) {
    const value = String(code || "").trim();
    if (value !== "" && !SPOKEN.test(value)) {
      return "Use an ISO 639 code such as en or spa, or leave it empty.";
    }
    return value !== "" && constructed(value)
      ? `Speech to text does not hear ${languageLabel(value)}; choose it as a language to translate into instead.`
      : null;
  }

  /**
   * The spoken languages to choose from, as [code, label]: detecting it, then those Scribe hears by name, and a saved
   * code Scribe's list lacks, such as one typed before, so the field keeps it until another is chosen.
   */
  function spokenOptions(saved) {
    const code = String(saved || "").trim();
    const kept = code && !hears(code) ? [[code, languageLabel(code)]] : [];
    return [["", "Detect it automatically"]].concat(kept, spokenLanguages());
  }

  function dollars(value) {
    const amount = Number(value) || 0;
    return `$${amount < 0.1 ? amount.toFixed(3).replace(/0$/, "") : amount.toFixed(2)}`;
  }

  /** Exactly what subtitles send, where, and when, as `[label, text]` points; translations go where `ai` sends. */
  function subtitleConsentPoints(subtitles, ai) {
    const settings = subtitles || {};
    const targets = Array.isArray(settings.targets) ? settings.targets : [];
    const listening = ai && ai.voice && ai.voice.stt_price_per_hour
      ? `Listening costs about ${dollars(ai.voice.stt_price_per_hour)} an hour`
      : "Listening costs what ElevenLabs charges";
    const translating = settings.translation_price_known
      ? `, translations ${dollars(settings.translation_input_price_per_mtok)} per million tokens read and ` +
        `${dollars(settings.translation_output_price_per_mtok)} written`
      : ", and translations at a price Sceneuron does not know, which count as $0";
    const where = destination(ai);
    const google = dataUse(ai, "what is sent to Gemini");
    return [
      ["What", "Audio from your microphone, all the time while subtitles are on, even with voice control off."],
      ["Where", "api.elevenlabs.io (ElevenLabs), which turns it into text."],
      [
        "Translations",
        targets.length
          ? `Each final line, never words still being said, goes to ${where} with ` +
            `${settings.translation_model}, with the three lines before it, to be translated into ` +
            `${targets.map(languageName).join(", ")}.`
          : "None until you choose languages below; until then no text leaves this computer.",
      ],
      ...(google ? [["Google", google]] : []),
      ["Shown", "Subtitles appear on your overlay and in this editor, and are never written to the event log."],
      ["Cost", `${listening}${targets.length ? translating : ""}; all of it counts toward the hourly budget.`],
    ];
  }

  /** The "Show subtitles" switch as `{ checked, disabled, note }`: it needs consent first. */
  function subtitleSwitch(subtitles) {
    const consent = Boolean(subtitles && subtitles.consent);
    return {
      checked: consent && Boolean(subtitles.enabled),
      disabled: !consent,
      note: consent
        ? "Turn this off at any time; no audio is sent while it is off."
        : "Tick the consent box above first.",
    };
  }

  /**
   * The PUT /api/ai body for the subtitles form: the fields whose values differ from `subtitles`. `values` holds
   * text from the form: `language`, `targets` and `latin_targets` (lists), `translation_model`, `lines`,
   * `clear_after_s`, `hide_commands` and `latin_lines` (booleans), and `stream_language`. Returns null when nothing
   * changed.
   */
  function subtitleUpdate(subtitles, values) {
    const current = subtitles || {};
    const changed = {};
    for (const field of ["language", "translation_model", "stream_language"]) {
      const value = String(values[field] === undefined ? current[field] : values[field]).trim();
      if (value !== current[field]) {
        changed[field] = value;
      }
    }
    for (const field of ["targets", "latin_targets"]) {
      const codes = (values[field] || current[field] || []).map((code) => String(code).trim()).filter(Boolean);
      if (JSON.stringify(codes) !== JSON.stringify(current[field] || [])) {
        changed[field] = codes;
      }
    }
    const lines = Number(values.lines);
    if (Number.isInteger(lines) && lines !== current.lines) {
      changed.lines = lines;
    }
    const clear = Math.round(Number(values.clear_after_s) * 1000);
    if (Number.isFinite(clear) && clear !== current.clear_after_ms) {
      changed.clear_after_ms = clear;
    }
    for (const field of ["hide_commands", "latin_lines"]) {
      if (typeof values[field] === "boolean" && values[field] !== current[field]) {
        changed[field] = values[field];
      }
    }
    return Object.keys(changed).length ? { subtitles: changed } : null;
  }

  /** The subtitles chip from the live status; the state is also a CSS hook. */
  function subtitleState(view) {
    const state = view && view.status ? view.status.state : "off";
    const chips = {
      listening: ["watching", "Listening"],
      connecting: ["connecting", "Connecting"],
      retrying: ["connecting", "Retrying"],
      paused: ["paused", "Paused"],
      stopped: ["failed", "Stopped"],
      needs_key: ["needs_setup", "Needs a key"],
      needs_consent: ["needs_setup", "Needs consent"],
    };
    const [chip, label] = chips[state] || ["off", "Off"];
    return { state: chip, label };
  }

  /**
   * What the preview shows: the final lines, the words still being said, and each target's lines, or why there is
   * nothing to show. Partial words are drawn apart from the final lines, and only final segments are announced.
   */
  function previewText(view) {
    const lines = view && Array.isArray(view.lines) ? view.lines : [];
    const live = view && view.live ? String(view.live) : "";
    const status = (view && view.status) || {};
    const translations = (status.targets || []).map((code) => {
      const shown = view.translations && Array.isArray(view.translations[code]) ? view.translations[code] : [];
      return { code, name: languageName(code), lines: shown };
    });
    let note = "";
    if (!status.enabled) {
      note = status.state === "off" || !status.message ? "Subtitles are off." : `Subtitles are off: ${status.message}.`;
    } else if (!lines.length && !live) {
      note = "Waiting for speech.";
    }
    return { lines, live, translations, note };
  }

  /**
   * What a screen reader should hear about the preview: each new final segment once and whole, however it wraps and
   * even when it repeats the one before, never words still being said or segments that were there when the editor
   * opened. `seen` is the ids of the final segments last seen, undefined before the first live view. Returns
   * `{ seen, message }`, `message` being null when nothing is said.
   */
  function finalAnnouncement(seen, view) {
    if (!view) {
      return { seen, message: null };
    }
    const finals = Array.isArray(view.finals) ? view.finals : [];
    const fresh = seen === undefined ? [] : finals.filter((segment) => !seen.includes(segment.id));
    const message = fresh.length ? `Subtitle: ${fresh.map((segment) => segment.text).join(" ")}` : null;
    return { seen: finals.map((segment) => segment.id), message };
  }

  /** The stream caption languages to choose from: the spoken one, then each target. */
  function streamOptions(subtitles) {
    const targets = (subtitles && subtitles.targets) || [];
    return [["", "The spoken language"]].concat(targets.map((code) => [code, languageLabel(code)]));
  }

  /**
   * What stream captions in the chosen language can carry, and how viewers see them. A constructed or fictional
   * language is translated in Latin script, so its captions carry it, without the letters CEA-608 lacks.
   */
  function captionNote(subtitles) {
    const code = (subtitles && subtitles.stream_language) || (subtitles && subtitles.language) || "";
    const { name, captions, letter } = language(code) || { name: code };
    const base = "Captions use CEA-608, which carries Latin-script text only; viewers turn captions on in their " +
      "player. They go out only while you stream.";
    if (constructed(code)) {
      const latin = `${base} ${name} is translated in Latin script`;
      return captions === "some"
        ? `${latin}, so captions carry it without some letters, such as ${letter}.`
        : `${latin}, so captions carry ${captions === "all" ? "all of it" : "it"}.`;
    }
    if (captions === "none") {
      return `${base} ${name} uses letters CEA-608 cannot show, so most of its captions would be blank.`;
    }
    if (captions === "some") {
      return `${base} In ${name}, some letters, such as ${letter}, are left out of captions.`;
    }
    return base;
  }

  const subtitlesText = {
    MAX_TARGETS,
    OTHER_LANGUAGE,
    captionNote,
    finalAnnouncement,
    languageName,
    previewText,
    spokenOptions,
    spokenProblem,
    streamOptions,
    subtitleConsentPoints,
    subtitleState,
    subtitleSwitch,
    subtitleUpdate,
    targetsProblem,
  };
  if (inNode) {
    module.exports = subtitlesText;
  } else {
    window.SceneuronEditor.subtitlesText = subtitlesText;
  }
})();
