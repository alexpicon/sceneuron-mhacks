// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * The control view: the small state Sceneuron shares with a control token. It names decks,
 * cards, and elements and says what is showing, and nothing about the camera.
 */

export type RaisedBy = "manual" | "camera";

export interface ControlCard {
  readonly id: string;
  readonly name: string;
  readonly text: string;
  readonly color: string | null;
  readonly background: string | null;
}

export interface ControlDeck {
  readonly id: string;
  readonly name: string;
  readonly raised: string | null;
  readonly raised_by: RaisedBy | null;
  readonly hide_at_ms: number | null;
  /** The raised card shows a cover on the overlay, such as a hidden vote in a shared room. */
  readonly covered: boolean;
  readonly cards: readonly ControlCard[];
}

export interface ControlElement {
  readonly id: string;
  readonly name: string;
  readonly visible: boolean;
  readonly held: boolean;
}

/** Voice control: whether it is on, and whether push to talk is held anywhere. */
export interface ControlVoice {
  readonly enabled: boolean;
  readonly talking: boolean;
}

/** The shared room as counts only: whether one is joined, revealed, and how many have voted. */
export interface ControlRoom {
  readonly joined: boolean;
  readonly revealed: boolean;
  readonly voted: number;
  readonly people: number;
}

/** The reaction playing on the overlay when the view was taken: its preset, and the time it has left. */
export interface ControlFlood {
  readonly flood: string;
  readonly nonce: number;
  readonly left_ms: number;
}

export interface ControlView {
  readonly revision: number;
  readonly automation_enabled: boolean;
  readonly decks: readonly ControlDeck[];
  readonly elements: readonly ControlElement[];
  /** Present only when Sceneuron shares it; keys that need it show less without it. */
  readonly voice?: ControlVoice;
  /** Present only when Sceneuron shares it; see `voice`. */
  readonly room?: ControlRoom;
  /** Present only while a reaction plays. */
  readonly flood?: ControlFlood;
}

type Fields = Record<string, unknown>;

function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function optionalString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function card(value: unknown): ControlCard | null {
  if (!isObject(value) || typeof value.id !== "string" || typeof value.name !== "string") {
    return null;
  }
  return {
    id: value.id,
    name: value.name,
    text: typeof value.text === "string" ? value.text : "",
    color: optionalString(value.color),
    background: optionalString(value.background),
  };
}

function deck(value: unknown): ControlDeck | null {
  if (!isObject(value) || typeof value.id !== "string" || typeof value.name !== "string") {
    return null;
  }
  const raised = value.raised;
  if (!Array.isArray(value.cards) || (raised !== null && typeof raised !== "string")) {
    return null;
  }
  const cards = value.cards.map(card);
  if (cards.some((entry) => entry === null)) {
    return null;
  }
  const raisedBy = value.raised_by === "manual" || value.raised_by === "camera" ? value.raised_by : null;
  return {
    id: value.id,
    name: value.name,
    raised,
    raised_by: raisedBy,
    hide_at_ms: typeof value.hide_at_ms === "number" ? value.hide_at_ms : null,
    covered: value.covered === true,
    cards: cards as ControlCard[],
  };
}

function element(value: unknown): ControlElement | null {
  if (!isObject(value) || typeof value.id !== "string" || typeof value.name !== "string") {
    return null;
  }
  if (typeof value.visible !== "boolean" || typeof value.held !== "boolean") {
    return null;
  }
  return { id: value.id, name: value.name, visible: value.visible, held: value.held };
}

function count(value: unknown): number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 ? value : 0;
}

function voice(value: unknown): ControlVoice | null {
  if (!isObject(value) || typeof value.enabled !== "boolean" || typeof value.talking !== "boolean") {
    return null;
  }
  return { enabled: value.enabled, talking: value.talking };
}

function room(value: unknown): ControlRoom | null {
  if (!isObject(value) || typeof value.joined !== "boolean") {
    return null;
  }
  return {
    joined: value.joined,
    revealed: value.revealed === true,
    voted: count(value.voted),
    people: count(value.people),
  };
}

function flood(value: unknown): ControlFlood | null {
  if (!isObject(value) || typeof value.flood !== "string" || typeof value.nonce !== "number") {
    return null;
  }
  const left = value.left_ms;
  return { flood: value.flood, nonce: value.nonce, left_ms: typeof left === "number" && left > 0 ? left : 0 };
}

/**
 * A control view read from JSON; null when `value` does not have its shape. Every deck must say
 * which card is raised and every element whether it is visible and held, so a full snapshot,
 * whose elements carry `props` instead, is never read as one and never blanks the keys. The voice,
 * room, and flood parts are optional: one that is missing or misshapen is left out, not the whole view.
 */
export function parseControlView(value: unknown): ControlView | null {
  if (!isObject(value) || typeof value.revision !== "number" || typeof value.automation_enabled !== "boolean") {
    return null;
  }
  if (!Array.isArray(value.decks) || !Array.isArray(value.elements)) {
    return null;
  }
  const decks = value.decks.map(deck);
  const elements = value.elements.map(element);
  if (decks.some((entry) => entry === null) || elements.some((entry) => entry === null)) {
    return null;
  }
  const voiced = voice(value.voice);
  const roomed = room(value.room);
  const playing = flood(value.flood);
  return {
    revision: value.revision,
    automation_enabled: value.automation_enabled,
    decks: decks as ControlDeck[],
    elements: elements as ControlElement[],
    ...(voiced ? { voice: voiced } : {}),
    ...(roomed ? { room: roomed } : {}),
    ...(playing ? { flood: playing } : {}),
  };
}

/** A control view read from JSON text, such as an event's data; null when it is not one. */
export function readControlView(text: string): ControlView | null {
  try {
    return parseControlView(JSON.parse(text));
  } catch {
    return null;
  }
}

export function findDeck(view: ControlView | null, id: string | undefined): ControlDeck | undefined {
  return id ? view?.decks.find((entry) => entry.id === id) : undefined;
}

export function findCard(deck: ControlDeck | undefined, id: string | undefined | null): ControlCard | undefined {
  return id ? deck?.cards.find((entry) => entry.id === id) : undefined;
}

export function findElement(view: ControlView | null, id: string | undefined): ControlElement | undefined {
  return id ? view?.elements.find((entry) => entry.id === id) : undefined;
}
