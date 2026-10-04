// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Key and touch strip images as SVG, in Sceneuron's Graphite look: flat graphite keys with a warm white line icon
 * at about half the key and a short label under it, or the card's own words when they are its face, and solid amber
 * with dark ink while lit. Stream Deck renders a small SVG subset, so drawings use only rectangles, circles, paths,
 * and plain text with explicit positions.
 */

import { STROKE_WIDTH } from "./icons.ts";
import { LINE_HEIGHT, ellipsize, fitText, type FitBox } from "./text-fit.ts";

/** Keys are drawn at the @2x size; Stream Deck scales them for each device. */
const KEY_SIZE = 144;
const STRIP_WIDTH = 200;
const STRIP_HEIGHT = 100;
/** The corner radius, 8 pixels on a 72-pixel key. */
const RADIUS = 16;
/** The largest icon or emoji, half the key. */
const FIGURE = 72;

const FONT = "Helvetica Neue, Helvetica, Arial, sans-serif";
/** The Graphite palette: the canvas around a key, the key, its edge, its type, and the accent. */
const CANVAS = "#1c1c1e";
const GRAPHITE = "#2a2a2c";
const EDGE = "#3a3a3c";
const TEXT = "#f2f0ed";
const MUTED = "#b9b5ae";
const AMBER = "#d6a76c";
const INK = "#1c1c1e";

/** What any drawing shows. */
export interface Surface {
  /** The words: the label under an icon, or the face itself. */
  readonly title: string;
  /** Amber, for a raised card, a shown element, or a held key. */
  readonly lit: boolean;
  /** Darkened, while the control cannot act. */
  readonly dim: boolean;
}

/** What one key shows. */
export interface KeyFace extends Surface {
  /** A short line along the bottom. */
  readonly caption?: string;
  /** Whether the caption asks for something to be done, drawn in amber. */
  readonly alert?: boolean;
  /** A short line along the top. */
  readonly tag?: string;
  /** A line icon's shapes in a 24 by 24 box (see icons.ts), drawn above the title. */
  readonly icon?: string;
  /** An emoji drawn above the title, in place of an icon. */
  readonly emoji?: string;
}

/** What a dial's touch strip shows: a card with a header line above it. */
export interface StripFace extends Surface {
  /** Left side of the header, such as the deck name. */
  readonly header: string;
  /** Right side of the header, such as "3/10". */
  readonly position: string;
}

/** `text` safe inside an SVG element or attribute. */
export function escapeXml(text: string): string {
  return text.replace(/[&<>"']/g, (char) => `&#${char.charCodeAt(0)};`);
}

/** Removes control characters, which XML cannot carry. */
function printable(text: string): string {
  return text.replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, "").replace(/[\t\n\r]+/g, " ");
}

function round(value: number): number {
  return Math.round(value * 10) / 10;
}

function text(x: number, y: number, size: number, anchor: string, color: string, body: string): string {
  return (
    `<text x="${x}" y="${round(y)}" font-family="${FONT}" font-size="${size}" font-weight="bold" ` +
    `text-anchor="${anchor}" fill="${color}">${escapeXml(body)}</text>`
  );
}

/** Lines of `body` centered in `box`, whose top-left corner is at (`left`, `top`). */
function block(body: string, left: number, top: number, box: FitBox, color: string): string {
  const { size, lines } = fitText(printable(body), box);
  const height = lines.length * size * LINE_HEIGHT;
  const first = top + (box.height - height) / 2 + size * 0.86;
  return lines
    .map((line, index) => text(left + box.width / 2, first + index * size * LINE_HEIGHT, size, "middle", color, line))
    .join("");
}

/** A single line of small text, cut short to fit `width`. */
function line(body: string, x: number, y: number, size: number, anchor: string, width: number, color: string): string {
  return text(x, y, size, anchor, color, ellipsize(printable(body), width / size));
}

/** The type colors of a face: the words and figure, and the small lines. */
function inks(face: Surface): { main: string; small: string } {
  return face.lit ? { main: INK, small: INK } : { main: TEXT, small: MUTED };
}

/**
 * The drawing: the canvas, the rounded key, `body`, the shade while dim, and `after`, which stays clear of the
 * shade so a key that cannot act still says why.
 */
function wrapSvg(width: number, height: number, face: Surface, body: string, after = ""): string {
  const key = face.lit
    ? `<rect width="${width}" height="${height}" rx="${RADIUS}" fill="${AMBER}"/>`
    : `<rect x="1" y="1" width="${width - 2}" height="${height - 2}" rx="${RADIUS - 1}" fill="${GRAPHITE}" ` +
      `stroke="${EDGE}" stroke-width="2"/>`;
  const shade = face.dim
    ? `<rect width="${width}" height="${height}" rx="${RADIUS}" fill="${CANVAS}" fill-opacity="0.6"/>`
    : "";
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">` +
    `<rect width="${width}" height="${height}" fill="${CANVAS}"/>` +
    key +
    body +
    shade +
    after +
    `</svg>`
  );
}

