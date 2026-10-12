// SG-000108 slice 4: an attribute-only DOM model of the main document, taken
// with CDP `DOMSnapshot.captureSnapshot`. The engine serialises its own DOM
// tree; no script runs in the page (so page-defined getters, prototypes and
// polyfills are never invoked) and nothing in the page changes. Open and
// closed shadow trees are part of the snapshot.

import type { DomElement, DomNode } from "./declarative.js";

/// Upper bound on snapshot nodes taken into the model.
export const MAX_SNAPSHOT_NODES = 100_000;

export interface CapturedSnapshot {
  documents: Array<{
    documentURL?: number;
    nodes: {
      parentIndex?: number[];
      nodeType?: number[];
      nodeName?: number[];
      nodeValue?: number[];
      backendNodeId?: number[];
      attributes?: number[][];
    };
  }>;
  strings: string[];
}

interface MutableElement {
  kind: "element";
  tag: string;
  attrs: Map<string, string>;
  children: DomNode[];
  parent: MutableElement | null;
  node: number;
}

export interface DocumentModel {
  root: DomElement;
  truncated: boolean;
  /// The snapshot's own document URL.
  url: string;
}

/// Build the model of the first (main-frame) document. Element names are
/// lowercased; text nodes keep their value; comments, doctypes and other
/// node kinds are skipped. Nodes past the bound are dropped (truncated).
export function documentModel(snapshot: CapturedSnapshot): DocumentModel {
  const doc = snapshot.documents?.[0];
  if (!doc) throw new Error("no document");
  const strings = snapshot.strings ?? [];
  const nodes = doc.nodes;
  const parentIndex = nodes.parentIndex ?? [];
  const nodeType = nodes.nodeType ?? [];
  const nodeName = nodes.nodeName ?? [];
  const nodeValue = nodes.nodeValue ?? [];
  const backend = nodes.backendNodeId ?? [];
  const attributes = nodes.attributes ?? [];
  const text = (index: number | undefined): string => (index !== undefined && index >= 0 ? (strings[index] ?? "") : "");

  const root: MutableElement = { kind: "element", tag: "#root", attrs: new Map(), children: [], parent: null, node: 0 };
  // Each snapshot index maps to the element its children attach to.
  const slots: Array<MutableElement | undefined> = [];
  const count = Math.min(nodeType.length, MAX_SNAPSHOT_NODES);
  for (let i = 0; i < count; i++) {
    const parentSlot = (parentIndex[i] ?? -1) >= 0 ? slots[parentIndex[i] ?? -1] : root;
    const parent = parentSlot ?? root;
    const type = nodeType[i];
    if (type === 1) {
      const attrs = new Map<string, string>();
      const flat = attributes[i] ?? [];
      for (let a = 0; a + 1 < flat.length; a += 2) {
        // Exact names: the HTML parser and setAttribute lowercase them, so a
        // differently cased name (setAttributeNS) is not a declaration.
        const name = text(flat[a]);
        if (name && !attrs.has(name)) attrs.set(name, text(flat[a + 1]));
      }
      const element: MutableElement = {
        kind: "element",
        tag: text(nodeName[i]).toLowerCase(),
        attrs,
        children: [],
        parent,
        node: backend[i] ?? 0
      };
      parent.children.push(element);
      slots[i] = element;
    } else if (type === 3) {
      parent.children.push({ kind: "text", text: text(nodeValue[i]), parent });
    } else if (type === 9) {
      // The document: its children attach to the root.
      slots[i] = parent;
    } else if (type === 11) {
      // A shadow root: a separate tree scope under its host.
      const shadow: MutableElement = { kind: "element", tag: "#shadow-root", attrs: new Map(), children: [], parent, node: backend[i] ?? 0 };
      parent.children.push(shadow);
      slots[i] = shadow;
    }
  }
  return { root, truncated: nodeType.length > MAX_SNAPSHOT_NODES, url: text(doc.documentURL) };
}
