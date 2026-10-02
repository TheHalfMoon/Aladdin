import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  CHALLENGE_EXPIRY_SECONDS,
  createDeviceChallenge,
  DEVICE_ID_PATTERN,
  deviceIdentityFailuresAreFrozenSubset,
  emergencyRevoke,
  generateDeviceId,
  generateDeviceKeyPair,
  isDeviceId,
  isPairingCode,
  issuePairingCode,
  isRouteRevoked,
  PAIRING_CODE_EXPIRY_SECONDS,
  PAIRING_CODE_MAX_ATTEMPTS,
  publicDeviceIdentity,
  redactedDeviceSummary,
  redactedPairingSummary,
  revokeAllRoutes,
  revokeDevice,
  revokePrincipal,
  rotateDeviceKeyPair,
  signDeviceChallenge,
  verifyAuthorizationEnvelope,
  verifyChallengeResponse,
  verifyPairingAttempt
} from "./device_identity.js";
import {
  DENIED_RELAY_CAPABILITIES,
  RELAY_BOUNDS,
  RELAY_FAILURE_CODES,
  RELAY_FRAME_KINDS
} from "./relay_contract.js";
import { RELAY_TRANSPORT_RESERVED } from "./transports/relay_device.js";

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = join(here, "..", "src");
const repoDoc = (...parts: string[]): string =>
  join(here, "..", "..", "..", ...parts);

function readSource(rel: string): string {
  try {
    return readFileSync(join(srcDir, rel), "utf8");
  } catch {
    return readFileSync(join(here, rel), "utf8");
  }
}

function readDoc(rel: string): string {
  return readFileSync(repoDoc(rel), "utf8");
}
const NOW = 1798765432100;

function sampleDeviceId(): string {
  return "dev-" + "ab".repeat(16);
}

function sampleDigest(): string {
  return "cd".repeat(32);
}

test("device identity format is tenant-scoped and opaque", () => {
  assert.ok(isDeviceId(sampleDeviceId()));
  assert.ok(!isDeviceId("dev-SHORT"));
  assert.ok(!isDeviceId("device-abcd"));
  assert.ok(!isDeviceId(""));
  assert.ok(DEVICE_ID_PATTERN.test(generateDeviceId()));
  const first = generateDeviceId();
  const second = generateDeviceId();
  assert.notEqual(first, second);
});

test("protected device key separates public identity from private key", () => {
  const pair = generateDeviceKeyPair(sampleDeviceId(), NOW, 1);
  assert.equal(pair.deviceId, sampleDeviceId());
  assert.equal(pair.epoch, 1);
  assert.notEqual(pair.publicKeyJwkBase64, pair.privateKeyJwkBase64);
  const identity = publicDeviceIdentity(pair);
  assert.equal(identity.deviceId, pair.deviceId);
  assert.equal(identity.publicKeyJwkBase64, pair.publicKeyJwkBase64);
  assert.equal(identity.epoch, pair.epoch);
  const serialized = JSON.stringify(identity);
  assert.ok(!serialized.includes(pair.privateKeyJwkBase64));
  const summary = JSON.stringify(redactedDeviceSummary(identity));
  assert.ok(summary.includes(identity.deviceId));
  assert.ok(!summary.includes(pair.privateKeyJwkBase64));
  assert.ok(!summary.includes(identity.publicKeyJwkBase64));
});

test("challenge and response authentication binds device and epoch", () => {
  const pair = generateDeviceKeyPair(sampleDeviceId(), NOW, 1);
  const identity = publicDeviceIdentity(pair);
  const challenge = createDeviceChallenge(identity.deviceId, identity.epoch, NOW);
  assert.equal(
    challenge.expiresAtMs - challenge.createdAtMs,
    CHALLENGE_EXPIRY_SECONDS * 1000
  );
  const response = signDeviceChallenge(pair.privateKeyJwkBase64, challenge);
  assert.deepEqual(verifyChallengeResponse(identity, challenge, response, NOW), {
    ok: true
  });
});

