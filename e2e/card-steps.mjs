// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// The card part of the end-to-end proof: a pack added through the project API, or the same deck of
// the starter project, is raised and lowered with obs-websocket vendor requests and with the
// hotkeys Sceneuron registers for it, and the deck and its hotkeys come back after a restart.
// Results are read back with the vendor GetState. The card scenario and the restart check record
// failures as failed checks instead of throwing them, so the rest of the proof still runs.

const DECK = "poker";
const SLOT = "poker-card";
const CARD_HOTKEY = "sceneuron.card.poker.p8";
const LOWER_HOTKEY = "sceneuron.deck.poker.lower";

/** The project with a pack's elements and decks added; a pack whose ids are taken is refused. */
export function withPack(project, pack) {
  const decks = project.decks ?? [];
  const clash =
    pack.elements.find((element) => project.elements.some((other) => other.id === element.id)) ??
    pack.decks.find((deck) => decks.some((other) => other.id === deck.id));
  if (clash) throw new Error(`${pack.name} needs the id ${clash.id}, which the project already uses`);
  return { ...project, elements: [...project.elements, ...pack.elements], decks: [...decks, ...pack.decks] };
}

/**
 * The project with a pack's elements and decks added, unless it has every one of them already, as the starter project
 * of a fresh install does; a pack whose ids are only partly taken is refused.
 */
export function withPackOnce(project, pack) {
  const decks = project.decks ?? [];
  const has =
    pack.elements.every((element) => project.elements.some((other) => other.id === element.id)) &&
    pack.decks.every((deck) => decks.some((other) => other.id === deck.id));
  return has ? project : withPack(project, pack);
}

/** The exact state a vendor reply carries; an error reply throws. */
export function exactState(reply) {
  if (reply.error !== undefined) throw new Error(reply.error);
  return JSON.parse(reply.state_json);
}

/** The id of a deck's raised card, or null while it is lowered. */
export function raisedCard(state, deck) {
  const found = state.decks?.find((d) => d.id === deck);
  if (!found) throw new Error(`the state has no deck ${deck}`);
  return found.raised ?? null;
}

/** Waits until both of the pack's hotkeys are registered; false when they never are. */
function hotkeysRegistered(ws, waitFor, what) {
  return waitFor(what, async () => {
    const { hotkeys } = await ws.request("GetHotkeyList");
    return hotkeys.includes(CARD_HOTKEY) && hotkeys.includes(LOWER_HOTKEY);
  }, 15_000).catch(() => false);
}

/** Runs the card scenario against a running OBS; `t` holds the harness helpers. */
export async function runCardScenario(t) {
  try {
    await cardSteps(t);
  } catch (error) {
    t.check("Cards", "the card scenario ran to the end", false, error.message);
  }
}

async function cardSteps(t) {
  const { api, ws, check, waitFor, element } = t;
  const state = async () => exactState(await ws.vendor("GetState"));
  const until = (what, wanted) =>
    waitFor(what, async () => {
      const current = await state();
      return raisedCard(current, DECK) === wanted ? current : null;
    }, 10_000).catch(() => null);

  const packs = await api("GET", "/api/packs").catch((error) => error);
  const pack = Array.isArray(packs) ? packs.find((p) => p?.id === DECK) : undefined;
  check("Cards", "the editor API offers the poker pack", pack, packs instanceof Error ? packs.message : "");
  if (!pack) return;
  await api("PUT", "/api/project", withPackOnce(await api("GET", "/api/project"), pack));
  const raised = exactState(await ws.vendor("RaiseCard", { deck: DECK, card: "p5" }));
  const read = await state();
  check("Cards", "a pack added through the project API raises a card with the vendor RaiseCard",
    raisedCard(raised, DECK) === "p5" && raisedCard(read, DECK) === "p5" &&
      element(read, SLOT).props.visible && element(read, SLOT).props.text === "5");
  const refused = await ws.vendor("RaiseCard", { deck: DECK, card: "p5", colour: "red" });
  check("Cards", "card requests refuse unknown fields",
    refused.error?.includes("unknown field") && refused.state_json === undefined, refused.error);
  const stepped = exactState(await ws.vendor("StepCard", { deck: DECK, step: 1 }));
  check("Cards", "the vendor StepCard raises the next card", raisedCard(stepped, DECK) === "p8");
  await ws.vendor("LowerCard", {});
  const lowered = await state();
  check("Cards", "the vendor LowerCard with no deck lowers the card",
    raisedCard(lowered, DECK) === null && !element(lowered, SLOT).props.visible);

  const registered = await hotkeysRegistered(ws, waitFor, "the pack's hotkeys");
  check("Cards", "adding a pack registers its card and lower hotkeys", registered);
  if (!registered) return;
  await ws.request("TriggerHotkeyByName", { hotkeyName: CARD_HOTKEY });
  const byHotkey = await until("the card raised by its hotkey", "p8");
  check("Cards", "a card hotkey raises its card", byHotkey && element(byHotkey, SLOT).props.text === "8");
  await ws.request("TriggerHotkeyByName", { hotkeyName: CARD_HOTKEY });
  check("Cards", "pressing the card hotkey again lowers the card", await until("the card lowered by its hotkey", null));
  exactState(await ws.vendor("RaiseCard", { deck: DECK, card: "p3" }));
  await ws.request("TriggerHotkeyByName", { hotkeyName: LOWER_HOTKEY });
  check("Cards", "the deck's lower hotkey lowers its card", await until("the deck lowered by its hotkey", null));
}

/** After a restart: the deck is still in the project and its hotkeys are registered again. */
export async function checkCardsRestored(t) {
  const { api, ws, check, waitFor } = t;
  const name = "the deck and its hotkeys come back after a restart";
  try {
    const project = await api("GET", "/api/project");
    const registered = await hotkeysRegistered(ws, waitFor, "card hotkeys after a restart");
    check("Cards", name, project.decks?.some((d) => d.id === DECK) && registered);
  } catch (error) {
    check("Cards", name, false, error.message);
  }
}
