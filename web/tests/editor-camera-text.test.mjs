// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/camera-text.js: why the camera preview has no picture, what the picture is, the line that says what
// on-device analysis found, the live description, the face boxes in percent of the picture, and what "Set up OBS for
// me" did and skipped, the Stream Deck's pairing link included.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const C = require("../editor/camera-text.js");

describe("why there is no picture", () => {
  test("each reason is said in plain words", () => {
    assert.match(C.previewProblem("not_in_obs"), /^Sceneuron is not running inside OBS, so there is no camera/);
    assert.match(C.previewProblem("no_source"), /^No camera is analyzed yet\. Choose your camera under Camera source/);
    assert.equal(C.previewProblem("waiting"), "Starting the preview...");
    assert.equal(C.previewProblem("something new"), "The preview is not available right now.");
  });

  test("a camera that stopped is named, and a failed analysis says why", () => {
    assert.equal(
      C.previewProblem("stale", { source: "Video Capture Device" }),
      "No frames from Video Capture Device for a while. Check that the camera is on and that its source is in the " +
        "scene OBS shows.",
    );
    assert.match(C.previewProblem("stale", {}), /^No frames from the analyzed camera for a while/);
    const vision = { state: "failed", message: "unsupported camera pixel format 25" };
    assert.equal(
      C.previewProblem("stale", { source: "Cam", vision }),
      "Camera analysis failed: unsupported camera pixel format 25",
    );
    assert.match(C.previewProblem("stale", { vision: { state: "running" } }), /^No frames from/);
  });
});

describe("the caption", () => {
  test("a camera picture is named and stays on this computer; a simulated one says so", () => {
    assert.equal(
      C.previewCaption(false, "Video Capture Device"),
      "What Sceneuron sees from Video Capture Device, a few times a second. The picture never leaves this computer.",
    );
    assert.match(C.previewCaption(false, null), /^What Sceneuron sees, a few times a second/);
    assert.match(C.previewCaption(true, "Webcam (simulated)"), /^Simulated camera: a drawn test picture/);
  });

  test("the preview asks two to five times a second", () => {
    assert.ok(C.PREVIEW_INTERVAL_MS >= 200 && C.PREVIEW_INTERVAL_MS <= 500);
  });
});

describe("what on-device analysis found", () => {
  test("lists what was seen, or says there is nothing", () => {
    const analysis = { seen: ["face in view (0.98)", "right hand raised, thumbs up", "3 fingers up"], age_ms: 120 };
    assert.equal(
      C.analysisLine(analysis),
      "On-device analysis sees: face in view (0.98); right hand raised, thumbs up; 3 fingers up.",
    );
    assert.equal(C.analysisLine({ seen: [], faces: [], age_ms: 0 }), "On-device analysis finds no face, hand, or " +
      "sign right now.");
    assert.equal(C.analysisLine({ seen: [], faces: [], age_ms: null }), "On-device analysis has nothing to report yet.");
    assert.equal(C.analysisLine(null), "On-device analysis has nothing to report yet.");
  });

  test("with nothing found lately, a failed or starting analysis says so", () => {
    const none = { seen: [], faces: [], age_ms: null };
    const failed = { state: "failed", message: "the face model could not load" };
    assert.equal(C.analysisLine(none, failed), "Camera analysis failed: the face model could not load");
    assert.equal(C.analysisLine(null, { state: "failed" }), "Camera analysis failed.");
    assert.equal(C.analysisLine(none, { state: "starting" }), "Camera analysis is starting.");
    const nothing = "On-device analysis has nothing to report yet.";
    assert.equal(C.analysisLine(none, { state: "running", latency_ms: 12 }), nothing);
    assert.equal(C.analysisLine(none, null), nothing);
    const fresh = { seen: ["face in view (0.98)"], faces: [], age_ms: 80 };
    assert.equal(C.analysisLine(fresh, failed), "On-device analysis sees: face in view (0.98).", "a fresh result wins");
  });

  test("the live description is the snapshot's line, or says why there is none", () => {
    const running = { state: "running", latency_ms: 12 };
    assert.equal(C.liveDescriptionLine({ seen: "1 person; mouth open", vision: running }),
      "Live description: 1 person; mouth open");
    const waiting = "Live description: waiting for camera analysis to describe its frames.";
    assert.equal(C.liveDescriptionLine({ seen: "", vision: running }), waiting);
    assert.equal(C.liveDescriptionLine({ seen: "", vision: { state: "starting" } }), waiting);
    assert.equal(C.liveDescriptionLine({ seen: "", vision: { state: "failed", message: "no camera" } }),
      "Live description: none, since camera analysis failed.");
    const idle = "Live description: none while no camera is analyzed.";
    assert.equal(C.liveDescriptionLine({ seen: "", vision: { state: "idle" } }), idle);
    assert.equal(C.liveDescriptionLine({ vision: { state: "idle" } }), idle, "a server without the line");
    assert.equal(C.liveDescriptionLine(null), idle);
  });

  test("face boxes are percent of the picture, clamped to it, and labeled with their probability", () => {
    const analysis = {
      faces: [
        { x1: 0.25, y1: 0.125, x2: 0.5, y2: 0.5, score: 0.981 },
        { x1: -0.1, y1: 0.9, x2: 0.2, y2: 1.2, score: 0.5 },
        { x1: 0.6, y1: 0.6, x2: 0.6, y2: 0.8, score: 0.4 },
        { x1: "a", y1: 0, x2: 1, y2: 1 },
      ],
    };
    const boxes = C.faceBoxes(analysis);
    assert.equal(boxes.length, 2, "an empty and a malformed box are dropped");
    assert.deepEqual(boxes[0], { left: 25, top: 12.5, width: 25, height: 37.5, label: "Face 0.98" });
    assert.equal(boxes[1].left, 0);
    assert.equal(boxes[1].width, 20);
    assert.ok(Math.abs(boxes[1].height - 10) < 1e-9 && boxes[1].top === 90);
    assert.deepEqual(C.faceBoxes(null), []);
    assert.deepEqual(C.faceBoxes({ faces: [{ x1: 0, y1: 0, x2: 1, y2: 1 }] })[0].label, "Face");
  });
});

