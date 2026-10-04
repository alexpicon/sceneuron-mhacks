// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises what each control shows for a control view: card keys drawn as the card's icon and
// short label, or its words, and lit while raised, lower and element keys, the automation states,
// push to talk and voice control, the room keys with and without the room's counts, whose caption
// fits two-digit counts, offline, unpaired, and refused keys, names the view no longer has, and the
// card a dial stands on.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  automationFace,
  cardFace,
  dialCard,
  dialFace,
  elementFace,
  lowerFace,
  lowerScope,
  roomFace,
  talkFace,
} from "../src/lib/faces.ts";
import { iconMarkup } from "../src/lib/icons.ts";
import { keySvg } from "../src/lib/svg.ts";
import { sampleView } from "./fake-engine.mjs";

function raised(deckId, cardId, by = "manual") {
  const view = sampleView();
  const deck = view.decks.find((entry) => entry.id === deckId);
  deck.raised = cardId;
  deck.raised_by = by;
  return view;
}

test("a card key shows the card's icon and short label, lit only while it is up", () => {
  const view = raised("meeting", "wrap-up");
  const up = cardFace(view, "live", { deck: "meeting", card: "wrap-up" });
  assert.deepEqual([up.title, up.icon, up.emoji], ["Wrap", iconMarkup("timer"), undefined]);
  assert.equal(up.lit, true);
  assert.equal(up.dim, false);
  assert.equal(up.tag, undefined);
  const down = cardFace(view, "live", { deck: "meeting", card: "agree" });
  assert.deepEqual([down.title, down.icon], ["Agree", iconMarkup("thumbs-up")], "a card without text, by its name");
  assert.equal(down.lit, false);
  const value = cardFace(view, "live", { deck: "poker", card: "p5" });
  assert.deepEqual([value.title, value.icon, value.emoji], ["5", undefined, undefined], "a value is its own face");
});

test("a custom card shows its leading emoji, its words alone when short, or the fallback icon", () => {
  const view = raised("meeting", "wrap-up");
  view.decks[1].cards.push(
    { id: "cake", name: "Cake", text: "🎂 Cake time", color: null, background: null },
    { id: "pizza", name: "Pizza", text: "Pizza is here", color: null, background: null },
  );
  const cake = cardFace(view, "live", { deck: "meeting", card: "cake" });
  assert.deepEqual([cake.title, cake.emoji, cake.icon], ["Cake time", "🎂", undefined]);
  const pizza = cardFace(view, "live", { deck: "meeting", card: "pizza" });
  assert.deepEqual([pizza.title, pizza.icon], ["Pizza is here", iconMarkup("spark")]);
});

test("a card the camera raised says so", () => {
  const face = cardFace(raised("poker", "p3", "camera"), "live", { deck: "poker", card: "p3" });
  assert.equal(face.tag, "Camera");
});

test("a card covered as a hidden vote keeps its face and says Voted", () => {
  const view = raised("poker", "p5");
  view.decks[0].covered = true;
  const voted = cardFace(view, "live", { deck: "poker", card: "p5" });
  assert.deepEqual([voted.title, voted.caption, voted.lit], ["5", "Voted", true]);
  assert.equal(cardFace(view, "live", { deck: "poker", card: "p3" }).caption, undefined, "only the raised card");
  assert.equal(cardFace(view, "offline", { deck: "poker", card: "p5" }).caption, "Offline");
});

test("keys dim while offline, unpaired, or refused and say why", () => {
  const view = raised("poker", "p5");
  const offline = cardFace(view, "offline", { deck: "poker", card: "p5" });
  assert.equal(offline.dim, true);
  assert.equal(offline.caption, "Offline");
  assert.equal(offline.title, "5", "the last view still names the card");
  assert.equal(cardFace(null, "unpaired", { deck: "poker", card: "p5" }).caption, "Not paired");
  const refused = cardFace(view, "refused", { deck: "poker", card: "p5" });
  assert.deepEqual([refused.caption, refused.dim], ["Pair again", true]);
  assert.equal(lowerFace(view, "refused", {}).caption, "Pair again");
});

