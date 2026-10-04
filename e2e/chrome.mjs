// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Finds and launches headless Chrome with a throwaway profile for browser tests, opens tabs as
// DevTools Protocol pages, and stops Chrome and deletes the profile afterwards.

import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { join } from "node:path";

import { Page, sleep } from "./cdp.mjs";

const MAC_CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
// Media may play without a click, as in OBS's browser sources, so the overlay can speak cards.
const CHROME_FLAGS = ["--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check", "--mute-audio"]
  .concat(["--disable-extensions", "--disable-sync", "--disable-background-networking"])
  .concat(["--autoplay-policy=no-user-gesture-required"]);

/** The Chrome to run: the given path, then $CHROME, then the usual install location. */
export function findChrome(given = process.env.CHROME) {
  return given || MAC_CHROME;
}

export class Browser {
  #pages = [];

  constructor(proc, profile) {
    this.proc = proc;
    this.profile = profile;
    /** The browser's own DevTools endpoint, once Chrome reports it. */
    this.url = null;
  }

  /** Starts headless Chrome with a new profile directory under `workDir`. */
  static async launch({ chrome, workDir }) {
    const profile = mkdtempSync(join(workDir, "chrome-profile-"));
    const args = [...CHROME_FLAGS, `--user-data-dir=${profile}`, "--remote-debugging-port=0", "about:blank"];
    const browser = new Browser(spawn(chrome, args, { stdio: ["ignore", "ignore", "pipe"] }), profile);
    try {
      browser.url = await new Promise((resolve, reject) => {
        let output = "";
        const timer = setTimeout(() => reject(new Error(`Chrome did not start: ${output.trim()}`)), 20_000);
        browser.proc.stderr.on("data", (chunk) => {
          output += chunk;
          const match = /DevTools listening on (ws:\/\/\S+)/.exec(output);
          if (match) {
            clearTimeout(timer);
            resolve(match[1]);
          }
        });
        browser.proc.on("error", (error) => reject(new Error(`cannot run Chrome at ${chrome}: ${error.message}`)));
        browser.proc.on("exit", (code) => reject(new Error(`Chrome exited with ${code}: ${output.trim()}`)));
      });
      return browser;
    } catch (error) {
      await browser.close();
      throw error;
    }
  }

  /** Opens a new tab with the given viewport size. */
  async newPage({ width = 1280, height = 800 } = {}) {
    const response = await fetch(`http://${new URL(this.url).host}/json/new?about:blank`, { method: "PUT" });
    const page = new Page((await response.json()).webSocketDebuggerUrl);
    this.#pages.push(page);
    await page.open({ width, height });
    return page;
  }

  /** Stops Chrome and deletes its profile, even after a failed start. */
  async close() {
    for (const page of this.#pages) page.close();
    if (this.proc.exitCode === null && this.proc.signalCode === null) {
      const exited = new Promise((resolve) => this.proc.once("exit", resolve));
      // Closing through DevTools lets Chrome's helper processes finish writing the profile first.
      await this.#requestClose().catch(() => this.proc.kill());
      const timers = [setTimeout(() => this.proc.kill(), 5000), setTimeout(() => this.proc.kill("SIGKILL"), 8000)];
      await exited;
      timers.forEach(clearTimeout);
    }
    // A helper process that outlives the browser can still recreate part of the profile.
    for (let attempt = 0; attempt < 10 && existsSync(this.profile); attempt += 1) {
      rmSync(this.profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
      await sleep(300);
    }
  }

  async #requestClose() {
    if (!this.url) throw new Error("Chrome has no DevTools endpoint");
    const socket = new WebSocket(this.url);
    await new Promise((resolve, reject) => {
      socket.onopen = resolve;
      socket.onerror = () => reject(new Error("cannot reach Chrome's DevTools endpoint"));
    });
    socket.send(JSON.stringify({ id: 1, method: "Browser.close" }));
  }
}
