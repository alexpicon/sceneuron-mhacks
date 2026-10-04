// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * What each control shows for a control view and its settings, in the Graphite look: a line icon with a short
 * label, lit amber while its card is up or its state is on. Keys keep the last view while the plugin is offline and
 * draw it darkened, so a press that cannot work does not look live, and say why.
 */

import type { CardSettings, DialSettings, ElementSettings, LowerSettings } from "./settings.ts";
import type { KeyFace, StripFace } from "./svg.ts";
import { BUILT_IN_DECKS } from "./built-in-cards.ts";
import { ICONS, cardIcon, iconMarkup } from "./icons.ts";
import { type ControlCard, type ControlDeck, type ControlView, findCard, findDeck, findElement } from "./view.ts";

/** Whether the plugin can act: connected, waiting for Sceneuron, never paired, or turned away. */
export type Reach = "live" | "offline" | "unpaired" | "refused";

/** The caption that explains why a control is not live. */
const CAPTIONS: Record<Reach, string> = { live: "", offline: "Offline", unpaired: "Not paired", refused: "Pair again" };

export function reachCaption(reach: Reach): string {
  return CAPTIONS[reach];
}

/** The caption a key shows: `caption` while live, else why it is not, in amber when pairing again would fix it. */
export function status(reach: Reach, caption?: string): Pick<KeyFace, "caption" | "alert"> {
  return reach === "live" ? { caption } : { caption: reachCaption(reach), alert: reach === "refused" };
}

function plain(title: string, reach: Reach, caption?: string): KeyFace {
  return { title, ...status(reach, caption), lit: false, dim: reach !== "live" };
}

/**
 * A control key with its icon. Like a card before any view, it is drawn in full until the plugin is paired, and
 * darkened while Sceneuron is out of reach or turned it away.
 */
export function control(title: string, icon: string, reach: Reach, caption?: string): KeyFace {
  return { title, icon, ...status(reach, caption), lit: false, dim: reach === "offline" || reach === "refused" };
}

/** A configured name that the view does not have; darkened, since a press cannot work. */
function missing(title: string, view: ControlView | null, reach: Reach): KeyFace {
  return { ...plain(title, reach, view ? "Not found" : undefined), dim: true };
}

/** A card of the built-in packs, for keys drawn before Sceneuron has sent any view. */
function builtInCard(deckId: string, cardId: string): ControlCard | undefined {
  return BUILT_IN_DECKS.find((deck) => deck.id === deckId)?.cards.find((card) => card.id === cardId);
}

/** The text a card shows, or its name when it has none. */
function cardTitle(card: { readonly text: string; readonly name: string }): string {
  return card.text.trim() || card.name;
}

/** A card as its key shows it: its icon and short label, its emoji and words, or its words alone, large. */
function cardLook(deckId: string, card: ControlCard): Pick<KeyFace, "title" | "icon" | "emoji"> {
  const shown = cardIcon(deckId, card.id, cardTitle(card));
  if (shown.icon) {
    return { title: shown.label, icon: iconMarkup(shown.icon) };
  }
  return shown.emoji ? { title: shown.label, emoji: shown.emoji } : { title: shown.label };
}

/**
 * A card key: drawn as the card, lit while it is raised. While the overlay covers it, as a hidden vote in a shared
 * room, the key still shows the card to whoever presses it and says "Voted".
 */
export function cardFace(view: ControlView | null, reach: Reach, settings: CardSettings): KeyFace {
  if (!settings.deck || !settings.card) {
    return plain("Choose a card", reach);
  }
  const deck = findDeck(view, settings.deck);
  const card = findCard(deck, settings.card);
  const known = view ? undefined : builtInCard(settings.deck, settings.card);
  if (known) {
    // Before any view, a built-in card is drawn as itself, so a new layout reads at a glance, and so a link that is
    // up but has no view yet draws what the view will show; it is darkened only while Sceneuron is out of reach or
    // turned the plugin away, as live keys are.
    const dim = reach === "offline" || reach === "refused";
    return { ...cardLook(settings.deck, known), ...status(reach), lit: false, dim };
  }
  if (!deck || !card) {
    return missing(settings.card, view, reach);
  }
  const raised = deck.raised === card.id;
  return {
    ...cardLook(deck.id, card),
    ...status(reach, raised && deck.covered ? "Voted" : undefined),
    tag: raised && deck.raised_by === "camera" ? "Camera" : undefined,
    lit: raised,
    dim: reach !== "live",
  };
}

/** The decks a lower key acts on: one deck, or all of them when none is chosen. */
export function lowerScope(view: ControlView | null, settings: LowerSettings): readonly ControlDeck[] {
  if (settings.deck) {
    const deck = findDeck(view, settings.deck);
    return deck ? [deck] : [];
  }
  return view?.decks ?? [];
}

