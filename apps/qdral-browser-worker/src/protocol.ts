// SG-000108 private worker protocol.
//
// The Rust browser host and this worker exchange length-prefixed JSON frames
// (u32 little-endian length, then UTF-8 JSON) over the worker's anonymous
// stdio pipes, the same framing as the SG-000074 host protocol. There is no
// listener. The vocabulary is closed and versioned; unknown frames or fields
// are protocol violations. The host remains the authority: everything the
// worker checks here is defense in depth against a host bug.

import { isIPv4, isIPv6 } from "node:net";

export const WORKER_PROTOCOL_GENERATION = 1;
export const WORKER_IDENTITY = "qdral-browser-worker";
export const MAX_FRAME_BYTES = 64 * 1024;

export const MAX_ARGV_ENTRIES = 64;
export const MAX_ARG_BYTES = 8192;
export const MAX_PATH_BYTES = 1024;
export const MAX_ENV_ENTRIES = 16;
export const MAX_ENV_VALUE_BYTES = 4096;
/// Mirrors MAX_ADMITTED_DESTINATIONS in the Rust host.
export const MAX_ADMITTED_DESTINATIONS = 16;

/// Engine environment keys the host may pass (its scrubbed environment).
export const ENGINE_ENV_ALLOWLIST: readonly string[] = ["SystemRoot", "SystemDrive", "PATH", "TEMP", "TMP", "LANG"];

/// The SG-000074 frozen flags, in order.
export const FROZEN_FLAGS: readonly string[] = [
  "--headless",
  "--no-first-run",
  "--no-default-browser-check",
  "--disable-extensions",
  "--disable-background-networking",
  "--disable-sync",
  "--no-service-autorun"
];

/// Fixed confinement values from the SG-000108 decision record. The two
/// host-built values (resolver rules, bypass list) are checked for shape.
export const NO_ROUTE_PROXY = "--proxy-server=http://0.0.0.0:9";
export const WEBRTC_POLICY = "--webrtc-ip-handling-policy=disable_non_proxied_udp";
export const DISABLE_QUIC = "--disable-quic";
export const PIPE = "--remote-debugging-pipe";
export const INITIAL_URL = "about:blank";

export type HostFrame =
  | { frame: "hello"; generation: number }
  | { frame: "launch"; engine: string; profile_dir: string; argv: string[]; env: Record<string, string> }
  | { frame: "ping"; nonce: string }
  | { frame: "shutdown" };

export type ErrorCode = "protocol_violation" | "launch_refused" | "launch_failed" | "engine_exited" | "not_ready";

export type WorkerFrame =
  | { frame: "hello"; generation: number; worker: string }
  | { frame: "launched" }
  | { frame: "pong"; nonce: string }
  | { frame: "bye" }
  | { frame: "error"; code: ErrorCode };

export class ProtocolViolation extends Error {}

export function encodeFrame(value: WorkerFrame | HostFrame): Buffer {
  const body = Buffer.from(JSON.stringify(value), "utf8");
  if (body.length === 0 || body.length > MAX_FRAME_BYTES) {
    throw new ProtocolViolation("frame exceeds the size bound");
  }
  const prefix = Buffer.alloc(4);
  prefix.writeUInt32LE(body.length, 0);
  return Buffer.concat([prefix, body]);
}

/// Incremental decoder: push stream chunks, take complete frame bodies. The
/// length bound is enforced before the body is buffered.
export class FrameDecoder {
  private buffered: Buffer = Buffer.alloc(0);
  // Invalid UTF-8 is a violation, as in the Rust host (no replacement).
  private readonly utf8 = new TextDecoder("utf-8", { fatal: true });

