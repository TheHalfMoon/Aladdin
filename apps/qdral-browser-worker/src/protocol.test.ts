import assert from "node:assert/strict";
import { test } from "node:test";
import {
  encodeFrame,
  FrameDecoder,
  launchRefusal,
  MAX_FRAME_BYTES,
  parseHostFrame,
  ProtocolViolation,
} from "./protocol.js";
import { validLaunch } from "./fixtures.js";

function frameBytes(json: string): Buffer {
  const body = Buffer.from(json, "utf8");
  const prefix = Buffer.alloc(4);
  prefix.writeUInt32LE(body.length, 0);
  return Buffer.concat([prefix, body]);
}

test("frames round-trip and decode incrementally across chunk boundaries", () => {
  const bytes = Buffer.concat([encodeFrame({ frame: "ping", nonce: "n-1" }), encodeFrame({ frame: "shutdown" })]);
  const decoder = new FrameDecoder();
  const seen: unknown[] = [];
  for (const byte of bytes) seen.push(...decoder.push(Buffer.from([byte])));
  assert.deepEqual(seen, [{ frame: "ping", nonce: "n-1" }, { frame: "shutdown" }]);
  assert.equal(decoder.pendingBytes, 0);
});

test("length bounds and non-JSON bodies are violations before buffering", () => {
  const oversized = Buffer.alloc(4);
  oversized.writeUInt32LE(MAX_FRAME_BYTES + 1, 0);
  assert.throws(() => new FrameDecoder().push(oversized), ProtocolViolation);
  assert.throws(() => new FrameDecoder().push(Buffer.alloc(4)), ProtocolViolation);
  assert.throws(() => new FrameDecoder().push(frameBytes("{not json")), ProtocolViolation);
  assert.throws(() => encodeFrame({ frame: "ping", nonce: "x".repeat(MAX_FRAME_BYTES) }), ProtocolViolation);
});

test("the host vocabulary is closed and exact", () => {
  assert.deepEqual(parseHostFrame({ frame: "hello", generation: 1 }), { frame: "hello", generation: 1 });
  assert.deepEqual(parseHostFrame(validLaunch()), validLaunch());
  for (const raw of [
    null,
    [],
    "hello",
    { generation: 1 },
    { frame: "navigate", url: "https://example.com" },
    { frame: "evaluate", script: "1+1" },
    { frame: "cdp", method: "Runtime.evaluate" },
    { frame: "hello", generation: 1, extra: true },
    { frame: "hello", generation: "1" },
    { frame: "hello" },
    { frame: "shutdown", now: true },
    { frame: "ping", nonce: "" },
    { frame: "ping", nonce: "a b" },
    { frame: "ping", nonce: "x".repeat(65) },
    { ...validLaunch(), extra: 1 },
    { ...validLaunch(), argv: [] },
    { ...validLaunch(), argv: "--headless" },
    { ...validLaunch(), argv: [...validLaunch().argv, 7] },
    { ...validLaunch(), argv: Array(65).fill("--headless") },
    { ...validLaunch(), engine: "C:\\x\\msedge.exe\u0000" },
    { ...validLaunch(), env: { PATH: 1 } },
    { ...validLaunch(), env: [] },
    { ...validLaunch(), env: Object.fromEntries(Array.from({ length: 17 }, (_, i) => [`K${i}`, "v"])) }
  ]) {
    assert.throws(() => parseHostFrame(raw), ProtocolViolation, JSON.stringify(raw)?.slice(0, 80));
  }
  // A prototype-polluting key is just an unexpected field.
  assert.throws(() => parseHostFrame(JSON.parse('{"frame":"shutdown","__proto__":{"x":1}}')), ProtocolViolation);
});

test("a launch frame must match the SG-000108 confinement contract", () => {
  assert.equal(launchRefusal(validLaunch()), null);
  const mutate = (change: (launch: ReturnType<typeof validLaunch>) => void) => {
    const launch = validLaunch();
    change(launch);
    return launchRefusal(launch);
  };
  const replaceArg = (prefix: string, value: string) => (launch: ReturnType<typeof validLaunch>) => {
    const index = launch.argv.findIndex((arg) => arg.startsWith(prefix));
    launch.argv[index] = value;
  };
  const refusals = [
    mutate((l) => (l.engine = "msedge.exe")),
    mutate((l) => (l.engine = "C:\\tools\\node.exe")),
    mutate((l) => (l.engine = "C:\\x\\..\\msedge.exe")),
    mutate((l) => (l.engine = "\\\\server\\share\\msedge.exe")),
    mutate((l) => (l.profile_dir = "relative\\profile")),
    mutate((l) => (l.env = { ...l.env, NODE_OPTIONS: "--require x" })),
    mutate((l) => (l.env = { ...l.env, USERPROFILE: "C:\\Users\\me" })),
    mutate((l) => l.argv.splice(0, 1)),
    mutate((l) => l.argv.splice(8, 1)),
    mutate((l) => l.argv.splice(9, 0, "--remote-debugging-port=9222")),
    mutate((l) => l.argv.splice(9, 0, "--enable-automation")),
    mutate(replaceArg("--user-data-dir=", "--user-data-dir=C:\\Users\\me\\AppData\\Local\\Microsoft\\Edge\\User Data")),
    mutate(replaceArg("--host-resolver-rules=", "--host-resolver-rules=MAP example.com 93.184.215.14:443")),
    mutate(replaceArg("--proxy-server=", "--proxy-server=http://127.0.0.1:9")),
    mutate(replaceArg("--proxy-bypass-list=", "--proxy-bypass-list=*;<-loopback>")),
    mutate(replaceArg("--proxy-bypass-list=", "--proxy-bypass-list=example.com")),
    mutate(replaceArg("--webrtc-ip-handling-policy=", "--force-webrtc-ip-handling-policy=disable_non_proxied_udp")),
    mutate(replaceArg("--webrtc-ip-handling-policy=", "--webrtc-ip-handling-policy=default")),
    mutate(replaceArg("--disable-quic", "--enable-quic")),
    mutate(replaceArg("about:blank", "https://example.com/"))
  ];
  for (const [index, reason] of refusals.entries()) {
    assert.equal(typeof reason, "string", `case ${index} was accepted`);
  }
});
