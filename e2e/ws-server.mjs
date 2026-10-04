// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// The server side of RFC 6455 WebSockets, as small as the room stand-in needs: the opening handshake with one
// subprotocol, the frames a client sends (masked, possibly fragmented), and text, pong, and close frames back.

import { createHash } from "node:crypto";

const GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const TEXT = 0x1;
const CLOSE = 0x8;
const PING = 0x9;
const PONG = 0xa;

/** The Sec-WebSocket-Accept value that answers a client's Sec-WebSocket-Key. */
export function acceptKey(key) {
  return createHash("sha1").update(key + GUID).digest("base64");
}

/** One unmasked frame with FIN set, as a server sends it. */
export function encodeFrame(opcode, payload) {
  const body = Buffer.from(payload);
  let head;
  if (body.length < 126) {
    head = Buffer.from([0x80 | opcode, body.length]);
  } else if (body.length < 0x10000) {
    head = Buffer.alloc(4);
    head.writeUInt16BE(body.length, 2);
    head[1] = 126;
  } else {
    head = Buffer.alloc(10);
    head.writeBigUInt64BE(BigInt(body.length), 2);
    head[1] = 127;
  }
  head[0] = 0x80 | opcode;
  return Buffer.concat([head, body]);
}

/** The whole frames at the start of `buffer`, unmasked, and the bytes of a frame still arriving. */
export function decodeFrames(buffer) {
  const frames = [];
  let at = 0;
  while (buffer.length - at >= 2) {
    const [first, second] = [buffer[at], buffer[at + 1]];
    let length = second & 0x7f;
    let offset = at + 2;
    if (length === 126) {
      if (buffer.length < offset + 2) break;
      length = buffer.readUInt16BE(offset);
      offset += 2;
    } else if (length === 127) {
      if (buffer.length < offset + 8) break;
      length = Number(buffer.readBigUInt64BE(offset));
      offset += 8;
    }
    const masked = (second & 0x80) !== 0;
    const start = offset + (masked ? 4 : 0);
    if (buffer.length < start + length) break;
    const payload = Buffer.from(buffer.subarray(start, start + length));
    if (masked) {
      for (let index = 0; index < length; index += 1) payload[index] ^= buffer[offset + (index % 4)];
    }
    frames.push({ fin: (first & 0x80) !== 0, opcode: first & 0x0f, payload });
    at = start + length;
  }
  return { frames, rest: buffer.subarray(at) };
}

/**
 * Answers an HTTP upgrade request on `socket` when it asks for `protocol`; returns a peer that calls
 * `onText(text)` for each message and `onClose()` once, or null after refusing the request.
 */
export function upgrade(request, socket, protocol, { onText, onClose }) {
  const offered = (request.headers["sec-websocket-protocol"] ?? "").split(",").map((name) => name.trim());
  const key = request.headers["sec-websocket-key"];
  if (!key || !offered.includes(protocol)) {
    socket.end("HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n");
    return null;
  }
  socket.write(
    "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n" +
      `Sec-WebSocket-Accept: ${acceptKey(key)}\r\nSec-WebSocket-Protocol: ${protocol}\r\n\r\n`,
  );
  let buffered = Buffer.alloc(0);
  let message = [];
  let closed = false;
  const close = () => {
    if (closed) return;
    closed = true;
    onClose();
  };
  const peer = {
    send: (text) => !closed && socket.write(encodeFrame(TEXT, text)),
    end: () => {
      if (!closed) socket.end(encodeFrame(CLOSE, Buffer.from([0x03, 0xe8])));
      close();
    },
  };
  socket.on("data", (chunk) => {
    const { frames, rest } = decodeFrames(Buffer.concat([buffered, chunk]));
    buffered = rest;
    for (const frame of frames) {
      if (frame.opcode === CLOSE) return peer.end();
      if (frame.opcode === PING) socket.write(encodeFrame(PONG, frame.payload));
      if (frame.opcode === TEXT || frame.opcode === 0x0) message.push(frame.payload);
      if (frame.fin && message.length && frame.opcode !== PING && frame.opcode !== PONG) {
        const text = Buffer.concat(message).toString("utf8");
        message = [];
        onText(text);
      }
    }
  });
  socket.on("close", close);
  socket.on("error", close);
  return peer;
}
