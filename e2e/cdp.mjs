// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Minimal Chrome DevTools Protocol driver for browser tests: drives one tab over its WebSocket
// (Node 22+ has WebSocket built in) and records console messages, uncaught exceptions, and
// Content Security Policy violations.

const CALL_TIMEOUT_MS = 15_000;
const KEYS = { Enter: 13, Escape: 27, End: 35, Home: 36, ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, ArrowDown: 40 };
// Reports violations as console errors, so the console check catches them too.
const CSP_LISTENER = `document.addEventListener("securitypolicyviolation", (event) =>
  console.error("CSP violation", event.violatedDirective, event.blockedURI, event.sourceFile, event.lineNumber));`;

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Polls `probe` until it returns a truthy value; throws with the last value after the timeout. */
export async function waitFor(what, probe, timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  let last;
  for (;;) {
    try {
      last = await probe();
      if (last) return last;
    } catch (error) {
      last = error;
    }
    if (Date.now() > deadline) {
      const shown = last instanceof Error ? last.message : JSON.stringify(last);
      throw new Error(`timed out waiting for ${what} (last: ${shown})`);
    }
    await sleep(50);
  }
}

export class Page {
  #ws;
  #nextId = 1;
  #pending = new Map();
  #handlers = new Map();
  /** Console messages, exceptions, and browser log entries as `{level, source, text, url}`. */
  entries = [];

  constructor(url) {
    this.url = url;
  }

  async open({ width, height }) {
    this.#ws = new WebSocket(this.url);
    await new Promise((resolve, reject) => {
      this.#ws.onopen = resolve;
      this.#ws.onerror = () => reject(new Error(`cannot connect to ${this.url}`));
    });
    this.#ws.onmessage = (event) => this.#receive(JSON.parse(event.data));
    for (const method of ["Runtime.enable", "Log.enable", "Page.enable"]) await this.send(method);
    await this.send("Page.addScriptToEvaluateOnNewDocument", { source: CSP_LISTENER });
    await this.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  }

  #receive(message) {
    const pending = this.#pending.get(message.id);
    if (pending) {
      this.#pending.delete(message.id);
      clearTimeout(pending.timer);
      if (message.error) pending.reject(new Error(`${pending.method}: ${message.error.message}`));
      else pending.resolve(message.result);
      return;
    }
    const { method, params } = message;
    const level = (name) => (name === "warning" ? "warn" : name);
    if (method === "Runtime.consoleAPICalled") {
      const text = params.args.map((arg) => arg.value ?? arg.description ?? "").join(" ");
      this.entries.push({ level: level(params.type), source: "console", text, url: "" });
    } else if (method === "Runtime.exceptionThrown") {
      const text = params.exceptionDetails.exception?.description ?? params.exceptionDetails.text;
      this.entries.push({ level: "error", source: "exception", text, url: "" });
    } else if (method === "Log.entryAdded") {
      const { source, text, url = "" } = params.entry;
      this.entries.push({ level: level(params.entry.level), source, text, url });
    }
    for (const handler of this.#handlers.get(method) ?? []) handler(params);
  }

  /** Errors of any kind, except those that `allowed` accepts. */
  errors(allowed = () => false) {
    return this.entries.filter((entry) => entry.level === "error" && !allowed(entry));
  }

  on(method, handler) {
    this.#handlers.set(method, [...(this.#handlers.get(method) ?? []), handler]);
  }

  off(method, handler) {
    this.#handlers.set(method, (this.#handlers.get(method) ?? []).filter((other) => other !== handler));
  }

  /** The parameters of the next `method` event. */
  next(method, timeoutMs = CALL_TIMEOUT_MS) {
    return new Promise((resolve, reject) => {
      const handler = (params) => {
        clearTimeout(timer);
        this.off(method, handler);
        resolve(params);
      };
      const timer = setTimeout(() => {
        this.off(method, handler);
        reject(new Error(`no ${method} event`));
      }, timeoutMs);
      this.on(method, handler);
    });
  }

