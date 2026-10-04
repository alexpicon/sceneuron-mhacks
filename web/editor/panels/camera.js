// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the Camera & mic tab's camera source list, the OBS video sources to analyze, its overlay
   controls: adding the overlay to the current scene, and the overlay URL, and "Set up OBS for me", which adds the
   overlay, camera analysis, and the voice filter where they are missing and says what it did and skipped. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { h, hidden, withFocusRestore } = Editor.dom;
  const { setupLines, setupSummary } = Editor.cameraText;

  /**
   * Wires the camera source list, the overlay controls, and "Set up OBS for me"; returns the view with load, which
   * lists the sources.
   */
  Editor.camera = function setupCamera(editor) {
    const { app, ui, api, byId, views } = editor;

    function renderSources(sources) {
      withFocusRestore(ui.sourceList, () => {
        const items = sources.map((source) => {
          const name = String(source.name);
          const action = source.analyzed ? h("span", { class: "chip", "data-state": "running" }, "Analyzing") : h(
            "button",
            {
              type: "button",
              "data-source-uuid": source.uuid,
              "data-source-name": name,
              "data-focus-key": `source:${source.uuid}`,
            },
            "Analyze this source",
            hidden(`: ${name}`),
          );
          return h(
            "li",
            {},
            h(
              "div",
              {},
              h("span", {}, name),
              h("span", { class: "source-kind" }, String(source.kind)),
            ),
            action,
          );
        });
        ui.sourceList.replaceChildren(...items);
      });
      ui.sourcesEmpty.hidden = sources.length > 0;
      const analyzed = sources.find((source) => source.analyzed);
      app.analyzedSource = analyzed ? String(analyzed.name) : null;
    }

    async function load() {
      try {
        const sources = await api.sources();
        renderSources(Array.isArray(sources) ? sources : []);
      } catch (error) {
        editor.report(error);
      }
    }

    ui.refreshSources.addEventListener("click", load);

    ui.sourceList.addEventListener("click", async (event) => {
      const button = event.target.closest("[data-source-uuid]");
      if (!button) {
        return;
      }
      const name = button.getAttribute("data-source-name");
      try {
        await api.attachSource(button.getAttribute("data-source-uuid"));
        editor.toast(`Sceneuron now analyzes ${name}.`, "info");
      } catch (error) {
        editor.report(error);
      }
      await load();
      if (!ui.sourceList.contains(document.activeElement)) {
        ui.refreshSources.focus();
      }
    });

    ui.addOverlay.addEventListener("click", async () => {
      try {
        const result = await api.addOverlay();
        const name = result && result.name ? result.name : "the Sceneuron overlay";
        editor.toast(`Added ${name} to the current scene.`, "info");
      } catch (error) {
        editor.report(error);
      }
    });

    // The button stays enabled and focused while it works, so keyboard focus never drops to the page; a second click
    // meanwhile is ignored.
    const setupButton = byId("setup-obs");
    const setupReport = byId("setup-obs-report");
    setupButton.addEventListener("click", async () => {
      if (setupButton.getAttribute("aria-busy") === "true") {
        return;
      }
      setupButton.setAttribute("aria-busy", "true");
      try {
        const report = await api.setUp();
        const lines = setupLines(report).map(({ part, outcome, text }) =>
          h("li", { "data-part": part, "data-outcome": outcome }, text),
        );
        setupReport.replaceChildren(...lines);
        setupReport.hidden = false;
        editor.toast(setupSummary(report), "info");
      } catch (error) {
        editor.report(error);
      } finally {
        setupButton.removeAttribute("aria-busy");
      }
      await load();
      if (views.microphones) {
        await views.microphones.load();
      }
    });

    // The server hands out the overlay's read-only address; the editor's own token never goes in it.
    api
      .request("GET", "/api/overlay-url")
      .then((reply) => {
        ui.overlayUrl.value = reply.url;
      })
      .catch(() => {
        ui.overlayUrl.value = "";
      });
    ui.copyOverlayUrl.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(ui.overlayUrl.value);
        editor.toast("Overlay URL copied.", "info");
      } catch {
        ui.overlayUrl.focus();
        ui.overlayUrl.select();
        editor.toast("Copying failed. The URL is selected; press Cmd+C to copy it.", "error");
      }
    });

    return { load };
  };
})();