test("challenge verification fails closed on remap, epoch, expiry, and forgery", () => {
  const deviceA = "dev-" + "ab".repeat(16);
  const deviceB = "dev-" + "cd".repeat(16);
  const pair = generateDeviceKeyPair(deviceA, NOW, 1);
  const identity = publicDeviceIdentity(pair);
  const challenge = createDeviceChallenge(identity.deviceId, identity.epoch, NOW);
  const response = signDeviceChallenge(pair.privateKeyJwkBase64, challenge);
  const otherPair = generateDeviceKeyPair(deviceB, NOW, 1);
  const otherIdentity = publicDeviceIdentity(otherPair);
  assert.deepEqual(
    verifyChallengeResponse(otherIdentity, challenge, response, NOW),
    { ok: false, failure: "ROUTE_MISMATCH" }
  );
  const rotated = rotateDeviceKeyPair(pair, NOW + 1000);
  const rotatedIdentity = publicDeviceIdentity(rotated);
  assert.equal(rotatedIdentity.epoch, 2);
  assert.equal(rotatedIdentity.deviceId, identity.deviceId);
  assert.notEqual(rotatedIdentity.publicKeyJwkBase64, identity.publicKeyJwkBase64);
  assert.deepEqual(
    verifyChallengeResponse(rotatedIdentity, challenge, response, NOW),
    { ok: false, failure: "REMOTE_AUTH_INVALID" }
  );
  assert.deepEqual(
    verifyChallengeResponse(
      identity,
      challenge,
      response,
      challenge.expiresAtMs + 1
    ),
    { ok: false, failure: "REMOTE_AUTH_INVALID" }
  );
  const forged = {
    ...response,
    signatureBase64: Buffer.from("forged-signature-material-0123456789abcd").toString(
      "base64"
    )
  };
  assert.deepEqual(verifyChallengeResponse(identity, challenge, forged, NOW), {
    ok: false,
    failure: "REMOTE_AUTH_INVALID"
  });
});

test("one-time pairing enforces entropy, expiry, one-shot, and rate limits", () => {
  assert.equal(PAIRING_CODE_MAX_ATTEMPTS, 5);
  assert.equal(PAIRING_CODE_MAX_ATTEMPTS, RELAY_BOUNDS.maxPairingAttemptsPerCode);
  const issued = issuePairingCode(NOW, "pair-1");
  assert.ok(isPairingCode(issued.code));
  assert.equal(issued.code.length, 64);
  assert.equal(
    issued.record.expiresAtMs - issued.record.createdAtMs,
    PAIRING_CODE_EXPIRY_SECONDS * 1000
  );
  assert.equal(issued.record.attempts, 0);
  assert.equal(issued.record.consumed, false);
  assert.equal(issued.record.invalidated, false);
  const second = issuePairingCode(NOW, "pair-2");
  assert.notEqual(issued.code, second.code);
  const accepted = verifyPairingAttempt(issued.record, issued.code, NOW + 1000);
  assert.deepEqual(accepted.check, { ok: true });
  assert.equal(accepted.next.consumed, true);
  const replay = verifyPairingAttempt(accepted.next, issued.code, NOW + 2000);
  assert.deepEqual(replay.check, {
    ok: false,
    failure: "PAIRING_DENIED"
  });
  const expiredRecord = issuePairingCode(NOW, "pair-expired").record;
  const expired = verifyPairingAttempt(
    expiredRecord,
    issuePairingCode(NOW, "unused").code,
    expiredRecord.expiresAtMs + 1
  );
  assert.equal(expired.check.ok, false);
  if (!expired.check.ok) {
    assert.equal(expired.check.failure, "PAIRING_EXPIRED");
  }
  assert.equal(expired.next.invalidated, true);
});

test("pairing rate limiting invalidates after five failures without trust", () => {
  let record = issuePairingCode(NOW, "pair-rate").record;
  const wrong = "00".repeat(32);
  const probe = issuePairingCode(NOW, "pair-probe").code;
  assert.notEqual(wrong, probe);
  for (let attempt = 1; attempt <= 5; attempt += 1) {
    const result = verifyPairingAttempt(record, wrong, NOW + attempt);
    record = result.next;
    assert.equal(result.check.ok, false);
    assert.equal(record.attempts, attempt);
  }
  assert.equal(record.invalidated, true);
  const afterLock = verifyPairingAttempt(record, wrong, NOW + 100);
  assert.deepEqual(afterLock.check, {
    ok: false,
    failure: "PAIRING_DENIED"
  });
  const fresh = issuePairingCode(NOW, "pair-clean");
  const freshResult = verifyPairingAttempt(fresh.record, fresh.code, NOW + 1);
  assert.deepEqual(freshResult.check, { ok: true });
  const serialized = JSON.stringify(freshResult);
  assert.ok(!serialized.includes("workspace_id"));
  assert.ok(!serialized.includes("policy_revision"));
  assert.ok(!serialized.includes("approval_digest"));
  assert.ok(!serialized.includes(fresh.code));
  const summary = JSON.stringify(redactedPairingSummary(freshResult.next));
  assert.ok(summary.includes("pair-clean"));
  assert.ok(!summary.includes(fresh.code));
  assert.ok(!summary.includes(fresh.record.codeHashHex));
});