  push(chunk: Buffer): unknown[] {
    this.buffered = Buffer.concat([this.buffered, chunk]);
    const frames: unknown[] = [];
    for (;;) {
      if (this.buffered.length < 4) return frames;
      const length = this.buffered.readUInt32LE(0);
      if (length === 0 || length > MAX_FRAME_BYTES) {
        throw new ProtocolViolation("frame length out of bounds");
      }
      if (this.buffered.length < 4 + length) return frames;
      const bytes = this.buffered.subarray(4, 4 + length);
      this.buffered = this.buffered.subarray(4 + length);
      let body: string;
      try {
        body = this.utf8.decode(bytes);
      } catch {
        throw new ProtocolViolation("frame is not valid UTF-8");
      }
      try {
        frames.push(JSON.parse(body) as unknown);
      } catch {
        throw new ProtocolViolation("frame is not JSON");
      }
    }
  }

  get pendingBytes(): number {
    return this.buffered.length;
  }
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value) && Object.getPrototypeOf(value) === Object.prototype;
}

function exactKeys(value: Record<string, unknown>, keys: string[]): void {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    throw new ProtocolViolation("frame has unexpected or missing fields");
  }
}

function boundedString(value: unknown, maxBytes: number, what: string): string {
  if (typeof value !== "string" || value.length === 0 || Buffer.byteLength(value, "utf8") > maxBytes) {
    throw new ProtocolViolation(`${what} is not a bounded string`);
  }
  // Control characters (including NUL) never cross the boundary.
  if (/[\u0000-\u001f\u007f]/.test(value)) {
    throw new ProtocolViolation(`${what} carries control characters`);
  }
  return value;
}

/// Decode one host frame into the closed vocabulary.
export function parseHostFrame(raw: unknown): HostFrame {
  if (!isPlainObject(raw) || typeof raw.frame !== "string") {
    throw new ProtocolViolation("frame is not a tagged object");
  }
  switch (raw.frame) {
    case "hello": {
      exactKeys(raw, ["frame", "generation"]);
      if (!Number.isSafeInteger(raw.generation)) throw new ProtocolViolation("hello generation is not an integer");
      return { frame: "hello", generation: raw.generation as number };
    }
    case "ping": {
      exactKeys(raw, ["frame", "nonce"]);
      const nonce = boundedString(raw.nonce, 64, "ping nonce");
      if (!/^[A-Za-z0-9_-]+$/.test(nonce)) throw new ProtocolViolation("ping nonce is malformed");
      return { frame: "ping", nonce };
    }
    case "shutdown": {
      exactKeys(raw, ["frame"]);
      return { frame: "shutdown" };
    }
    case "launch": {
      exactKeys(raw, ["frame", "engine", "profile_dir", "argv", "env"]);
      const engine = boundedString(raw.engine, MAX_PATH_BYTES, "engine path");
      const profileDir = boundedString(raw.profile_dir, MAX_PATH_BYTES, "profile directory");
      if (!Array.isArray(raw.argv) || raw.argv.length === 0 || raw.argv.length > MAX_ARGV_ENTRIES) {
        throw new ProtocolViolation("argv is not a bounded array");
      }
      const argv = raw.argv.map((arg, index) => boundedString(arg, MAX_ARG_BYTES, `argv[${index}]`));
      if (!isPlainObject(raw.env)) throw new ProtocolViolation("env is not an object");
      const entries = Object.entries(raw.env);
      if (entries.length > MAX_ENV_ENTRIES) throw new ProtocolViolation("env has too many entries");
      // A null prototype: no key (including "__proto__") is special.
      const env: Record<string, string> = Object.create(null) as Record<string, string>;
      for (const [key, value] of entries) {
        env[key] = boundedString(value, MAX_ENV_VALUE_BYTES, `env ${key}`);
      }
      return { frame: "launch", engine, profile_dir: profileDir, argv, env };
    }
    default:
      throw new ProtocolViolation("unknown frame");
  }
}

/// Mirrors validate_admitted_host in the Rust host: lowercase ASCII LDH, two
/// or more labels of 1..=63 bytes, at most 253 bytes, no label starting or
/// ending with a hyphen, and a last label that is not numeric or 0x-prefixed.
export function isAdmittedHost(host: string): boolean {
  if (host.length === 0 || host.length > 253 || !/^[a-z0-9.-]+$/.test(host)) return false;
  const labels = host.split(".");
  if (labels.length < 2) return false;
  for (const label of labels) {
    if (label.length === 0 || label.length > 63 || label.startsWith("-") || label.endsWith("-")) return false;
  }
  const last = labels[labels.length - 1] ?? "";
  return !/^[0-9]+$/.test(last) && !last.startsWith("0x");
}

