// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: what the fun reactions do now, why the camera cannot play one when it cannot, and what saying a
   reaction's trigger phrase does, for the Settings tab. */
(function () {
  "use strict";

  const SECOND_MS = 1000;

  /**
   * The fun reactions' state and a sentence about it, from the draft's reactions and the live snapshot: `off` while
   * the camera and trigger phrases play none, `paused` while automation is paused, or `on`, with the cooldown.
   */
  function reactionsStatus(reactions, live) {
    if (!reactions || reactions.enabled === false) {
      return {
        state: "off",
        text: "Off: the camera and trigger phrases play no reaction. Rules, Try it, and asking Sceneuron still do.",
      };
    }
    if (live && live.automation_enabled === false) {
      return {
        state: "paused",
        text: "Paused while automation is paused. Try it and asking Sceneuron still play.",
      };
    }
    const seconds = Math.round((Number(reactions.cooldown_ms) || 3 * SECOND_MS) / SECOND_MS);
    return {
      state: "on",
      text:
        `On: the camera, rules, and trigger phrases play at most one reaction every ${seconds} s; Try it and ` +
        "asking Sceneuron play one at once.",
    };
  }

  /** Why the camera status keeps the camera from playing, or null while it is analyzed with `ability`. */
  function cameraProblem(live, ability, what) {
    const camera = live && live.vision;
    const waiting = (text) => ({ state: "paused", text });
    if (camera && camera.state === "idle") {
      return waiting("Waiting for the camera: choose one under Camera source in the Camera & mic tab.");
    }
    if (camera && camera.state === "starting") {
      return waiting("Waiting for camera analysis to start.");
    }
    if (camera && camera.state === "failed") {
      const why = camera.message ? `camera analysis failed: ${camera.message}` : "camera analysis failed";
      return { state: "unavailable", text: `Unavailable: ${why}.` };
    }
    if (camera && camera.state === "running" && camera[ability] === false) {
      return { state: "unavailable", text: `Unavailable: this camera analysis cannot see ${what}.` };
    }
    return null;
  }

  /** Why the vision model cannot look at the tongue, or null when it can. */
  function lookProblem(policy, live, ai) {
    const unavailable = (why) => ({ state: "unavailable", text: `Unavailable: ${why}.` });
    if (!ai) {
      return { state: "checking", text: "Checking the AI settings..." };
    }
    if (!ai.available) {
      return unavailable("this Sceneuron has no AI support");
    }
    if (ai.simulated) {
      return unavailable("the simulated AI cannot see a tongue; it needs a real vision model");
    }
    if (!ai.has_key) {
      return unavailable("add a vision model API key in the AI tab");
    }
    if (!ai.consent) {
      return unavailable("in the AI tab, confirm that camera images may be sent to the provider");
    }
    if (!ai.enabled) {
      return unavailable("turn on scene understanding in the AI tab");
    }
    if (policy && policy.enabled === false) {
      return unavailable("scene understanding is off for this project");
    }
    return null;
  }

  /** The vision model's pause or failure, or null while it runs. */
  function modelProblem(live) {
    const vision = (live && live.ai) || {};
    if (vision.state !== "paused" && vision.state !== "failed") {
      return null;
    }
    const why = vision.message ? `: ${vision.message}` : "";
    const which = vision.state === "failed" ? "failed" : "is paused";
    return { state: "paused", text: `Waiting for the vision model, which ${which}${why}.` };
  }

  /**
   * What the camera does for `preset` now, from the draft's reactions and AI policy, the live snapshot, and the AI
   * settings: `off` with the switch or the preset's camera, `unavailable` (with what to do about it), `paused`
   * while it waits for automation, the camera, or the vision model, `checking` while the AI settings load, or `on`
   * with how to play it. A preset the camera never plays has no camera status: null.
   */
  function cameraStatus(preset, reactions, policy, live, ai) {
    if (!preset || !preset.camera) {
      return null;
    }
    if (!reactions || reactions.enabled === false) {
      return { state: "off", text: "Off with fun reactions." };
    }
    if ((reactions.camera_off || []).includes(preset.id)) {
      return { state: "off", text: "Off: the camera never plays this one." };
    }
    const look = preset.camera === "look";
    const problem = look ? lookProblem(policy, live, ai) : null;
    if (problem) {
      return problem;
    }
    if (live && live.automation_enabled === false) {
      return { state: "paused", text: "Paused while automation is paused." };
    }
    const camera = look ? cameraProblem(live, "mouths", "the mouth") : cameraProblem(live, "hands", "hands");
    const waiting = camera || (look ? modelProblem(live) : null);
    if (waiting) {
      return waiting;
    }
    const text = look
      ? "On: stick your tongue out for a moment and the screen floods with 😜 a few seconds later, once the " +
        "vision model has seen it."
      : `On: hold a hand over your eyes or forehead for a second and ${preset.emojis[0]} "${preset.banner}" ` +
        "plays. It is seen on this computer, with no AI call.";
    return { state: "on", text };
  }

  /** What a preset's camera switch says it watches for. */
  function cameraLabel(preset) {
    return preset.camera === "look" ? "Camera: tongue out" : "Camera: facepalm for a second";
  }

  /** What a preset's voice switch says to say. */
  function voiceLabel(preset) {
    return `Voice: "${preset.phrases[0]}"`;
  }

  /**
   * What saying `preset`'s trigger phrases does now, from the draft's reactions and the live snapshot: `off` with the
   * switch or the preset's voice, `paused` while voice control is off or listens only with push to talk held, or
   * while automation is paused, or `on` with what to say. A preset without trigger phrases has no voice status: null.
   */
  function voiceStatus(preset, reactions, live) {
    if (!preset || !preset.phrases || preset.phrases.length === 0) {
      return null;
    }
    if (!reactions || reactions.enabled === false) {
      return { state: "off", text: "Off with fun reactions." };
    }
    if ((reactions.voice_off || []).includes(preset.id)) {
      return { state: "off", text: "Off: saying it plays nothing." };
    }
    const voice = live && live.voice;
    const said = preset.phrases.map((phrase) => `"${phrase}"`);
    if (!voice || !voice.enabled) {
      return { state: "paused", text: "Waiting for voice control: turn it on in the AI tab." };
    }
    if (voice.mode !== "always_on") {
      return {
        state: "paused",
        text: `Waiting for voice control to listen always. Holding push to talk, ${said[0]} plays it at once.`,
      };
    }
    if (live.automation_enabled === false) {
      return { state: "paused", text: "Paused while automation is paused." };
    }
    const others = said.length > 1 ? ` (or ${said.slice(1).join(" or ")})` : "";
    return {
      state: "on",
      text: `On: say ${said[0]}${others} anywhere in a sentence and ${preset.emojis[0]} plays, at most once per cooldown.`,
    };
  }

  const reactionsText = { cameraLabel, cameraStatus, reactionsStatus, voiceLabel, voiceStatus };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = reactionsText;
  } else {
    window.SceneuronEditor.reactionsText = reactionsText;
  }
})();
