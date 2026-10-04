// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: voice words and checks: the ElevenLabs key's state, what leaves the computer, which voice
   settings changed, the live heard line, and trigger phrases and speech lines as the server accepts them. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);

  /** Voice settings the editor's form edits; the key is handled apart because it is write-only. */
  const VOICE_FIELDS = ["wake_word", "language", "voice_id", "tts_model", "stt_model"];
  const MAX_PHRASES = 4;
  const PHRASE_CHARS = [2, 60];
  const MAX_SPEECH_CHARS = 200;
  // Alphabetic and numeric, as the server's char::is_alphanumeric reads them, so vowel signs of scripts such as
  // Devanagari or Thai count as letters.
  const PHRASE_CHAR = /^[\p{Alphabetic}\p{N} '’,.!?¿¡-]$/u;
  const WORDY = /[\p{Alphabetic}\p{N}]/u;

  /** What the ElevenLabs key field says; the key itself is never sent back. */
  function voiceKeyStatus(settings) {
    if (!settings || !settings.has_elevenlabs_key) {
      return "No ElevenLabs key; voice control and spoken cards stay off, and everything else works";
    }
    const hint = String(settings.elevenlabs_key_hint || "").replace(/^\.+/, "");
    const shown = hint ? ` (...${hint})` : "";
    if (settings.elevenlabs_key_source === "environment") {
      return `Key from .env or the environment${shown}; change it there`;
    }
    return `Key saved${shown}`;
  }

  /** Whether the editor may remove the ElevenLabs key: only one it saved itself. */
  function voiceKeyRemovable(settings) {
    return Boolean(settings && settings.has_elevenlabs_key && settings.elevenlabs_key_source !== "environment");
  }

  function dollars(value) {
    const amount = Number(value) || 0;
    return `$${amount < 0.1 ? amount.toFixed(3).replace(/0$/, "") : amount.toFixed(2)}`;
  }

  /** What voice spend is counted at, and which models have no list price and count at the highest known one. */
  function voiceCost(voice) {
    const settings = voice || {};
    const listening = settings.stt_price_per_hour ? `about ${dollars(settings.stt_price_per_hour)} an hour` : "";
    const keyterms = settings.keyterm_price_per_hour ? `, ${dollars(settings.keyterm_price_per_hour)} more with keyterms` : "";
    const speaking = settings.tts_price_per_1k_chars
      ? `about ${dollars(settings.tts_price_per_1k_chars)} per 1,000 characters`
      : "";
    if (!listening || !speaking) {
      return "Voice spend counts toward the hourly budget.";
    }
    const unlisted = [
      [settings.stt_price_listed, settings.stt_model],
      [settings.tts_price_listed, settings.tts_model],
    ]
      .filter(([listed, model]) => listed === false && model)
      .map(([, model]) => model);
    const note = unlisted.length
      ? ` No list price is known for ${unlisted.join(" or ")}, so it counts at the highest known rate.`
      : "";
    return `Listening costs ${listening}${keyterms}, and spoken cards ${speaking}; both count toward the hourly ` +
      `budget.${note}`;
  }

  /** Exactly what voice features send, where, and when, as `[label, text]` points. */
  function voiceConsentPoints(voice) {
    const always = voice && voice.mode === "always_on";
    const when = always
      ? `all the time while voice control is on, so "${(voice && voice.wake_word) || "Sceneuron"}" and trigger ` +
        "phrases are heard"
      : "only while push to talk is held";
    return [
      ["What", `Audio from your microphone, ${when}.`],
      [
        "Keyterms",
        "With it go the wake word and the names and trigger phrases of cards that voice can raise, so they are " +
          "recognized.",
      ],
      ["Where", "api.elevenlabs.io (ElevenLabs), which turns it into text."],
      [
        "Spoken cards",
        "When they are on, the words of a card raised on a deck that speaks, once per voice; the audio is kept " +
          "on this computer.",
      ],
      [
        "Commands",
        "Words you say to Sceneuron that it does not know by itself go as text, with your project, to the AI " +
          "provider in this tab, once it has a key.",
      ],
      ["Cost", voiceCost(voice)],
      ["Key", "Your ElevenLabs key stays on this computer and is only sent to ElevenLabs."],
    ];
  }

  /** The "Listen for voice commands" switch as `{ checked, disabled, note }`: it needs consent first. */
  function listenSwitch(voice) {
    const consent = Boolean(voice && voice.consent);
    return {
      checked: consent && Boolean(voice.enabled),
      disabled: !consent,
      note: consent
        ? "Turn this off at any time; no audio is sent while it is off."
        : "Tick the consent box above first.",
    };
  }

  /**
   * The PUT /api/ai body for the voice form: the voice fields whose values differ from `voice`, plus the typed
   * key when there is one. Returns null when nothing changed.
   */
  function voiceUpdate(voice, values) {
    const changed = {};
    for (const field of VOICE_FIELDS) {
      const value = typeof values[field] === "string" ? values[field].trim() : undefined;
      if (value !== undefined && value !== (voice || {})[field]) {
        changed[field] = value;
      }
    }
    const update = Object.keys(changed).length ? { voice: changed } : {};
    const key = typeof values.elevenlabs_api_key === "string" ? values.elevenlabs_api_key.trim() : "";
    if (key) {
      update.elevenlabs_api_key = key;
    }
    return Object.keys(update).length ? update : null;
  }

  /** The voice chip: off, listening, push to talk held, or ready; the state is also a CSS hook. */
  function voiceState(view) {
    if (!view || !view.enabled) {
      return { state: "off", label: "Off" };
    }
    if (view.talking) {
      return { state: "thinking", label: "Talking" };
    }
    return view.listening ? { state: "watching", label: "Listening" } : { state: "ready", label: "Ready" };
  }

  /**
   * The live heard line, such as `Heard "Sceneuron, wrap it up": raised Meeting signals: Wrap it up.`, or `Typed`
   * for words typed in the editor. While the AI works on the words, the result says so, and it changes once the AI
   * answers.
   */
  function heardLine(view) {
    const heard = view && view.heard;
    if (!heard || !heard.text) {
      return "Nothing heard yet.";
    }
    if (!heard.is_final) {
      return `Hearing "${heard.text}"...`;
    }
    const verb = heard.typed ? "Typed" : "Heard";
    return heard.result ? `${verb} "${heard.text}": ${heard.result}.` : `${verb} "${heard.text}".`;
  }

  /**
   * What a screen reader should hear about the heard line: each final result once, the AI's later answer included,
   * and nothing for partial words or for what was already there when the editor opened. `seen` identifies the last
   * final result, and is undefined before the first live view. Returns `{ seen, message }`, `message` being null
   * when nothing is said.
   */
  function heardAnnouncement(seen, view) {
    const heard = view && view.heard;
    const final = heard && heard.is_final ? `${heard.at_ms} ${heard.text} ${heard.result}` : null;
    if (seen === undefined || final === null || final === seen) {
      return { seen: seen === undefined || final !== null ? final : seen, message: null };
    }
    return { seen: final, message: heardLine(view) };
  }

  /** Trigger phrases from a text field, one per line, without blank lines. */
  function phrasesFrom(text) {
    return String(text || "")
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
  }

  /** Why the server would refuse these trigger phrases, or null when it accepts them. */
  function phrasesProblem(phrases) {
    if (phrases.length > MAX_PHRASES) {
      return `Use at most ${MAX_PHRASES} trigger phrases, one per line.`;
    }
    const [min, max] = PHRASE_CHARS;
    const bad = phrases.find((phrase) => {
      const chars = Array.from(phrase);
      return chars.length < min || chars.length > max || !chars.every((c) => PHRASE_CHAR.test(c)) ||
        !WORDY.test(phrase);
    });
    return bad === undefined
      ? null
      : `"${bad}" must be ${min} to ${max} letters, digits, spaces, apostrophes, or basic punctuation.`;
  }

  /** Why the server would refuse this speech line, or null; an empty line means the card says its text. */
  function speechProblem(text) {
    const value = String(text || "");
    // Control characters as the server's char::is_control reads them, the C1 range included.
    if (Array.from(value.trim()).length > MAX_SPEECH_CHARS || /\p{Cc}/u.test(value)) {
      return `Use at most ${MAX_SPEECH_CHARS} characters on one line.`;
    }
    return null;
  }

  const voiceText = {
    heardAnnouncement,
    heardLine,
    listenSwitch,
    phrasesFrom,
    phrasesProblem,
    speechProblem,
    voiceConsentPoints,
    voiceKeyRemovable,
    voiceKeyStatus,
    voiceState,
    voiceUpdate,
  };
  if (inNode) {
    module.exports = voiceText;
  } else {
    window.SceneuronEditor.voiceText = voiceText;
  }
})();
