// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Live test of subtitles against the real ElevenLabs and Gemini APIs, without OBS or a microphone. It SPENDS
// CREDITS on both: it synthesizes an English sentence with ElevenLabs text to speech, plays it into sceneuron-dev --ai
// with --voice-wav and subtitles on, translated into Spanish by Gemini 3.5 Flash-Lite, and passes when a {{subtitles}}
// element shows the English and a {{subtitles:es}} element its Spanish. It prints the characters, seconds, and tokens
// it used. Nothing is spent before the free checks pass (the build, its settings without keys, and the keys, through
// the free voice list and model details); a failed one exits with status 2. It refuses to run unless
// SCENEURON_ELEVENLABS_API_KEY and SCENEURON_GEMINI_API_KEY hold keys, taken only from the environment. Build first:
//   cargo build --locked -p sceneuron-dev
//   SCENEURON_ELEVENLABS_API_KEY=... SCENEURON_GEMINI_API_KEY=... node e2e/subtitles-live.mjs
//     [--dev-binary PATH] [--voice-id ID] [--work-dir DIR] [--keep]
// SCENEURON_ELEVENLABS_URL and SCENEURON_GEMINI_URL, such as http://127.0.0.1:9, send every request to other
// servers; the tests point them at a closed address, so a broken guard fails here instead of reaching either API.

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

import { DevServer, devBinary } from "./dev-server.mjs";
import { keyRefusal, modelProblem } from "./gemini.mjs";
import { chooseVoice } from "./voices.mjs";
import { padded, pcm16Wav, pcmSeconds } from "./wav.mjs";

const EL_VAR = "SCENEURON_ELEVENLABS_API_KEY";
const GEMINI_VAR = "SCENEURON_GEMINI_API_KEY";
const base = (name, fallback) => process.env[name]?.trim().replace(/\/+$/, "") || fallback;
const OTHER_ELEVENLABS = base("SCENEURON_ELEVENLABS_URL", "");
const ELEVENLABS = OTHER_ELEVENLABS || "https://api.elevenlabs.io";
const REALTIME = OTHER_ELEVENLABS && `${OTHER_ELEVENLABS.replace(/^http/, "ws")}/v1/speech-to-text/realtime`;
const GEMINI = base("SCENEURON_GEMINI_URL", "https://generativelanguage.googleapis.com");
const SENTENCE = "Good morning everyone, let's start the weekly meeting.";
const TTS_MODEL = "eleven_flash_v2_5";
const TRANSLATION_MODEL = "gemini-3.5-flash-lite";
// Raw 16-bit little-endian mono PCM, the format Scribe takes as is:
// https://elevenlabs.io/docs/api-reference/text-to-speech/convert.
const OUTPUT_FORMAT = "pcm_16000";
const SAMPLE_RATE = 16_000;
const PAD_SECONDS = 0.5;

