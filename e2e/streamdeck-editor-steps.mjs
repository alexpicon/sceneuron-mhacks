// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Stream Deck steps of the browser regression test (editor-browser.mjs): Pair the Stream Deck on this computer in
// the Cards tab opens a streamdeck:// deep link carrying the editor's port and a fresh one-time code, which the
// page catches instead of following, so no Stream Deck app is ever asked; the code in the link trades once for the
// control token, as the plugin trades it, and the editor then says Paired. In a tab with the user agent of an OBS
// dock, the editor offers only a code to type, since an OBS dock cannot open the link.

import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";
import { expectedEditorError } from "./live-steps.mjs";

const js = JSON.stringify;
const PAIR_LINK = { protocol: "streamdeck:", host: "plugins", pathname: "/message/ai.chaski.sceneuron/pair" };
const OBS_DOCK =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) " +
  "Chrome/127.0.6533.120 OBS/31.1.2 Safari/537.36";
// Following a link would ask the operating system for the Stream Deck app; the page records it instead.
const RECORD_LINKS = `document.addEventListener("click", (event) => {
  const link = event.target.closest && event.target.closest('a[href^="streamdeck:"]');
  if (link) {
    event.preventDefault();
    (window.deepLinks = window.deepLinks || []).push(link.getAttribute("href"));
  }
}, true)`;

/** Runs the Stream Deck steps; `step(name, body)` reports each one. */
export async function runStreamDeckEditorSteps({ page, browser, step, same, api, base, token }) {
  const { q, check, toasts } = editorHelpers(page, api);
  const pairings = async () => (await api("GET", "/api/control/pairing")).pairings;
  let code;
  let before;

  await step("pairing the Stream Deck on this computer opens a streamdeck:// link with a fresh code", async () => {
    await page.click("#tab-cards");
    same(await q(`document.getElementById("pairing-in-obs").hidden`), true, "the note on pairing in an OBS dock");
    await q(RECORD_LINKS);
    before = await pairings();
    await page.click("#pair-here");
    await waitFor("the deep link", async () => (await q(`(window.deepLinks || []).length`)) === 1);
    const link = new URL(await q(`window.deepLinks[0]`));
    same({ protocol: link.protocol, host: link.host, pathname: link.pathname }, PAIR_LINK, "where the link goes");
    const query = Object.fromEntries(link.searchParams);
    same(Object.keys(query), ["port", "code", "streamdeck"], "the link's parameters");
    same([query.port, query.streamdeck], [new URL(base).port, "hidden"], "the port and the passive window");
    check(/^\d{6}$/.test(query.code), `the code is ${js(query.code)}`);
    code = query.code;
    same((await page.text("#pairing-digits")).replace(" ", ""), code, "the code on screen");
    same(await q(`document.getElementById("pairing-sent").hidden`), false, "the note that the code was sent");
    same(await page.focused(), "pairing-sent", "focus");
    same(await q(`location.pathname`), "/editor", "the page address");
    return `${link.protocol}//${link.host}${link.pathname}?port=${query.port}&code=(6 digits)&streamdeck=hidden`;
  });

  await step("the code in the link trades once for the control token, and the editor then says Paired", async () => {
    const pair = () => fetch(`${base}/api/control/pair`, { method: "POST", body: js({ code }) });
    const paired = await pair();
    same(paired.status, 200, "pairing reply");
    const { token } = await paired.json();
    const view = await fetch(`${base}/api/control/state`, { headers: { "X-Sceneuron-Token": token } });
    same(view.status, 200, "control view with the token");
    same((await pair()).status, 403, "the same code again");
    await waitFor("Paired", async () => (await page.text("#pairing-digits")) === "Paired");
    same(await pairings(), before + 1, "codes traded");
    const said = await page.text("#pairing-expiry");
    same(said, "A Stream Deck paired with this code. Every Sceneuron key on it works now.", "the note");
    check((await toasts()).includes("A Stream Deck paired. Its Sceneuron keys work now."), "no notification");
    same(await q(`document.getElementById("pairing-steps").hidden`), true, "the steps to type the code");
  });

  await step("in an OBS dock, the editor offers a code to type and no link, which the dock cannot open", async () => {
    // OBS adds a dock 460 px wide.
    const dock = await browser.newPage({ width: 460, height: 800 });
    try {
      const dq = (expression) => dock.eval(expression);
      // At dock width the panels stack and still move while the editor loads, so the step presses keys, which go
      // to the focused control wherever it is, rather than points on the screen.
      const press = async (id) => {
        await dq(`document.getElementById(${js(id)}).focus()`);
        await dock.key("Enter", { text: "\r" });
      };
      // A tab takes input only while it is in front.
      await dock.send("Page.bringToFront");
      await dock.send("Emulation.setUserAgentOverride", { userAgent: OBS_DOCK });
      await dock.goto(`${base}/editor?token=${token}`);
      await waitFor("the Stream Deck section", () => dq(`Boolean(document.getElementById("pair-stream-deck"))`));
      await press("tab-cards");
      await waitFor("the Cards tab", () => dq(`!document.getElementById("panel-cards").hidden`));
      await dq(RECORD_LINKS);
      same(await dq(`document.getElementById("pair-here")`), null, "the one-click button");
      same(await dq(`document.getElementById("pairing-in-obs").hidden`), false, "the note on pairing in an OBS dock");
      check(/choose Connect a Stream Deck/.test(await dock.text("#pairing-in-obs")), "the note names the way");
      await press("pair-stream-deck");
      await waitFor("a code", async () => /^\d{3} \d{3}$/.test(await dock.text("#pairing-digits")));
      same(await dock.focused(), "pairing-code", "focus");
      const hidden = await dq(`["pairing-sent", "pairing-steps"].map((id) => document.getElementById(id).hidden)`);
      same(hidden, [true, false], "the note that a code was sent, and the steps to type it");
      same(await dq(`window.deepLinks || []`), [], "links followed");
      same(await dq(`location.pathname`), "/editor", "the page address");
      same(dock.errors(expectedEditorError), [], "console errors in the dock");
    } finally {
      await dock.send("Page.close").catch(() => {});
      dock.close();
      await page.send("Page.bringToFront");
    }
  });
}
