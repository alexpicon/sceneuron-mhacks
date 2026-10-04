// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Subtitle steps of the browser regression test (editor-browser.mjs), against the dev server's simulated subtitles
// (--demo-ai): the AI tab's Subtitles section and its consent gate, a typed target code refused and a language picked
// by name saved, Quenya picked among the constructed and fictional languages and named, the preview of the simulated
// speaker's lines and their translations with final lines announced, the Subtitles pack showing them on the overlay,
// each bar named by its language, English as detected, and Quenya written in Tengwar with the bundled font, and the
// bars hiding when subtitles stop. They run after the voice steps.

import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";
import { overlayBars } from "./subtitle-bars.mjs";

const js = JSON.stringify;
// What the simulated speaker says, in English, Spanish, and Quenya (crates/sceneuron-dev/src/demo_ai/subtitles.rs).
const SPOKEN = [
  [
    "Good morning everyone, and welcome to the stream.",
    "Buenos días a todos y bienvenidos a la transmisión.",
    "Elen síla lúmenn' omentielvo!",
  ],
  ["Today we are planning the next sprint together.", "Hoy planificamos juntos el próximo sprint.",
    "Síra carilmë vinya tië."],
  ["Let's start with the questions from the chat.", "Empecemos con las preguntas del chat.", "Nai yestuvalvë!"],
];
// What the consent says Google may do with the lines the simulated AI's Gemini provider would translate.
const FREE_TIER =
  "Google: With a free-tier key (a Google project without billing), Google may use what is sent to Gemini to " +
  "improve its products, and people may review it. A key from a project with billing is not used that way.";
// A line takes about two seconds to say, stays four, and the next starts five seconds later, so seeing one may take
// a full cycle.
const LINE_WAIT_MS = 15_000;

