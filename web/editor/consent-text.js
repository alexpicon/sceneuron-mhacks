// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: consent words: where the AI settings send data, what Google may do with it on a free-tier
   Gemini key, the consent wording for scene understanding, and the switches and choices that need consent. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const { formatSeconds } = inNode ? require("./ai-text.js") : window.SceneuronEditor.aiText;

  const PROVIDERS = {
    gemini: "Google Gemini",
    "openai-compatible": "an OpenAI-compatible service",
  };
  /** The Gemini API's own address, which Google's Gemini API terms cover whichever provider speaks to it. */
  const GEMINI_API_HOST = "generativelanguage.googleapis.com";

  function hostOf(url) {
    try {
      return new URL(url).host;
    } catch {
      return String(url || "the configured address");
    }
  }

  function hostnameOf(url) {
    try {
      return new URL(url).hostname;
    } catch {
      return "";
    }
  }

  function isLocalHost(host) {
    return /^(localhost|127\.|\[::1\]|0\.0\.0\.0)/.test(host);
  }

  /**
   * Where the AI settings send data: the host and the provider, such as
   * "generativelanguage.googleapis.com (Google Gemini)".
   */
  function destination(settings) {
    const host = hostOf(settings && settings.base_url);
    const provider = PROVIDERS[settings && settings.provider] || "the AI provider";
    return isLocalHost(host) ? `${host}, a model on your own computer` : `${host} (${provider})`;
  }

  /**
   * What Google may do with `sent` under the Gemini API terms, or "" unless the AI settings send to the Gemini API's
   * own address, through the Gemini provider or Google's OpenAI-compatible endpoint. Which applies depends on the
   * key's Google project, which Sceneuron cannot see, so both are named.
   */
  function dataUse(settings, sent = "what is sent") {
    const google = hostnameOf(settings && settings.base_url) === GEMINI_API_HOST;
    return google
      ? `With a free-tier key (a Google project without billing), Google may use ${sent} to improve its products, ` +
          "and people may review it. A key from a project with billing is not used that way."
      : "";
  }

  /** Exactly what turning on scene understanding sends, where, and how often, as `[label, text]` points. */
  function consentPoints(settings, policy) {
    const every = formatSeconds(policy ? policy.min_interval_ms : 2500);
    const refresh = formatSeconds(policy ? policy.refresh_ms : 20000);
    const width = settings && settings.image_width ? `${settings.image_width} pixels wide` : "small";
    const google = dataUse(settings, "what is sent, camera images included,");
    return [
      ["What", `A still image from your camera (${width}) and the conditions you wrote.`],
      ["Where", `${destination(settings)}.`],
      ...(google ? [["Google", google]] : []),
      [
        "When",
        `Only while "Watch the camera with AI" is on: at most one image every ${every}, when the picture ` +
        `changes or ${refresh} have passed.`,
      ],
      [
        "Assistant",
        'Your request and project, plus one camera image only when you tick "Use the camera image".',
      ],
      ["Key", "Your API key stays on this computer and is only sent to that address."],
    ];
  }

  /** The "Watch the camera with AI" switch as `{ checked, disabled, note }`: it needs consent first. */
  function watchSwitch(settings) {
    const consent = Boolean(settings && settings.consent);
    return {
      checked: consent && Boolean(settings.enabled),
      disabled: !consent,
      note: consent
        ? "Turn this off at any time; nothing is sent while it is off."
        : "Tick the consent box above first.",
    };
  }

  /** The update the consent checkbox sends; withdrawing consent also stops watching. */
  function consentUpdate(checked) {
    return checked ? { consent: true } : { consent: false, enabled: false };
  }

  /**
   * The assistant's "Use the camera image" choice as `{ disabled, note }`: it needs consent too. Plans go to the
   * provider with or without it, so the note always says what Google may do with them.
   */
  function cameraChoice(settings) {
    const consent = Boolean(settings && settings.consent);
    const note = consent
      ? `Sends one still image with your request. ${dataUse(settings)}`
      : `Needs your consent in the AI tab. Plans work without it. ${dataUse(settings, "your request and project")}`;
    return { disabled: !consent, note: note.trim() };
  }

  const consentText = {
    cameraChoice,
    consentPoints,
    consentUpdate,
    dataUse,
    destination,
    watchSwitch,
  };
  if (inNode) {
    module.exports = consentText;
  } else {
    window.SceneuronEditor.consentText = consentText;
  }
})();
