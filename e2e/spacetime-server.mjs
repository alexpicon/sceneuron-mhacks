// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// A throwaway SpacetimeDB for room-live.mjs: `spacetime start` on a free port with its own data
// directory, signing keys, and CLI config, all inside the run's work directory, so nothing of the
// SpacetimeDB install on this computer is read or changed; publishing room/module to it; and stopping
// the server with everything it started.

import { execFileSync, spawn, spawnSync } from "node:child_process";
import { mkdirSync, openSync } from "node:fs";
import { createServer } from "node:net";
import { delimiter, dirname, join, resolve } from "node:path";

const REPO = resolve(import.meta.dirname, "..");

/** A TCP port on 127.0.0.1 that was free a moment ago. */
export async function freePort() {
  const server = createServer();
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  const { port } = server.address();
  await new Promise((done) => server.close(done));
  return port;
}

/**
 * The environment for building the module: the toolchain rustup selects goes first on the PATH, since
 * a Rust from another package manager, such as Homebrew's, has no wasm32 standard library. Running a
 * toolchain's binaries directly needs its libraries, which rustup's own proxies would add.
 */
export function rustupEnv(env = process.env) {
  let cargo;
  try {
    const options = { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] };
    cargo = execFileSync("rustup", ["which", "cargo"], options).trim();
  } catch {
    return { ...env };
  }
  const bin = dirname(cargo);
  const next = { ...env, PATH: [bin, env.PATH].filter(Boolean).join(delimiter) };
  next.DYLD_FALLBACK_LIBRARY_PATH = [join(dirname(bin), "lib"), env.DYLD_FALLBACK_LIBRARY_PATH].filter(Boolean)
    .join(delimiter);
  return next;
}

/** One SpacetimeDB server; `url` is its address once `start()` returned. */
export class Spacetime {
  url = null;
  #binary;
  #dir;
  #proc = null;

  constructor(binary, dir) {
    this.#binary = binary;
    this.#dir = dir;
    mkdirSync(join(dir, "keys"), { recursive: true });
  }

  /** The CLI with this run's own config file, so a token from publishing stays in the work directory. */
  #args(...rest) {
    return ["--config-path", join(this.#dir, "cli.toml"), ...rest];
  }

  /** Starts the server and waits until it answers, at most 60 seconds. */
  async start() {
    const port = await freePort();
    const log = openSync(join(this.#dir, "spacetime.log"), "a");
    const args = this.#args(
      "start",
      "--listen-addr",
      `127.0.0.1:${port}`,
      "--data-dir",
      join(this.#dir, "data"),
      "--jwt-pub-key-path",
      join(this.#dir, "keys", "id_ecdsa.pub"),
      "--jwt-priv-key-path",
      join(this.#dir, "keys", "id_ecdsa"),
      "--non-interactive",
    );
    // Its own process group, so stopping it also stops the server process the CLI starts.
    this.#proc = spawn(this.#binary, args, { detached: true, stdio: ["ignore", log, log] });
    const exited = new Promise((done) => this.#proc.once("exit", (code) => done(code)));
    this.#proc.once("error", () => {});
    const url = `http://127.0.0.1:${port}`;
    const deadline = Date.now() + 60_000;
    while (Date.now() < deadline) {
      const code = await Promise.race([exited, new Promise((done) => setTimeout(() => done(undefined), 250))]);
      if (code !== undefined) throw new Error(`spacetime start exited with ${code}; see ${this.#dir}/spacetime.log`);
      const answered = await fetch(`${url}/v1/ping`).then((reply) => reply.ok, () => false);
      if (answered) {
        this.url = url;
        return url;
      }
    }
    throw new Error(`SpacetimeDB did not answer on ${url} within a minute`);
  }

  /** Builds room/module and publishes it as `database`; returns the CLI's line about the database. */
  publish(database) {
    const module = join(REPO, "room", "module");
    const args = this.#args("publish", "--server", this.url, "--module-path", module, "--yes", database);
    const result = spawnSync(this.#binary, args, { env: rustupEnv(), encoding: "utf8", timeout: 600_000 });
    const output = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim().split("\n");
    if (result.status !== 0) throw new Error(`spacetime publish failed: ${output.slice(-8).join(" | ")}`);
    return output.find((line) => /database with name/i.test(line)) ?? output.at(-1);
  }

  /** Stops the server and its process group; kills them when they do not stop within five seconds. */
  async stop() {
    const proc = this.#proc;
    if (!proc || proc.exitCode !== null || proc.signalCode !== null) return;
    const exited = new Promise((done) => proc.once("exit", done));
    const signal = (name) => {
      try {
        process.kill(-proc.pid, name);
      } catch {
        // The group is already gone.
      }
    };
    signal("SIGTERM");
    const timer = setTimeout(() => signal("SIGKILL"), 5000);
    await exited;
    clearTimeout(timer);
  }
}
