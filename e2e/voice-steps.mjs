// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Voice steps of the browser regression test (editor-browser.mjs), against the dev server's simulated voice
// (--demo-ai): the AI tab's Voice section and its consent gate, a deck's voice and speech switches and a card's
// trigger phrases and speech line, Hold to talk with the mouse and the keyboard and the live heard line, and the
// overlay playing each spoken card once. They run after the Cards tab steps.

import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const js = JSON.stringify;
// Playback waits on Chrome's media pipeline, whose start slows down under load, so it gets more than waitFor's 5 s.
const MEDIA_WAIT_MS = 15_000;
const MEDIA_EVENTS = ["loadstart", "loadedmetadata", "canplay", "playing", "pause", "ended", "stalled", "abort", "error"];

/** Runs the voice steps; `overlay` is the overlay page the overlay steps opened. */
export async function runVoiceSteps({ page, overlay, step, same, api }) {
  const helpers = editorHelpers(page, api);
  const { q, check, project, clean, attribute } = helpers;
  const hidden = (id) => q(`document.getElementById(${js(id)}).hidden`);
  const disabled = (id) => q(`document.getElementById(${js(id)}).disabled`);
  const heard = () => page.text("#voice-heard");
  let di = -1;

  await step("the Voice section shows the simulated voice, which needs consent before it listens", async () => {
    await page.send("Page.bringToFront");
    await page.click("#tab-ai");
    await waitFor("the Voice section", async () => !(await hidden("voice")));
    same(await hidden("voice-simulated"), false, "simulated voice note hidden");
    same(await page.text("#voice-key-status"), "Key saved (...demo)", "key status");
    same(await page.value("voice-key"), "", "key field");
    same(await page.text("#voice-state"), "Off", "voice state");
    same(await disabled("voice-enabled"), true, "listen switch before consent");
    same(await disabled("voice-talk"), true, "talk button while voice is off");
    same(
      await page.text("#voice-consent-text li:first-child"),
      "What: Audio from your microphone, only while push to talk is held.",
      "consent",
    );
    same(await heard(), "Nothing heard yet.", "heard line");
    same(await attribute("#voice-heard", "role"), null, "the heard line is no live region");
  });

  await step("a deck's voice and speech switches and a card's trigger phrases and speech line are saved", async () => {
    await page.click("#tab-cards");
    await waitFor("the packs", async () => (await q(`document.querySelectorAll("#pack-list .pack-name").length`)) === 6);
    await page.click('[data-pack-id="meeting"]');
    const index = `[...document.querySelectorAll("#deck-list article h4")].findIndex((h) => h.textContent === "Meeting signals")`;
    await waitFor("the meeting deck", async () => (di = await q(index)) >= 0);
    await page.click(`#deck-${di}-voice`);
    await page.click(`#deck-${di}-speak`);
    await page.click(`#deck-list article[data-deck-index="${di}"] .card-editor summary`);
    await page.choose(`deck-${di}-edit-card`, "offline");
    const phrases = `deck-${di}-card-phrases`;
    await waitFor("the card's phrases", async () => (await page.value(phrases)) === "let's take this offline");
    await page.fill(phrases, "x");
    same(await attribute(`#${phrases}`, "aria-invalid"), "true", "a one-letter phrase");
    check((await page.text(`#${phrases}-error`)).startsWith('"x" must be 2 to 60'), "no refusal shown");
    await page.fill(phrases, "let's take this offline\ntake it offline");
    same(await attribute(`#${phrases}`, "aria-invalid"), null, "fixed phrases");
    await page.fill(`deck-${di}-card-speech`, "Let's talk about this later");
    await page.click("#save-button");
    await clean();
    const deck = (await project()).decks.find((candidate) => candidate.id === "meeting");
    same([deck.voice, deck.speak], [true, true], "deck switches");
    const offline = deck.cards.find((card) => card.id === "offline");
    same(
      [offline.phrases, offline.speech],
      [["let's take this offline", "take it offline"], "Let's talk about this later"],
      "card voice",
    );
  });

  await step("Hold to talk hears a scripted command and the heard line shows what it did", async () => {
    await page.click("#tab-ai");
    await page.click("#voice-consent");
    await waitFor("the listen switch", async () => !(await disabled("voice-enabled")));
    await page.click("#voice-enabled");
    await waitFor("voice on", async () => (await page.text("#voice-state")) === "Ready");
    check((await page.text("#voice-message")).startsWith("Simulated voice"), "no simulated message");
    await page.click("#voice-talk");
    const wrapped = 'Heard "Sceneuron, wrap it up": raised Meeting signals: Wrap it up.';
    await waitFor("the heard line", async () => (await heard()) === wrapped);
    await waitFor("the final result announced", async () => (await page.text("#announcer")) === wrapped);
    const meeting = (await api("GET", "/api/state")).decks.find((deck) => deck.id === "meeting");
    same([meeting.raised, meeting.raised_by], ["wrap-up", "voice"], "the raised card");
    same(await attribute("#voice-talk", "aria-pressed"), "false", "talk button after letting go");
    await q(`document.getElementById("voice-talk").focus()`);
    await page.key(" ", { code: "Space" });
    const voted = 'Heard "Sceneuron, my vote is eight": no card for the vote "8".';
    await waitFor("the second heard line", async () => (await heard()) === voted);
    const logged = await q(`[...document.querySelectorAll('#event-log li[data-source="voice"] .event-detail')]
      .map((node) => node.textContent)`);
    check(logged.includes('heard: "Sceneuron, wrap it up"'), `voice events: ${js(logged)}`);
  });

  await step("the overlay plays each spoken card once, from this server", async () => {
    await page.click("#voice-speak");
    await waitFor("spoken cards on", async () => (await api("GET", "/api/ai")).voice.speak === true);
    // Chrome holds back media in a hidden tab; an OBS browser source is always shown.
    await overlay.send("Page.bringToFront");
    // Counts each play() that starts, with the audio element's events, so a slow or failed start says why.
    await overlay.eval(`(() => {
      window.spoken = { urls: [], played: 0, errors: [], events: [] };
      const note = (what) => window.spoken.events.push(Math.round(performance.now()) + " " + what);
      const play = HTMLMediaElement.prototype.play;
      HTMLMediaElement.prototype.play = function () {
        if (window.spokenAudio !== this) {
          window.spokenAudio = this;
          for (const name of ${js(MEDIA_EVENTS)}) this.addEventListener(name, () => note(name));
        }
        window.spoken.urls.push(this.src);
        note("play() " + this.src.split("?")[0].slice(-4));
        const started = play.call(this);
        started.then(() => (window.spoken.played += 1), (error) => window.spoken.errors.push(String(error)));
        return started;
      };
      // Marks when the overlay first sees each cue with its audio, in the same clock.
      const parse = JSON.parse;
      const ready = new Set();
      JSON.parse = function (...args) {
        const value = parse.apply(this, args);
        const cue = value && value.speech;
        if (cue && cue.audio && !ready.has(cue.nonce)) {
          ready.add(cue.nonce);
          note("ready " + cue.nonce);
        }
        return value;
      };
    })()`);
    const raise = async (card) => {
      await overlay.eval(`window.spoken.events.push(Math.round(performance.now()) + " raise ${card}")`);
      await api("POST", "/api/card", { deck: "meeting", card, action: "raise" });
    };
    const spoken = () => overlay.eval("window.spoken");
    const audioState = () => overlay.eval(`(() => {
      const audio = window.spokenAudio;
      if (!audio) return null;
      const { readyState, networkState, paused, currentTime } = audio;
      const error = audio.error && { code: audio.error.code, message: audio.error.message };
      return { readyState, networkState, error, paused, currentTime, visibility: document.visibilityState };
    })()`);
    const played = async (what, count) => {
      const asked = Date.now();
      try {
        await waitFor(what, async () => (await spoken()).played === count, MEDIA_WAIT_MS);
      } catch (error) {
        const { speech } = await api("GET", "/api/state");
        throw new Error(`${error.message}; overlay: ${js(await spoken())}; audio: ${js(await audioState())}; ` +
          `server cue: ${js(speech)}`);
      }
      return Date.now() - asked;
    };
    await raise("agree");
    await played("the first spoken card", 1);
    await raise("disagree");
    await played("the second spoken card", 2);
    await new Promise((resolve) => setTimeout(resolve, 500));
    const { urls, played: count, errors, events } = await spoken();
    same([count, errors], [2, []], "plays");
    check(urls.length === 2 && urls.every((url) => /\/api\/speech\/[0-9a-f]{64}\?token=/.test(url)), js(urls));
    check(urls[0] !== urls[1], "each card has its own audio");
    return `${urls.length} cards spoken; ${phases(events).join("; ")}`;
  });
}

/** Each card's time from its raise to its cue being ready, to play(), and to playing, from the page's notes. */
export function phases(events) {
  const notes = events.map((line) => {
    const [at, ...rest] = line.split(" ");
    return { at: Number(at), what: rest.join(" ") };
  });
  const after = (from, prefix) => notes.find((note) => note.at >= from && note.what.startsWith(prefix));
  return notes
    .filter((note) => note.what.startsWith("raise "))
    .map((raised) => {
      const ready = after(raised.at, "ready ");
      const play = ready && after(ready.at, "play() ");
      const canplay = play && after(play.at, "canplay");
      const playing = play && after(play.at, "playing");
      const gap = (from, to) => (from && to ? `${to.at - from.at} ms` : "never");
      return `${raised.what.slice(6)}: ready ${gap(raised, ready)}, play() ${gap(ready, play)}, ` +
        `canplay ${gap(play, canplay)}, playing ${gap(play, playing)}`;
    });
}