describe("what Set up OBS for me did", () => {
  const step = (outcome, source = "", reason = "") => ({ outcome, source, reason });
  const added = {
    overlay: step("added", "Sceneuron Overlay"),
    camera: step("added", "MacBook Camera"),
    microphone: step("added", "Mic/Aux"),
    stream_deck: step("added", "Stream Deck"),
  };
  const present = {
    overlay: step("present", "Sceneuron Overlay"),
    camera: step("present", "MacBook Camera"),
    microphone: step("present", "Mic/Aux"),
    stream_deck: step("present", "Stream Deck"),
  };
  const NOT_INSTALLED =
    "Sceneuron's plugin for Stream Deck is not installed on this Mac; double-click " +
    "ai.chaski.sceneuron.streamDeckPlugin to install it in the Stream Deck app, then choose Set up OBS for me again";

  test("each part added says where it went, and the Stream Deck that a pairing link went out", () => {
    assert.deepEqual(C.setupLines(added), [
      { part: "overlay", outcome: "added", text: "Added Sceneuron Overlay to the current scene." },
      { part: "camera", outcome: "added", text: "Camera analysis now watches MacBook Camera." },
      { part: "microphone", outcome: "added", text: "Voice control and subtitles now listen to Mic/Aux." },
      {
        part: "stream_deck",
        outcome: "added",
        text: "Sent Sceneuron's plugin in the Stream Deck app a pairing link; your Stream Deck pairs in a moment.",
      },
    ]);
    assert.equal(
      C.setupSummary(added),
      "Set up OBS: added the overlay, camera analysis, and the voice filter, and sent the Stream Deck a pairing link.",
    );
    assert.equal(C.setupSummary({ ...present, stream_deck: added.stream_deck }),
      "Set up OBS: sent the Stream Deck a pairing link.");
    assert.equal(C.setupSummary({ ...added, stream_deck: present.stream_deck }),
      "Set up OBS: added the overlay, camera analysis, and the voice filter.");
  });

  test("a part already in place is skipped and named, and a paired Stream Deck is left as it is", () => {
    assert.deepEqual(C.setupLines(present).map((line) => [line.outcome, line.text]), [
      ["present", "Skipped the overlay: Sceneuron Overlay is already in the current scene."],
      ["present", "Skipped the camera: MacBook Camera is already analyzed."],
      ["present", "Skipped the microphone: Mic/Aux already has the Sceneuron Voice filter."],
      ["present", "Skipped the Stream Deck: it is already paired."],
    ]);
    assert.equal(C.setupSummary(present), "OBS was already set up; nothing changed.");
  });

  test("without Sceneuron's Stream Deck plugin, the Stream Deck is skipped with how to install it", () => {
    const report = { ...added, stream_deck: step("skipped", "", `${NOT_INSTALLED}.`) };
    assert.deepEqual(C.setupLines(report)[3], {
      part: "stream_deck",
      outcome: "skipped",
      text: `Skipped the Stream Deck: ${NOT_INSTALLED}.`,
    });
    assert.equal(
      C.setupSummary(report),
      "Set up OBS: added the overlay, camera analysis, and the voice filter. Could not pair a Stream Deck; the " +
        "Camera & mic tab says why.",
    );
    const alsoNoMic = { ...report, microphone: step("skipped", "", "OBS has no Mic/Aux device") };
    assert.equal(
      C.setupSummary(alsoNoMic),
      "Set up OBS: added the overlay and camera analysis. Could not add the voice filter or pair a Stream Deck; the " +
        "Camera & mic tab says why.",
    );
  });

  test("a part that could not be added says why, once, with one full stop", () => {
    const report = {
      ...present,
      camera: step("added", "MacBook Camera"),
      microphone: step("skipped", "", "OBS has no Mic/Aux device; choose your microphone in Settings > Audio."),
    };
    const lines = C.setupLines(report);
    assert.deepEqual(lines[2], {
      part: "microphone",
      outcome: "skipped",
      text: "Skipped the microphone: OBS has no Mic/Aux device; choose your microphone in Settings > Audio.",
    });
    assert.equal(
      C.setupSummary(report),
      "Set up OBS: added camera analysis. Could not add the voice filter; the Camera & mic tab says why.",
    );
    const none = {
      overlay: step("skipped", "", "there is no current scene"),
      camera: report.microphone,
      microphone: {},
      stream_deck: step("skipped", "", "the pairing link could not be sent to the Stream Deck app: no handler"),
    };
    assert.equal(
      C.setupSummary(none),
      "Set up OBS: nothing was added. Could not add the overlay, camera analysis, and the voice filter or pair a " +
        "Stream Deck; the Camera & mic tab says why.",
    );
  });

  test("a report missing a part, or naming no source, counts it as not added", () => {
    const lines = C.setupLines({ overlay: step("added", ""), camera: step("strange", "Cam") });
    assert.deepEqual(lines.map((line) => line.outcome), ["skipped", "skipped", "skipped", "skipped"]);
    assert.equal(lines[0].text, "Skipped the overlay: it could not be added.");
    assert.equal(lines[3].text, "Skipped the Stream Deck: it could not be added.");
    assert.deepEqual(C.setupLines(null).map((line) => line.part), ["overlay", "camera", "microphone", "stream_deck"]);
  });
});
