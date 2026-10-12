import assert from "node:assert/strict";
import { test } from "node:test";
import { discoverDeclarativeTools, LIMITS, type DomElement, type DomNode } from "./declarative.js";
import { documentModel, MAX_SNAPSHOT_NODES, type CapturedSnapshot } from "./dom-snapshot.js";
import { readFileSync } from "node:fs";
import { vectorCases } from "./vectors.fixture.js";
import { fitTools } from "./fit.js";
import { encodeFrame } from "../protocol.js";

let nextNode = 1;

interface Mutable {
  kind: "element";
  tag: string;
  attrs: Map<string, string>;
  children: DomNode[];
  parent: Mutable | null;
  node: number;
}

function el(tag: string, attrs: Record<string, string> = {}, ...children: Array<Mutable | string>): Mutable {
  const element: Mutable = { kind: "element", tag, attrs: new Map(Object.entries(attrs)), children: [], parent: null, node: nextNode++ };
  for (const child of children) {
    if (typeof child === "string") {
      element.children.push({ kind: "text", text: child, parent: element });
    } else {
      child.parent = element;
      element.children.push(child);
    }
  }
  return element;
}

const doc = (...children: Array<Mutable | string>): DomElement => el("#root", {}, el("html", {}, el("body", {}, ...children)));

test("a declared form becomes a bounded tool with a schema from its controls", () => {
  const { tools, truncated } = discoverDeclarativeTools(
    doc(
      el(
        "form",
        { toolname: "search_flights", tooldescription: "Search flights.", tooltitle: "Flights", toolautosubmit: "" },
        el("label", { for: "from" }, "From airport"),
        el("input", { id: "from", name: "from", required: "", pattern: "[A-Z]{3}" }),
        el("input", { name: "seats", type: "number", min: "1", max: "9" }),
        el("select", { name: "cabin" }, el("option", { value: "eco" }, "Economy"), el("option", {}, "Business")),
        el(
          "fieldset",
          { toolparamdescription: "Trip type" },
          el("input", { type: "radio", name: "trip", value: "one" }),
          el("input", { type: "radio", name: "trip", value: "return" })
        ),
        el("input", { type: "checkbox", name: "meal", value: "veg" }),
        el("input", { type: "checkbox", name: "meal", value: "halal" }),
        el("input", { type: "date", name: "when" }),
        el("input", { type: "hidden", name: "csrf", value: "tok-123" })
      )
    )
  );
  assert.equal(truncated, false);
  assert.equal(tools.length, 1);
  const [tool] = tools;
  assert.ok(tool);
  assert.equal(tool.name, "search_flights");
  assert.equal(tool.title, "Flights");
  assert.equal(tool.autosubmit, true);
  assert.equal(tool.has_password, false);
  assert.deepEqual(tool.params, [
    { name: "from", required: true, type: "string", pattern: "[A-Z]{3}", description: "From airport" },
    { name: "seats", required: false, type: "number", minimum: 1, maximum: 9, multiple_of: 1 },
    { name: "cabin", required: false, type: "string", enum: ["eco", "Business"] },
    { name: "trip", required: false, type: "string", enum: ["one", "return"], description: "Trip type" },
    { name: "meal", required: false, type: "array", items_enum: ["veg", "halal"] },
    { name: "when", required: false, type: "string", format: "date", description: "Dates MUST be provided in 'YYYY-MM-DD' format." }
  ]);
  // The hidden field (no description) is not a parameter and its value never appears.
  assert.ok(!JSON.stringify(tools).includes("tok-123"));
});

test("values never leave, and password controls flag the tool instead of becoming parameters", () => {
  const { tools } = discoverDeclarativeTools(
    doc(
      el(
        "form",
        { toolname: "login", tooldescription: "Sign in." },
        el("input", { name: "user", value: "alice" }),
        el("input", { name: "pass", type: "password", value: "hunter2" })
      )
    )
  );
  assert.equal(tools[0]?.has_password, true);
  assert.deepEqual(tools[0]?.params.map((p) => p.name), ["user"]);
  const text = JSON.stringify(tools);
  assert.ok(!text.includes("alice") && !text.includes("hunter2"));
});

