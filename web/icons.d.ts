// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/** Types of icons.js, for the Stream Deck plugin, which bundles it so its keys draw the overlay's icons. */

type Shape = readonly [tag: "path" | "circle" | "rect", attributes: Readonly<Record<string, number | string>>];

/** How a card or element shows as an icon; see icons.js. */
interface IconChoice {
  readonly icon: string | null;
  readonly emoji: string | null;
  readonly text: string;
}

interface CardIconChoice extends IconChoice {
  readonly label: string;
}

declare const icons: {
  readonly STROKE_WIDTH: number;
  readonly FALLBACK: string;
  readonly ICONS: Readonly<Record<string, readonly Shape[]>>;
  readonly CARDS: Readonly<Record<string, Readonly<Record<string, readonly [icon: string, label: string] | null>>>>;
  readonly ELEMENTS: Readonly<Record<string, string>>;
  iconMarkup(name: string): string;
  leadingEmoji(text: string): { readonly emoji: string; readonly rest: string } | null;
  cardIcon(deck: string, card: string, text: string): CardIconChoice;
  elementIcon(id: string, text: string): IconChoice;
};

export = icons;
