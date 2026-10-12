import assert from "node:assert/strict";
import { test } from "node:test";
import { discoverDeclarativeTools, LIMITS, type DomElement, type DomNode } from "./declarative.js";
import { documentModel, MAX_SNAPSHOT_NODES, type CapturedSnapshot } from "./dom-snapshot.js";

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

test("bounds cut whole code points and controls never cross", () => {
  const emoji = "\u{1F600}";
  const { tools } = discoverDeclarativeTools(
    doc(
      el(
        "form",
        { toolname: "e", tooldescription: "a".repeat(LIMITS.description - 1) + emoji + "tail", tooltitle: "T\u0001itle" },
        el("input", { name: "bad\u0007name" }),
        el("input", { name: "ok", pattern: "a\u0000b" }),
        el("select", { name: "pick" }, el("option", { value: "lone\ud800" }))
      )
    )
  );
  const [tool] = tools;
  assert.ok(tool);
  assert.ok(tool.description.endsWith(emoji), "the cut kept the whole emoji");
  assert.equal(Array.from(tool.description).length, LIMITS.description);
  assert.equal(tool.title, "Title");
  assert.deepEqual(tool.params, [{ name: "ok", required: false, type: "string" }]);
  // Nothing the worker sends carries a lone surrogate (strict JSON on the host).
  const lone = /[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/;
  assert.ok(!lone.test(JSON.stringify(tools)));
});

test("a DOM deeper than any call stack is walked without recursion", () => {
  let inner = el("form", { toolname: "deep", tooldescription: "At the bottom." }, el("label", {}, "Deep field ", el("input", { name: "d" })));
  for (let i = 0; i < 200_000; i++) inner = el("div", {}, inner);
  const { tools } = discoverDeclarativeTools(doc(inner));
  assert.deepEqual(tools.map((t) => [t.name, t.params]), [["deep", [{ name: "d", required: false, type: "string", description: "Deep field" }]]]);
});
