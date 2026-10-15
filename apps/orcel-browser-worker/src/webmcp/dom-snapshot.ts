// SG-000108 slice 4: an attribute-only DOM model of the main document, taken
// with CDP `DOMSnapshot.captureSnapshot`. The engine serialises its own DOM
// tree; no script runs in the page (so page-defined getters, prototypes and
// polyfills are never invoked) and nothing in the page changes. Open and
// closed shadow trees are part of the snapshot, flattened: the engine
// reports no shadow-root node, only a `shadowRootType` mark on every node
// inside a shadow tree. A marked node under an unmarked parent starts a
// shadow tree of that parent, and the model inserts a `#shadow-root` there.
// A shadow tree nested in another one cannot be told apart from its parent
// tree (both are marked), so it is modelled as part of it.

import type { DomElement, DomNode } from "./declarative.js";

/// Upper bound on snapshot nodes taken into the model.
export const MAX_SNAPSHOT_NODES = 100_000;

export interface CapturedSnapshot {
  documents: Array<{
    documentURL?: number;
    encodingName?: number;
    nodes: {
      parentIndex?: number[];
      nodeType?: number[];
      nodeName?: number[];
      nodeValue?: number[];
      backendNodeId?: number[];
      attributes?: number[][];
      shadowRootType?: { index: number[]; value: number[] };
      inputValue?: { index: number[]; value: number[] };
      inputChecked?: { index: number[] };
      optionSelected?: { index: number[] };
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
  inShadow?: boolean;
  live?: { value?: string; checked?: boolean; selected?: boolean };
}

export interface DocumentModel {
  root: DomElement;
  truncated: boolean;
  /// The snapshot's own document URL.
  url: string;
  /// The document's encoding as the engine reports it, lowercased ("" when
  /// not reported).
  encoding: string;
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
  const shadowed = new Set(nodes.shadowRootType?.index ?? []);
  // Live control state (rare data, by node index).
  const values = new Map<number, number>();
  (nodes.inputValue?.index ?? []).forEach((at, k) => values.set(at, nodes.inputValue?.value[k] ?? -1));
  const checked = new Set(nodes.inputChecked?.index ?? []);
  const selected = new Set(nodes.optionSelected?.index ?? []);
  // The `#shadow-root` inserted under each host, by host index.
  const shadowRoots = new Map<number, MutableElement>();
  for (let i = 0; i < count; i++) {
    const parentAt = parentIndex[i] ?? -1;
    const parentSlot = parentAt >= 0 ? slots[parentAt] : root;
    let parent = parentSlot ?? root;
    const marked = shadowed.has(i);
    // A shadow tree starts where a marked node sits under an unmarked one.
    if (marked && parentAt >= 0 && !shadowed.has(parentAt) && nodeType[parentAt] === 1) {
      let shadow = shadowRoots.get(parentAt);
      if (!shadow) {
        shadow = { kind: "element", tag: "#shadow-root", attrs: new Map(), children: [], parent, node: 0, inShadow: true };
        parent.children.push(shadow);
        shadowRoots.set(parentAt, shadow);
      }
      parent = shadow;
    }
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
        node: backend[i] ?? 0,
        inShadow: marked
      };
      if (values.has(i) || checked.has(i) || selected.has(i)) {
        element.live = {
          ...(values.has(i) ? { value: text(values.get(i)) } : {}),
          ...(checked.has(i) ? { checked: true } : {}),
          ...(selected.has(i) ? { selected: true } : {})
        };
      }
      parent.children.push(element);
      slots[i] = element;
    } else if (type === 3) {
      parent.children.push({ kind: "text", text: text(nodeValue[i]), parent });
    } else if (type === 9) {
      // The document: its children attach to the root.
      slots[i] = parent;
    } else if (type === 11) {
      // A shadow root, if an engine reports one: a separate tree scope.
      const shadow: MutableElement = { kind: "element", tag: "#shadow-root", attrs: new Map(), children: [], parent, node: backend[i] ?? 0, inShadow: true };
      parent.children.push(shadow);
      slots[i] = shadow;
    }
  }
  return { root, truncated: nodeType.length > MAX_SNAPSHOT_NODES, url: text(doc.documentURL), encoding: text(doc.encodingName).trim().toLowerCase() };
}
