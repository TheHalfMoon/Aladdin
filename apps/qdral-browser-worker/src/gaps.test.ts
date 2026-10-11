// Runtime mirror of the compile-time slice-3 gate in gaps.ts: while known
// confinement gaps remain, the host frame parser accepts exactly the launch
// vocabulary, and every known gap is still exercised by its todo probe.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { KNOWN_GAPS, LAUNCH_VOCABULARY, SLICE3_GATE } from "./gaps.js";
import { parseHostFrame, ProtocolViolation } from "./protocol.js";

const src = join(dirname(fileURLToPath(import.meta.url)), "..", "src");

test("while known gaps remain, the parser accepts only the launch vocabulary", () => {
  assert.equal(SLICE3_GATE, true);
  if (Object.keys(KNOWN_GAPS).length === 0) return;
  for (const frame of ["navigate", "observe", "act", "download", "webmcp_list", "Navigate", "hello "]) {
    assert.throws(() => parseHostFrame({ frame }), ProtocolViolation, frame);
  }
  // Each vocabulary frame is recognised: a bare frame either parses or
  // fails its own field checks, never as an unknown frame.
  for (const frame of LAUNCH_VOCABULARY) {
    try {
      parseHostFrame({ frame });
    } catch (error) {
      assert.notEqual((error as Error).message, "unknown frame", frame);
    }
  }
});

test("every known gap is still exercised by its todo probe", () => {
  const probes = readFileSync(join(src, "native.test.ts"), "utf8");
  for (const key of Object.keys(KNOWN_GAPS)) {
    assert.match(probes, new RegExp(`KNOWN_GAPS\\.${key}\\b`), `no todo probe references KNOWN_GAPS.${key}`);
  }
});
