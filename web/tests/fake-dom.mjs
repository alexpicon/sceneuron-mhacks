// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// A minimal DOM stand-in with just the surface render.js uses, so render() runs under Node.

class FakeClassList {
  constructor() {
    this.names = new Set();
  }

  add(...names) {
    for (const name of names) {
      this.names.add(name);
    }
  }

  remove(...names) {
    for (const name of names) {
      this.names.delete(name);
    }
  }

  contains(name) {
    return this.names.has(name);
  }

  toggle(name, force) {
    const on = force === undefined ? !this.names.has(name) : Boolean(force);
    if (on) {
      this.names.add(name);
    } else {
      this.names.delete(name);
    }
    return on;
  }
}

class FakeStyle {
  constructor() {
    this.values = new Map();
  }

  setProperty(name, value) {
    this.values.set(name, String(value));
  }

  removeProperty(name) {
    this.values.delete(name);
  }

  getPropertyValue(name) {
    return this.values.get(name) ?? "";
  }
}

export class FakeElement {
  constructor(ownerDocument, tagName) {
    this.ownerDocument = ownerDocument;
    this.tagName = tagName.toUpperCase();
    this.children = [];
    this.parentNode = null;
    this.attributes = new Map();
    this.classList = new FakeClassList();
    this.style = new FakeStyle();
    this.listeners = new Map();
    this.text = "";
    this.reflows = 0;
  }

  get firstChild() {
    return this.children[0] ?? null;
  }

  get nextSibling() {
    if (!this.parentNode) {
      return null;
    }
    const siblings = this.parentNode.children;
    return siblings[siblings.indexOf(this) + 1] ?? null;
  }

  /** Reading layout forces a reflow; counted so tests can see animation restarts. */
  get offsetWidth() {
    this.reflows += 1;
    return 0;
  }

  get textContent() {
    return this.text + this.children.map((child) => child.textContent).join("");
  }

  set textContent(value) {
    for (const child of this.children) {
      child.parentNode = null;
    }
    this.children = [];
    this.text = String(value);
  }

  setAttribute(name, value) {
    this.attributes.set(name, String(value));
  }

  getAttribute(name) {
    return this.attributes.has(name) ? this.attributes.get(name) : null;
  }

  appendChild(child) {
    return this.insertBefore(child, null);
  }

  insertBefore(child, reference) {
    if (child.parentNode) {
      child.parentNode.detach(child);
    }
    const index = reference ? this.children.indexOf(reference) : this.children.length;
    this.children.splice(index, 0, child);
    child.parentNode = this;
    return child;
  }

  detach(child) {
    this.children.splice(this.children.indexOf(child), 1);
    child.parentNode = null;
  }

  remove() {
    if (this.parentNode) {
      this.parentNode.detach(this);
    }
  }

  addEventListener(type, listener) {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  dispatch(type, fields) {
    for (const listener of this.listeners.get(type) ?? []) {
      listener({ type, target: this, ...fields });
    }
  }
}

/**
 * A document whose `defaultView` optionally provides constructable stylesheets. Without one,
 * render.js falls back to a <style> element in the head.
 */
export function createDocument({ constructable = false } = {}) {
  const doc = {
    createElement: (tag) => new FakeElement(doc, tag),
    createElementNS: (namespace, tag) => Object.assign(new FakeElement(doc, tag), { namespace }),
  };
  doc.documentElement = new FakeElement(doc, "html");
  doc.head = doc.documentElement.appendChild(new FakeElement(doc, "head"));
  doc.body = doc.documentElement.appendChild(new FakeElement(doc, "body"));
  if (constructable) {
    doc.defaultView = {
      CSSStyleSheet: class {
        replaceSync(text) {
          this.text = text;
        }
      },
    };
    doc.adoptedStyleSheets = [];
  } else {
    doc.defaultView = null;
  }
  return doc;
}

/** Collects element nodes in the subtree, depth first. */
export function descendants(node) {
  return node.children.flatMap((child) => [child, ...descendants(child)]);
}
