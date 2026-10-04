// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay text fitting for themes: a theme's panel, decorations, prompt, and font can leave a text
   element less room than its own look does, so its text shrinks, measured in the page's layout, until it fits
   again, without wrapping a line that the element's own look keeps on one. Text that shows live subtitles is
   sized for as much as its box holds beside its label and with any Latin lines under it, so it keeps one size
   while words arrive. Shared by render.js and the Node tests. */
(function () {
  "use strict";

  const MIN_TEXT_SCALE = 0.4;
  const FIT_STEPS = 7;
  // How the core sizes subtitles to a text box (crates/sceneuron-core/src/subtitles/lines.rs), so a line it
  // wrapped is one line in the element's own look: the average width of a bold sans-serif character and a text
  // box's padding, as fractions of the font size, the line height, the shortest and longest line in characters,
  // and the lines each language shows by default and at most.
  const CHAR_WIDTH_EM = 0.55;
  const PADDING_EM = 0.8;
  const LINE_HEIGHT = 1.15;
  const MIN_LINE_CHARS = 8;
  const MAX_LINE_CHARS = 84;
  const DEFAULT_SUBTITLE_LINES = 2;
  const MAX_SUBTITLE_LINES = 3;
  // How much taller a line may grow, as one with an emoji does, before it counts as two.
  const LINE_SLACK = 1.25;
  // Words as wide as everyday speech, repeated to fill each line of subtitles a box holds.
  const SAMPLE_WORDS = "Good morning everyone, and welcome to the stream. Today we plan the next sprint together. ";

  /**
   * The largest scale from MIN_TEXT_SCALE to `most` at which `fits(scale)` holds, by bisection; the smallest if
   * none.
   */
  function fitScale(fits, most = 1) {
    if (fits(most)) {
      return most;
    }
    let low = MIN_TEXT_SCALE;
    let high = most;
    for (let step = 0; step < FIT_STEPS; step += 1) {
      const middle = (low + high) / 2;
      if (fits(middle)) {
        low = middle;
      } else {
        high = middle;
      }
    }
    return Math.floor(low * 1000) / 1000;
  }

  /** How many lines `content` takes in a text box's own look when each line fits on one; null when one wraps. */
  function ownLines(content, width, fontSize) {
    const room = width - PADDING_EM * fontSize;
    const lines = String(content).split("\n");
    return lines.every((line) => Array.from(line).length * CHAR_WIDTH_EM * fontSize <= room) ? lines.length : null;
  }

  /** The lines of subtitles each language shows, from a snapshot's subtitle settings: 1 to 3, 2 unless set. */
  function subtitleLines(snapshot) {
    const status = snapshot && snapshot.subtitles && snapshot.subtitles.status;
    const lines = status && Number.isInteger(status.lines) ? status.lines : DEFAULT_SUBTITLE_LINES;
    return Math.min(Math.max(lines, 1), MAX_SUBTITLE_LINES);
  }

  /**
   * The most subtitles a text box shows, as the core wraps them: as many lines as `lines` asks for and the box is
   * tall enough for, each as many characters as fit on a line. `width` is what a label beside them leaves, and a
   * line with a Latin line under it is `lineSize` lines tall.
   */
  function subtitleSample(width, height, fontSize, lines, lineSize = 1) {
    const chars = Math.floor((width - fontSize * PADDING_EM) / (fontSize * CHAR_WIDTH_EM));
    const perLine = Number.isFinite(chars) ? Math.min(Math.max(chars, MIN_LINE_CHARS), MAX_LINE_CHARS) : MAX_LINE_CHARS;
    const tall = Math.floor(height / (fontSize * LINE_HEIGHT * lineSize) + 1e-9);
    const count = Math.min(lines, Number.isFinite(tall) && tall >= 1 ? tall : 1);
    // A line never ends in a space, which takes no room at the end of a line.
    const line = SAMPLE_WORDS.repeat(Math.ceil(MAX_LINE_CHARS / SAMPLE_WORDS.length)).slice(0, perLine);
    return Array(count).fill(line.endsWith(" ") ? `${line.slice(0, -1)}s` : line).join("\n");
  }

  /**
   * Whether the text node's current words fit at `scale`: not taller than the room, nor wrapped by the theme. A note
   * a theme shows under the words, such as a vote card's count, takes a line of its own. Words with Latin lines under
   * them take `units` lines, as render-subtitles.js counts them.
   */
  function fitsAt(state, room, scale, units) {
    const { text } = state;
    const style = state.applied;
    const fontSize = parseFloat(style["font-size"]);
    const note = text.getAttribute("data-note") ? 1 : 0;
    const words = ownLines(text.textContent, parseFloat(style.width), fontSize);
    const lines = units || (words === null ? null : words + note);
    text.style.setProperty("font-size", `${scale}em`);
    const own = lines === null ? Infinity : lines * LINE_HEIGHT * LINE_SLACK * fontSize * scale;
    return text.offsetHeight <= Math.min(room(), own) + 0.5 && text.scrollWidth <= text.clientWidth + 0.5;
  }

  /** Shows the text at `scale`, or at its own size. */
  function applyScale(text, scale) {
    if (scale < 1) {
      text.style.setProperty("font-size", `${scale}em`);
    } else {
      text.style.removeProperty("font-size");
    }
  }

  /**
   * Shrinks a shown, themed text element's text until it fits, only when something that changes its layout
   * changed. `look` holds the theme, the decoration unit, and the lines subtitles show; `live` marks text that
   * shows live subtitles, which keeps the size its box's full lines need, and shrinks further only for words wider
   * than those. Elsewhere, and where there is no layout (Node), the text keeps its size. An element that is not
   * laid out yet, such as one in a hidden page, is measured on a later render.
   */
  function fitText(state, look, live) {
    const style = state.applied;
    const shown = look.theme !== "none" && state.kind === "text" && style.display !== "none";
    const fontSize = parseFloat(style["font-size"]);
    const bar = state.subtitleFit || { roomEm: 0, lineSize: 1 };
    const width = parseFloat(style.width) - bar.roomEm * fontSize;
    const sample = shown && live
      ? subtitleSample(width, parseFloat(style.height), fontSize, look.lines, bar.lineSize)
      : null;
    const key = shown
      ? [look.theme, look.unit, state.node.classList.contains("sn-card"), style.width, style.height,
        style["font-size"], style["font-weight"], sample === null ? state.content : sample,
        state.tileKey || "", state.text.getAttribute("data-note") || ""].join("|")
      : "";
    const remeasure = state.fitKey !== key;
    if (!remeasure && (sample === null || state.fitContent === state.content)) {
      return;
    }
    const { body, text } = state;
    const view = body.ownerDocument && body.ownerDocument.defaultView;
    if (remeasure) {
      state.fitKey = key;
      state.fitContent = null;
      text.style.removeProperty("font-size");
      if (!key || !view || typeof view.getComputedStyle !== "function") {
        return;
      }
      if (body.getClientRects().length === 0) {
        state.fitKey = null;
        return;
      }
    }
    const padding = view.getComputedStyle(body);
    const room = () => body.clientHeight - parseFloat(padding.paddingTop) - parseFloat(padding.paddingBottom);
    if (remeasure && sample !== null) {
      text.textContent = sample;
      state.boxScale = fitScale((candidate) => fitsAt(state, room, candidate, null));
      if (state.refill) {
        state.refill();
      } else {
        text.textContent = state.content;
      }
    }
    state.fitContent = state.content;
    const fits = (candidate) => fitsAt(state, room, candidate, state.lineUnits);
    applyScale(text, fitScale(fits, sample === null ? 1 : state.boxScale));
  }

  const api = { fitScale, fitText, ownLines, subtitleLines, subtitleSample };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  } else {
    window.SceneuronFit = api;
  }
})();