test("hard, principal-wide, all-route, and emergency revocation invalidate sessions", () => {
  const deviceId = sampleDeviceId();
  const route = {
    principal: "principal-a",
    stableConnectionId: "stable-1",
    deviceId,
    epoch: 1
  };
  assert.deepEqual(isRouteRevoked(route, []), { ok: true });
  assert.deepEqual(
    isRouteRevoked(route, [revokeDevice(deviceId, 1, NOW, "lost device")]),
    { ok: false, failure: "DEVICE_REVOKED" }
  );
  assert.deepEqual(
    isRouteRevoked(route, [
      revokePrincipal("principal-a", 1, NOW, "principal compromise")
    ]),
    { ok: false, failure: "DEVICE_REVOKED" }
  );
  assert.deepEqual(
    isRouteRevoked(route, [revokeAllRoutes(1, NOW, "all routes closed")]),
    { ok: false, failure: "DEVICE_REVOKED" }
  );
  assert.deepEqual(
    isRouteRevoked(route, [emergencyRevoke(1, NOW, "emergency stop")]),
    { ok: false, failure: "DEVICE_REVOKED" }
  );
  const otherDevice = {
    principal: "principal-a",
    stableConnectionId: "stable-1",
    deviceId: generateDeviceId(),
    epoch: 1
  };
  assert.deepEqual(
    isRouteRevoked(otherDevice, [revokeDevice(deviceId, 1, NOW, "lost device")]),
    { ok: true }
  );
  const otherPrincipal = {
    principal: "principal-b",
    stableConnectionId: "stable-1",
    deviceId,
    epoch: 1
  };
  assert.deepEqual(
    isRouteRevoked(otherPrincipal, [
      revokePrincipal("principal-a", 1, NOW, "principal compromise")
    ]),
    { ok: true }
  );
});

test("authorization envelope verifies exact principal, connection, device, digest, epoch, and session", () => {
  const expected = {
    principal: "principal-a",
    connection: "stable-1",
    deviceId: sampleDeviceId(),
    requestDigestHex: sampleDigest(),
    epoch: 3,
    sessionContext: "session-7"
  };
  const valid = {
    principal: "principal-a",
    connection: "stable-1",
    device: sampleDeviceId(),
    requestDigest: sampleDigest(),
    epoch: 3,
    sessionContext: "session-7"
  };
  assert.deepEqual(verifyAuthorizationEnvelope(valid, expected, []), {
    ok: true
  });
});

