// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: camera preview words: why there is no picture, what the picture is, what on-device analysis
   found in it, the live description it makes of it, and where the boxes around the faces it found go; and what "Set up
   OBS for me" did and skipped, pairing the Stream Deck included. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);

  /** How often the Camera & mic tab asks for a new picture while it is shown: about three times a second. */
  const PREVIEW_INTERVAL_MS = 300;

  /**
   * Says, in plain words, why the preview has no picture. `context` may name the analyzed `source` and carry the
   * snapshot's `vision` status, whose failure explains a camera that stopped sending frames.
   */
  function previewProblem(reason, context) {
    const { source, vision } = context || {};
    switch (reason) {
      case "not_in_obs":
        return "Sceneuron is not running inside OBS, so there is no camera to show. Open the editor from OBS " +
          "(Tools > Sceneuron) to see your camera here.";
      case "no_source":
        return "No camera is analyzed yet. Choose your camera under Camera source below, and its picture appears " +
          "here.";
      case "waiting":
        return "Starting the preview...";
      case "stale": {
        if (vision && vision.state === "failed" && vision.message) {
          return `Camera analysis failed: ${vision.message}`;
        }
        const named = source ? String(source) : "the analyzed camera";
        return `No frames from ${named} for a while. Check that the camera is on and that its source is in the ` +
          "scene OBS shows.";
      }
      default:
        return "The preview is not available right now.";
    }
  }

  /** What the picture is, said under it. */
  function previewCaption(simulated, source) {
    if (simulated) {
      return "Simulated camera: a drawn test picture from the development server, not a camera.";
    }
    const from = source ? ` from ${source}` : "";
    return `What Sceneuron sees${from}, a few times a second. The picture never leaves this computer.`;
  }

  /**
   * The line under the picture: what on-device analysis found lately. When it found nothing lately, the snapshot's
   * `vision` status says why: the picture keeps coming while analysis fails or starts.
   */
  function analysisLine(analysis, vision) {
    const seen = analysis && Array.isArray(analysis.seen) ? analysis.seen.map(String) : [];
    if (!analysis || !Number.isFinite(analysis.age_ms)) {
      const state = vision ? vision.state : null;
      if (state === "failed") {
        return vision.message ? `Camera analysis failed: ${vision.message}` : "Camera analysis failed.";
      }
      return state === "starting" ? "Camera analysis is starting." : "On-device analysis has nothing to report yet.";
    }
    if (seen.length === 0) {
      return "On-device analysis finds no face, hand, or sign right now.";
    }
    return `On-device analysis sees: ${seen.join("; ")}.`;
  }

  /**
   * The live description, from the snapshot: the line "{{seen}}" shows on the overlay now, or why there is none.
   * Unlike the picture, it shows whether or not this tab asks for pictures.
   */
  function liveDescriptionLine(live) {
    const line = live && typeof live.seen === "string" ? live.seen : "";
    if (line) {
      return `Live description: ${line}`;
    }
    const state = live && live.vision ? live.vision.state : null;
    if (state === "running" || state === "starting") {
      return "Live description: waiting for camera analysis to describe its frames.";
    }
    return state === "failed"
      ? "Live description: none, since camera analysis failed."
      : "Live description: none while no camera is analyzed.";
  }

  const clampPercent = (value) => Math.min(100, Math.max(0, value * 100));

  /** Each face's box in percent of the picture, from its top left, with its probability as a label. */
  function faceBoxes(analysis) {
    const faces = analysis && Array.isArray(analysis.faces) ? analysis.faces : [];
    return faces
      .filter((face) => [face.x1, face.y1, face.x2, face.y2].every(Number.isFinite))
      .map((face) => {
        const [left, top, right, bottom] = [face.x1, face.y1, face.x2, face.y2].map(clampPercent);
        const score = Number.isFinite(face.score) ? ` ${face.score.toFixed(2)}` : "";
        return { left, top, width: right - left, height: bottom - top, label: `Face${score}` };
      })
      .filter((box) => box.width > 0 && box.height > 0);
  }

  /**
   * The parts "Set up OBS for me" puts in place, in the report's order, with the words for each. The summary names an
   * OBS part by its `noun`, and says the Stream Deck's part with its own words: what it `did`, and what it `couldNot`.
   */
  const SETUP_PARTS = [
    {
      key: "overlay",
      label: "the overlay",
      added: (source) => `Added ${source} to the current scene.`,
      present: (source) => `${source} is already in the current scene.`,
      noun: "the overlay",
    },
    {
      key: "camera",
      label: "the camera",
      added: (source) => `Camera analysis now watches ${source}.`,
      present: (source) => `${source} is already analyzed.`,
      noun: "camera analysis",
    },
    {
      key: "microphone",
      label: "the microphone",
      added: (source) => `Voice control and subtitles now listen to ${source}.`,
      present: (source) => `${source} already has the Sceneuron Voice filter.`,
      noun: "the voice filter",
    },
    {
      key: "stream_deck",
      label: "the Stream Deck",
      added: () => "Sent Sceneuron's plugin in the Stream Deck app a pairing link; your Stream Deck pairs in a moment.",
      present: () => "it is already paired.",
      did: "sent the Stream Deck a pairing link",
      couldNot: "pair a Stream Deck",
    },
  ];

  /**
   * What "Set up OBS for me" did, one line per part as `{ part, outcome, text }`, where the outcome is `added`,
   * `present` (skipped, since it was already set up), or `skipped` (it could not be added; the text says why).
   */
  function setupLines(report) {
    return SETUP_PARTS.map(({ key, label, added, present }) => {
      const step = (report && report[key]) || {};
      const source = typeof step.source === "string" ? step.source : "";
      const outcome = ["added", "present"].includes(step.outcome) && source ? step.outcome : "skipped";
      if (outcome === "added") {
        return { part: key, outcome, text: added(source) };
      }
      if (outcome === "present") {
        return { part: key, outcome, text: `Skipped ${label}: ${present(source)}` };
      }
      const reason = typeof step.reason === "string" ? step.reason.trim().replace(/\.$/, "") : "";
      return { part: key, outcome, text: `Skipped ${label}: ${reason || "it could not be added"}.` };
    });
  }

  /** `a`, `a and b`, or `a, b, and c`. */
  function listed(items) {
    return items.length < 3 ? items.join(" and ") : `${items.slice(0, -1).join(", ")}, and ${items.at(-1)}`;
  }

  /**
   * What happened to the parts whose line has `outcome`: `verb` with the OBS parts' nouns listed, then the Stream
   * Deck's own words, its `wording`.
   */
  function phrases(lines, outcome, verb, wording) {
    const parts = SETUP_PARTS.filter((_, index) => lines[index].outcome === outcome);
    const nouns = parts.filter((part) => part.noun).map((part) => part.noun);
    const listedNouns = nouns.length ? [`${verb} ${listed(nouns)}`] : [];
    return [...listedNouns, ...parts.filter((part) => part[wording]).map((part) => part[wording])];
  }

  /** One sentence for the notification: what was added, or that nothing had to be, and what could not be. */
  function setupSummary(report) {
    const lines = setupLines(report);
    const done = phrases(lines, "added", "added", "did");
    const failed = phrases(lines, "skipped", "add", "couldNot");
    const did = done.length
      ? `Set up OBS: ${done.join(", and ")}.`
      : failed.length
        ? "Set up OBS: nothing was added."
        : "OBS was already set up; nothing changed.";
    return failed.length ? `${did} Could not ${failed.join(" or ")}; the Camera & mic tab says why.` : did;
  }

  const cameraText = {
    PREVIEW_INTERVAL_MS,
    analysisLine,
    faceBoxes,
    liveDescriptionLine,
    previewCaption,
    previewProblem,
    setupLines,
    setupSummary,
  };
  if (inNode) {
    module.exports = cameraText;
  } else {
    window.SceneuronEditor.cameraText = cameraText;
  }
})();
