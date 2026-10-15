import assert from "node:assert/strict";
import { test } from "node:test";
import { boundSnapshot, MAX_SNAPSHOT_JSON_BYTES, MAX_SNAPSHOT_NODES } from "./snapshot.js";

// The shape Playwright 1.63's ariaSnapshotJSON({ mode: "ai" }) produced for a
// page with a report list, a login form, shadow DOM and an iframe (measured).
const MEASURED = [
  {
    role: "generic",
    active: true,
    ref: "e1",
    children: [
      { role: "heading", name: "Reports", level: 1, ref: "e2" },
      {
        role: "list",
        ref: "e3",
        children: [
          { role: "listitem", ref: "e4", children: [{ role: "link", name: "Quarterly report 2026-09-30", ref: "e5", cursor: "pointer", url: "/r/2026-09-30.pdf" }] }
        ]
      },
      {
        role: "generic",
        ref: "e11",
        children: ["Password", { role: "textbox", name: "Password", ref: "e12", text: "hunter2" }]
      },
      { role: "button", name: "Shadow button", ref: "e15" },
      { role: "iframe", ref: "e16", children: [{ role: "button", name: "In frame", ref: "f1e2" }] }
    ]
  }
];

test("field values never leave the worker, links keep their URL", () => {
  const snapshot = boundSnapshot(MEASURED);
  const text = JSON.stringify(snapshot);
  assert.equal(text.includes("hunter2"), false, "a field value was forwarded");
  assert.equal(snapshot.truncated, false);
  const root = snapshot.nodes[0];
  assert.equal(root?.ref, "e1");
  const link = root?.children[1]?.children[0]?.children[0];
  assert.deepEqual(link, { ref: "e5", role: "link", name: "Quarterly report 2026-09-30", url: "/r/2026-09-30.pdf", children: [] });
  const password = root?.children[2]?.children[0];
  assert.deepEqual(password, { ref: "e12", role: "textbox", name: "Password", children: [] });
  // Iframe content keeps its frame-scoped ref.
  assert.equal(root?.children[4]?.children[0]?.ref, "f1e2");
});

test("malformed nodes, refs and roles are dropped", () => {
  const snapshot = boundSnapshot([
    {
      role: "generic",
      ref: "e1",
      children: [
        { role: "button", ref: "../e2", name: "x" },
        { role: "Button<script>", ref: "e3", name: "x" },
        { role: "button", name: "no ref" },
        { role: "button", ref: "e4", name: "ok\u0000name" },
        "text",
        42,
        null
      ]
    }
  ]);
  assert.deepEqual(snapshot.nodes[0]?.children, [{ ref: "e4", role: "button", name: "ok name", children: [] }]);
});

test("node, depth and byte ceilings truncate and say so", () => {
  const wide = { role: "list", ref: "e1", children: Array.from({ length: MAX_SNAPSHOT_NODES + 50 }, (_, i) => ({ role: "listitem", ref: `e${i + 2}`, name: "x" })) };
  const many = boundSnapshot([wide]);
  assert.equal(many.truncated, true);
  let count = 0;
  const walk = (nodes: { children: unknown[] }[]): void => {
    for (const node of nodes) {
      count++;
      walk(node.children as { children: unknown[] }[]);
    }
  };
  walk(many.nodes);
  assert.ok(count <= MAX_SNAPSHOT_NODES, `${count} nodes`);

  let deep: Record<string, unknown> = { role: "generic", ref: "e999", name: "leaf" };
  for (let i = 0; i < 40; i++) deep = { role: "generic", ref: `e${i + 1}`, children: [deep] };
  assert.equal(boundSnapshot([deep]).truncated, true);

  const big = { role: "list", ref: "e1", children: Array.from({ length: 300 }, (_, i) => ({ role: "link", ref: `e${i + 2}`, name: "n".repeat(900), url: "u".repeat(1500) })) };
  const bounded = boundSnapshot([big]);
  assert.equal(bounded.truncated, true);
  assert.ok(Buffer.byteLength(JSON.stringify(bounded), "utf8") <= MAX_SNAPSHOT_JSON_BYTES + 1024);
});
