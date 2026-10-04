// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Minimal obs-websocket v5 client using the WebSocket built into Node 22+.

const OP = { hello: 0, identify: 1, identified: 2, event: 5, request: 6, response: 7 };
const VENDOR_EVENTS = 1 << 9;

export class ObsWebSocket {
  #socket;
  #pending = new Map();
  #nextId = 1;
  events = [];

  static async connect(url, { timeoutMs = 10_000 } = {}) {
    const client = new ObsWebSocket();
    await client.#open(url, timeoutMs);
    return client;
  }

  #open(url, timeoutMs) {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error(`no obs-websocket handshake from ${url}`)), timeoutMs);
      this.#socket = new WebSocket(url);
      this.#socket.addEventListener("error", () => reject(new Error(`cannot connect to ${url}`)));
      this.#socket.addEventListener("message", (message) => {
        const { op, d } = JSON.parse(message.data);
        if (op === OP.hello) {
          if (d.authentication) reject(new Error("obs-websocket requires a password; the e2e config disables it"));
          this.#send(OP.identify, { rpcVersion: 1, eventSubscriptions: VENDOR_EVENTS });
        } else if (op === OP.identified) {
          clearTimeout(timer);
          resolve();
        } else if (op === OP.response) {
          this.#settle(d);
        } else if (op === OP.event) {
          this.events.push(d);
        }
      });
    });
  }

  #send(op, d) {
    this.#socket.send(JSON.stringify({ op, d }));
  }

  #settle(d) {
    const pending = this.#pending.get(d.requestId);
    if (!pending) return;
    this.#pending.delete(d.requestId);
    if (d.requestStatus.result) pending.resolve(d.responseData ?? {});
    else pending.reject(new Error(`${d.requestType} failed: ${d.requestStatus.code} ${d.requestStatus.comment ?? ""}`));
  }

  request(requestType, requestData = {}, { timeoutMs = 15_000 } = {}) {
    const requestId = String(this.#nextId++);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(requestId);
        reject(new Error(`${requestType} timed out`));
      }, timeoutMs);
      this.#pending.set(requestId, {
        resolve: (value) => (clearTimeout(timer), resolve(value)),
        reject: (error) => (clearTimeout(timer), reject(error)),
      });
      this.#send(OP.request, { requestType, requestId, requestData });
    });
  }

  async vendor(requestType, requestData = {}) {
    const reply = await this.request("CallVendorRequest", { vendorName: "sceneuron", requestType, requestData });
    return reply.responseData;
  }

  close() {
    this.#socket?.close();
  }
}