test("undeclared, misnamed and duplicate forms are not tools; the first name wins", () => {
  const { tools, truncated } = discoverDeclarativeTools(
    doc(
      el("form", { toolname: "no_description" }),
      el("form", { toolname: "bad name!", tooldescription: "x" }),
      el("form", { toolname: "dup", tooldescription: "first" }),
      el("form", { toolname: "dup", tooldescription: "second" })
    )
  );
  assert.deepEqual(tools.map((t) => [t.name, t.description]), [["dup", "first"]]);
  assert.equal(truncated, true, "an invalid name is reported as dropped");
});

test("disabled, read-only, nested and foreign controls are excluded; form= owners are included", () => {
  const { tools } = discoverDeclarativeTools(
    doc(
      el(
        "form",
        { toolname: "t", tooldescription: "d", id: "f1" },
        el("input", { name: "ok" }),
        el("input", { name: "off", disabled: "" }),
        el("fieldset", { disabled: "" }, el("input", { name: "inside_disabled" })),
        el("input", { name: "ro", readonly: "" }),
        el("input", { name: "elsewhere", form: "f2" })
      ),
      el("input", { name: "outside", form: "f1" })
    )
  );
  assert.deepEqual(tools[0]?.params.map((p) => p.name), ["ok", "outside"]);
});

test("every collection and string is bounded", () => {
  const many = Array.from({ length: LIMITS.tools + 3 }, (_, i) => el("form", { toolname: `t${i}`, tooldescription: "d" }));
  const result = discoverDeclarativeTools(doc(...many));
  assert.equal(result.tools.length, LIMITS.tools);
  assert.equal(result.truncated, true);
  const long = discoverDeclarativeTools(doc(el("form", { toolname: "long", tooldescription: "x".repeat(5000) })));
  assert.equal(long.tools[0]?.description.length, LIMITS.description);
  assert.equal(long.truncated, true);
  const options = Array.from({ length: LIMITS.choices + 1 }, (_, i) => el("option", { value: `v${i}` }));
  const select = discoverDeclarativeTools(doc(el("form", { toolname: "s", tooldescription: "d" }, el("select", { name: "pick" }, ...options))));
  assert.deepEqual(select.tools[0]?.params, [], "too many choices drops the parameter");
});

function snapshot(nodes: Array<[parent: number, type: number, name: string, value?: string | undefined, attrs?: string[]]>): CapturedSnapshot {
  const strings: string[] = [];
  const id = (s: string): number => {
    const at = strings.indexOf(s);
    if (at >= 0) return at;
    strings.push(s);
    return strings.length - 1;
  };
  return {
    strings,
    documents: [
      {
        nodes: {
          parentIndex: nodes.map((n) => n[0]),
          nodeType: nodes.map((n) => n[1]),
          nodeName: nodes.map((n) => id(n[2])),
          nodeValue: nodes.map((n) => (n[3] === undefined ? -1 : id(n[3]))),
          backendNodeId: nodes.map((_, i) => 100 + i),
          attributes: nodes.map((n) => (n[4] ?? []).map(id))
        }
      }
    ]
  };
}

test("the snapshot model keeps shadow trees and text, and drops other node kinds", () => {
  const model = documentModel(
    snapshot([
      [-1, 9, "#document"],
      [0, 1, "HTML"],
      [1, 1, "BODY"],
      [2, 1, "X-HOST"],
      [3, 11, "#document-fragment"],
      [4, 1, "FORM", undefined, ["toolname", "shadowed", "tooldescription", "In a shadow root"]],
      [5, 1, "LABEL"],
      [6, 3, "#text", "Query"],
      [6, 1, "INPUT", undefined, ["name", "q"]],
      [2, 8, "#comment", "ignored"]
    ])
  );
  assert.equal(model.truncated, false);
  const { tools } = discoverDeclarativeTools(model.root);
  assert.deepEqual(tools.map((t) => [t.name, t.node, t.params]), [
    ["shadowed", 105, [{ name: "q", required: false, type: "string", description: "Query" }]]
  ]);
});

