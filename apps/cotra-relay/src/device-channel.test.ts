import assert from "node:assert/strict";
import test from "node:test";
import {
  generateDeviceId,
  generateDeviceKeyPair,
  publicDeviceIdentity,
  signDeviceChallenge,
  type DeviceKeyPair
} from "@cotra/mcp/dist/device_identity.js";
import {
  computeRequestDigest,
  DeviceUplinkCore,
  sessionContextFor,
  type ConnectionBinding,
  type PairedConnection,
  type UplinkDispatcher
} from "@cotra/mcp/dist/device_uplink.js";
import { generateRemoteConnectionId, generateRemotePrincipalId } from "@cotra/mcp/dist/oauth_authorization.js";
import { RELAY_BOUNDS, RELAY_PROTOCOL_VERSION } from "@cotra/mcp/dist/relay_contract.js";
import { DeviceChannelHub } from "./device_channel.js";
import { MemoryRelayStore } from "./store.js";

interface Fixture {
  readonly hub: DeviceChannelHub;
  readonly store: MemoryRelayStore;
  readonly pair: DeviceKeyPair;
  readonly deviceId: string;
  readonly principal: string;
  readonly connection: string;
  readonly token: string;
}

function open(): Fixture {
  const store = new MemoryRelayStore();
  const deviceId = generateDeviceId();
  const pair = generateDeviceKeyPair(deviceId, Date.now(), 1);
  store.putDevice(publicDeviceIdentity(pair));
  const hub = new DeviceChannelHub(store, Date.now, 50);
  const issued = hub.challenge({ deviceId, epoch: 1 });
  assert.equal(issued.status, 200);
  const challenge = (issued.json as { challenge: never }).challenge;
  const session = hub.session({ response: signDeviceChallenge(pair.privateKeyJwkBase64, challenge) });
  assert.equal(session.status, 200);
  return {
    hub,
    store,
    pair,
    deviceId,
    principal: generateRemotePrincipalId(),
    connection: generateRemoteConnectionId(),
    token: (session.json as { channelToken: string }).channelToken
  };
}

let counter = 0;
function requestFrame(f: Fixture, sequence: number, connectionId = "cn-" + "a".repeat(32)): Record<string, unknown> {
  counter += 1;
  const payload = JSON.stringify({ jsonrpc: "2.0", id: sequence, method: "tools/call", params: { name: "fs_read", arguments: {} } });
  const correlationId = `corr-${counter}`;
  const now = Date.now();
  return {
    protocolVersion: RELAY_PROTOCOL_VERSION,
    routeDeviceId: f.deviceId,
    remoteConnectionId: f.connection,
    connectionId,
    sequence,
    replayNonce: Buffer.from(`nonce-${counter}-${"y".repeat(20)}`).toString("base64url"),
    correlationId,
    messageKind: "mcp_request",
    payloadLength: Buffer.byteLength(payload),
    createdAt: now,
    expiresAt: now + 60_000,
    channelAuth: "",
    authorizationEnvelope: {
      principal: f.principal,
      connection: f.connection,
      device: f.deviceId,
      requestDigest: computeRequestDigest({
        principal: f.principal,
        remoteConnectionId: f.connection,
        connectionId,
        deviceId: f.deviceId,
        correlationId,
        sequence,
        method: "tools/call",
        payload
      }),
      epoch: 1,
      sessionContext: sessionContextFor(["cotra.read"])
    },
    payload
  };
}

async function pollFrames(f: Fixture): Promise<Record<string, unknown>[]> {
  const reply = await f.hub.poll({ channelToken: f.token });
  assert.equal(reply.status, 200);
  return (reply.json as { frames: Record<string, unknown>[] }).frames;
}

function responseFrame(f: Fixture, request: Record<string, unknown>, overrides: Record<string, unknown> = {}): Record<string, unknown> {
  counter += 1;
  const payload = JSON.stringify({ jsonrpc: "2.0", id: request.sequence, result: {} });
  return {
    protocolVersion: RELAY_PROTOCOL_VERSION,
    routeDeviceId: f.deviceId,
    remoteConnectionId: f.connection,
    connectionId: request.connectionId,
    sequence: 1,
    replayNonce: Buffer.from(`resp-${counter}-${"z".repeat(20)}`).toString("base64url"),
    correlationId: request.correlationId,
    messageKind: "mcp_response",
    payloadLength: Buffer.byteLength(payload),
    createdAt: Date.now(),
    expiresAt: Date.now() + 60_000,
    channelAuth: f.token,
    payload,
    failure: null,
    ...overrides
  };
}

test("refusals are known before any sequence is used", () => {
  const f = open();
  const store = new MemoryRelayStore();
  const offline = new DeviceChannelHub(store);
  assert.equal(offline.refusal(f.deviceId), "DEVICE_OFFLINE");
  assert.equal(f.hub.refusal(f.deviceId), null);
  for (let i = 1; i <= RELAY_BOUNDS.maxQueuedFrames; i += 1) {
    void f.hub.dispatch(f.deviceId, f.connection, requestFrame(f, i), 60_000, true);
  }
  assert.equal(f.hub.refusal(f.deviceId), "REMOTE_RATE_LIMITED");
  f.hub.closeChannel(f.deviceId);
});

