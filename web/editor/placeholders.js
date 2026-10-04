// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: placeholders in overlay text. "{{seen}}" shows the live description of what the camera sees,
   from on-device analysis, "{{scene}}" what the AI last saw and "{{scene:age}}" that with how long ago, "{{room}}" the
   shared room's votes, "{{subtitles}}" what the speaker says and "{{subtitles:es}}" or "{{subtitles:1}}" a
   translation of it, "{{on-air}}" OBS's scene and LIVE while it streams or records, "{{on-air-time}}" the time on
   air, which the renderer counts, and "{{condition-id}}" the detail the AI reported for that condition. The overlay
   renders unknown or empty ones as nothing, and leaves out a line of placeholders alone that all show nothing; the
   editor's preview shows a stand-in instead, so the layout stays readable. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const {
    ON_AIR_PLACEHOLDER,
    ON_AIR_TIME_PLACEHOLDER,
    ROOM_PLACEHOLDER,
    SCENE_PLACEHOLDER,
    SEEN_PLACEHOLDER,
    SUBTITLES_PLACEHOLDER,
  } = inNode ? require("./constants.js") : window.SceneuronEditor.constants;
  const SCENE_AGE = `${SCENE_PLACEHOLDER}:age`;

  /** Splits text into literal strings and placeholder names, scanning like the server does. */
  function parse(text) {
    const parts = [];
    let rest = String(text);
    for (;;) {
      const start = rest.indexOf("{{");
      const end = start < 0 ? -1 : rest.indexOf("}}", start + 2);
      if (end < 0) {
        parts.push({ text: rest });
        return parts;
      }
      parts.push({ text: rest.slice(0, start) }, { name: rest.slice(start + 2, end).trim() });
      rest = rest.slice(end + 2);
    }
  }

  /** The placeholder names in text, in order and without repeats. */
  function placeholderNames(text) {
    const names = parse(text).filter((part) => "name" in part).map((part) => part.name);
    return Array.from(new Set(names));
  }

  /**
   * Replaces every placeholder with `valueOf(name)`. A line of the text that holds placeholders and nothing else,
   * which all show nothing, is left out with its line break, as the server does.
   */
  function renderPlaceholders(text, valueOf) {
    if (!String(text).includes("{{")) {
      return String(text);
    }
    const newLine = () => ({ text: "", placeholders: false, shows: false });
    const lines = [newLine()];
    const add = (words, placeholder) => {
      const line = lines[lines.length - 1];
      line.text += words;
      line.placeholders = line.placeholders || placeholder;
      line.shows = line.shows || words.trim() !== "";
    };
    for (const part of parse(text)) {
      if ("name" in part) {
        add(String(valueOf(part.name)), true);
      } else {
        part.text.split("\n").forEach((words, index) => {
          if (index > 0) {
            lines.push(newLine());
          }
          add(words, false);
        });
      }
    }
    return lines.filter((line) => !line.placeholders || line.shows).map((line) => line.text).join("\n");
  }

  /** Whether a placeholder name asks for the scene caption with its age, as "scene:age" or "scene: age" do. */
  function isSceneAge(name) {
    const prefix = `${SCENE_PLACEHOLDER}:`;
    return name.startsWith(prefix) && name.slice(prefix.length).trim() === "age";
  }

  /** For a subtitles placeholder, "" for the spoken language or the target it names; null for any other name. */
  function subtitleTarget(name) {
    if (name === SUBTITLES_PLACEHOLDER) {
      return "";
    }
    const prefix = `${SUBTITLES_PLACEHOLDER}:`;
    return name.startsWith(prefix) ? name.slice(prefix.length).trim() : null;
  }

  /** Whether text shows subtitles, in any language, as the server marks such text in its snapshot. */
  function showsSubtitles(text) {
    return placeholderNames(text).some((name) => subtitleTarget(name) !== null);
  }

  /** Whether text names the placeholder `name`, as the server marks the on-air pill and timer. */
  function shows(text, name) {
    return placeholderNames(text).includes(name);
  }

  /** What "{{on-air}}" shows for a snapshot's on_air: the scene, then LIVE while OBS streams or records. */
  function onAirText(onAir) {
    const scene = onAir && typeof onAir.scene === "string" ? onAir.scene : "";
    const live = Boolean(onAir && Number.isFinite(onAir.since_ms));
    return live ? [scene, "LIVE"].filter(Boolean).join(" · ") : scene;
  }

  /** The live lines a subtitles placeholder shows, one per line, as the server renders them. */
  function subtitleLines(subtitles, target) {
    if (!subtitles) {
      return "";
    }
    if (target === "") {
      return (subtitles.lines || []).concat(subtitles.live ? [subtitles.live] : []).join("\n");
    }
    const targets = (subtitles.status && subtitles.status.targets) || [];
    const code = /^\d+$/.test(target)
      ? targets[Number(target) - 1]
      : targets.find((candidate) => candidate.toLowerCase() === target.toLowerCase());
    return code && subtitles.translations ? (subtitles.translations[code] || []).join("\n") : "";
  }

  /**
   * What the preview shows for each placeholder: the live description, caption, subtitles, or condition detail when
   * there is one, a bracketed stand-in naming it otherwise, and nothing for unknown names, as on stream.
   */
  function previewValues(draft, live) {
    const conditions = (draft && draft.conditions) || [];
    const views = new Map(((live && live.conditions) || []).map((view) => [view.id, view]));
    const caption = live && live.scene ? String(live.scene.caption || "") : "";
    const age = live && live.scene ? String(live.scene.age || "") : "";
    const seen = live && typeof live.seen === "string" ? live.seen : "";
    return (name) => {
      if (name === SEEN_PLACEHOLDER) {
        return seen || "[Live description]";
      }
      if (name === SCENE_PLACEHOLDER) {
        return caption || "[Scene description]";
      }
      if (isSceneAge(name)) {
        return caption ? `${caption} (${age || "just now"})` : "[Scene description (its age)]";
      }
      if (name === ROOM_PLACEHOLDER) {
        return "[Room votes]";
      }
      if (name === ON_AIR_PLACEHOLDER) {
        return onAirText(live && live.on_air) || "[Scene · LIVE]";
      }
      if (name === ON_AIR_TIME_PLACEHOLDER) {
        // The renderer fills in the time and counts it, as on stream.
        return `{{${ON_AIR_TIME_PLACEHOLDER}}}`;
      }
      const target = subtitleTarget(name);
      if (target !== null) {
        return subtitleLines(live && live.subtitles, target) || (target ? `[Subtitles: ${target}]` : "[Subtitles]");
      }
      const condition = conditions.find((candidate) => candidate.id === name);
      if (!condition) {
        return "";
      }
      const view = views.get(name);
      return view && view.detail ? view.detail : `[${condition.name}]`;
    };
  }

  /**
   * Placeholder names in text that are neither the live description, the scene with or without its age, the room,
   * subtitles, on air, nor a condition of the draft.
   */
  function unknownPlaceholders(text, draft) {
    const ids = new Set(((draft && draft.conditions) || []).map((condition) => condition.id));
    ids.add(SEEN_PLACEHOLDER).add(SCENE_PLACEHOLDER).add(ROOM_PLACEHOLDER);
    ids.add(ON_AIR_PLACEHOLDER).add(ON_AIR_TIME_PLACEHOLDER);
    const known = (name) => ids.has(name) || isSceneAge(name) || subtitleTarget(name) !== null;
    return placeholderNames(text).filter((name) => !known(name));
  }

  /** The placeholders a text can use, each with its token and plain-word name and meaning. */
  function availablePlaceholders(draft) {
    const seen = {
      token: `{{${SEEN_PLACEHOLDER}}}`,
      name: "Live description",
      meaning: "what the camera sees now, from on-device analysis: free, and nothing is sent",
    };
    const scene = {
      token: `{{${SCENE_PLACEHOLDER}}}`,
      name: "Scene description",
      meaning: "what the AI last saw, in one sentence",
    };
    const sceneAge = {
      token: `{{${SCENE_AGE}}}`,
      name: "Scene description with its age",
      meaning: "what the AI last saw and how long ago; each look costs money",
    };
    const room = {
      token: `{{${ROOM_PLACEHOLDER}}}`,
      name: "Room votes",
      meaning: "who voted in the shared room, then the revealed cards",
    };
    const conditions = ((draft && draft.conditions) || []).map((condition) => ({
      token: `{{${condition.id}}}`,
      name: condition.name,
      meaning: condition.detail ? `what it reports: ${condition.detail}` : "what it reports while it is on",
    }));
    const subtitles = [
      { token: `{{${SUBTITLES_PLACEHOLDER}}}`, name: "Subtitles", meaning: "what you say, live, in your language" },
      {
        token: `{{${SUBTITLES_PLACEHOLDER}:1}}`,
        name: "Translated subtitles",
        meaning: "what you say, in the first language subtitles translate into",
      },
    ];
    const onAir = [
      {
        token: `{{${ON_AIR_PLACEHOLDER}}}`,
        name: "On air",
        meaning: "OBS's scene, then LIVE while it streams or records",
      },
      {
        token: `{{${ON_AIR_TIME_PLACEHOLDER}}}`,
        name: "Time on air",
        meaning: "how long OBS has streamed or recorded, counting every second",
      },
    ];
    return [seen, scene, sceneAge, room].concat(subtitles, onAir, conditions);
  }

  /** Text with `token` replacing the selection from `start` to `end`, and the caret after it. */
  function insertToken(text, start, end, token) {
    const value = String(text);
    const from = Math.max(0, Math.min(value.length, Number.isFinite(start) ? start : value.length));
    const to = Math.max(from, Math.min(value.length, Number.isFinite(end) ? end : from));
    return { text: value.slice(0, from) + token + value.slice(to), caret: from + token.length };
  }

  /** Text with every placeholder for `from` renamed to `to`. */
  function renamePlaceholder(text, from, to) {
    if (!placeholderNames(text).includes(from)) {
      return String(text);
    }
    return parse(text).map((part) => ("name" in part ? `{{${part.name === from ? to : part.name}}}` : part.text))
      .join("");
  }

  const placeholders = {
    availablePlaceholders,
    insertToken,
    onAirText,
    placeholderNames,
    previewValues,
    renamePlaceholder,
    renderPlaceholders,
    shows,
    showsSubtitles,
    unknownPlaceholders,
  };
  if (inNode) {
    module.exports = placeholders;
  } else {
    window.SceneuronEditor.placeholders = placeholders;
  }
})();
