// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the settings file, one JSON file with the project (layout, rules, conditions, decks, and the
   project settings) and the AI, voice, and subtitle settings, to save, share, and load. API keys and consents
   never go in it: keys are secrets, and consent belongs to whoever runs Sceneuron on their own computer. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Editor = inNode ? {} : window.SceneuronEditor;

  const FORMAT = "sceneuron-settings";
  const VERSION = 1;
  /** The AI settings a file carries; each is a field of both GET /api/ai and PUT /api/ai. */
  const AI_FIELDS = [
    "enabled",
    "provider",
    "base_url",
    "vision_model",
    "planner_model",
    "vision_effort",
    "planner_effort",
    "image_width",
    "input_price_per_mtok",
    "output_price_per_mtok",
    "jev_base_url",
    "jev_model",
  ];
  const VOICE_FIELDS = ["enabled", "mode", "wake_word", "language", "speak", "voice_id", "tts_model", "stt_model"];
  const SUBTITLE_FIELDS = [
    "enabled",
    "language",
    "targets",
    "translation_model",
    "lines",
    "clear_after_ms",
    "hide_commands",
    "stream_captions",
    "stream_language",
    "labels",
    "latin_targets",
    "latin_lines",
  ];

  const isObject = (value) => Boolean(value) && typeof value === "object" && !Array.isArray(value);

  function pick(source, fields) {
    const out = {};
    for (const field of fields) {
      if (isObject(source) && field in source) {
        out[field] = source[field];
      }
    }
    return out;
  }

  /** The AI settings update a file carries, from GET /api/ai's answer, without keys or consents. */
  function aiPart(view) {
    const ai = pick(view, AI_FIELDS);
    ai.voice = pick(view && view.voice, VOICE_FIELDS);
    ai.subtitles = pick(view && view.subtitles, SUBTITLE_FIELDS);
    return ai;
  }

  /** The settings file for `project` (GET /api/project) and `aiView` (GET /api/ai), saved at `savedAt`. */
  function build(project, aiView, savedAt) {
    return { format: FORMAT, version: VERSION, saved_at: savedAt, project, ai: aiPart(aiView) };
  }

  /** The project and the AI settings update in a file's `text`; throws an Error that says what is wrong. */
  function parse(text) {
    let file;
    try {
      file = JSON.parse(text);
    } catch {
      throw new Error("This file is not JSON. Choose a Sceneuron settings file.");
    }
    if (!isObject(file) || file.format !== FORMAT) {
      throw new Error("This file is not a Sceneuron settings file.");
    }
    if (file.version !== VERSION) {
      throw new Error(`This settings file is version ${file.version}; this Sceneuron reads version ${VERSION}.`);
    }
    if (!isObject(file.project) || !Array.isArray(file.project.elements)) {
      throw new Error("This settings file has no project.");
    }
    const ai = isObject(file.ai) ? aiPart(file.ai) : null;
    return { project: file.project, ai };
  }

  /** A file name for a settings file saved at `date`, such as sceneuron-settings-2026-10-03.json. */
  function fileName(date) {
    return `${FORMAT}-${date.toISOString().slice(0, 10)}.json`;
  }

  const settingsFile = { FORMAT, VERSION, build, fileName, parse };
  if (inNode) {
    module.exports = settingsFile;
  } else {
    Editor.settingsFile = settingsFile;
  }
})();
