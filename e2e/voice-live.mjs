// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Live test of voice control against the real ElevenLabs API, without OBS or a microphone. It
// SPENDS ELEVENLABS CREDITS: it synthesizes "Sceneuron, wrap it up" with text to speech (Flash
// v2.5) as 16 kHz PCM, writes it to a WAV file with half a second of silence on each side, and
// runs sceneuron-dev with the meeting pack, voice on for its deck, and --voice-wav, which sends the
// file to Scribe v2 Realtime speech to text as one push-to-talk turn. It passes when the meeting
// card "Wrap it up" rises, and prints the characters and seconds of audio it used.
//
// Nothing is spent before the free checks pass: that this build has voice control, that the meeting
// deck keeps its voice switch, and that ElevenLabs accepts the key and lists a voice to use (listing
// voices costs nothing). Any of them failing exits with status 2. Then it says it is about to spend.
//
// Opt-in: it refuses to run unless SCENEURON_ELEVENLABS_API_KEY holds a key. Build first; this
// script builds nothing:
//   cargo build --locked -p sceneuron-dev
// Usage: SCENEURON_ELEVENLABS_API_KEY=... node e2e/voice-live.mjs [--dev-binary PATH]
//          [--voice-id ID] [--work-dir DIR] [--keep]
// SCENEURON_ELEVENLABS_URL, such as http://127.0.0.1:9, sends every request, speech to text
// included, to that server instead of https://api.elevenlabs.io; the tests point it at a closed
// address, so a broken guard fails on this computer instead of reaching ElevenLabs.

import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

import { raisedCard, withPackOnce } from "./card-steps.mjs";
import { DevServer, devBinary } from "./dev-server.mjs";
import { chooseVoice } from "./voices.mjs";
import { padded, pcm16Wav, pcmSeconds } from "./wav.mjs";

const KEY_VAR = "SCENEURON_ELEVENLABS_API_KEY";
const URL_VAR = "SCENEURON_ELEVENLABS_URL";
const OTHER_API = process.env[URL_VAR]?.trim().replace(/\/+$/, "");
const API = OTHER_API || "https://api.elevenlabs.io";
// The speech to text WebSocket on the same server, for sceneuron-dev.
const REALTIME = OTHER_API && `${OTHER_API.replace(/^http/, "ws")}/v1/speech-to-text/realtime`;
const PHRASE = "Sceneuron, wrap it up";
const TTS_MODEL = "eleven_flash_v2_5";
// Raw 16-bit little-endian mono PCM, the format Scribe takes as is:
// https://elevenlabs.io/docs/api-reference/text-to-speech/convert.
const OUTPUT_FORMAT = "pcm_16000";
const SAMPLE_RATE = 16_000;
const PAD_SECONDS = 0.5;
const DECK = "meeting";
const CARD = "wrap-up";

const key = process.env[KEY_VAR]?.trim();
if (!key) {
  console.error(
    `voice-live.mjs talks to the real ElevenLabs API and spends credits: ${PHRASE.length} characters of ` +
      `text to speech and a few seconds of speech to text. Set ${KEY_VAR} to an ElevenLabs API key to run it.`,
  );
  process.exit(2);
}

const { values: options } = parseArgs({
  options: {
    "dev-binary": { type: "string" },
    "voice-id": { type: "string" },
    "work-dir": { type: "string" },
    keep: { type: "boolean" },
  },
});
const binary = devBinary(options["dev-binary"]);
const ownWork = !options["work-dir"];
const work = ownWork ? mkdtempSync(join(tmpdir(), "sceneuron-voice-live-")) : resolve(options["work-dir"]);
const token = "voice-live-editor-token-0123456789";
const js = JSON.stringify;
const hidden = (text) => text.replaceAll(key, "[key]");

let failures = 0;
async function step(name, body) {
  try {
    const detail = await body();
    console.log(`PASS ${name}${detail ? ` - ${detail}` : ""}`);
    return true;
  } catch (error) {
    failures += 1;
    console.log(`FAIL ${name} - ${hidden(error.message)}`);
    return false;
  }
}

/** Calls a sceneuron-dev API with the editor token; an error reply throws. */
async function api(server, method, path, body) {
  const response = await fetch(`${server.base}${path}`, {
    method,
    headers: { "X-Sceneuron-Token": token, "Content-Type": "application/json" },
    body: body === undefined ? undefined : js(body),
  });
  const json = await response.json();
  if (!response.ok) throw new Error(`${method} ${path}: ${response.status} ${json.error}`);
  return json;
}

/** Calls ElevenLabs with the key in its header; an error reply throws without the key. */
async function elevenlabs(path, init = {}) {
  const response = await fetch(`${API}${path}`, {
    ...init,
    headers: { "xi-api-key": key, "Content-Type": "application/json", ...init.headers },
  });
  if (!response.ok) {
    const detail = hidden(await response.text()).slice(0, 300);
    throw new Error(`ElevenLabs answered ${path.split("?")[0]} with HTTP ${response.status}: ${detail}`);
  }
  return response;
}

/** Waits up to `ms` for the deck's raised card to be `card`; returns the last state seen. */
async function waitForCard(server, card, ms) {
  const deadline = Date.now() + ms;
  let state = await api(server, "GET", "/api/state");
  while (raisedCard(state, DECK) !== card && Date.now() < deadline) {
    await new Promise((done) => setTimeout(done, 100));
    state = await api(server, "GET", "/api/state");
  }
  return state;
}

