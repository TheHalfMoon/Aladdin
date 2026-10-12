// SG-000108 slice 6: the rendered content of the main document, from a CDP
// `DOMSnapshot.captureSnapshot` taken with layout styles. The engine
// serialises its own DOM and layout; no page script runs and nothing in the
// page changes.
//
// The visibility rule is adapted from browser-use `browser_use/dom/
// service.py` (`is_element_visible_according_to_all_parents`) and
// `browser_use/dom/enhanced_snapshot.py` (layout and style parsing) at
// browser-use/browser-use c75e8476e26d18b7617643bc2ae082fae8eae431,
// MIT License, Copyright (c) 2024 Gregor Zunic; the full notice is in
// docs/aladdin/notices/browser-use-MIT.txt. Modifications for Aladdin: only
// rendered text is content. A text node without a layout object (inside
// `display: none`, collapsed, or never rendered) is dropped, as is text
// whose computed visibility is hidden, and every subtree with opacity 0;
// the donor's viewport threshold does not apply (the whole page is
// content). Hidden text is the classic way to smuggle instructions to a
// model, so it never reaches the caller. Live form values are never read.

/// The computed styles requested from the engine, in this order.
export const CONTENT_STYLES = ["display", "visibility", "opacity"] as const;

/// Upper bound on snapshot nodes taken into the model.
export const MAX_CONTENT_NODES = 200_000;

export interface ContentElement {
  readonly kind: "element";
  /// Lowercase tag name ("#shadow-root" for a shadow root).
  readonly tag: string;
  readonly attrs: ReadonlyMap<string, string>;
  readonly children: readonly ContentNode[];
}

export interface ContentText {
  readonly kind: "text";
  readonly text: string;
}

export type ContentNode = ContentElement | ContentText;

/// The parts of a CDP DOMSnapshot this module reads.
export interface LayoutSnapshot {
  documents?: Array<{
    documentURL?: number;
    nodes?: {
      parentIndex?: number[];
      nodeType?: number[];
      nodeName?: number[];
      nodeValue?: number[];
      attributes?: number[][];
    };
    layout?: {
      nodeIndex?: number[];
      styles?: number[][];
    };
  }>;
  strings?: string[];
}

export interface ContentModel {
  root: ContentElement;
  url: string;
  /// Nodes past the bound or the budget were dropped.
  truncated: boolean;
}

/// Elements whose content is never page text.
const SKIPPED = new Set([
  "head",
  "script",
  "style",
  "template",
  "noscript",
  "svg",
  "math",
  "canvas",
  "iframe",
  "frame",
  "object",
  "embed",
  "video",
  "audio",
  "select",
  "datalist",
  "textarea",
  "option"
]);

interface Building {
  kind: "element";
  tag: string;
  attrs: Map<string, string>;
  children: ContentNode[];
}

export function contentModel(snapshot: LayoutSnapshot, deadline = Number.POSITIVE_INFINITY): ContentModel {
  const doc = snapshot.documents?.[0];
  if (!doc) throw new Error("no document");
  const strings = snapshot.strings ?? [];
  const text = (index: number | undefined): string => (index !== undefined && index >= 0 ? (strings[index] ?? "") : "");
  const nodes = doc.nodes ?? {};
  const parentIndex = nodes.parentIndex ?? [];
  const nodeType = nodes.nodeType ?? [];
  const nodeName = nodes.nodeName ?? [];
  const nodeValue = nodes.nodeValue ?? [];
  const attributes = nodes.attributes ?? [];

  // Layout: which nodes render, and their computed styles.
  const styleOf = new Map<number, number[]>();
  const layoutIndex = doc.layout?.nodeIndex ?? [];
  const layoutStyles = doc.layout?.styles ?? [];
  layoutIndex.forEach((node, at) => {
    if (!styleOf.has(node)) styleOf.set(node, layoutStyles[at] ?? []);
  });
  const style = (node: number, name: (typeof CONTENT_STYLES)[number]): string | undefined => {
    const styles = styleOf.get(node);
    const at = CONTENT_STYLES.indexOf(name);
    return styles ? text(styles[at]).toLowerCase() : undefined;
  };

  const root: Building = { kind: "element", tag: "#root", attrs: new Map(), children: [] };
  // Each snapshot index maps to the element its children attach to, or to
  // null when its subtree is dropped.
  const slots: Array<Building | null | undefined> = [];
  const count = Math.min(nodeType.length, MAX_CONTENT_NODES);
  let truncated = nodeType.length > MAX_CONTENT_NODES;
  for (let i = 0; i < count; i++) {
    if ((i & 1023) === 0 && Date.now() > deadline) {
      truncated = true;
      break;
    }
    const parentAt = parentIndex[i] ?? -1;
    const parent = parentAt >= 0 ? slots[parentAt] : root;
    if (parent === null || parent === undefined) {
      slots[i] = null;
      continue;
    }
    const type = nodeType[i];
    if (type === 1) {
      const tag = text(nodeName[i]).toLowerCase();
      const opacity = Number(style(i, "opacity") ?? "1");
      if (SKIPPED.has(tag) || (Number.isFinite(opacity) && opacity <= 0)) {
        slots[i] = null;
        continue;
      }
      const attrs = new Map<string, string>();
      const flat = attributes[i] ?? [];
      for (let a = 0; a + 1 < flat.length; a += 2) {
        const name = text(flat[a]);
        if (name && !attrs.has(name)) attrs.set(name, text(flat[a + 1]));
      }
      const element: Building = { kind: "element", tag, attrs, children: [] };
      parent.children.push(element);
      slots[i] = element;
    } else if (type === 3) {
      // Only rendered text is content.
      if (!styleOf.has(i)) continue;
      const visibility = style(i, "visibility");
      if (visibility === "hidden" || visibility === "collapse") continue;
      parent.children.push({ kind: "text", text: text(nodeValue[i]) });
    } else if (type === 9) {
      slots[i] = parent;
    } else if (type === 11) {
      const shadow: Building = { kind: "element", tag: "#shadow-root", attrs: new Map(), children: [] };
      parent.children.push(shadow);
      slots[i] = shadow;
    } else {
      slots[i] = null;
    }
  }
  return { root, url: text(doc.documentURL), truncated };
}
