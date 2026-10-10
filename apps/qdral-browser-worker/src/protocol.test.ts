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
  const parsed = parseHostFrame(validLaunch());
  assert.deepEqual(JSON.parse(JSON.stringify(parsed)), validLaunch());
  assert.equal(parsed.frame === "launch" && Object.getPrototypeOf(parsed.env), null);
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

test("resolver rules and bypass list must equal the host grammar exactly", () => {
  const withArgs = (resolver: string, bypass: string) => {
    const launch = validLaunch();
    launch.argv[9] = `--host-resolver-rules=${resolver}`;
    launch.argv[11] = `--proxy-bypass-list=${bypass}`;
    return launchRefusal(launch);
  };
  // Accepted: several destinations, IPv4 and bracketed IPv6 pins, names in order.
  assert.equal(
    withArgs(
      "MAP example.com 93.184.215.14:443, MAP docs.example.org [2606:2800:21f:cb07:6820:80da:af6b:8b2c]:443, MAP * ~NOTFOUND",
      "example.com;docs.example.org;<-loopback>"
    ),
    null
  );
  const refused: Array<[string, string]> = [
    // Reviewer-demonstrated bypasses of the previous shape-only check.
    ["MAP example.com 93.184.215.14:443, MAP * ~NOTFOUND", "10.0.0.0/8;example.com;<-loopback>"],
    ["MAP *.com 10.0.0.1:80, MAP * ~NOTFOUND", "*.com;<-loopback>"],
    ["MAP example.com 93.184.215.14:443, EXCLUDE evil.example, MAP * ~NOTFOUND", "example.com;<-loopback>"],
    // Wrong port, unbracketed IPv6, bad address, bad or duplicate names.
    ["MAP example.com 93.184.215.14:8443, MAP * ~NOTFOUND", "example.com;<-loopback>"],
    ["MAP example.com 2606:2800::1:443, MAP * ~NOTFOUND", "example.com;<-loopback>"],
    ["MAP example.com 999.1.1.1:443, MAP * ~NOTFOUND", "example.com;<-loopback>"],
    ["MAP example.com [::ffff:zz]:443, MAP * ~NOTFOUND", "example.com;<-loopback>"],
    ["MAP example.com [fe80::1%eth0]:443, MAP * ~NOTFOUND", "example.com;<-loopback>"],
    ["MAP Example.com 93.184.215.14:443, MAP * ~NOTFOUND", "Example.com;<-loopback>"],
    ["MAP a.123 93.184.215.14:443, MAP * ~NOTFOUND", "a.123;<-loopback>"],
    ["MAP example.com 93.184.215.14:443, MAP example.com 93.184.215.15:443, MAP * ~NOTFOUND", "example.com;example.com;<-loopback>"],
    // Missing or extra catch-all, empty map.
    ["MAP example.com 93.184.215.14:443", "example.com;<-loopback>"],
    ["MAP * ~NOTFOUND", "<-loopback>"],
    ["MAP example.com 93.184.215.14:443, MAP * ~NOTFOUND, MAP evil.example 10.0.0.1:443", "example.com;<-loopback>"],
    // Bypass list not exactly the mapped names in order.
    ["MAP a.example 93.184.215.14:443, MAP b.example 93.184.215.14:443, MAP * ~NOTFOUND", "b.example;a.example;<-loopback>"],
    ["MAP example.com 93.184.215.14:443, MAP * ~NOTFOUND", "example.com;evil.example;<-loopback>"],
    ["MAP example.com 93.184.215.14:443, MAP * ~NOTFOUND", "example.com"],
    ["MAP example.com 93.184.215.14:443, MAP * ~NOTFOUND", "example.com;<-loopback>;<local>"]
  ];
  for (const [resolver, bypass] of refused) {
    assert.equal(typeof withArgs(resolver, bypass), "string", `${resolver} | ${bypass}`);
  }
  const tooMany = Array.from({ length: 17 }, (_, i) => `MAP h${i}.example 93.184.215.14:443`);
  assert.equal(
    typeof withArgs([...tooMany, "MAP * ~NOTFOUND"].join(", "), [...tooMany.map((_, i) => `h${i}.example`), "<-loopback>"].join(";")),
    "string"
  );
});

test("paths with dot segments are refused and invalid UTF-8 is a violation", () => {
  for (const profile of ["C:\\x\\..", "C:\\x\\.\\y", "C:\\x\\\\y", "C:\\x\\..\\y"]) {
    const launch = validLaunch();
    launch.profile_dir = profile;
    launch.argv[7] = `--user-data-dir=${profile}`;
    assert.equal(typeof launchRefusal(launch), "string", profile);
  }
  const body = Buffer.from([0x7b, 0x22, 0x66, 0xff, 0x22, 0x7d]);
  const prefix = Buffer.alloc(4);
  prefix.writeUInt32LE(body.length, 0);
  assert.throws(() => new FrameDecoder().push(Buffer.concat([prefix, body])), ProtocolViolation);
});
