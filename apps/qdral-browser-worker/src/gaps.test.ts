// Code-level gate for the known confinement gaps tracked as `todo` probes in
// native.test.ts (#300 WebTransport to admitted names, #301 the bypassable
// service-worker block). node:test `todo` results never fail CI, so this test
// enforces the decision-record rule instead: while any known-gap marker
// remains, the worker protocol may not grow beyond the launch vocabulary,
// i.e. slice 3 cannot add live-navigation frames until both gaps are closed
// (or accepted as founder-reviewed residual risks and the markers removed).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const src = join(dirname(fileURLToPath(import.meta.url)), "..", "src");
const LAUNCH_VOCABULARY = ["hello", "launch", "ping", "shutdown"];

function hostFrames(): string[] {
  const protocol = readFileSync(join(src, "protocol.ts"), "utf8");
  const start = protocol.indexOf("export type HostFrame");
  assert.ok(start >= 0, "HostFrame declaration not found");
  const declaration = protocol.slice(start, protocol.indexOf(";\n", start) + 1);
  const frames = [...declaration.matchAll(/frame: "([a-z_]+)"/g)].map((match) => match[1] as string);
  assert.ok(frames.length > 0, "no host frames parsed");
  return frames;
}

test("known confinement gaps block new host frames until they are closed", () => {
  const probes = readFileSync(join(src, "native.test.ts"), "utf8");
  const gaps = [...probes.matchAll(/"known gap (#\d+):/g)].map((match) => match[1] as string);
  if (gaps.length === 0) {
    return; // every gap is closed; the protocol may grow under its own review
  }
  assert.deepEqual(gaps.sort(), ["#300", "#301"], "unexpected known-gap markers");
  const extra = hostFrames().filter((frame) => !LAUNCH_VOCABULARY.includes(frame));
  assert.deepEqual(
    extra,
    [],
    `host frames ${extra.join(", ")} need the known gaps ${gaps.join(", ")} closed first (SG-000108 slice 3 gate)`
  );
});

test("the gate parses the real protocol vocabulary", () => {
  assert.deepEqual(hostFrames().sort(), [...LAUNCH_VOCABULARY].sort());
});
