// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Line icons for the keys, from the icons Sceneuron's overlay draws too (web/icons.js, which the build bundles):
 * each icon's shapes in a 24 by 24 box, stroked by whoever draws them, and which icon and short label each card of
 * the built-in packs shows. The action images in `imgs/actions/` show the same shapes, so a key looks the same in
 * Stream Deck's action list and on the device.
 */

import shared from "../../../web/icons.js";

export const { CARDS, STROKE_WIDTH, cardIcon, iconMarkup } = shared;

export type IconName = "talk" | "reveal" | "next-round" | "lower" | "pause" | "play" | "reaction" | "eye";

/** The shared icon each control draws. */
const SHARED: Readonly<Record<IconName, string>> = {
  talk: "mic",
  reveal: "reveal",
  "next-round": "next-round",
  lower: "lower",
  pause: "pause",
  play: "play",
  reaction: "reaction",
  eye: "eye",
};

export const ICONS: Readonly<Record<IconName, string>> = Object.fromEntries(
  Object.entries(SHARED).map(([name, icon]) => [name, iconMarkup(icon)]),
) as Record<IconName, string>;