const servers = [];
const deadline = setTimeout(() => {
  console.log("FAIL the run took longer than two minutes");
  finish(1);
}, 120_000);
process.on("SIGINT", () => finish(130));

async function finish(code) {
  clearTimeout(deadline);
  await Promise.all(servers.map((server) => server.stop()));
  if (ownWork && code === 0 && !options.keep) rmSync(work, { recursive: true, force: true });
  else console.log(`Work directory: ${work}`);
  process.exit(code);
}

const usage = { characters: 0, seconds: 0 };
const project = join(work, "project.json");
const wav = join(work, "wrap-it-up.wav");
let voiceId = null;

function checkBuild() {
  const help = spawnSync(binary, ["--help"], { encoding: "utf8", timeout: 10_000 });
  if (help.error || help.status !== 0) {
    throw new Error(`cannot run ${binary}; build it with cargo build --locked -p sceneuron-dev`);
  }
  if (!help.stdout.includes("--voice-wav") || !help.stdout.includes("keyterms")) {
    throw new Error(`${binary} has no voice control wired to its engine; rebuild it`);
  }
}

async function addMeetingPack() {
  const setup = new DevServer(binary, ["--port", "0", "--token", token, "--project", project]);
  servers.push(setup);
  await setup.ready();
  const pack = (await api(setup, "GET", "/api/packs")).find((offered) => offered.id === DECK);
  if (!pack) throw new Error("GET /api/packs offers no meeting pack");
  // A new project starts from the starter setup, which has the meeting deck already.
  const current = withPackOnce(await api(setup, "GET", "/api/project"), pack);
  const decks = current.decks.map((deck) => (deck.id === DECK ? { ...deck, voice: true } : deck));
  await api(setup, "PUT", "/api/project", { ...current, decks });
  const saved = (await api(setup, "GET", "/api/project")).decks.find((deck) => deck.id === DECK);
  if (saved?.voice !== true) throw new Error("the meeting deck did not keep its voice switch; rebuild sceneuron-dev");
  await setup.stop();
}

async function pickVoice() {
  const listed = await (await elevenlabs("/v2/voices?page_size=100")).json();
  const chosen = chooseVoice(listed, options["voice-id"]);
  if (!chosen) throw new Error("the account lists no voice to speak with");
  voiceId = chosen.voice_id;
  return `${chosen.name || "a voice"} (${voiceId}) of ${listed.voices.length} listed`;
}

/**
 * Synthesizes the phrase; returns its PCM and the characters it cost, from the `character-cost`
 * header (https://elevenlabs.io/docs/api-reference/introduction) or else the phrase's length.
 */
async function speak() {
  const path = `/v1/text-to-speech/${encodeURIComponent(voiceId)}?output_format=${OUTPUT_FORMAT}`;
  const response = await elevenlabs(path, { method: "POST", body: js({ text: PHRASE, model_id: TTS_MODEL }) });
  const pcm = Buffer.from(await response.arrayBuffer());
  const cost = Number(response.headers.get("character-cost"));
  usage.characters = Number.isFinite(cost) && cost > 0 ? cost : PHRASE.length;
  if (pcmSeconds(pcm, SAMPLE_RATE) < 0.5) throw new Error(`only ${pcm.length} bytes of audio came back`);
  const audio = padded(pcm, SAMPLE_RATE, PAD_SECONDS);
  usage.seconds = pcmSeconds(audio, SAMPLE_RATE);
  writeFileSync(wav, pcm16Wav(audio, SAMPLE_RATE));
  return `${pcmSeconds(pcm, SAMPLE_RATE).toFixed(2)} s of speech`;
}

async function hear() {
  const args = ["--port", "0", "--token", token, "--project", project, "--voice-wav", wav];
  if (REALTIME) args.push("--voice-url", REALTIME);
  const server = new DevServer(binary, [...args, "--env-file", join(work, "no.env")]);
  servers.push(server);
  await server.ready();
  const state = await waitForCard(server, CARD, 30_000);
  const voiceLog = (state.events ?? []).filter((event) => event.source === "voice").map((event) => event.detail);
  if (raisedCard(state, DECK) !== CARD) {
    throw new Error(`the raised card is ${js(raisedCard(state, DECK))}; voice log: ${js(voiceLog)}`);
  }
  return voiceLog.join("; ");
}

try {
  const free =
    (await step("this build has voice control", checkBuild)) &&
    (await step("the meeting pack is added, and its deck keeps voice on", addMeetingPack)) &&
    (await step("ElevenLabs accepts the key and lists a voice to use, at no cost", pickVoice));
  if (!free) {
    console.log("Nothing was spent: a free check failed");
    await finish(2);
  }
  console.log(
    `About to spend ElevenLabs credits: ${PHRASE.length} characters of text to speech and a few seconds of ` +
      "speech to text",
  );
  const spoken = await step(`ElevenLabs text to speech says "${PHRASE}"`, speak);
  if (spoken) await step(`saying "${PHRASE}" raises the ${CARD} card`, hear);
} catch (error) {
  failures += 1;
  console.log(`FAIL the run stopped - ${hidden(error.message)}`);
}
console.log(
  `ElevenLabs use: ${usage.characters} characters of text to speech (${TTS_MODEL}), ` +
    `${usage.seconds.toFixed(1)} s of audio sent to speech to text`,
);
console.log(failures ? `${failures} step(s) failed` : "All steps passed");
await finish(failures ? 1 : 0);
