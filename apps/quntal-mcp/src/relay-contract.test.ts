import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  DENIED_RELAY_CAPABILITIES,
  isDeniedRelayCapability,
  isPayloadWithinRelayBound,
  isRelayFailureCode,
  missingAuthorizationEnvelopeFields,
  missingEnvelopeFields,
  RELAY_AUTHORIZATION_ENVELOPE_FIELDS,
  RELAY_BOUNDS,
  RELAY_ENVELOPE_FIELDS,
  RELAY_FAILURE_CODES,
  RELAY_FRAME_KINDS,
  RELAY_PROTOCOL_VERSION
} from "./relay_contract.js";
import { RELAY_TRANSPORT_RESERVED } from "./transports/relay_device.js";

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = join(here, "..", "src");

function readSource(rel: string): string {
  return readFileSync(join(srcDir, rel), "utf8");
}

const LOCAL_FAILURE_SAMPLE = [
  "CAPABILITY_DENIED",
  "WORKSPACE_DENIED",
  "APPROVAL_DENIED",
  "APPROVAL_EXPIRED",
  "POLICY_DENIED"
];

test("relay protocol version is frozen", () => {
  assert.equal(RELAY_PROTOCOL_VERSION, "quntal-relay/1");
});

test("relay frame kinds are exactly the frozen set", () => {
  assert.deepEqual([...RELAY_FRAME_KINDS], [
    "mcp_request",
    "mcp_response",
    "mcp_error",
    "cancel",
    "heartbeat"
  ]);
});

test("relay envelope requires every frozen field", () => {
  assert.equal(RELAY_ENVELOPE_FIELDS.length, 13);
  const complete: Record<string, unknown> = {
    protocolVersion: RELAY_PROTOCOL_VERSION,
    routeDeviceId: "route-1",
    remoteConnectionId: "stable-1",
    connectionId: "live-1",
    sequence: 1,
    replayNonce: "nonce-1",
    correlationId: "corr-1",
    messageKind: "mcp_request",
    payloadLength: 12,
    createdAt: "2026-10-01T00:00:00.000Z",
    expiresAt: "2026-10-01T00:02:00.000Z",
    channelAuth: "channel-proof",
    authorizationEnvelope: {}
  };
  assert.deepEqual(missingEnvelopeFields(complete), []);
  const partial = { ...complete };
  delete partial.sequence;
  delete partial.authorizationEnvelope;
  assert.deepEqual(missingEnvelopeFields(partial), [
    "sequence",
    "authorizationEnvelope"
  ]);
});

test("authorization envelope binds principal, connection, device, and digest", () => {
  assert.deepEqual([...RELAY_AUTHORIZATION_ENVELOPE_FIELDS], [
    "principal",
    "connection",
    "device",
    "requestDigest",
    "epoch",
    "sessionContext"
  ]);
  assert.deepEqual(
    missingAuthorizationEnvelopeFields({ principal: "p", device: "d" }),
    ["connection", "requestDigest", "epoch", "sessionContext"]
  );
});

test("relay failure vocabulary is frozen at 17 codes", () => {
  assert.equal(RELAY_FAILURE_CODES.length, 17);
  assert.deepEqual([...RELAY_FAILURE_CODES], [
    "TRANSPORT_UNAVAILABLE",
    "REMOTE_AUTH_REQUIRED",
    "REMOTE_AUTH_INVALID",
    "REMOTE_SCOPE_DENIED",
    "DEVICE_OFFLINE",
    "DEVICE_REVOKED",
    "PAIRING_REQUIRED",
    "PAIRING_EXPIRED",
    "PAIRING_DENIED",
    "ROUTE_MISMATCH",
    "RELAY_REPLAY_DETECTED",
    "RELAY_SEQUENCE_INVALID",
    "REMOTE_RATE_LIMITED",
    "REMOTE_QUEUE_EXPIRED",
    "REMOTE_SESSION_INACTIVE",
    "MCP_PROTOCOL_UNSUPPORTED",
    "TOOL_SURFACE_DENIED"
  ]);
  assert.ok(isRelayFailureCode("REMOTE_SESSION_INACTIVE"));
  assert.ok(isRelayFailureCode("ROUTE_MISMATCH"));
  assert.ok(!isRelayFailureCode("WORKSPACE_DENIED"));
  assert.ok(!isRelayFailureCode("SOMETHING_ELSE"));
});

test("relay failures stay distinct from local failures", () => {
  for (const local of LOCAL_FAILURE_SAMPLE) {
    assert.ok(
      !isRelayFailureCode(local),
      `${local} must not be a relay failure code`
    );
  }
  assert.equal(new Set(RELAY_FAILURE_CODES).size, RELAY_FAILURE_CODES.length);
});

test("denied relay capabilities are frozen and detectable", () => {
  assert.equal(DENIED_RELAY_CAPABILITIES.length, 10);
  assert.ok(isDeniedRelayCapability("tcp_forward"));
  assert.ok(isDeniedRelayCapability("socks"));
  assert.ok(isDeniedRelayCapability("http_connect"));
  assert.ok(isDeniedRelayCapability("websocket_tunnel"));
  assert.ok(isDeniedRelayCapability("remote_shell"));
  assert.ok(!isDeniedRelayCapability("mcp_request"));
  assert.ok(!isDeniedRelayCapability("heartbeat"));
});

