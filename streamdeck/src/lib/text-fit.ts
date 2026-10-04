// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Fits card text into a box without font metrics. Widths are estimates in ems for a bold
 * sans-serif face, generous on purpose: text that comes out a little small still reads, text
 * that runs off a key does not.
 */

/** Where text may go, in pixels, and the font sizes to try. */
export interface FitBox {
  readonly width: number;
  readonly height: number;
  readonly maxSize: number;
  readonly minSize: number;
  readonly maxLines: number;
}

/** The chosen font size and the lines to draw. */
export interface FittedText {
  readonly size: number;
  readonly lines: readonly string[];
}

/** Line advance as a multiple of the font size. */
export const LINE_HEIGHT = 1.15;

const ELLIPSIS = "…";
const graphemes = new Intl.Segmenter("en", { granularity: "grapheme" });
const PICTOGRAPH = /\p{Extended_Pictographic}|\p{Regional_Indicator}/u;
const WIDE = /[ᄀ-ᅟ⺀-꓏가-힣豈-﫿︰-﹏＀-｠￠-￦]/u;

/** Estimated advance of one grapheme, in ems. */
export function graphemeWidth(grapheme: string): number {
  if (PICTOGRAPH.test(grapheme)) {
    return 1.25;
  }
  if (WIDE.test(grapheme)) {
    return 1.05;
  }
  if (grapheme === " ") {
    return 0.3;
  }
  if ("il.,:;'|!".includes(grapheme)) {
    return 0.32;
  }
  if ("fjrtI()[]-".includes(grapheme)) {
    return 0.42;
  }
  if ("mwMW".includes(grapheme)) {
    return 0.95;
  }
  if (/[A-Z]/.test(grapheme)) {
    return 0.74;
  }
  return 0.62;
}

/** Estimated width of `text`, in ems. */
export function measure(text: string): number {
  let width = 0;
  for (const { segment } of graphemes.segment(text)) {
    width += graphemeWidth(segment);
  }
  return width;
}

/** Splits `text` into graphemes so emoji sequences and flags stay whole. */
function split(text: string): string[] {
  return Array.from(graphemes.segment(text), ({ segment }) => segment);
}

/** Lines no wider than `maxEm`; `broke` tells whether a word had to be split to get there. */
function wrap(words: readonly string[], maxEm: number): { lines: string[]; broke: boolean } {
  const lines: string[] = [];
  let line = "";
  let broke = false;
  for (const word of words) {
    const candidate = line ? `${line} ${word}` : word;
    if (measure(candidate) <= maxEm) {
      line = candidate;
      continue;
    }
    if (line) {
      lines.push(line);
    }
    line = word;
    if (measure(word) > maxEm) {
      broke = true;
      line = "";
      for (const grapheme of split(word)) {
        if (line && measure(line + grapheme) > maxEm) {
          lines.push(line);
          line = "";
        }
        line += grapheme;
      }
    }
  }
  if (line) {
    lines.push(line);
  }
  return { lines, broke };
}

/** `line` shortened until it fits `maxEm` with an ellipsis at the end. */
export function ellipsize(line: string, maxEm: number): string {
  if (measure(line) <= maxEm) {
    return line;
  }
  const parts = split(line);
  while (parts.length > 0 && measure(`${parts.join("").trimEnd()}${ELLIPSIS}`) > maxEm) {
    parts.pop();
  }
  return `${parts.join("").trimEnd()}${ELLIPSIS}`;
}

/**
 * The largest size at which `text` fits `box` without splitting a word. When none does, the
 * smallest size, with words split as needed and the last line cut short with an ellipsis.
 */
export function fitText(text: string, box: FitBox): FittedText {
  const words = text.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) {
    return { size: box.maxSize, lines: [] };
  }
  for (let size = box.maxSize; size >= box.minSize; size -= 1) {
    const { lines, broke } = wrap(words, box.width / size);
    if (!broke && lines.length <= box.maxLines && lines.length * size * LINE_HEIGHT <= box.height) {
      return { size, lines };
    }
  }
  const size = box.minSize;
  const limit = Math.max(1, Math.min(box.maxLines, Math.floor(box.height / (size * LINE_HEIGHT))));
  const { lines } = wrap(words, box.width / size);
  if (lines.length <= limit) {
    return { size, lines };
  }
  const kept = lines.slice(0, limit);
  kept[limit - 1] = ellipsize(`${kept[limit - 1]}${ELLIPSIS}`, box.width / size);
  return { size, lines: kept };
}