test("before any view, built-in cards are drawn as themselves, darkened unless never paired", () => {
  const wrap = cardFace(null, "unpaired", { deck: "meeting", card: "wrap-up" });
  assert.deepEqual(
    [wrap.title, wrap.icon, wrap.caption, wrap.alert, wrap.dim, wrap.lit],
    ["Wrap", iconMarkup("timer"), "Not paired", false, false, false],
  );
  const muted = cardFace(null, "refused", { deck: "status", card: "muted" });
  assert.deepEqual(
    [muted.title, muted.icon, muted.caption, muted.alert, muted.dim],
    ["Mute", iconMarkup("mic-off"), "Pair again", true, true],
  );
  const five = cardFace(null, "offline", { deck: "poker", card: "p5" });
  assert.deepEqual([five.title, five.icon, five.caption, five.dim], ["5", undefined, "Offline", true]);
  assert.equal(lowerFace(null, "unpaired", { deck: "meeting" }).caption, "Not paired");
  const custom = cardFace(null, "unpaired", { deck: "my-deck", card: "x" });
  assert.deepEqual([custom.title, custom.dim], ["x", true], "a card no pack has stays a placeholder");
});

test("unchosen and missing cards ask for attention instead", () => {
  assert.equal(cardFace(sampleView(), "live", {}).title, "Choose a card");
  const missing = cardFace(sampleView(), "live", { deck: "poker", card: "p13" });
  assert.equal(missing.title, "p13");
  assert.equal(missing.caption, "Not found");
  assert.equal(missing.dim, true);
  assert.equal(cardFace(null, "offline", { deck: "poker", card: "p13" }).caption, "Offline");
});

test("a lower key is lit while any card it would lower is up", () => {
  const view = raised("meeting", "agree");
  assert.equal(lowerFace(view, "live", {}).lit, true);
  assert.equal(lowerFace(view, "live", {}).tag, "All decks");
  assert.equal(lowerFace(view, "live", { deck: "meeting" }).tag, "Meeting signals");
  assert.equal(lowerFace(view, "live", { deck: "poker" }).lit, false);
  assert.deepEqual(
    lowerScope(view, {}).map((deck) => deck.id),
    ["poker", "meeting"],
  );
  assert.deepEqual(lowerScope(view, { deck: "gone" }), []);
  const missing = lowerFace(view, "live", { deck: "gone" });
  assert.equal(missing.caption, "Not found");
  assert.equal(missing.tag, "gone");
});

test("an element key is lit while shown, tagged while held", () => {
  const view = sampleView();
  view.elements[0].held = true;
  const hidden = elementFace(view, "live", { element: "brb" });
  assert.deepEqual([hidden.title, hidden.caption, hidden.tag, hidden.lit], ["Be right back", "Hidden", "Held", false]);
  const shown = elementFace(view, "live", { element: "lower-third" });
  assert.deepEqual([shown.caption, shown.tag, shown.lit], ["Shown", undefined, true]);
  assert.equal(elementFace(view, "live", {}).title, "Choose an element");
  assert.equal(elementFace(view, "live", { element: "gone" }).caption, "Not found");
});

test("the automation key has state 0 while on and 1 while paused", () => {
  const view = sampleView();
  assert.equal(automationFace(view, "live").state, 0);
  assert.equal(automationFace(view, "live").face.caption, "On");
  view.automation_enabled = false;
  assert.equal(automationFace(view, "live").state, 1);
  assert.equal(automationFace(view, "live").face.caption, "Paused");
  assert.equal(automationFace(null, "unpaired").face.caption, "Not paired");
});