const keys = { elevenlabs: process.env[EL_VAR]?.trim(), gemini: process.env[GEMINI_VAR]?.trim() };
if (!keys.elevenlabs || !keys.gemini) {
  console.error(
    `subtitles-live.mjs talks to the real ElevenLabs and Gemini APIs and spends credits: ${SENTENCE.length} ` +
      "characters of text to speech, a few seconds of speech to text, and one translation by Gemini 3.5 Flash-Lite. " +
      `Set ${EL_VAR} and ${GEMINI_VAR} to run it.`,
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
const work = ownWork ? mkdtempSync(join(tmpdir(), "sceneuron-subtitles-live-")) : resolve(options["work-dir"]);
mkdirSync(work, { recursive: true });
const token = "subtitles-live-editor-token-0123456789";
const js = JSON.stringify;
const hidden = (text) => text.replaceAll(keys.elevenlabs, "[key]").replaceAll(keys.gemini, "[key]");
const noEnvFile = join(work, "no.env");
// The environment sceneuron-dev runs with: no key from a .env file, and the free checks without these keys at all.
const withKeys = { ...process.env, SCENEURON_ENV_FILE: noEnvFile };
const withoutKeys = { ...withKeys };
delete withoutKeys[EL_VAR];
delete withoutKeys[GEMINI_VAR];

let failures = 0;
async function step(name, body) {
  try {
    const detail = await body();
    console.log(`PASS ${name}${detail ? ` - ${hidden(String(detail))}` : ""}`);
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

/** Calls an API with `headers`; an error reply throws without the keys, in `explain`'s words when it has some. */
async function call(url, headers, init = {}, explain = () => null) {
  const response = await fetch(url, { ...init, headers: { "Content-Type": "application/json", ...headers } });
  if (!response.ok) {
    const body = await response.text();
    const detail = `${new URL(url).host} answered ${new URL(url).pathname} with HTTP ${response.status}`;
    throw new Error(explain(response.status, body) ?? `${detail}: ${hidden(body).slice(0, 300)}`);
  }
  return response;
}

const elevenlabs = (path, init) => call(`${ELEVENLABS}${path}`, { "xi-api-key": keys.elevenlabs }, init);
// The key goes in this header and never in the URL: https://ai.google.dev/gemini-api/docs/api-key
const gemini = (path) => call(`${GEMINI}${path}`, { "x-goog-api-key": keys.gemini }, {}, keyRefusal);

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

const usage = { characters: 0, seconds: 0, input: 0, output: 0, translationUsd: 0 };
const [settingsFile, project, wav] = ["ai.json", "project.json", "sentence.wav"].map((name) => join(work, name));
const devArgs = ["--port", "0", "--token", token, "--project", project, "--ai", "--ai-settings", settingsFile,
  "--env-file", noEnvFile];
let voiceId = null;

function checkBuild() {
  const help = spawnSync(binary, ["--help"], { encoding: "utf8", timeout: 10_000 });
  if (help.error || help.status !== 0) {
    throw new Error(`cannot run ${binary}; build it with cargo build --locked -p sceneuron-dev`);
  }
  if (!help.stdout.includes("--voice-wav") || !help.stdout.includes("subtitles")) {
    throw new Error(`${binary} has no subtitles; rebuild it`);
  }
}

/** Two text elements, one for each language, in a project of their own. */
function bars(current) {
  const text = (id, words, y) => ({
    id,
    name: id,
    kind: "text",
    props: { x: 100, y, width: 1720, height: 120, text: words, font_size: 40, background: "#111827ff" },
  });
  const elements = [text("live-english", "{{subtitles}}", 700), text("live-spanish", "{{subtitles:es}}", 840)];
  return { ...current, elements: [...current.elements, ...elements] };
}

async function checkSettings() {
  const subtitles = { enabled: true, consent: true, targets: ["es"], translation_model: TRANSLATION_MODEL,
    clear_after_ms: 60_000 };
  // Gemini is named, not left to the default provider, so the test keeps testing it if the default changes.
  writeFileSync(settingsFile, js({ provider: "gemini", base_url: GEMINI, subtitles }));
  const setup = new DevServer(binary, devArgs, withoutKeys);
  servers.push(setup);
  await setup.ready();
  const ai = await api(setup, "GET", "/api/ai");
  if (ai.provider !== "gemini") throw new Error(`the AI settings name ${js(ai.provider)}, not "gemini"`);
  const saved = ai.subtitles;
  if (!saved?.enabled || js(saved.targets) !== js(["es"]) || saved.translation_model !== TRANSLATION_MODEL) {
    throw new Error(`the AI settings did not keep subtitles into Spanish: ${js(saved)}`);
  }
  const state = (await api(setup, "GET", "/api/subtitles")).status.state;
  if (state !== "needs_key") throw new Error(`subtitles without a key say ${js(state)}, not "needs_key"`);
  await api(setup, "PUT", "/api/project", bars(await api(setup, "GET", "/api/project")));
  await setup.stop();
  return "subtitles into Spanish by Gemini, waiting for a key";
}

async function pickVoice() {
  const listed = await (await elevenlabs("/v2/voices?page_size=100")).json();
  const chosen = chooseVoice(listed, options["voice-id"]);
  if (!chosen) throw new Error("the account lists no voice to speak with");
  voiceId = chosen.voice_id;
  return `${chosen.name || "a voice"} (${voiceId}) of ${listed.voices.length} listed`;
}

/** Asks for the model's details, which costs nothing and uses no tokens. */
async function checkModel() {
  const model = await (await gemini(`/v1beta/models/${TRANSLATION_MODEL}`)).json();
  const problem = modelProblem(model, TRANSLATION_MODEL);
  if (problem) throw new Error(problem);
  return `${model.displayName || model.name}`;
}

/** Synthesizes the sentence; the characters it cost come from the `character-cost` header, or else its length. */
async function speak() {
  const path = `/v1/text-to-speech/${encodeURIComponent(voiceId)}?output_format=${OUTPUT_FORMAT}`;
  const response = await elevenlabs(path, { method: "POST", body: js({ text: SENTENCE, model_id: TTS_MODEL }) });
  const pcm = Buffer.from(await response.arrayBuffer());
  const cost = Number(response.headers.get("character-cost"));
  usage.characters = Number.isFinite(cost) && cost > 0 ? cost : SENTENCE.length;
  if (pcmSeconds(pcm, SAMPLE_RATE) < 0.5) throw new Error(`only ${pcm.length} bytes of audio came back`);
  writeFileSync(wav, pcm16Wav(padded(pcm, SAMPLE_RATE, PAD_SECONDS), SAMPLE_RATE));
  return `${pcmSeconds(pcm, SAMPLE_RATE).toFixed(2)} s of speech`;
}

/** Plays the file and waits for both elements; returns what they showed. */
async function subtitle() {
  const args = [...devArgs, "--voice-wav", wav, ...(REALTIME ? ["--voice-url", REALTIME] : [])];
  const server = new DevServer(binary, args, withKeys);
  servers.push(server);
  await server.ready();
  const textOf = (state, id) => state.elements.find((element) => element.id === id)?.props.text ?? "";
  const until = Date.now() + 45_000;
  let state = await api(server, "GET", "/api/state");
  while (!(textOf(state, "live-english") && textOf(state, "live-spanish")) && Date.now() < until) {
    await new Promise((done) => setTimeout(done, 200));
    state = await api(server, "GET", "/api/state");
  }
  const [english, spanish] = [textOf(state, "live-english"), textOf(state, "live-spanish")];
  Object.assign(usage, {
    seconds: state.ai.voice_seconds,
    input: state.ai.translation_input_tokens,
    output: state.ai.translation_output_tokens,
    translationUsd: state.ai.translation_cost_usd,
  });
  const log = (state.events ?? []).filter((event) => event.source === "subtitles").map((event) => event.detail);
  if (!/morning/i.test(english) || !/meeting/i.test(english)) {
    throw new Error(`the English line is ${js(english)}; subtitles log: ${js(log)}`);
  }
  if (!/buen|reuni|semanal|empe/i.test(spanish) || spanish === english) {
    throw new Error(`the Spanish line is ${js(spanish)}; subtitles log: ${js(log)}`);
  }
  return `${js(english)} and ${js(spanish)}`;
}

try {
  const free =
    (await step("this build has subtitles", checkBuild)) &&
    (await step("sceneuron-dev takes subtitles into Spanish, and waits for a key without one", checkSettings)) &&
    (await step("ElevenLabs accepts the key and lists a voice to use, at no cost", pickVoice)) &&
    (await step(`Gemini accepts the key and knows ${TRANSLATION_MODEL}, at no cost`, checkModel));
  if (!free) {
    console.log("Nothing was spent: a free check failed");
    await finish(2);
  }
  console.log(
    `About to spend credits: ${SENTENCE.length} characters of ElevenLabs text to speech, a few seconds of speech ` +
      `to text, and one translation by ${TRANSLATION_MODEL}`,
  );
  const spoken = await step(`ElevenLabs text to speech says "${SENTENCE}"`, speak);
  if (spoken) await step("the subtitles show the sentence in English and in Spanish", subtitle);
} catch (error) {
  failures += 1;
  console.log(`FAIL the run stopped - ${hidden(error.message)}`);
}
console.log(
  `Spent: ${usage.characters} characters of text to speech (${TTS_MODEL}), ${Number(usage.seconds).toFixed(1)} s ` +
    `of audio sent to speech to text, and ${usage.input} input and ${usage.output} output tokens of translation ` +
    `(about $${Number(usage.translationUsd).toFixed(5)})`,
);
console.log(failures ? `${failures} step(s) failed` : "All steps passed");
await finish(failures ? 1 : 0);
