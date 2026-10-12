// Test support: awkward declarative markup whose discovered tools are pinned
// in `shared-vectors.json`. The worker's tests require its output to equal
// those vectors, and the host's tests require its validator to accept every
// one, so a disagreement between the two sides fails in CI rather than on a
// real page.

import type { DomElement, DomNode } from "./declarative.js";

interface Mutable {
  kind: "element";
  tag: string;
  attrs: Map<string, string>;
  children: DomNode[];
  parent: Mutable | null;
  node: number;
}

export function vectorCases(): Record<string, DomElement> {
  let next = 1;
  const el = (tag: string, attrs: Record<string, string> = {}, ...children: Array<Mutable | string>): Mutable => {
    const element: Mutable = { kind: "element", tag, attrs: new Map(Object.entries(attrs)), children: [], parent: null, node: next++ };
    for (const child of children) {
      if (typeof child === "string") {
        element.children.push({ kind: "text", text: child, parent: element });
      } else {
        child.parent = element;
        element.children.push(child);
      }
    }
    return element;
  };
  const form = (name: string, ...children: Array<Mutable | string>): DomElement =>
    el("#root", {}, el("form", { toolname: name, tooldescription: `Tool ${name}.`, tooltitle: name.toUpperCase() }, ...children));
  return {
    numbers: form(
      "numbers",
      el("input", { name: "plain", type: "number" }),
      el("input", { name: "with_pattern", type: "number", pattern: "[0-9]+" }),
      el("input", { name: "inverted", type: "number", min: "10", max: "1" }),
      el("input", { name: "stepped", type: "number", min: "0.5", step: "0.5" }),
      el("input", { name: "any_step", type: "number", step: "any", min: "-3", max: "3" }),
      el("input", { name: "range_default", type: "range" }),
      el("input", { name: "range_high", type: "range", min: "200" }),
      el("input", { name: "infinite", type: "number", min: "Infinity", max: "1e999" })
    ),
    texts: form(
      "texts",
      el("input", { name: "email", type: "email", required: "" }),
      el("input", { name: "bad_pattern", pattern: "(" }),
      el("input", { name: "long_pattern", pattern: "a".repeat(300) }),
      el("textarea", { name: "notes", toolparamdescription: "Free text ‮ reversed" }),
      el("input", { name: "secret", type: "hidden", toolparamdescription: "A described hidden field" }),
      el("input", { name: "token", type: "hidden", value: "never-sent" })
    ),
    temporal: form(
      "temporal",
      el("input", { name: "day", type: "date" }),
      el("input", { name: "month", type: "month" }),
      el("input", { name: "week", type: "week" }),
      el("input", { name: "clock", type: "time", step: "1" }),
      el("input", { name: "stamp", type: "datetime-local", step: "0.5" }),
      el("input", { name: "colour", type: "color" })
    ),
    choices: form(
      "choices",
      el("select", { name: "single" }, el("option", { value: "a" }, "A"), el("option", {}, " Text option ")),
      el("select", { name: "many", multiple: "" }, el("option", { value: "x" }), el("option", { value: "y" })),
      el("input", { type: "radio", name: "pick", value: "one" }),
      el("input", { type: "radio", name: "pick" }),
      el("input", { type: "checkbox", name: "flags", value: "f1" }),
      el("input", { type: "checkbox", name: "flags", value: "f2" }),
      el("input", { type: "checkbox", name: "agree" }),
      el("select", { name: "too_many" }, ...Array.from({ length: 65 }, (_, i) => el("option", { value: `v${i}` })))
    ),
    text_limits: el(
      "#root",
      {},
      el(
        "form",
        { toolname: "text_limits", tooldescription: "x\ud800 " + "d".repeat(1100) + "\u{1F600}", tooltitle: "t".repeat(200) },
        el("label", {}, "L".repeat(600), el("input", { name: "q" }))
      )
    )
  };
}
