// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Starting and stopping sceneuron-dev for the end-to-end runners that test against it without OBS:
// editor-browser.mjs and streamdeck-live.mjs.

import { spawn } from "node:child_process";
import { join, resolve } from "node:path";
import { createInterface } from "node:readline";

const REPO = resolve(import.meta.dirname, "..");

/** The sceneuron-dev to run: `given` when a runner was told one, else the debug build. */
export function devBinary(given) {
  return resolve(given ?? join(REPO, "target", "debug", "sceneuron-dev"));
}

/** One sceneuron-dev process; `base` is its address, such as http://127.0.0.1:4466, once it printed one. */
export class DevServer {
  base = null;
  #proc;
  #ready;

  /**
   * Spawns `binary` with `args`, and with `env` in place of this process's environment when given. The process runs
   * from here on, so `stop()` also ends a server that never printed an address.
   */
  constructor(binary, args, env) {
    const proc = spawn(binary, args, { stdio: "pipe", env: env ?? process.env });
    this.#proc = proc;
    // Asking a server that already exited to quit must not crash the run.
    proc.stdin.on("error", () => {});
    const output = [];
    proc.stderr.on("data", (chunk) => output.push(String(chunk).trim()));
    this.#ready = new Promise((resolve, reject) => {
      const silent = () => reject(new Error(`no editor URL from sceneuron-dev: ${output.join(" ")}`));
      const timer = setTimeout(silent, 15_000);
      createInterface({ input: proc.stdout }).on("line", (line) => {
        output.push(line);
        const match = /^Editor:\s+(http:\/\/127\.0\.0\.1:\d+)\/editor[?#]/.exec(line);
        if (match) {
          clearTimeout(timer);
          resolve(match[1]);
        }
      });
      proc.on("error", (error) => reject(new Error(`cannot run ${binary}; build it first: ${error.message}`)));
      proc.on("exit", (code) => reject(new Error(`sceneuron-dev exited with ${code}: ${output.join(" ")}`)));
    });
    // A runner that stops before awaiting `ready()` must not see this as an unhandled rejection.
    this.#ready.catch(() => {});
  }

  /** Waits until the server prints its editor URL, and returns the address from it. */
  async ready() {
    this.base = await this.#ready;
    return this.base;
  }

  /** Types `line` on the server's standard input, as a user does to simulate OBS going on air. */
  say(line) {
    this.#proc.stdin.write(`${line}\n`);
  }

  /** Asks the server to quit as a user would, and kills it when it does not. */
  async stop() {
    const proc = this.#proc;
    if (proc.exitCode !== null || proc.signalCode !== null) return;
    const exited = new Promise((resolve) => proc.once("exit", resolve));
    proc.stdin.end("q\n");
    const timers = [setTimeout(() => proc.kill(), 3000), setTimeout(() => proc.kill("SIGKILL"), 6000)];
    await exited;
    timers.forEach(clearTimeout);
  }
}
