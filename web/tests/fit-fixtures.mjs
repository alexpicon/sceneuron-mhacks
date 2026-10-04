// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Shared fixtures for the text fitting tests: a text element in a fake layout, the font size fitText shows it at,
// and what fitText draws with.

const LINE = 1.15;
/** What fitText draws with: the theme, its decoration unit, and the lines subtitles show. */
export const look = (theme, unit = "1px") => ({ theme, unit, lines: 2 });

/** A style with just the calls fitText makes. */
function fakeStyle() {
  const values = new Map();
  return {
    values,
    setProperty: (name, value) => values.set(name, String(value)),
    removeProperty: (name) => values.delete(name),
  };
}

/**
 * A shown text element whose words take `linesAt(scale, words)` lines in a body with `room` pixels for them;
 * `reads` counts the layout reads. `laidOut` false stands for a node no page lays out yet; `note` is a note the
 * theme shows under the words, as on a vote card.
 */
export function fakeState(options = {}) {
  const { content = "Wrap it up", fontSize = 72, width = 1160, height = 180, room = 120, note = null } = options;
  const linesAt = options.linesAt || (() => 1);
  const reads = { count: 0 };
  const text = {
    style: fakeStyle(),
    textContent: content,
    clientWidth: width,
    scrollWidth: width,
    getAttribute: (name) => (name === "data-note" ? note : null),
    get offsetHeight() {
      reads.count += 1;
      const scale = parseFloat(this.style.values.get("font-size") || "1");
      return linesAt(scale, this.textContent) * LINE * fontSize * scale;
    },
  };
  const view = { getComputedStyle: () => ({ paddingTop: "12px", paddingBottom: "8px" }) };
  const body = { clientHeight: room + 20, laidOut: true, ownerDocument: { defaultView: view } };
  body.getClientRects = () => (body.laidOut ? [{}] : []);
  const applied = { display: "block", width: `${width}px`, height: `${height}px`, "font-size": `${fontSize}px` };
  const state = { kind: "text", content, applied, text, body, node: { classList: { contains: () => false } } };
  return { state, reads };
}

export const shown = (state) => state.text.style.values.get("font-size") ?? null;