/** The caption along the bottom, amber when it asks for something to be done on an unlit key. */
function caption(face: KeyFace, y: number): string {
  if (!face.caption) {
    return "";
  }
  const color = face.alert && !face.lit ? AMBER : inks(face).small;
  return line(face.caption, KEY_SIZE / 2, y, 16, "middle", 124, color);
}

/** A 144x144 key: an icon or emoji above its label, or else its words large in the middle. */
export function keySvg(face: KeyFace): string {
  if (face.icon || face.emoji) {
    return figureKeySvg(face);
  }
  const { main, small } = inks(face);
  const top = face.tag ? 34 : 14;
  const bottom = face.caption ? 112 : 130;
  const box = { width: 116, height: bottom - top, maxSize: 52, minSize: 15, maxLines: 4 };
  const tag = face.tag ? line(face.tag, 72, 26, 16, "middle", 120, small) : "";
  return wrapSvg(KEY_SIZE, KEY_SIZE, face, tag + block(face.title, 14, top, box, main), caption(face, 132));
}

/**
 * A key with a figure, a line icon or an emoji, at most half the key, above a label of one or two lines, between
 * the tag and the caption. The label is at least 17 pixels, 8.5 on a 72-pixel key, and 22 to 24 when it is a word or
 * two.
 */
function figureKeySvg(face: KeyFace): string {
  const { main, small } = inks(face);
  const top = face.tag ? 32 : 10;
  const bottom = face.caption ? 116 : 136;
  const label = fitText(printable(face.title), { width: 128, height: 48, maxSize: 24, minSize: 17, maxLines: 2 });
  const labelHeight = label.lines.length * label.size * LINE_HEIGHT;
  const gap = label.lines.length > 0 ? 6 : 0;
  const size = Math.min(FIGURE, bottom - top - labelHeight - gap);
  const start = top + (bottom - top - size - gap - labelHeight) / 2;
  const figure = face.icon
    ? `<g transform="translate(${round(72 - size / 2)} ${round(start)}) scale(${round(size / 24)})" fill="none" ` +
      `stroke="${main}" stroke-width="${STROKE_WIDTH}" stroke-linecap="round" stroke-linejoin="round">` +
      `${face.icon}</g>`
    : text(72, start + size * 0.86, Math.round(size * 0.86), "middle", TEXT, printable(face.emoji ?? ""));
  const first = start + size + gap + label.size * 0.86;
  const words = label.lines
    .map((body, index) => text(72, first + index * label.size * LINE_HEIGHT, label.size, "middle", main, body))
    .join("");
  const body = (face.tag ? line(face.tag, 72, 25, 16, "middle", 120, small) : "") + figure + words;
  return wrapSvg(KEY_SIZE, KEY_SIZE, face, body, caption(face, 134));
}

/** A 200x100 touch strip: a header line, then the card fitted below it. */
export function stripSvg(face: StripFace): string {
  const { main, small } = inks(face);
  const box = { width: 172, height: 64, maxSize: 40, minSize: 12, maxLines: 2 };
  const body =
    line(face.header, 16, 26, 14, "start", 124, small) +
    line(face.position, 184, 26, 14, "end", 44, small) +
    block(face.title, 14, 30, box, main);
  return wrapSvg(STRIP_WIDTH, STRIP_HEIGHT, face, body);
}

/** An SVG as the base64 data URL that `setImage` and pixmap feedback take. */
export function svgDataUrl(svg: string): string {
  return `data:image/svg+xml;base64,${Buffer.from(svg, "utf8").toString("base64")}`;
}