test("a push to talk key is lit while held and says whether voice control is on", () => {
  const idle = talkFace(sampleView(), "live", false);
  assert.deepEqual([idle.title, idle.caption, idle.lit, idle.dim], ["Talk", undefined, false, false]);
  assert.equal(idle.tag, undefined, "nothing about voice control while Sceneuron does not say");
  const held = talkFace(sampleView(), "live", true);
  assert.deepEqual([held.caption, held.lit], ["Talking", true]);
  const on = { ...sampleView(), voice: { enabled: true, talking: false } };
  assert.equal(talkFace(on, "live", false).tag, "Voice on");
  const elsewhere = { ...sampleView(), voice: { enabled: true, talking: true } };
  assert.equal(talkFace(elsewhere, "live", false).lit, true, "held from the editor");
  const lapsed = talkFace(on, "live", true);
  assert.deepEqual([lapsed.caption, lapsed.lit], [undefined, false], "Sceneuron let go by itself");
  const off = talkFace({ ...sampleView(), voice: { enabled: false, talking: false } }, "live", false);
  assert.deepEqual([off.tag, off.dim], ["Voice off", true]);
  assert.equal(talkFace(on, "offline", false).caption, "Offline");
});

test("room keys say No room without one, and show the votes with the room's counts", () => {
  const plain = roomFace("reveal", sampleView(), "live", false);
  assert.deepEqual([plain.title, plain.caption, plain.dim], ["Reveal votes", undefined, false]);
  assert.equal(roomFace("next-round", null, "live", false).title, "Next round");
  const learned = roomFace("reveal", sampleView(), "live", true);
  assert.deepEqual([learned.caption, learned.dim], ["No room", true]);
  const room = { joined: true, revealed: false, voted: 2, people: 5 };
  const voting = roomFace("reveal", { ...sampleView(), room }, "live", true);
  assert.deepEqual([voting.caption, voting.lit, voting.dim], ["2/5 voted", false, false], "the view wins");
  const shown = roomFace("next-round", { ...sampleView(), room: { ...room, revealed: true } }, "live", false);
  assert.deepEqual([shown.caption, shown.lit], ["Revealed", true]);
  const none = roomFace("next-round", { ...sampleView(), room: { ...room, joined: false } }, "live", false);
  assert.deepEqual([none.caption, none.dim], ["No room", true]);
  assert.equal(roomFace("reveal", null, "refused", true).caption, "Pair again");
});

test("a room key's vote count fits its key whole with two-digit counts", () => {
  const room = { joined: true, revealed: false, voted: 12, people: 14 };
  const svg = keySvg(roomFace("reveal", { ...sampleView(), room }, "live", false));
  assert.match(svg, />12\/14 voted</);
  assert.doesNotMatch(svg, /…/);
});

test("a dial stands on the raised card, else the one it stood on, else the first", () => {
  const up = raised("poker", "p3").decks[0];
  const down = sampleView().decks[0];
  assert.equal(dialCard(up, "p5"), "p3");
  assert.equal(dialCard(down, "p5"), "p5");
  assert.equal(dialCard(down, "gone"), "p1");
  assert.equal(dialCard(down, undefined), "p1");
  assert.equal(dialCard({ ...up, raised: "gone" }, null), "p1");
  assert.equal(dialCard(undefined, "p1"), null);
  assert.equal(dialCard({ ...up, cards: [] }, "p1"), null);
});

test("the strip shows the dial's card and its position, framed while it is up", () => {
  const view = raised("poker", "p5");
  const up = dialFace(view, "live", { deck: "poker" }, "p2");
  assert.deepEqual([up.header, up.position, up.title, up.lit, up.dim], ["Planning poker", "4/4", "5", true, false]);
  const down = dialFace(sampleView(), "live", { deck: "poker" }, "p2");
  assert.deepEqual([down.header, down.position, down.title, down.lit], ["Planning poker", "2/4", "2", false]);
  assert.equal(dialFace(view, "offline", { deck: "poker" }, "p2").header, "Offline");
  assert.equal(dialFace(view, "refused", { deck: "poker" }, "p2").header, "Pair again");
  assert.equal(dialFace(view, "live", {}, null).title, "Choose a deck");
  assert.equal(dialFace(view, "live", { deck: "gone" }, null).title, "Deck not found");
  assert.equal(dialFace(view, "live", { deck: "poker" }, null).title, "5");
});