/// Parse the resolver rules into admitted names, in order, or null when the
/// value is not exactly `MAP <name> <ipv4|[ipv6]>:443, …, MAP * ~NOTFOUND`.
/// Address class is the host's policy (test builds pin fixtures to loopback);
/// the worker checks the grammar so nothing else can be smuggled in.
export function parseResolverRules(value: string): string[] | null {
  const entries = value.split(", ");
  if (entries.pop() !== "MAP * ~NOTFOUND") return null;
  if (entries.length === 0 || entries.length > MAX_ADMITTED_DESTINATIONS) return null;
  const names: string[] = [];
  for (const entry of entries) {
    const match = /^MAP ([^ ]+) ([^ ]+):443$/.exec(entry);
    if (!match) return null;
    const [, name = "", address = ""] = match;
    if (!isAdmittedHost(name) || names.includes(name)) return null;
    const v6 = /^\[(.+)\]$/.exec(address);
    if (v6 ? !isIPv6(v6[1] ?? "") : !isIPv4(address)) return null;
    names.push(name);
  }
  return names;
}

function isAbsoluteWindowsPath(path: string): boolean {
  if (!/^[A-Za-z]:\\/.test(path) || path.includes("/")) return false;
  // No empty, "." or ".." segments anywhere.
  return path.slice(3).split("\\").every((segment) => segment !== "" && segment !== "." && segment !== "..");
}

/// Defense-in-depth check of a launch frame against the SG-000108 contract.
/// Returns a reason when the frame must be refused, otherwise null.
export function launchRefusal(launch: Extract<HostFrame, { frame: "launch" }>): string | null {
  if (!isAbsoluteWindowsPath(launch.engine)) return "engine path is not an absolute Windows path";
  const leaf = launch.engine.split("\\").pop()?.toLowerCase();
  if (leaf !== "msedge.exe" && leaf !== "chrome.exe") return "engine is not a supported browser executable";
  if (!isAbsoluteWindowsPath(launch.profile_dir)) return "profile directory is not an absolute Windows path";
  for (const key of Object.keys(launch.env)) {
    if (!ENGINE_ENV_ALLOWLIST.includes(key)) return `engine environment key ${key} is not allowed`;
  }
  const { argv } = launch;
  const frozenCount = FROZEN_FLAGS.length;
  for (let index = 0; index < frozenCount; index++) {
    if (argv[index] !== FROZEN_FLAGS[index]) return "argv does not start with the frozen SG-000074 flags";
  }
  const rest = argv.slice(frozenCount);
  if (rest.length !== 8) return "argv does not carry exactly the profile and the six confinement flags";
  const [profile, pipe, resolver, proxy, bypass, webrtc, quic, initial] = rest;
  if (profile !== `--user-data-dir=${launch.profile_dir}`) return "argv profile differs from the launch profile";
  if (pipe !== PIPE) return "argv lacks the debugging pipe";
  const resolverPrefix = "--host-resolver-rules=";
  const names = resolver?.startsWith(resolverPrefix) ? parseResolverRules(resolver.slice(resolverPrefix.length)) : null;
  if (names === null) return "argv resolver rules are not an exact pinned map ending in MAP * ~NOTFOUND";
  if (proxy !== NO_ROUTE_PROXY) return "argv proxy is not the no-route proxy";
  if (bypass !== `--proxy-bypass-list=${[...names, "<-loopback>"].join(";")}`) {
    return "argv bypass list is not exactly the admitted names followed by <-loopback>";
  }
  if (webrtc !== WEBRTC_POLICY) return "argv lacks the WebRTC UDP policy";
  if (quic !== DISABLE_QUIC) return "argv lacks the QUIC refusal";
  if (initial !== INITIAL_URL) return "argv does not end with the blank page";
  return null;
}
