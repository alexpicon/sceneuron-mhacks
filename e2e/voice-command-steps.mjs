// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Voice command steps of the browser regression test (editor-browser.mjs), against the dev server's simulated AI
// (--demo-ai): the Voice section lists what can be said, from the server's guide; a reaction's voice switch in the
// Settings tab says what to say and is saved with the project; typed commands play a reaction, switch the theme, and
// hide an element at once; and words no local command knows come back from the simulated AI as a project edit, each
// said on the heard line. They run last, after the voice steps turned voice control on with push to talk.

import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const js = JSON.stringify;

/** Runs the voice command steps. */
export async function runVoiceCommandSteps({ page, step, same, api }) {
  const { q, check, project, state, clean } = editorHelpers(page, api);
  const heard = () => page.text("#voice-heard");
  /** Waits for the heard line to start with `line`, saying what it showed instead when it never does. */
  const answered = async (line) => {
    try {
      await waitFor(js(line), async () => (await heard()).startsWith(line));
    } catch (error) {
      throw new Error(`${error.message}; the heard line: ${js(await heard())}`);
    }
  };
  /** The element `id` as the server saved it, whose name earlier steps may have changed. */
  const saved = async (id) => (await project()).elements.find((element) => element.id === id);
  const run = async (words) => {
    await page.fill("voice-command-text", words);
    await page.click("#voice-command-run");
  };

  await step("the Voice section lists what can be said, from the server's guide", async () => {
    await page.send("Page.bringToFront");
    await page.click("#tab-ai");
    await waitFor("the Voice section", async () => !(await q(`document.getElementById("voice").hidden`)));
    await page.click("#voice-guide summary");
    const titles = `[...document.querySelectorAll("#voice-guide-list h4")].map((title) => title.textContent)`;
    await waitFor("the guide", async () => (await q(titles)).length > 0);
    const shown = await q(titles);
    for (const title of ["Reactions", "Themes", "Overlay elements", "Camera, subtitles, and automation", "Anything else"]) {
      check(shown.includes(title), `${title} in ${js(shown)}`);
    }
    const lines = await q(`[...document.querySelectorAll("#voice-guide-list li")].map((line) => line.textContent)`);
    for (const line of [
      '"mind blown" or "play mind blown": plays 🤯 Mind blown at once',
      '"hacker theme" or "use the hacker theme": switches the overlay to the Hacker theme',
    ]) {
      check(lines.includes(line), `${line} in ${js(lines)}`);
    }
    const notes = await q(`[...document.querySelectorAll("#voice-guide-list .hint")].map((note) => note.textContent)`);
    check(notes.includes("Hold to talk while you say them, or type them below."), js(notes));
    return `${shown.length} groups, ${lines.length} commands`;
  });

  await step("a reaction's voice switch says what to say and is saved with the project", async () => {
    await page.click("#tab-settings");
    const switches = `[...document.querySelectorAll("[data-reaction-voice]")]`;
    const voices = await q(`${switches}.map((input) => [input.dataset.reactionVoice, input.checked])`);
    same(voices, [
      ["email", true], ["mind-blown", true], ["ship-it", true], ["coffee", true],
      ["absolutely-right", true], ["more-tokens", true], ["works-on-my-gpu", true],
      ["meatproxy", true], ["clanker", true],
    ], "the voice switches");
    same(await page.text('label[for="reaction-voice-email"]'), 'Voice: "could have been an email"', "the phrase");
    const note = () => page.text("#reaction-voice-status-email");
    await waitFor("waiting for always on", async () => (await note()).startsWith("Waiting for voice control"));
    await page.click("#reaction-voice-email");
    await waitFor("the email's voice off", async () => (await note()) === "Off: saying it plays nothing.");
    await page.click("#save-button");
    await clean();
    same((await project()).reactions.voice_off, ["email"], "saved");
    await page.click("#reaction-voice-email");
    await page.click("#save-button");
    await clean();
    same((await project()).reactions.voice_off, [], "restored");
  });

  await step("typed commands play a reaction, switch the theme, and hide an element at once", async () => {
    await page.click("#tab-ai");
    await run("");
    same(await page.text("#voice-command-error"), "Type 1 to 200 characters.", "an empty command");
    const before = (await state()).flood?.nonce ?? 0;
    await run("mind blown");
    await answered('Typed "mind blown": played 🤯 Mind blown.');
    const { flood } = await state();
    same([flood.flood, flood.nonce > before], ["mind-blown", true], "the flood");
    same(await page.value("voice-command-text"), "", "the box empties");
    await run("hacker theme");
    await answered('Typed "hacker theme": switched the overlay to the Hacker theme.');
    same((await project()).theme, "hacker", "the saved theme");
    await run("theme off");
    await answered('Typed "theme off": turned the theme off.');
    const { name } = await saved("brb");
    await run(`hide the ${name}`);
    await answered(`Typed "hide the ${name}": hid ${name}.`);
    const brb = (await state()).elements.find((element) => element.id === "brb");
    same([brb.props.visible, brb.overridden], [false, ["visible"]], "the held element");
    await api("POST", "/api/release", { element: "brb" });
  });

  await step("words no local command knows come back from the simulated AI as a project edit", async () => {
    const { name } = await saved("brb");
    await run(`make the ${name} blue`);
    await answered(`Typed "make the ${name} blue": the AI took it as "a simulated plan": changed the project: Simulated`);
    same((await saved("brb")).props.background, "#3b82f6ee", "the edit applied at once");
    const logged = await q(`[...document.querySelectorAll('#event-log li[data-source="voice"] .event-detail')]
      .map((node) => node.textContent)`);
    check(logged.some((line) => line.startsWith('the AI took it as "a simulated plan"')), js(logged));
  });
}
