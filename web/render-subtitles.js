// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay subtitle bars: what the core adds to text that shows subtitles
   (crates/sceneuron-core/src/subtitles/bars.rs), drawn into an element's nodes: a small label naming their language
   beside the words, in the room the core left it; the script the words are written in, whose fonts
   (script-fonts.css) come before the theme's own for subtitle text; and a small Latin line under each line in that
   script. It tells
   render-fit.js the room the label and the Latin lines take, and how to put the words back after measuring.
   Shared by render.js and the Node tests. */
(function () {
  "use strict";

  // How the core sizes a label and the Latin lines (bars.rs): the label's type and a Latin line's as fractions of
  // the words', a Latin line's height in lines of the words, the width of a label's character and its padding in
  // its own type, and the gap after it in the words' type.
  const LABEL_SCALE = 0.4;
  const LATIN_SCALE = 0.5;
  const LATIN_LINE = 0.65;
  const LABEL_CHAR_EM = 0.64;
  const LABEL_PADDING_EM = 1.0;
  const LABEL_GAP_EM = 0.35;
  // The words' line height (render.js), which a Latin line keeps; the rest of its height is room above it.
  const LINE_HEIGHT = 1.15;
  // The scripts the core writes subtitles in, by the id the snapshot gives them.
  const SCRIPTS = Object.freeze(["tengwar", "piqad", "sitelen-pona"]);
  // Each covers only its script's code points, so any other character keeps the theme's type.
  const SCRIPT_FONTS = '"Sceneuron Tengwar","Sceneuron pIqaD","Sceneuron sitelen pona"';

  const round = (value) => Math.round(value * 1000) / 1000;

  const STYLESHEET = `
.sn-label{flex:none;box-sizing:border-box;margin-right:${round(LABEL_GAP_EM / LABEL_SCALE)}em;padding:.2em .5em;
font-size:${LABEL_SCALE}em;font-weight:700;font-style:normal;line-height:1.2;letter-spacing:0;white-space:nowrap;
overflow:hidden;text-overflow:ellipsis;border-radius:1em;background:rgba(255,255,255,.14);
box-shadow:inset 0 0 0 .1em currentColor}
.sn-label[hidden]{display:none}
.sn-line,.sn-latin{display:block}
.sn-latin{font-size:${LATIN_SCALE}em;line-height:${LINE_HEIGHT};
padding-top:${round((LATIN_LINE * LINE_HEIGHT) / LATIN_SCALE - LINE_HEIGHT)}em;font-weight:600;opacity:.85}
`;

  /** The widest a label may be, in its own type: the room the core leaves it, without the gap after it. */
  function labelWidthEm(label) {
    const chars = Array.from(String(label || "")).length;
    return chars ? chars * LABEL_CHAR_EM + LABEL_PADDING_EM : 0;
  }

  /** The room a label takes beside the words, its gap included, in the words' type, as the core leaves it. */
  function labelRoomEm(label) {
    const width = labelWidthEm(label);
    return width ? width * LABEL_SCALE + LABEL_GAP_EM : 0;
  }

  /** A snapshot element's subtitle bar with its fields checked, or null for an element that shows no subtitles. */
  function barOf(element) {
    const bar = element && element.subtitle_bar;
    if (!bar || typeof bar !== "object") {
      return null;
    }
    return {
      label: typeof bar.label === "string" ? bar.label : "",
      script: SCRIPTS.includes(bar.script) ? bar.script : "",
      latin: Array.isArray(bar.latin) ? bar.latin.map(String) : [],
    };
  }

  /** Shows the label beside the words, made the first time one is needed and hidden while there is none. */
  function showLabel(state, text) {
    if (!text && !state.label) {
      return;
    }
    if (!state.label) {
      state.label = state.text.ownerDocument.createElement("span");
      state.label.classList.add("sn-label");
      state.body.insertBefore(state.label, state.text);
    }
    if (state.label.textContent !== text) {
      state.label.textContent = text;
      state.label.style.setProperty("max-width", `${round(labelWidthEm(text))}em`);
    }
    state.label.hidden = !text;
  }

  /** Writes each line, then its Latin line under it. */
  function writeLines(state, lines, latin) {
    const doc = state.text.ownerDocument;
    state.text.textContent = "";
    lines.forEach((line, index) => {
      for (const [cls, words] of [["sn-line", line], ["sn-latin", latin[index]]]) {
        if (words !== undefined) {
          const node = state.text.appendChild(doc.createElement("span"));
          node.classList.add(cls);
          node.textContent = words;
        }
      }
    });
  }

  /** Puts the script fonts before `font`, the theme's own type, for the words of a subtitle bar. */
  function useFonts(state, font) {
    const family = font ? `${SCRIPT_FONTS},${font}` : "";
    if (state.scriptFonts !== family) {
      state.scriptFonts = family;
      if (family) {
        state.text.style.setProperty("font-family", family);
      } else {
        state.text.style.removeProperty("font-family");
      }
    }
  }

  /**
   * Shows `content` in a text element's nodes with what `bar`, its subtitle bar or null, adds to it, its words in
   * the script fonts and then `font`, the theme's type. Leaves on the state what render-fit.js reads: `subtitleFit`,
   * the label's room in the words' type and the height of a line in lines; `lineUnits`, the lines the words take when
   * Latin lines go under them; and `refill`, which puts structured words back after measuring.
   */
  function fill(state, content, bar, font) {
    showLabel(state, bar ? bar.label : "");
    useFonts(state, bar ? font : "");
    const script = bar ? bar.script : "";
    for (const id of SCRIPTS) {
      state.node.classList.toggle(`sn-script-${id}`, id === script);
    }
    const latin = script ? bar.latin : [];
    state.node.classList.toggle("sn-subtitle", Boolean(bar));
    state.node.classList.toggle("sn-glossed", latin.length > 0);
    const lines = content.split("\n");
    state.subtitleFit = { roomEm: bar ? labelRoomEm(bar.label) : 0, lineSize: latin.length ? 1 + LATIN_LINE : 1 };
    state.lineUnits = latin.length ? lines.length * (1 + LATIN_LINE) : null;
    const key = latin.length ? JSON.stringify([content, latin]) : content;
    if (state.filled === key) {
      return;
    }
    state.filled = key;
    state.content = content;
    if (latin.length) {
      state.refill = () => writeLines(state, lines, latin);
      state.refill();
    } else {
      state.refill = null;
      state.text.textContent = content;
    }
  }

  const api = { LABEL_SCALE, LATIN_LINE, LATIN_SCALE, SCRIPTS, STYLESHEET, barOf, fill, labelRoomEm, labelWidthEm };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  } else {
    window.SceneuronSubtitles = api;
  }
})();
