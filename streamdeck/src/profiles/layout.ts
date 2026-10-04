// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * The ready-made layout for the 15-key Stream Deck: four pages of five columns and three rows.
 * The bottom row is alike on every page: the corners go to the previous and next page, in a cycle
 * that wraps around, and push to talk is second. The build writes every bundled profile from this
 * file alone (see rollup-profiles.mjs), so a profile never drifts from it: one for the 15-key
 * Stream Deck, and one for the Stream Deck XL that puts the same keys in its top-left five by
 * three keys and leaves the rest empty, to try the 15-key layout on an XL.
 */

/** A Sceneuron key, named by its action's UUID suffix, with the settings it is saved with. */
export type SceneuronKey =
  | { readonly kind: "card"; readonly deck: string; readonly card: string }
  | { readonly kind: "lower"; readonly deck: string }
  | { readonly kind: "reaction"; readonly reaction: string }
  | { readonly kind: "talk" | "reveal" | "next-round" | "automation" };

/** One of Stream Deck's own page keys: go to page `page`, counting from 1, or show the page number. */
export type PageKey =
  | { readonly kind: "go-to-page"; readonly page: number; readonly title: string }
  | { readonly kind: "page-indicator" };

export type LayoutKey = SceneuronKey | PageKey;

/** A page: its name in Stream Deck, and its keys row by row, top to bottom, left to right; `null`
 * leaves a key empty. */
export interface LayoutPage {
  readonly name: string;
  readonly rows: readonly (readonly (LayoutKey | null)[])[];
}

/** The 15-key Stream Deck's keys. */
export const COLUMNS = 5;
export const ROWS = 3;

const card = (deck: string, id: string): LayoutKey => ({ kind: "card", deck, card: id });
const meeting = (id: string): LayoutKey => card("meeting", id);
const poker = (id: string): LayoutKey => card("poker", id);
const status = (id: string): LayoutKey => card("status", id);
const reaction = (id: string): LayoutKey => ({ kind: "reaction", reaction: id });
const goTo = (page: number, title: string): LayoutKey => ({ kind: "go-to-page", page, title });
const TALK: LayoutKey = { kind: "talk" };
const AUTOMATION: LayoutKey = { kind: "automation" };

export const PAGES: readonly LayoutPage[] = [
  {
    name: "Meeting",
    rows: [
      [meeting("wrap-up"), meeting("off-track"), meeting("question"), meeting("offline"), meeting("next-topic")],
      [meeting("agree"), meeting("disagree"), meeting("cant-hear"), status("muted"), status("brb")],
      [goTo(4, "Reactions"), TALK, { kind: "lower", deck: "meeting" }, AUTOMATION, goTo(2, "Poker")],
    ],
  },
  {
    name: "Planning poker",
    rows: [
      [poker("p0"), poker("p1"), poker("p2"), poker("p3"), poker("p5")],
      [poker("p8"), poker("p13"), poker("p21"), poker("unsure"), poker("break")],
      [goTo(1, "Meeting"), TALK, { kind: "reveal" }, { kind: "next-round" }, goTo(3, "Status")],
    ],
  },
  {
    name: "Status",
    rows: [
      [status("muted"), status("brb"), status("lunch"), status("focus"), status("presenting")],
      [status("on-call"), { kind: "lower", deck: "status" }, null, null, null],
      [goTo(2, "Poker"), TALK, null, AUTOMATION, goTo(4, "Reactions")],
    ],
  },
  {
    name: "Reactions",
    rows: [
      [reaction("email"), reaction("mind-blown"), reaction("ship-it"), reaction("coffee"), reaction("tongue")],
      [
        reaction("absolutely-right"),
        reaction("more-tokens"),
        reaction("works-on-my-gpu"),
        reaction("meatproxy"),
        reaction("clanker"),
      ],
      [goTo(3, "Status"), TALK, null, AUTOMATION, goTo(1, "Meeting")],
    ],
  },
];

/** A bundled profile: one device type, with the layout in its top-left corner. */
export interface DeviceProfile {
  /** The file in the plugin folder, without its extension, as the manifest's `Profiles` names it. */
  readonly file: string;
  /** Stream Deck's device type: 0 for the 15-key Stream Deck, 2 for the Stream Deck XL. */
  readonly deviceType: number;
  /** The model Stream Deck's own default profile for this device type carries. */
  readonly model: string;
  /** The profile's name in Stream Deck. */
  readonly name: string;
  readonly columns: number;
  readonly rows: number;
}

export const PROFILES: readonly DeviceProfile[] = [
  {
    file: "profiles/sceneuron-stream-deck",
    deviceType: 0,
    model: "20GAA9902",
    name: "Sceneuron",
    columns: COLUMNS,
    rows: ROWS,
  },
  {
    file: "profiles/sceneuron-stream-deck-xl",
    deviceType: 2,
    model: "20GAT9902",
    name: "Sceneuron (15-key layout)",
    columns: 8,
    rows: 4,
  },
];