test("a request that times out before delivery becomes a cancel and is never executed", async () => {
  const f = open();
  const first = requestFrame(f, 1);
  const second = requestFrame(f, 2);
  const outcome = await f.hub.dispatch(f.deviceId, f.connection, first, 20, true);
  assert.deepEqual(outcome, { kind: "failure", failure: "REMOTE_QUEUE_EXPIRED" });
  void f.hub.dispatch(f.deviceId, f.connection, second, 60_000, true);
  const delivered = await pollFrames(f);
  assert.deepEqual(delivered.map((frame) => [frame.sequence, frame.messageKind]), [
    [1, "cancel"],
    [2, "mcp_request"]
  ]);
  // The real device core accepts the converted cancel (sequence stays
  // contiguous) and executes only the second request.
  const executed: string[] = [];
  const dispatcher: UplinkDispatcher = {
    async dispatch(_binding: ConnectionBinding, payload: string) {
      executed.push(payload);
      return JSON.stringify({ jsonrpc: "2.0", id: 2, result: {} });
    },
    closeConnection() {}
  };
  const paired: PairedConnection = {
    principal: f.principal,
    remoteConnectionId: f.connection,
    providerKind: "generic",
    clientProfileId: "profile-1",
    clientProfileRevision: 1,
    toolSurfaceProfile: "core",
    scopeCeiling: ["cotra.read"]
  };
  const core = new DeviceUplinkCore({ deviceId: f.deviceId, deviceEpoch: 1 }, [paired], dispatcher, () => []);
  core.setChannelToken(f.token);
  core.receive(delivered, Date.now());
  await Promise.all(core.pump(Date.now()));
  assert.equal(executed.length, 1);
  assert.ok(executed[0]?.includes('"id":2'));
  const errors = core.drainOutbox(Date.now()).filter((frame) => frame.messageKind === "mcp_error");
  assert.deepEqual(errors, [], "the converted cancel is accepted without any protocol error");
  f.hub.closeChannel(f.deviceId);
});

test("a request that times out after delivery reports an unknown outcome", async () => {
  const f = open();
  const request = requestFrame(f, 1);
  const pending = f.hub.dispatch(f.deviceId, f.connection, request, 30, true);
  const delivered = await pollFrames(f);
  assert.equal(delivered.length, 1);
  assert.deepEqual(await pending, { kind: "failure", failure: "TRANSPORT_UNAVAILABLE" });
  f.hub.closeChannel(f.deviceId);
});

test("closing a channel distinguishes never-executed from unknown outcomes", async () => {
  const f = open();
  const delivered = f.hub.dispatch(f.deviceId, f.connection, requestFrame(f, 1), 60_000, true);
  await pollFrames(f);
  const queued = f.hub.dispatch(f.deviceId, f.connection, requestFrame(f, 2), 60_000, true);
  f.hub.closeChannel(f.deviceId);
  assert.deepEqual(await delivered, { kind: "failure", failure: "TRANSPORT_UNAVAILABLE" });
  assert.deepEqual(await queued, { kind: "failure", failure: "DEVICE_OFFLINE" });
  assert.equal(f.hub.refusal(f.deviceId), "DEVICE_OFFLINE");
});

test("push accepts only exact, correlated, single-use responses for this device", async () => {
  const f = open();
  const request = requestFrame(f, 1);
  const outcome = f.hub.dispatch(f.deviceId, f.connection, request, 60_000, true);
  const [signed] = await pollFrames(f);
  assert.ok(signed !== undefined);
  const good = responseFrame(f, signed);
  const forgedToken = responseFrame(f, signed, { channelAuth: "Q".repeat(43) });
  const otherDevice = responseFrame(f, signed, { routeDeviceId: generateDeviceId() });
  const otherRoute = responseFrame(f, signed, { remoteConnectionId: generateRemoteConnectionId() });
  const extraField = { ...responseFrame(f, signed), result: "x" };
  const badFailure = responseFrame(f, signed, { messageKind: "mcp_error", payload: null, payloadLength: 0, failure: "EVERYTHING_OK" });
  const lying = responseFrame(f, signed, { payloadLength: 3 });
  const reply = f.hub.push({ channelToken: f.token, frames: [forgedToken, otherDevice, otherRoute, extraField, badFailure, lying, good, good] });
  assert.deepEqual(reply.json, { accepted: 1 });
  const result = await outcome;
  assert.equal(result.kind, "response");
  assert.equal(f.hub.pendingCount(), 0);
  const late = f.hub.push({ channelToken: f.token, frames: [responseFrame(f, signed)] });
  assert.deepEqual(late.json, { accepted: 0 }, "a correlation resolves exactly once");
  assert.equal(f.hub.push({ channelToken: "bad", frames: [] }).status, 401);
  assert.equal(f.hub.push({ channelToken: f.token, frames: new Array(RELAY_BOUNDS.maxQueuedFrames + 1).fill({}) }).status, 400);
  f.hub.closeChannel(f.deviceId);
});

test("a revoked device loses its channel and an unproven device never opens one", async () => {
  const f = open();
  f.store.revokeDevice(f.deviceId);
  assert.equal((await f.hub.poll({ channelToken: f.token })).status, 401);
  assert.equal(f.hub.refusal(f.deviceId), "DEVICE_OFFLINE");
  const g = open();
  const issued = g.hub.challenge({ deviceId: g.deviceId, epoch: 1 });
  const stale = (issued.json as { challenge: never }).challenge;
  const other = generateDeviceKeyPair(g.deviceId, Date.now(), 1);
  assert.equal(g.hub.session({ response: signDeviceChallenge(other.privateKeyJwkBase64, stale) }).status, 401);
  assert.equal(g.hub.session({ response: signDeviceChallenge(g.pair.privateKeyJwkBase64, stale) }).status, 401, "the challenge was consumed");
  assert.equal(g.hub.challenge({ deviceId: "dev-nope", epoch: 1 }).status, 401);
  assert.equal(g.hub.challenge("x").status, 401);
  g.hub.closeChannel(g.deviceId);
});