  send(method, params = {}) {
    const id = this.#nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new Error(`${method} timed out`));
      }, CALL_TIMEOUT_MS);
      this.#pending.set(id, { method, resolve, reject, timer });
      this.#ws.send(JSON.stringify({ id, method, params }));
    });
  }

  async goto(url) {
    const loaded = this.next("Page.loadEventFired");
    await this.send("Page.navigate", { url });
    await loaded;
  }

  /** Evaluates an expression in the page, awaiting promises, and returns its JSON value. */
  async eval(expression) {
    const result = await this.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) {
      throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
    }
    return result.result.value;
  }

  /** Presses a key. Modifiers: 1 Alt, 2 Ctrl, 4 Meta, 8 Shift. */
  async key(key, { code = key, modifiers = 0, text } = {}) {
    const base = { key, code, modifiers, windowsVirtualKeyCode: KEYS[key] ?? key.toUpperCase().charCodeAt(0) };
    await this.send("Input.dispatchKeyEvent", { type: text ? "keyDown" : "rawKeyDown", ...base, text });
    await this.send("Input.dispatchKeyEvent", { type: "keyUp", ...base });
  }

  /** Types text into the focused control, as an input method would. */
  async type(text) {
    await this.send("Input.insertText", { text });
  }

  /** Sends a mouse event; `buttons` is 1 while the left button is held. */
  async mouse(type, x, y, buttons = 0) {
    const [button, clickCount] = type === "mouseMoved" ? [buttons ? "left" : "none", 0] : ["left", 1];
    await this.send("Input.dispatchMouseEvent", { type, x, y, buttons, button, clickCount });
  }

  /** Clicks like a user: scrolls the element into view, moves the mouse over it, presses, and releases. */
  async click(selector) {
    const box = await this.eval(`(() => {
      const node = document.querySelector(${JSON.stringify(selector)});
      if (!node) return null;
      node.scrollIntoView({ block: "center" });
      const rect = node.getBoundingClientRect();
      return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
    })()`);
    if (!box) throw new Error(`no element matches ${selector}`);
    const { x, y } = box;
    await this.mouse("mouseMoved", x, y);
    await this.mouse("mousePressed", x, y, 1);
    await this.mouse("mouseReleased", x, y);
  }

  /** The text of the first element matching `selector`, or null when there is none. */
  text(selector) {
    return this.eval(`document.querySelector(${JSON.stringify(selector)})?.textContent ?? null`);
  }

  /** The value of the control with this id. */
  value(id) {
    return this.eval(`document.getElementById(${JSON.stringify(id)}).value`);
  }

  /** The id of the focused element; empty when it has none. */
  focused() {
    return this.eval("document.activeElement.id");
  }

  /** Focuses a control, selects its contents, and types over them, like a user replacing a value. */
  async fill(id, text) {
    await this.eval(
      `(() => { const input = document.getElementById(${JSON.stringify(id)}); input.focus(); input.select(); })()`,
    );
    await this.type(text);
  }

  /** Picks a select option and fires the events that a user's choice fires. */
  async choose(id, option) {
    await this.eval(`(() => {
      const select = document.getElementById(${JSON.stringify(id)});
      select.value = ${JSON.stringify(option)};
      select.dispatchEvent(new Event("input", { bubbles: true }));
      select.dispatchEvent(new Event("change", { bubbles: true }));
    })()`);
  }

  /** A PNG screenshot of the viewport. */
  async screenshot() {
    const { data } = await this.send("Page.captureScreenshot", { format: "png" });
    return Buffer.from(data, "base64");
  }

  close() {
    for (const { reject, timer } of this.#pending.values()) {
      clearTimeout(timer);
      reject(new Error("the page was closed"));
    }
    this.#pending.clear();
    this.#ws?.close();
  }
}
