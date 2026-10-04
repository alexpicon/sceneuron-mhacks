// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Shared fixtures for the flood tests: snapshots on a 1920x1080 canvas, and a root in a fake document whose nodes
// record their animations, with a fake clock for the removal timer.

import { createDocument } from "./fake-dom.mjs";

export const CANVAS = { width: 1920, height: 1080 };

export function snapshot(flood, nowMs, elements = []) {
  return { revision: 1, now_ms: nowMs, canvas: CANVAS, elements, flood };
}

/** A root in a fake document whose nodes record their animations, and a fake clock for the removal timer. */
export function stage() {
  const doc = createDocument();
  const animations = [];
  const create = doc.createElement;
  doc.createElement = (tag) => {
    const node = create(tag);
    node.animate = (frames, timing) => animations.push({ node, frames, timing });
    return node;
  };
  const createNS = doc.createElementNS;
  doc.createElementNS = (namespace, tag) => {
    const node = createNS(namespace, tag);
    node.animate = (frames, timing) => animations.push({ node, frames, timing });
    return node;
  };
  const root = doc.body.appendChild(doc.createElement("div"));
  const timers = new Map();
  let next = 1;
  const options = {
    schedule: (callback, ms) => {
      timers.set(next, { callback, ms });
      return next++;
    },
    cancel: (timer) => timers.delete(timer),
  };
  const fire = () => {
    const pending = [...timers.values()];
    timers.clear();
    pending.forEach((timer) => timer.callback());
  };
  return { root, animations, timers, options, fire };
}

/** The flood layers inside `root`. */
export const layers = (root) => root.children.filter((child) => child.classList.contains("sn-flood"));
