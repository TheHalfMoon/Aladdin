// SG-000108 slice 6: extraction against the installed Edge's real
// DOMSnapshot (the synthetic snapshots in extract.test.ts encode this
// module's own assumptions; this file checks them against the engine).
// Offline: every request is aborted and pages come from setContent. On
// Windows a missing engine fails; elsewhere the test is skipped because only
// Windows results count as evidence.

import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { test } from "node:test";
import { chromium } from "playwright-core";
import { CONTENT_STYLES, extractMarkdown, type ExtractOptions } from "./index.js";
import type { LayoutSnapshot } from "./model.js";

const EDGE_CANDIDATES = ["C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe", "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe"];
const skip = process.platform === "win32" ? false : "native probes are Windows-only evidence";

async function extract(html: string, options: ExtractOptions = {}) {
  const engine = EDGE_CANDIDATES.find((path) => existsSync(path));
  assert.ok(engine, "Edge is not installed");
  const browser = await chromium.launch({ executablePath: engine, headless: true });
  try {
    const page = await browser.newPage();
    await page.route("**/*", (route) => route.abort());
    await page.setContent(html);
    const session = await page.context().newCDPSession(page);
    const snapshot = (await session.send("DOMSnapshot.captureSnapshot", { computedStyles: [...CONTENT_STYLES] })) as unknown as LayoutSnapshot;
    return extractMarkdown(snapshot, options);
  } finally {
    await browser.close();
  }
}

test("hidden text and hidden attribute text never reach the Markdown (real Edge)", { skip, timeout: 120_000 }, async () => {
  const leaks = [
    `<div style="display:none">LEAK-display</div>`,
    `<div style="visibility:hidden">LEAK-visibility</div>`,
    `<div style="opacity:0">LEAK-opacity</div>`,
    `<div style="opacity:0.001">LEAK-near-zero-opacity</div>`,
    `<p hidden>LEAK-hidden-attribute</p>`,
    `<p style="font-size:0">LEAK-font-size</p>`,
    `<p style="color:transparent">LEAK-transparent</p>`,
    `<p style="position:absolute;left:-99999px">LEAK-offscreen</p>`,
    `<p style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0)">LEAK-visually-hidden</p>`,
    `<div style="height:0;overflow:hidden">LEAK-zero-height</div>`,
    `<details><summary>More</summary>LEAK-closed-details</details>`,
    `<noscript>LEAK-noscript</noscript>`,
    `<div style="display:none"><img alt="LEAK-img-hidden-parent" src="https://x.test/a.png"></div>`,
    `<img style="visibility:hidden" alt="LEAK-img-visibility" src="https://x.test/b.png">`,
    `<img hidden alt="LEAK-img-hidden" src="https://x.test/c.png">`,
    `<a style="display:none" href="https://x.test/"><img alt="LEAK-img-in-hidden-link"></a>`,
    `<input value="LEAK-prefilled"><textarea>LEAK-textarea</textarea>`
  ];
  const html = `<!doctype html><body><p>Visible start</p>${leaks.join("\n")}
<div style="visibility:hidden"><span style="visibility:visible">Visible child</span></div>
<p style="display:contents">Visible contents</p>
<p><img alt="Visible alt" src="https://x.test/v.png"></p>
<p>Visible end</p></body>`;
  const out = await extract(html, { images: true });
  for (const marker of out.markdown.match(/LEAK-[a-z-]+/g) ?? []) assert.fail(`${marker} leaked:\n${out.markdown}`);
  for (const visible of ["Visible start", "Visible child", "Visible contents", "![Visible alt](https://x.test/v.png)", "Visible end"]) {
    assert.ok(out.markdown.includes(visible), `${visible} missing:\n${out.markdown}`);
  }
});

test("shadow trees, slots and page-injected syntax come out as structure and literal text (real Edge)", { skip, timeout: 120_000 }, async () => {
  const html = `<!doctype html><body>
<x-card><template shadowrootmode="closed"><h2>Card title</h2><p>Shadow before</p><slot></slot><p>Shadow after</p></template><p>Slotted</p></x-card>
<p>[click](javascript:alert(1)) &lt;img src=x onerror=alert(1)&gt;</p>
<p># not a heading</p></body>`;
  const out = await extract(html);
  assert.equal(
    out.markdown,
    ["## Card title", "", "Shadow before", "", "Slotted", "", "Shadow after", "", "\\[click\\](javascript:alert(1)) \\<img src=x onerror=alert(1)\\>", "", "\\# not a heading"].join("\n")
  );
});

test("off-screen lazy content is reported, not silently dropped (real Edge)", { skip, timeout: 120_000 }, async () => {
  const html = `<!doctype html><body><p>Top</p><div style="height:20000px"></div>
<section style="content-visibility:auto">Lazy section text</section></body>`;
  const out = await extract(html);
  assert.ok(out.markdown.includes("Top"));
  if (!out.markdown.includes("Lazy section text")) assert.equal(out.truncated, true, "lost lazy content was not reported");
});

test("hostile whitespace and wide tables stay within the budget (real Edge)", { skip, timeout: 120_000 }, async () => {
  const cells = "<td>c</td>".repeat(4_000);
  const rows = "<tr></tr>".repeat(4_000);
  const html = `<!doctype html><body><p>a${"&nbsp;".repeat(200_000)}b</p><table><tr>${cells}</tr>${rows}</table></body>`;
  const started = Date.now();
  const out = await extract(html);
  const elapsed = Date.now() - started;
  // Includes launching the engine; the extraction itself must stay inside
  // its 4 s budget.
  assert.ok(elapsed < 20_000, `took ${elapsed} ms`);
  assert.ok(out.markdown.length < 1_000_001);
});