/** Runs the subtitle steps; `overlay` is the overlay page the overlay steps opened. */
export async function runSubtitleSteps({ page, overlay, step, same, api }) {
  const { q, check, clean, attribute } = editorHelpers(page, api);
  const hidden = (id) => q(`document.getElementById(${js(id)}).hidden`);
  const disabled = (id) => q(`document.getElementById(${js(id)}).disabled`);
  const point = (n) => page.text(`#subtitles-consent-text li:nth-child(${n})`);
  const english = SPOKEN.map(([line]) => line);

  await step("the Subtitles section shows the simulated subtitles, which need consent first", async () => {
    // The voice steps leave the overlay in front; a hidden tab gets no frames to handle input.
    await page.send("Page.bringToFront");
    await page.click("#tab-ai");
    await waitFor("the Subtitles section", async () => !(await hidden("subtitles")));
    same(await hidden("subtitles-simulated"), false, "simulated subtitles note hidden");
    same(await page.text("#subtitles-state"), "Off", "subtitles state");
    same(await disabled("subtitles-enabled"), true, "the switch before consent");
    same(
      await point(1),
      "What: Audio from your microphone, all the time while subtitles are on, even with voice control off.",
      "what is sent",
    );
    same(
      await point(3),
      "Translations: None until you choose languages below; until then no text leaves this computer.",
      "translations",
    );
    same(await page.text("#subtitles-note"), "Subtitles are off.", "preview note");
    for (const id of ["subtitles-preview", "subtitles-live", "subtitles-lines-shown"]) {
      const live = [await attribute(`#${id}`, "aria-live"), await attribute(`#${id}`, "role")];
      same(live, [null, null], `${id} live region`);
    }
    const refused = await api("PUT", "/api/ai", { subtitles: { enabled: true } }).catch((error) => error.message);
    check(String(refused).includes("400"), `turning subtitles on without consent: ${js(refused)}`);
  });

  await step("a typed code is refused until valid, and a language picked by name is saved", async () => {
    await page.click("#subtitles-details summary");
    same(await hidden("subtitles-target-1-code-field"), true, "the code field before choosing to type one");
    await q(`document.getElementById("subtitles-target-1").focus()`);
    await page.choose("subtitles-target-1", "other");
    same(await hidden("subtitles-target-1-code-field"), false, "the code field after choosing to type one");
    same(await page.focused(), "subtitles-target-1", "the focus stays on the list after choosing to type a code");
    await page.fill("subtitles-target-1-code", "xx_y");
    await page.click("#subtitles-save");
    same(await attribute("#subtitles-target-1-code", "aria-invalid"), "true", "a bad code");
    const error = await page.text("#subtitles-target-1-code-error");
    check(error.startsWith('"xx_y" is not a language code'), `no refusal shown: ${js(error)}`);
    await page.choose("subtitles-target-1", "es");
    same(await hidden("subtitles-target-1-code-field"), true, "the code field once a language is picked");
    await page.click("#subtitles-save");
    await waitFor("the consent naming the target", async () => (await point(3)).endsWith("translated into Spanish."));
    const translations = await point(3);
    check(translations.includes(" goes to generativelanguage.googleapis.com (Google Gemini) with "),
      `translations go elsewhere: ${js(translations)}`);
    same(await point(4), FREE_TIER, "free-tier disclosure");
    same((await api("GET", "/api/ai")).subtitles.targets, ["es"], "saved targets");
    same(await attribute("#subtitles-target-1-code", "aria-invalid"), null, "the fixed code");
  });

  await step("Quenya is picked by name among the constructed and fictional languages, saved, and named", async () => {
    const option = (select, code) => q(`(() => {
      const option = document.querySelector(${js(`#${select} option[value="${code}"]`)});
      return option && [option.textContent, option.parentElement.label || ""];
    })()`);
    same(await option("subtitles-target-2", "qya"), ["Quenya (Elvish)", "Constructed and fictional"], "Quenya");
    same(await option("subtitles-target-2", "tlh"), ["Klingon", "Constructed and fictional"], "Klingon");
    same(await option("subtitles-target-2", "ja"), ["Japanese", "Languages"], "Japanese");
    same(await option("subtitles-language", "qya"), null, "Quenya as the spoken language");
    same(await option("subtitles-language", "en"), ["English", ""], "English as the spoken language");
    const refused = await api("PUT", "/api/ai", { subtitles: { language: "qya" } }).catch((error) => error.message);
    check(String(refused).includes("400"), `Quenya as the spoken language: ${js(refused)}`);
    await page.choose("subtitles-target-2", "qya");
    await page.click("#subtitles-save");
    await waitFor("the consent naming Quenya", async () =>
      (await point(3)).endsWith("translated into Spanish, Quenya (Elvish)."));
    same((await api("GET", "/api/ai")).subtitles.targets, ["es", "qya"], "saved targets");
    same(await page.value("subtitles-target-2"), "qya", "the picker after saving");
    same(await page.text("#subtitles-target-2 option:checked"), "Quenya (Elvish)", "the picker shows its name");
    same(await option("subtitles-stream-language", "qya"), ["Quenya (Elvish, qya)", ""], "the caption language");
    return "Quenya (Elvish)";
  });

  await step("the preview shows the lines and each translation by name, and announces final lines", async () => {
    await page.click("#subtitles-consent");
    await waitFor("the switch", async () => !(await disabled("subtitles-enabled")));
    await page.click("#subtitles-enabled");
    await waitFor("subtitles listening", async () => (await page.text("#subtitles-state")) === "Listening");
    check((await page.text("#subtitles-message")).startsWith("Simulated subtitles"), "no simulated message");
    const shown = () => page.text("#subtitles-lines-shown");
    await waitFor("a final line", async () => english.includes(await shown()), LINE_WAIT_MS);
    const line = await shown();
    const [, spanish, quenya] = SPOKEN.find(([spoken]) => spoken === line);
    await waitFor("its translation", async () =>
      (await page.text('#subtitles-translations [data-target="es"]')) === `Spanish: ${spanish}`, LINE_WAIT_MS);
    same(await attribute('#subtitles-translations [data-target="es"]', "lang"), "es", "the translation's language");
    // The preview keeps the romanization the overlay writes Tengwar from.
    await waitFor("its Quenya, by name", async () =>
      (await page.text('#subtitles-translations [data-target="qya"]')) === `Quenya (Elvish): ${quenya}`,
      LINE_WAIT_MS);
    same(await attribute('#subtitles-translations [data-target="qya"]', "lang"), "qya", "Quenya's language");
    await waitFor("a final line announced", async () => (await page.text("#announcer")).startsWith("Subtitle: "),
      LINE_WAIT_MS);
    const details = (await api("GET", "/api/state")).events.map((event) => event.detail).join(" ");
    check(!SPOKEN.flat().some((words) => details.includes(words)), `subtitles in the event log: ${details}`);
    return `${js(line)} and ${js(spanish)}`;
  });

  const bars = overlayBars(overlay, api);

  await step("the Subtitles pack shows the lines on the overlay", async () => {
    await page.click("#tab-cards");
    await page.click('[data-pack-id="subtitles"]');
    await page.click("#save-button");
    await clean();
    // A bar wraps a line beside its label, so a line shows as the end of the bar's words.
    let pair;
    await waitFor("a line and its translation on the overlay", async () => {
      const [spoken, translated] = [await bars.words("subtitles-spoken"), await bars.words("subtitles-translated")];
      pair = SPOKEN.find(([line, spanish]) => spoken.endsWith(line) && translated.endsWith(spanish));
      return Boolean(pair);
    }, LINE_WAIT_MS);
    return `${js(pair[0])} over ${js(pair[1])}`;
  });

  await step("each bar names its language, English as detected, and Quenya is written in Tengwar", async () => {
    await page.click("#tab-ai");
    same(await bars.label("subtitles-spoken"), "English (detected)", "the spoken bar's label");
    same(await bars.label("subtitles-translated"), "Spanish", "the translated bar's label");
    const tengwar = await bars.showTarget("subtitles-translated", 2, "tengwar", LINE_WAIT_MS);
    same(await bars.label("subtitles-translated"), "Quenya (Elvish)", "Quenya's label");
    same(await bars.fontLoaded("Sceneuron Tengwar"), true, "the bundled Tengwar font");
    await page.click("#subtitles-labels");
    await waitFor("the labels hidden", async () => (await bars.label("subtitles-spoken")) === null);
    await page.click("#subtitles-labels");
    await waitFor("the labels back", async () => (await bars.label("subtitles-spoken")) === "English (detected)");
    await api("POST", "/api/release", { element: "subtitles-translated" });
    return `${tengwar.length} Tengwar characters`;
  });

  await step("the bars hide when subtitles stop", async () => {
    await page.click("#subtitles-enabled");
    await waitFor("subtitles off", async () => (await page.text("#subtitles-state")) === "Off");
    await waitFor("the bars hidden", async () =>
      (await bars.display("subtitles-spoken")) === "none" && (await bars.display("subtitles-translated")) === "none");
  });
}