test("the snapshot model is bounded", () => {
  const nodes: Array<[number, number, string]> = [[-1, 9, "#document"]];
  for (let i = 1; i <= MAX_SNAPSHOT_NODES + 5; i++) nodes.push([0, 1, "DIV"]);
  assert.equal(documentModel(snapshot(nodes)).truncated, true);
});


test("a DOM deeper than any call stack is walked without recursion", () => {
  let inner = el("form", { toolname: "deep", tooldescription: "At the bottom." }, el("label", {}, "Deep field ", el("input", { name: "d" })));
  for (let i = 0; i < 200_000; i++) inner = el("div", {}, inner);
  const { tools } = discoverDeclarativeTools(doc(inner));
  assert.deepEqual(tools.map((t) => [t.name, t.params]), [["deep", [{ name: "d", required: false, type: "string", description: "Deep field" }]]]);
});

const LONE = /[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/;
function strings(value: unknown, out: string[] = []): string[] {
  if (typeof value === "string") out.push(value);
  else if (Array.isArray(value)) for (const item of value) strings(item, out);
  else if (value && typeof value === "object") for (const item of Object.values(value)) strings(item, out);
  return out;
}

test("text is well formed, control- and bidi-free, and cut at whole code points", () => {
  const emoji = "\u{1F600}";
  const { tools } = discoverDeclarativeTools(
    doc(
      el(
        "form",
        { toolname: "e", tooldescription: "x\ud800y " + "a".repeat(LIMITS.description) + emoji, tooltitle: "T\u0001it\u202ele\udc00" },
        el("label", {}, "Lab\ud800el ", el("input", { name: "q" })),
        el("input", { name: "bad\u0007name" }),
        el("input", { name: "pat", pattern: "a\u0000b" }),
        el("select", { name: "lone" }, el("option", { value: "lone\ud800" })),
        el("select", { name: "bidi" }, el("option", { value: "a\u202eb" }))
      )
    )
  );
  const [tool] = tools;
  assert.ok(tool);
  for (const text of strings(tools)) assert.ok(!LONE.test(text), JSON.stringify(text));
  assert.ok(tool.description.startsWith("x\ufffdy "));
  assert.equal(Array.from(tool.description).length, LIMITS.description);
  assert.equal(tool.title, "Title\ufffd");
  assert.deepEqual(tool.params, [
    { name: "q", required: false, type: "string", description: "Lab\ufffdel" },
    { name: "pat", required: false, type: "string" }
  ]);
});

test("number schemas stay inside the host's subset", () => {
  const { tools } = discoverDeclarativeTools(
    doc(
      el(
        "form",
        { toolname: "n", tooldescription: "d" },
        el("input", { name: "pat", type: "number", pattern: "[0-9]+" }),
        el("input", { name: "inverted", type: "number", min: "10", max: "1" }),
        el("input", { name: "range_high", type: "range", min: "200" }),
        el("input", { name: "range_ok", type: "range", min: "5" })
      )
    )
  );
  assert.deepEqual(tools[0]?.params, [
    { name: "pat", required: false, type: "number", multiple_of: 1 },
    { name: "inverted", required: false, type: "number", multiple_of: 1 },
    { name: "range_high", required: false, type: "number", multiple_of: 1 },
    { name: "range_ok", required: false, type: "number", minimum: 5, maximum: 100, multiple_of: 1 }
  ]);
});

test("discovery is linear and bounded by its CPU budget", () => {
  // A deep chain with many controls, and nested declared forms around it.
  let chain = el("div", {}, ...Array.from({ length: 40_000 }, (_, i) => el("input", { name: `c${i}` })));
  for (let i = 0; i < 40_000; i++) chain = el("div", {}, chain);
  let nested = el("form", { toolname: "inner", tooldescription: "d" }, chain);
  for (let i = 0; i < 31; i++) nested = el("form", { toolname: `outer${i}`, tooldescription: "d" }, nested);
  const started = Date.now();
  const found = discoverDeclarativeTools(doc(nested));
  assert.ok(Date.now() - started < 3_000, `took ${Date.now() - started} ms`);
  assert.equal(found.tools.length, 32);
  // An exhausted budget truncates instead of running late.
  const late = discoverDeclarativeTools(doc(nested), 0);
  assert.equal(late.truncated, true);
});

test("editable regions never contribute text, and labels label one control", () => {
  const { tools } = discoverDeclarativeTools(
    doc(
      el(
        "form",
        { toolname: "t", tooldescription: "d" },
        el("label", {}, "Query ", el("div", { contenteditable: "" }, "USER TYPED SECRET"), el("span", { role: "textbox" }, "ALSO TYPED"), el("input", { name: "q" }), el("input", { name: "second" }))
      )
    )
  );
  assert.deepEqual(tools[0]?.params, [
    { name: "q", required: false, type: "string", description: "Query" },
    { name: "second", required: false, type: "string" }
  ]);
});

test("tree scopes, legends, exact attribute names and reserved names follow the DOM", () => {
  const { tools } = discoverDeclarativeTools(
    doc(
      el(
        "form",
        { toolname: "scoped", tooldescription: "d", id: "f" },
        el("input", { name: "light" }),
        // Shadow-tree controls are never associated with a light-DOM form.
        el("x-host", {}, el("#shadow-root", {}, el("input", { name: "shadowed" }), el("input", { name: "by_id", form: "f" }))),
        el("fieldset", { disabled: "" }, el("legend", {}, el("input", { name: "in_legend" })), el("input", { name: "off" }))
      ),
      el("form", { TOOLNAME: "upper", tooldescription: "d" }),
      el("form", { toolname: "d1-w1", tooldescription: "looks like a host identity" })
    )
  );
  assert.deepEqual(tools.map((t) => [t.name, t.params.map((p) => p.name)]), [["scoped", ["light", "in_legend"]]]);
});

test("password-like controls flag the tool even when disabled", () => {
  const { tools } = discoverDeclarativeTools(
    doc(
      el("form", { toolname: "a", tooldescription: "d" }, el("input", { type: "password", name: "p", disabled: "" })),
      el("form", { toolname: "b", tooldescription: "d" }, el("input", { name: "code", autocomplete: "one-time-code" }), el("input", { name: "user" }))
    )
  );
  assert.deepEqual(tools.map((t) => [t.name, t.has_password, t.params.map((p) => p.name)]), [
    ["a", true, []],
    ["b", true, ["user"]]
  ]);
});

test("the worker's output for awkward markup equals the shared host vectors", () => {
  const pinned: unknown = JSON.parse(readFileSync(new URL("../../src/webmcp/shared-vectors.json", import.meta.url), "utf8"));
  const produced: Record<string, unknown> = {};
  for (const [name, root] of Object.entries(vectorCases())) produced[name] = discoverDeclarativeTools(root);
  assert.deepEqual(produced, pinned, "regenerate shared-vectors.json only together with the host validator");
});

test("the frame fit is linear and keeps the longest prefix that fits", () => {
  // 32 tools at their bounds, each far larger than one frame on its own.
  const options = Array.from({ length: LIMITS.choices }, (_, i) => el("option", { value: `${i}`.padEnd(LIMITS.choice, "v") }));
  const big = Array.from({ length: LIMITS.tools }, (_, t) =>
    el("form", { toolname: `t${t}`, tooldescription: "d".repeat(LIMITS.description) }, ...Array.from({ length: LIMITS.params }, (_, p) => el("select", { name: `p${p}` }, ...options.map((o) => el("option", { value: o.attrs.get("value") ?? "" })))))
  );
  const { tools } = discoverDeclarativeTools(doc(...big));
  assert.equal(tools.length, LIMITS.tools);
  const started = Date.now();
  const fitted = fitTools(123456, "https://fixture.test/x", false, tools);
  assert.ok(Date.now() - started < 1_000, `fit took ${Date.now() - started} ms`);
  assert.equal(fitted.truncated, true);
  // Whatever was kept encodes within the frame bound.
  encodeFrame({ frame: "discovered", disc_id: 123456, ...fitted });
  // Small tools all fit and nothing is marked truncated.
  const small = discoverDeclarativeTools(doc(el("form", { toolname: "a", tooldescription: "d" }), el("form", { toolname: "b", tooldescription: "d" })));
  const kept = fitTools(1, "https://fixture.test/", false, small.tools);
  assert.deepEqual([kept.truncated, kept.tools.length], [false, 2]);
});
