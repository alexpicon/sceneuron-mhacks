// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: what can be said to Sceneuron, as the Voice section lists it from GET /api/voice/guide, which the
   server builds from the tables its matcher reads, and the check a typed command must pass. */
(function () {
  "use strict";

  const MAX_COMMAND_CHARS = 200;

  /** How words are said in the guide's mode: addressed ones start with the wake word while voice listens always. */
  function spoken(say, guide, addressed) {
    return addressed && guide.mode === "always_on" ? `"${guide.wake_word}, ${say}"` : `"${say}"`;
  }

  /** When a group's words work in the guide's mode. */
  function groupNote(group, guide) {
    if (!group.addressed) {
      return guide.mode === "always_on"
        ? "Anywhere in what you say, without the wake word."
        : "Only while voice control listens always, after the wake word or not.";
    }
    return guide.mode === "always_on"
      ? `Start with "${guide.wake_word}", or type them below.`
      : "Hold to talk while you say them, or type them below.";
  }

  /**
   * The guide as sections for the page: `{ title, note, lines }`, each line `{ say, does }` with the ways to say it
   * quoted as they are said now. A line without ways says what to do first.
   */
  function guideSections(guide) {
    if (!guide || !Array.isArray(guide.groups)) {
      return [];
    }
    return guide.groups.map((group) => ({
      title: group.title,
      note: groupNote(group, guide),
      lines: group.entries.map((entry) => ({
        say: entry.say.map((way) => spoken(way, guide, group.addressed)),
        does: entry.does,
      })),
    }));
  }

  /** Why the server would refuse this typed command, or null. */
  function commandProblem(text) {
    const count = Array.from(String(text || "").trim()).length;
    return count >= 1 && count <= MAX_COMMAND_CHARS ? null : `Type 1 to ${MAX_COMMAND_CHARS} characters.`;
  }

  const voiceGuideText = { MAX_COMMAND_CHARS, commandProblem, guideSections };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = voiceGuideText;
  } else {
    window.SceneuronEditor.voiceGuideText = voiceGuideText;
  }
})();
