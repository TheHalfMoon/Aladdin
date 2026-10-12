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
// visibility is hidden, transparent text (colour or text fill, any colour
// syntax), text with a zero font size or a zero-area box or one past the
// document's top or left edge, subtrees with opacity (or filter opacity) below 0.01, and
// text inside a box (other than the root or body) that clips its content to
// at most four pixels, unless it escapes the clip by positioning
// (the "visually hidden" pattern). Attribute text (an image's alt, a link's
// href) is used only from elements that are themselves rendered and visible.
// The donor's viewport threshold does not apply (the whole page is content).
// Hidden text is the classic way to smuggle instructions to a model, so it
// never reaches the caller. Live form values are never read.

/// The computed styles requested from the engine, in this order.
export const CONTENT_STYLES = [
  "display",
  "visibility",
  "opacity",
  "font-size",
  "color",
  "-webkit-text-fill-color",
  "filter",
  "overflow-x",
  "overflow-y",
  "content-visibility",
  "position"
] as const;
type StyleName = (typeof CONTENT_STYLES)[number];

/// Upper bound on snapshot nodes taken into the model.
export const MAX_CONTENT_NODES = 200_000;
/// Opacity below this is invisible.
const MIN_OPACITY = 0.01;
/// A box that clips its content to this many pixels or fewer (either side)
/// shows no legible text.
const MAX_CLIPPED_PX = 4;

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
  /// Under a box that clips its content to a few pixels: its text is
  /// hidden, unless it escapes the clip (below).
  clipped: boolean;
  /// A positioned element lies between that clip box and here (inclusive of
  /// the clip box): absolutely positioned descendants stay clipped.
  positionedSinceClip: boolean;
}

/// The alpha of a computed colour in any syntax the engine reports: `/ a`
/// in modern syntax (`color()`, `oklch()`, `lab()`, `rgb(r g b / a)`), the
/// fourth component of `rgba()`, or `transparent`.
function alphaOf(color: string | undefined): number {
  if (!color) return 1;
  if (color === "transparent") return 0;
  const raw = color.includes("/")
    ? color.slice(color.lastIndexOf("/") + 1).replace(")", "").trim()
    : /^rgba\([^,]+,[^,]+,[^,]+,\s*([^)]+)\)$/.exec(color)?.[1]?.trim();
  if (raw === undefined) return 1;
  const value = raw.endsWith("%") ? Number.parseFloat(raw) / 100 : Number.parseFloat(raw);
  return Number.isFinite(value) ? value : 1;
}

/// The smallest `opacity(...)` in a computed filter, as a fraction.
function filterOpacity(filter: string | undefined): number {
  let least = 1;
  for (const match of (filter ?? "").matchAll(/opacity\(\s*([0-9.]+)(%?)\s*\)/g)) {
    const value = Number.parseFloat(match[1] ?? "1") / (match[2] ? 100 : 1);
    if (Number.isFinite(value)) least = Math.min(least, value);
  }
  return least;
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
    const parent: Slot | null | undefined = parentAt >= 0 ? slots[parentAt] : { element: root, lazy: false, clipped: false, positionedSinceClip: false };
    if (parent === null || parent === undefined) {
      slots[i] = null;
      continue;
    }
    const type = nodeType[i];
    if (type === 1) {
      const tag = text(nodeName[i]).toLowerCase();
      const rendered = layoutAt.has(i);
      const opacity = Math.min(Number(style(i, "opacity") ?? "1"), filterOpacity(style(i, "filter")));
      const bounds = box(i);
      const clips = ["hidden", "clip"].includes(style(i, "overflow-x") ?? "") || ["hidden", "clip"].includes(style(i, "overflow-y") ?? "");
      // The root and body never clip the page: their overflow applies to
      // the viewport.
      const clipsTiny =
        tag !== "html" && tag !== "body" && rendered && clips && bounds !== undefined && (bounds.w <= MAX_CLIPPED_PX || bounds.h <= MAX_CLIPPED_PX);
      if (SKIPPED.has(tag) || (Number.isFinite(opacity) && opacity < MIN_OPACITY)) {
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
      // An overflow clip does not apply to a fixed-position descendant, nor to
      // an absolutely positioned one whose containing block lies outside the
      // clip box (no positioned element in between).
      const position = style(i, "position") ?? "static";
      let clipped = parent.clipped;
      let positionedSinceClip = parent.positionedSinceClip;
      if (clipped && (position === "fixed" || (position === "absolute" && !positionedSinceClip))) clipped = false;
      if (clipped && position !== "static") positionedSinceClip = true;
      if (clipsTiny) {
        clipped = true;
        positionedSinceClip = position !== "static";
      }
      // Attribute text of a clipped element is hidden like its text.
      if (clipped) element.shown = false;
      slots[i] = { element, lazy: parent.lazy || style(i, "content-visibility") === "auto", clipped, positionedSinceClip };
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
      const fill = style(i, "-webkit-text-fill-color");
      if (
        !visible(i) ||
        parent.clipped ||
        alphaOf(style(i, "color")) < MIN_OPACITY ||
        (fill !== undefined && fill !== "" && alphaOf(fill) < MIN_OPACITY) ||
        (Number.isFinite(fontSize) && fontSize <= 0) ||
        (bounds !== undefined &&
          (bounds.w <= 0 ||
            bounds.h <= 0 ||
            bounds.x + bounds.w <= 0 ||
            bounds.y + bounds.h <= 0))
      ) {
        continue;
      }
      parent.element.children.push({ kind: "text", text: value });
    } else if (type === 9) {
      slots[i] = parent;
    } else if (type === 11) {
      const shadow: Building = { kind: "element", tag: "#shadow-root", attrs: new Map(), children: [], shown: false };
      parent.element.children.push(shadow);
      slots[i] = { element: shadow, lazy: parent.lazy, clipped: parent.clipped, positionedSinceClip: parent.positionedSinceClip };
    } else {
      slots[i] = null;
    }
  }
  return { root, url: text(doc.documentURL), truncated };
}
