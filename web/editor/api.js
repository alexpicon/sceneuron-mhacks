// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the client for the local HTTP API. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const { PROP_FIELDS } = inNode ? require("./constants.js") : window.SceneuronEditor.constants;

  /** A refused request: `status` is the HTTP status (0 when unreachable) and `data` the error body. */
  class ApiError extends Error {
    constructor(message, status, data) {
      super(message);
      this.name = "ApiError";
      this.status = status;
      this.data = data || null;
    }
  }

  /** True for a save refused because the project changed since the draft was loaded. */
  function isConflict(error) {
    return Boolean(error) && error.status === 409;
  }

  /** Builds an /api/override body; unknown or empty patches are rejected like the server does. */
  function overridePayload(element, patch) {
    const props = {};
    for (const [field, value] of Object.entries(patch || {})) {
      if (!PROP_FIELDS.includes(field)) {
        throw new Error(`Unknown property ${field}.`);
      }
      if (value !== undefined) {
        props[field] = value;
      }
    }
    if (Object.keys(props).length === 0) {
      throw new Error("The change sets no property.");
    }
    return { element, props };
  }

  /** The JSON in a response body, or null when it has none. */
  async function readJson(response) {
    const text = await response.text();
    try {
      return text ? JSON.parse(text) : null;
    } catch {
      return null;
    }
  }

  /**
   * The editor's API client. `trusted` resolves to true once the server proved it holds the token
   * (see link.js); requests wait for it, so the token never goes to another program on the port.
   */
  function createApi(token, fetchImpl, trusted = async () => true) {
    /** Sends a request once the server is trusted; a refused one throws its error. */
    async function send(method, path, body, headers) {
      if (!(await trusted())) {
        throw new ApiError(
          "Sceneuron is not answering at this address. Check that OBS is running, or open the editor again from OBS.",
          0,
        );
      }
      const init = { method, cache: "no-store", headers: Object.assign({ "X-Sceneuron-Token": token }, headers) };
      if (body !== undefined) {
        init.headers["Content-Type"] = "application/json";
        init.body = JSON.stringify(body);
      }
      let response;
      try {
        response = await fetchImpl(path, init);
      } catch {
        throw new ApiError("Cannot reach Sceneuron. Check that OBS is running.", 0);
      }
      if (!response.ok) {
        const data = await readJson(response);
        const message = data && typeof data.error === "string" && data.error ? data.error : null;
        throw new ApiError(message || `Request failed with HTTP ${response.status}.`, response.status, data);
      }
      return response;
    }

    async function request(method, path, body, headers) {
      return await readJson(await send(method, path, body, headers));
    }

    /**
     * The camera preview: `{image, simulated}` with the picture as a Blob, or `{reason}` saying why there is none.
     */
    async function cameraPreview() {
      const response = await send("GET", "/api/camera/preview");
      const type = (response.headers && response.headers.get("Content-Type")) || "";
      if (type.startsWith("image/")) {
        const simulated = response.headers.get("X-Sceneuron-Simulated") === "true";
        return { image: await response.blob(), simulated };
      }
      const data = await readJson(response);
      return { reason: data && typeof data.reason === "string" ? data.reason : "unknown" };
    }

    return {
      request,
      getState: () => request("GET", "/api/state"),
      getProject: () => request("GET", "/api/project"),
      // With the revision the project was loaded at, the server refuses the save (409) if it changed since.
      saveProject: (project, revision) =>
        request("PUT", "/api/project", project, Number.isFinite(revision) ? { "If-Match": String(revision) } : {}),
      // Building the payload can throw; inside an async function that becomes a rejection.
      override: async (element, patch) => {
        const body = overridePayload(element, patch);
        return await request("POST", "/api/override", body);
      },
      release: (element) => request("POST", "/api/release", { element: element === undefined ? null : element }),
      effect: (element, effect) => request("POST", "/api/effect", { element, effect }),
      flood: (flood) => request("POST", "/api/flood", { flood }),
      automation: (enabled) => request("POST", "/api/automation", { enabled: Boolean(enabled) }),
      simulate: (presence) => request("POST", "/api/simulate", { presence }),
      sources: () => request("GET", "/api/sources"),
      attachSource: (uuid) => request("POST", "/api/sources/attach", { uuid }),
      cameraPreview,
      cameraAnalysis: () => request("GET", "/api/camera/analysis"),
      audioSources: () => request("GET", "/api/audio-sources"),
      attachVoice: (uuid) => request("POST", "/api/audio-sources/attach", { uuid }),
      addOverlay: () => request("POST", "/api/overlay"),
      // "Set up OBS for me": the overlay, camera analysis, and the voice filter, each only where missing.
      setUp: () => request("POST", "/api/setup"),
      aiSettings: () => request("GET", "/api/ai"),
      updateAiSettings: (update) => request("PUT", "/api/ai", update),
      plan: (text, mode, useCamera) =>
        request("POST", "/api/ai/plan", { request: text, mode, use_camera: Boolean(useCamera) }),
      analyze: () => request("POST", "/api/ai/analyze"),
      packs: () => request("GET", "/api/packs"),
      // Lower, Next, and Previous need no card, so none is sent for them.
      card: (deck, card, action) =>
        request("POST", "/api/card", card === null || card === undefined ? { deck, action } : { deck, card, action }),
      // The shared room: its state, and create, join, leave, reveal, or next with the fields each one uses.
      room: () => request("GET", "/api/room"),
      roomAction: (action, fields) => request("POST", "/api/room", Object.assign({ action }, fields)),
      // Holds or lets go of push to talk.
      talk: (down) => request("POST", "/api/voice/talk", { down: Boolean(down) }),
      // What can be said to Sceneuron now, and typed words carried out as if said with push to talk held.
      voiceGuide: () => request("GET", "/api/voice/guide"),
      voiceCommand: (text) => request("POST", "/api/voice/command", { text }),
      pairing: () => request("POST", "/api/control/pairing"),
      // How many codes were traded for the control token, which rises when a Stream Deck pairs.
      pairings: () => request("GET", "/api/control/pairing"),
      revokeControl: () => request("POST", "/api/control/revoke"),
    };
  }

  const api = { ApiError, createApi, isConflict, overridePayload };
  if (inNode) {
    module.exports = api;
  } else {
    window.SceneuronEditor.api = api;
  }
})();