test("relay bounds are frozen, positive, and ordered", () => {
  assert.equal(RELAY_BOUNDS.maxFrameBytes, 1048576);
  assert.equal(RELAY_BOUNDS.maxResultBytes, 1048576);
  assert.equal(RELAY_BOUNDS.maxConcurrentRequestsPerConnection, 4);
  assert.equal(RELAY_BOUNDS.maxConnectionsPerDevice, 2);
  assert.equal(RELAY_BOUNDS.maxQueuedFrames, 16);
  assert.equal(RELAY_BOUNDS.reconnectGraceSeconds, 60);
  assert.equal(RELAY_BOUNDS.queueLifetimeSeconds, 60);
  assert.equal(RELAY_BOUNDS.maxFrameLifetimeSeconds, 120);
  assert.equal(RELAY_BOUNDS.heartbeatSeconds, 30);
  assert.equal(RELAY_BOUNDS.idleChannelSuspendSeconds, 120);
  assert.equal(RELAY_BOUNDS.maxRequestsPerMinutePerDevice, 60);
  assert.equal(RELAY_BOUNDS.maxApprovalPromptsPerMinute, 6);
  assert.equal(RELAY_BOUNDS.maxAuthFailuresPerMinute, 5);
  assert.equal(RELAY_BOUNDS.maxPairingAttemptsPerCode, 5);
  assert.equal(RELAY_BOUNDS.logRetentionDays, 30);
  assert.equal(RELAY_BOUNDS.remoteSessionLeaseMaxMinutes, 15);
  assert.ok(RELAY_BOUNDS.queueLifetimeSeconds <= RELAY_BOUNDS.maxFrameLifetimeSeconds);
  assert.ok(RELAY_BOUNDS.heartbeatSeconds < RELAY_BOUNDS.idleChannelSuspendSeconds);
});

test("relay payload bound accepts only in-range sizes", () => {
  assert.ok(isPayloadWithinRelayBound(0));
  assert.ok(isPayloadWithinRelayBound(1048576));
  assert.ok(!isPayloadWithinRelayBound(1048577));
  assert.ok(!isPayloadWithinRelayBound(-1));
  assert.ok(!isPayloadWithinRelayBound(1.5));
});

test("relay device transport stays a fail-closed skeleton", () => {
  assert.equal(RELAY_TRANSPORT_RESERVED, "QUNTAL-P15");
  for (const rel of ["transports/relay_device.ts", "entrypoints/relay_device.ts"]) {
    const text = readSource(rel);
    for (const token of DENIED_RELAY_CAPABILITIES) {
      assert.ok(
        !text.toLowerCase().includes(token),
        `${rel} must not contain denied capability ${token}`
      );
    }
    assert.ok(!text.includes("registerTool("), `${rel} must not register tools`);
    assert.ok(!text.includes("from \"node:net\""), `${rel} must not open sockets`);
    assert.ok(!text.includes("from \"node:http\""), `${rel} must not serve HTTP`);
  }
});

test("relay vocabulary module carries no network or tool authority", () => {
  const text = readSource("relay_contract.ts");
  assert.ok(!text.includes("registerTool("), "vocabulary must not register tools");
  assert.ok(!text.includes("node:net"), "vocabulary must not use sockets");
  assert.ok(!text.includes("node:http"), "vocabulary must not use HTTP");
  assert.ok(!text.includes("fetch("), "vocabulary must not fetch");
  assert.ok(!text.includes("WebSocket"), "vocabulary must not use WebSockets");
  const sources: string[] = [];
  const visit = (dir: string): void => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) {
        visit(path);
      } else if (entry.name.endsWith(".ts")) {
        sources.push(path);
      }
    }
  };
  visit(srcDir);
  assert.ok(sources.length > 0);
});

test("contract document freezes the vocabulary", () => {
  const doc = readFileSync(
    join(here, "..", "..", "..", "docs", "security", "RELAY_PROTOCOL_CONTRACT.md"),
    "utf8"
  );
  assert.ok(doc.includes("Status: FROZEN CONTRACT FOR QUNTAL-P15"));
  assert.ok(doc.includes("SpecGrain: SG-000052"));
  for (const code of RELAY_FAILURE_CODES) {
    assert.ok(doc.includes(code), `contract must freeze failure ${code}`);
  }
  for (const kind of RELAY_FRAME_KINDS) {
    assert.ok(doc.includes(kind), `contract must freeze frame kind ${kind}`);
  }
  for (const phrase of [
    "arbitrary TCP forwarding",
    "SOCKS proxying",
    "HTTP CONNECT proxying",
    "generic HTTP forwarding to arbitrary destinations",
    "arbitrary WebSocket tunneling",
    "arbitrary destination forwarding",
    "remote shell transport",
    "generic command or operation dispatch",
    "schema-fetch-and-execute",
    "hidden subcommands"
  ]) {
    assert.ok(doc.includes(phrase), `contract must deny ${phrase}`);
  }
  assert.ok(doc.includes("1048576"));
  assert.ok(doc.includes("15 minutes"));
});
