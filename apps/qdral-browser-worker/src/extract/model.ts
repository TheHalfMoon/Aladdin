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
// rendered, visible text is content. Dropped: text without a layout object
// (inside `display: none`, collapsed, never rendered), text whose computed
// visibility is hidden, transparent text, text with a zero font size or a
// zero-area or off-document box, subtrees with opacity below 0.01, and
// subtrees of rendered boxes that clip their content to at most one pixel
// (the "visually hidden" pattern). Attribute text (an image's alt, a link's
// href) is used only from elements that are themselves rendered and visible.
// The donor's viewport threshold does not apply (the whole page is content).
// Hidden text is the classic way to smuggle instructions to a model, so it
// never reaches the caller. Live form values are never read.

/// The computed styles requested from the engine, in this order.
export const CONTENT_STYLES = ["display", "visibility", "opacity", "font-size", "color", "overflow-x", "overflow-y", "content-visibility"] as const;
type StyleName = (typeof CONTENT_STYLES)[number];

/// Upper bound on snapshot nodes taken into the model.
export const MAX_CONTENT_NODES = 200_000;
/// Opacity below this is invisible.
const MIN_OPACITY = 0.01;

export interface ContentElement {
  readonly kind: "element";
  /// Lowercase tag name ("#shadow-root" for a shadow root).
  readonly tag: string;
  readonly attrs: ReadonlyMap<string, string>;
  readonly children: readonly ContentNode[];
  /// The element itself is rendered and visible: its attribute text (alt,
  /// href) may become content.
  readonly shown: boolean;
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
      bounds?: number[][];
    };
  }>;
  strings?: string[];
}

export interface ContentModel {
  root: ContentElement;
  url: string;
  /// Content was dropped by a bound or the budget, or was not rendered by
  /// the engine (`content-visibility: auto` off screen).
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
  shown: boolean;
}

interface Slot {
  element: Building;
  /// Under a `content-visibility: auto` box (its unrendered text is lost,
  /// not hidden).
  lazy: boolean;
}

function transparent(color: string | undefined): boolean {
  if (!color) return false;
  if (color === "transparent") return true;
  const alpha = /^rgba\([^,]+,[^,]+,[^,]+,\s*([0-9.]+)\s*\)$/.exec(color)?.[1];
  return alpha !== undefined && Number(alpha) === 0;
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

  // Layout: which nodes render, their computed styles and their boxes.
  const layoutAt = new Map<number, number>();
  (doc.layout?.nodeIndex ?? []).forEach((node, at) => {
    if (!layoutAt.has(node)) layoutAt.set(node, at);
  });
  const layoutStyles = doc.layout?.styles ?? [];
  const layoutBounds = doc.layout?.bounds ?? [];
  const style = (node: number, name: StyleName): string | undefined => {
    const at = layoutAt.get(node);
    if (at === undefined) return undefined;
    return text(layoutStyles[at]?.[CONTENT_STYLES.indexOf(name)]).trim().toLowerCase();
  };
  const box = (node: number): { x: number; y: number; w: number; h: number } | undefined => {
    const at = layoutAt.get(node);
    const bounds = at === undefined ? undefined : layoutBounds[at];
    if (!bounds || bounds.length < 4) return undefined;
    const [x = 0, y = 0, w = 0, h = 0] = bounds;
    return { x, y, w, h };
  };
  const visible = (node: number): boolean => {
    const visibility = style(node, "visibility");
    return visibility !== "hidden" && visibility !== "collapse";
  };

  const root: Building = { kind: "element", tag: "#root", attrs: new Map(), children: [], shown: false };
  // Each snapshot index maps to the slot its children attach to, or to null
  // when its subtree is dropped.
  const slots: Array<Slot | null | undefined> = [];
  const count = Math.min(nodeType.length, MAX_CONTENT_NODES);
  let truncated = nodeType.length > MAX_CONTENT_NODES;
  for (let i = 0; i < count; i++) {
    if ((i & 1023) === 0 && Date.now() > deadline) {
      truncated = true;
      break;
    }
    const parentAt = parentIndex[i] ?? -1;
    const parent = parentAt >= 0 ? slots[parentAt] : { element: root, lazy: false };
    if (parent === null || parent === undefined) {
      slots[i] = null;
      continue;
    }
    const type = nodeType[i];
    if (type === 1) {
      const tag = text(nodeName[i]).toLowerCase();
      const rendered = layoutAt.has(i);
      const opacity = Number(style(i, "opacity") ?? "1");
      const bounds = box(i);
      const clips = ["hidden", "clip"].includes(style(i, "overflow-x") ?? "") || ["hidden", "clip"].includes(style(i, "overflow-y") ?? "");
      const clippedAway = rendered && clips && bounds !== undefined && (bounds.w <= 1 || bounds.h <= 1);
      if (SKIPPED.has(tag) || (Number.isFinite(opacity) && opacity < MIN_OPACITY) || clippedAway) {
        slots[i] = null;
        continue;
      }
      const attrs = new Map<string, string>();
      const flat = attributes[i] ?? [];
      for (let a = 0; a + 1 < flat.length; a += 2) {
        const name = text(flat[a]);
        if (name && !attrs.has(name)) attrs.set(name, text(flat[a + 1]));
      }
      const shown = rendered && visible(i) && bounds !== undefined && bounds.w > 0 && bounds.h > 0;
      const element: Building = { kind: "element", tag, attrs, children: [], shown };
      parent.element.children.push(element);
      slots[i] = { element, lazy: parent.lazy || style(i, "content-visibility") === "auto" };
    } else if (type === 3) {
      const value = text(nodeValue[i]);
      if (!layoutAt.has(i)) {
        // Text the engine skipped rendering (off-screen lazy content) is
        // lost, not hidden: say so.
        if (parent.lazy && value.trim()) truncated = true;
        continue;
      }
      const bounds = box(i);
      const fontSize = Number.parseFloat(style(i, "font-size") ?? "1");
      if (
        !visible(i) ||
        transparent(style(i, "color")) ||
        (Number.isFinite(fontSize) && fontSize <= 0) ||
        (bounds !== undefined && (bounds.w <= 0 || bounds.h <= 0 || bounds.x + bounds.w <= 0 || bounds.y + bounds.h <= 0))
      ) {
        continue;
      }
      parent.element.children.push({ kind: "text", text: value });
    } else if (type === 9) {
      slots[i] = parent;
    } else if (type === 11) {
      const shadow: Building = { kind: "element", tag: "#shadow-root", attrs: new Map(), children: [], shown: false };
      parent.element.children.push(shadow);
      slots[i] = { element: shadow, lazy: parent.lazy };
    } else {
      slots[i] = null;
    }
  }
  return { root, url: text(doc.documentURL), truncated };
}
