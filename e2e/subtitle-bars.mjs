// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Reads the overlay page's subtitle bars for the browser steps (subtitle-steps.mjs): a bar's words, its language
// label, whether it shows, whether a bundled font loaded, and a bar held on a target until that target shows in its
// own script.

import { waitFor } from "./cdp.mjs";

const js = JSON.stringify;

/** Readers for the subtitle bars of `overlay`, a page; `api` holds a bar's text with an override. */
export function overlayBars(overlay, api) {
  const bar = (id) => `[data-element-id=${js(id)}]`;
  return {
    /** The bar's words on one line, as the core wraps them only between words. */
    async words(id) {
      return String(await overlay.text(`${bar(id)} .sn-text`)).replaceAll("\n", " ");
    },

    /** The label's text, or null while the bar shows none. */
    label(id) {
      return overlay.eval(`(() => {
        const label = document.querySelector(${js(`${bar(id)} .sn-label`)});
        return label && !label.hidden ? label.textContent : null;
      })()`);
    },

    display(id) {
      return overlay.eval(`getComputedStyle(document.querySelector(${js(bar(id))})).display`);
    },

    /** Whether the page's font `family` has loaded, once the fonts it uses are ready. */
    fontLoaded(family) {
      return overlay.eval(`document.fonts.ready.then(() => [...document.fonts].some((face) =>
        face.family.replace(/"/g, "") === ${js(family)} && face.status === "loaded"))`);
    },

    /**
     * Holds bar `id` on the target at `place` and waits until it shows in `script`, every character of it at that
     * script's code points; returns the text. Release the bar afterwards.
     */
    async showTarget(id, place, script, timeoutMs) {
      await api("POST", "/api/override", { element: id, props: { text: `{{subtitles:${place}}}` } });
      return waitFor(`the target in ${script}`, () => overlay.eval(`(() => {
        const node = document.querySelector(${js(bar(id))});
        const text = node.querySelector(".sn-text").textContent;
        return node.classList.contains(${js(`sn-script-${script}`)}) && /^[\\uE000-\\uE07F ]+$/.test(text) && text;
      })()`), timeoutMs);
    },
  };
}
