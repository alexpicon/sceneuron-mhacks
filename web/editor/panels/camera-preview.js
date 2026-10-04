// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the Camera & mic tab's camera preview: what camera analysis sees, asked for about three times a
   second only while the tab and the page are shown, with what on-device analysis finds and boxes around the faces it
   found; and the live description, which every snapshot brings. The picture lives in a blob URL of this page, is
   never stored, and is dropped as soon as the preview stops or has no picture to show. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { PREVIEW_INTERVAL_MS, analysisLine, faceBoxes, liveDescriptionLine, previewCaption, previewProblem } =
    Editor.cameraText;
  const { h } = Editor.dom;

  /** Wires the preview; returns the view with watch, which the tabs call when the Camera & mic tab shows or hides. */
  Editor.cameraPreview = function setupCameraPreview(editor) {
    const { app, byId, api } = editor;
    const frame = byId("camera-frame");
    const image = byId("camera-image");
    const faces = byId("camera-faces");
    const status = byId("camera-status");
    const caption = byId("camera-caption");
    const seen = byId("camera-seen");
    const description = byId("camera-live-description");
    let shown = false;
    let timer = null;
    let asking = false;
    let url = null;

    const active = () => shown && document.visibilityState === "visible";

    function set(node, text) {
      if (node.textContent !== text) {
        node.textContent = text;
      }
    }

    function drawFaces(analysis) {
      const boxes = faceBoxes(analysis).map((box) => {
        const node = h("div", { class: "camera-face", title: box.label });
        node.style.left = `${box.left}%`;
        node.style.top = `${box.top}%`;
        node.style.width = `${box.width}%`;
        node.style.height = `${box.height}%`;
        return node;
      });
      faces.replaceChildren(...boxes);
    }

    function showPicture(picture, analysis, vision) {
      const previous = url;
      url = URL.createObjectURL(picture.image);
      image.src = url;
      if (previous) {
        URL.revokeObjectURL(previous);
      }
      frame.hidden = false;
      status.hidden = true;
      frame.dataset.simulated = String(Boolean(picture.simulated));
      set(caption, previewCaption(picture.simulated, app.analyzedSource));
      set(seen, analysisLine(analysis, vision));
      seen.hidden = false;
      drawFaces(analysis);
    }

    /** Drops the picture, its blob URL, and what analysis found in it, so no frame stays in this page. */
    function clearPicture() {
      image.removeAttribute("src");
      if (url) {
        URL.revokeObjectURL(url);
        url = null;
      }
      frame.hidden = true;
      seen.hidden = true;
      faces.replaceChildren();
    }

    function showProblem(text) {
      clearPicture();
      status.hidden = false;
      set(status, text);
      set(caption, "");
    }

    async function ask() {
      timer = null;
      if (!active() || asking) {
        return;
      }
      asking = true;
      const started = Date.now();
      try {
        const [picture, analysis] = await Promise.all([api.cameraPreview(), api.cameraAnalysis()]);
        if (!active()) {
          return;
        }
        const vision = app.live ? app.live.vision : null;
        if (picture.image) {
          showPicture(picture, analysis, vision);
        } else {
          showProblem(previewProblem(picture.reason, { source: app.analyzedSource, vision }));
        }
      } catch (error) {
        showProblem(`The preview could not be loaded: ${error.message}`);
      } finally {
        asking = false;
        schedule(Math.max(0, PREVIEW_INTERVAL_MS - (Date.now() - started)));
      }
    }

    function schedule(delay) {
      if (active() && timer === null && !asking) {
        timer = window.setTimeout(ask, delay);
      }
    }

    function stop() {
      if (timer !== null) {
        window.clearTimeout(timer);
        timer = null;
      }
      // A picture left here would show again, possibly minutes old, when the preview comes back.
      if (url) {
        showProblem(previewProblem("waiting"));
      }
    }

    /** Starts asking for pictures while `on` and the page is visible, and stops otherwise. */
    function watch(on) {
      shown = Boolean(on);
      if (active()) {
        schedule(0);
      } else {
        stop();
      }
    }

    document.addEventListener("visibilitychange", () => watch(shown));

    /** Shows the live description from the latest snapshot. */
    function renderLive() {
      set(description, liveDescriptionLine(app.live));
    }

    return { renderLive, watch };
  };
})();