test("compromised relay cannot remap principal A onto device B", () => {
  const expected = {
    principal: "principal-a",
    connection: "stable-1",
    deviceId: sampleDeviceId(),
    requestDigestHex: sampleDigest(),
    epoch: 3,
    sessionContext: "session-7"
  };
  const remappedDevice = {
    ...{
      principal: "principal-a",
      connection: "stable-1",
      device: sampleDeviceId(),
      requestDigest: sampleDigest(),
      epoch: 3,
      sessionContext: "session-7"
    },
    device: generateDeviceId()
  };
  assert.deepEqual(verifyAuthorizationEnvelope(remappedDevice, expected, []), {
    ok: false,
    failure: "ROUTE_MISMATCH"
  });
  const remappedPrincipal = {
    principal: "principal-b",
    connection: "stable-1",
    device: expected.deviceId,
    requestDigest: expected.requestDigestHex,
    epoch: 3,
    sessionContext: "session-7"
  };
  assert.deepEqual(
    verifyAuthorizationEnvelope(remappedPrincipal, expected, []),
    { ok: false, failure: "ROUTE_MISMATCH" }
  );
  const wrongConnection = {
    principal: "principal-a",
    connection: "stable-2",
    device: expected.deviceId,
    requestDigest: expected.requestDigestHex,
    epoch: 3,
    sessionContext: "session-7"
  };
  assert.deepEqual(
    verifyAuthorizationEnvelope(wrongConnection, expected, []),
    { ok: false, failure: "ROUTE_MISMATCH" }
  );
  const wrongDigest = {
    principal: "principal-a",
    connection: "stable-1",
    device: expected.deviceId,
    requestDigest: "00".repeat(32),
    epoch: 3,
    sessionContext: "session-7"
  };
  assert.deepEqual(verifyAuthorizationEnvelope(wrongDigest, expected, []), {
    ok: false,
    failure: "REMOTE_AUTH_INVALID"
  });
  const wrongEpoch = {
    principal: "principal-a",
    connection: "stable-1",
    device: expected.deviceId,
    requestDigest: expected.requestDigestHex,
    epoch: 4,
    sessionContext: "session-7"
  };
  assert.deepEqual(verifyAuthorizationEnvelope(wrongEpoch, expected, []), {
    ok: false,
    failure: "REMOTE_AUTH_INVALID"
  });
  const wrongSession = {
    principal: "principal-a",
    connection: "stable-1",
    device: expected.deviceId,
    requestDigest: expected.requestDigestHex,
    epoch: 3,
    sessionContext: "session-8"
  };
  assert.deepEqual(verifyAuthorizationEnvelope(wrongSession, expected, []), {
    ok: false,
    failure: "REMOTE_AUTH_INVALID"
  });
  const revoked = verifyAuthorizationEnvelope(
    {
      principal: "principal-a",
      connection: "stable-1",
      device: expected.deviceId,
      requestDigest: expected.requestDigestHex,
      epoch: 3,
      sessionContext: "session-7"
    },
    expected,
    [revokeDevice(expected.deviceId, 3, NOW, "revoked")]
  );
  assert.deepEqual(revoked, { ok: false, failure: "DEVICE_REVOKED" });
});

test("device identity uses only the frozen relay failure vocabulary", () => {
  assert.equal(RELAY_FAILURE_CODES.length, 17);
  assert.equal(RELAY_FRAME_KINDS.length, 5);
  assert.ok(deviceIdentityFailuresAreFrozenSubset());
});

test("device identity module carries no network, tool, or approval authority", () => {
  const text = readSource("device_identity.ts");
  assert.ok(!text.includes("registerTool("), "identity must not register tools");
  assert.ok(!text.includes("node:net"), "identity must not use sockets");
  assert.ok(!text.includes("node:http"), "identity must not use HTTP");
  assert.ok(!text.includes("fetch("), "identity must not fetch");
  assert.ok(!text.includes("WebSocket"), "identity must not use WebSockets");
  assert.ok(
    !text.includes("requestApproval"),
    "identity must not request approval"
  );
  assert.ok(
    !text.includes("approvalHistory"),
    "identity must not touch approval history"
  );
  assert.ok(
    !text.includes("registerApproval"),
    "identity must not register approval"
  );
  assert.ok(
    text.includes("Pairing alone grants no workspace trust"),
    "identity must document no trust from pairing"
  );
  for (const token of DENIED_RELAY_CAPABILITIES) {
    assert.ok(
      !text.toLowerCase().includes(token),
      `identity must not contain denied capability ${token}`
    );
  }
});

test("relay device transport stays a fail-closed skeleton", () => {
  assert.equal(RELAY_TRANSPORT_RESERVED, "COTRA-P15");
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
    assert.ok(!text.includes("device_identity"), `${rel} must not wire identity yet`);
  }
});

test("device identity note documents pairing, revocation, and non-goals", () => {
  const doc = readDoc(join("docs", "security", "SG-000053_DEVICE_IDENTITY_NOTE.md"));
  assert.ok(doc.includes("SG-000053"));
  assert.ok(doc.includes("private device key never leaves"));
  assert.ok(doc.includes("grants no workspace trust"));
  assert.ok(doc.includes("ROUTE_MISMATCH"));
  assert.ok(doc.includes("DEVICE_REVOKED"));
  assert.ok(doc.includes("Non-goals"));
});