/** A lower key: lit while any card it would lower is up. */
export function lowerFace(view: ControlView | null, reach: Reach, settings: LowerSettings): KeyFace {
  const deck = findDeck(view, settings.deck);
  const known = view ? undefined : BUILT_IN_DECKS.find((entry) => entry.id === settings.deck);
  if (known) {
    return { ...control("Lower", ICONS.lower, reach), tag: known.name };
  }
  if (settings.deck && !deck) {
    return { ...missing("Lower", view, reach), tag: settings.deck };
  }
  const raised = lowerScope(view, settings).some((entry) => entry.raised !== null);
  return { ...control("Lower", ICONS.lower, reach), tag: deck ? deck.name : "All decks", lit: raised };
}

/** An element key: lit while the element shows, tagged while a manual change holds it. */
export function elementFace(view: ControlView | null, reach: Reach, settings: ElementSettings): KeyFace {
  if (!settings.element) {
    return plain("Choose an element", reach);
  }
  const element = findElement(view, settings.element);
  if (!element) {
    return missing(settings.element, view, reach);
  }
  return {
    ...control(element.name, ICONS.eye, reach, element.visible ? "Shown" : "Hidden"),
    tag: element.held ? "Held" : undefined,
    lit: element.visible,
    dim: reach !== "live",
  };
}

/** The automation key and its manifest state: 0 while automation runs, 1 while it is paused, lit. */
export function automationFace(view: ControlView | null, reach: Reach): { face: KeyFace; state: 0 | 1 } {
  if (!view) {
    return { face: control("Automation", ICONS.pause, reach), state: 0 };
  }
  const enabled = view.automation_enabled;
  const face = enabled
    ? control("Automation", ICONS.pause, reach, "On")
    : { ...control("Automation", ICONS.play, reach, "Paused"), lit: true };
  return { face, state: enabled ? 0 : 1 };
}

/**
 * A push to talk key: lit while push to talk is held. When Sceneuron shares voice control's state, that says
 * whether push to talk is held, from this plugin or anywhere else, so a hold Sceneuron let go of by itself does not
 * look held; the key also says whether voice control is on, and darkens while it is off, since a press then does
 * nothing. Without it, the key shows whether `held` says a key here holds push to talk.
 */
export function talkFace(view: ControlView | null, reach: Reach, held: boolean): KeyFace {
  const voice = view?.voice;
  const talking = voice ? voice.talking : held;
  const off = voice !== undefined && !voice.enabled;
  const face = control("Talk", ICONS.talk, reach, talking ? "Talking" : undefined);
  return {
    ...face,
    tag: voice ? (voice.enabled ? "Voice on" : "Voice off") : undefined,
    lit: talking,
    dim: face.dim || (off && !talking),
  };
}

/** The shared room keys. */
export type RoomKind = "reveal" | "next-round";

const ROOM_TITLES: Record<RoomKind, string> = { reveal: "Reveal votes", "next-round": "Next round" };

/**
 * A reveal or next round key. With the room's counts from Sceneuron it says how many voted, or "Revealed", lit, once
 * the votes show. Without them `noRoom` says whether a press just found no room. Either way a key with no room says
 * "No room" and darkens.
 */
export function roomFace(kind: RoomKind, view: ControlView | null, reach: Reach, noRoom: boolean): KeyFace {
  const title = ROOM_TITLES[kind];
  const icon = kind === "reveal" ? ICONS.reveal : ICONS["next-round"];
  const room = view?.room;
  if (room ? !room.joined : noRoom) {
    return { ...control(title, icon, reach, "No room"), dim: true };
  }
  if (!room) {
    return control(title, icon, reach);
  }
  // Short enough that two-digit counts still fit the caption line.
  const caption = room.revealed ? "Revealed" : `${room.voted}/${room.people} voted`;
  return { ...control(title, icon, reach, caption), lit: room.revealed };
}

/**
 * The card a dial stands on: the raised card; else, while the deck still has it, the one it stood on before, so a
 * press raises again what the last press lowered; else the first card.
 */
export function dialCard(deck: ControlDeck | undefined, last: string | null | undefined): string | null {
  const ids = deck?.cards.map((card) => card.id) ?? [];
  return [deck?.raised, last].find((id) => id && ids.includes(id)) ?? ids[0] ?? null;
}

/** A dial's touch strip: the card it stands on, lit while that card is up. */
export function dialFace(
  view: ControlView | null,
  reach: Reach,
  settings: DialSettings,
  last: string | null | undefined,
): StripFace {
  const caption = reachCaption(reach);
  const blank = { lit: false, dim: reach !== "live", position: "" };
  if (!settings.deck) {
    return { ...blank, header: caption || "Card dial", title: "Choose a deck" };
  }
  const deck = findDeck(view, settings.deck);
  if (!deck) {
    return { ...blank, header: caption || settings.deck, title: view && !caption ? "Deck not found" : settings.deck };
  }
  const card = findCard(deck, dialCard(deck, last));
  if (!card) {
    return { ...blank, header: caption || deck.name, title: "No cards" };
  }
  return {
    header: caption || deck.name,
    position: `${deck.cards.indexOf(card) + 1}/${deck.cards.length}`,
    title: cardTitle(card),
    lit: deck.raised === card.id,
    dim: reach !== "live",
  };
}
