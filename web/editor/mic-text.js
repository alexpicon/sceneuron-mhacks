// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: microphone words: what each OBS source that carries audio is, which one voice control and
   subtitles hear through the Sceneuron Voice filter, and where to choose one. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);

  const IN_OBS =
    "or in OBS open the Audio Mixer, choose Filters on your microphone, and add the Sceneuron Voice filter.";

  /** Names joined as in a sentence: "A", "A and B", or "A, B, and C". */
  function listNames(names) {
    if (names.length < 3) {
      return names.join(" and ");
    }
    return `${names.slice(0, -1).join(", ")}, and ${names.at(-1)}`;
  }

  /** What a source in the list is: a global audio device from OBS's Settings > Audio, or its source type. */
  function micKind(source) {
    return source.global ? "Global audio device" : String(source.kind || "");
  }

  /**
   * Which microphone voice control and subtitles hear, from the /api/audio-sources reply: `state` is unknown (not
   * loaded), unavailable (outside OBS), none, one, or several; `uuid` is the source heard, when it is known; and
   * `text` says it in a sentence or two.
   */
  function micSummary(reply) {
    if (!reply) {
      return { state: "unknown", uuid: null, text: "" };
    }
    if (!reply.available) {
      const text = "Sceneuron is not running inside OBS, so there is no microphone to listen to.";
      return { state: "unavailable", uuid: null, text };
    }
    const voiced = (Array.isArray(reply.sources) ? reply.sources : []).filter((source) => source.voice);
    if (voiced.length === 0) {
      const text = "No microphone has the Sceneuron Voice filter yet, so voice control and subtitles hear nothing.";
      return { state: "none", uuid: null, text };
    }
    const used = voiced.length === 1 ? voiced[0] : voiced.find((source) => source.in_use);
    if (voiced.length === 1) {
      const text = `Sceneuron listens to ${used.name} for voice control and subtitles.`;
      return { state: "one", uuid: used.uuid, text };
    }
    if (!used) {
      const names = listNames(voiced.map((source) => source.name));
      const text = `${names} have the Sceneuron Voice filter. Sceneuron hears one microphone at a time: the first of ` +
        "them to send audio once voice control or subtitles start.";
      return { state: "several", uuid: null, text };
    }
    const others = voiced.filter((source) => source !== used).map((source) => source.name);
    const verb = others.length === 1 ? "has" : "have";
    const text = `Sceneuron listens to ${used.name} for voice control and subtitles. ${listNames(others)} also ` +
      `${verb} the Sceneuron Voice filter, which is not used while ${used.name} has it.`;
    return { state: "several", uuid: used.uuid, text };
  }

  /**
   * What to do next, after the summary: `place` is "tab" for the Camera & mic tab, where the list is, and "ai" for
   * the AI tab's Voice and Subtitles sections.
   */
  function micAdvice(state, place) {
    const here = place === "tab";
    if (state === "none") {
      return here ? "Choose your microphone below." : `Choose your microphone in the Camera & mic tab, ${IN_OBS}`;
    }
    if (state === "several") {
      return here
        ? "Choose one below to keep the filter on that one only."
        : "Choose one in the Camera & mic tab to keep the filter on that one only.";
    }
    return "";
  }

  /** The summary and the advice as one text. */
  function micNote(reply, place) {
    const summary = micSummary(reply);
    return [summary.text, micAdvice(summary.state, place)].filter(Boolean).join(" ");
  }

  const micText = { listNames, micAdvice, micKind, micNote, micSummary };
  if (inNode) {
    module.exports = micText;
  } else {
    window.SceneuronEditor.micText = micText;
  }
})();
