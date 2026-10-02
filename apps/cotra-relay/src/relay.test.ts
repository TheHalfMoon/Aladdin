import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync, readdirSync } from "node:fs";
import { request as httpRequest } from "node:http";
import { createServer as createNetServer } from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  generateDeviceKeyPair,
  generateDeviceId,
  publicDeviceIdentity,
  signDeviceChallenge
} from "@cotra/mcp/dist/device_identity.js";
import {
  createMcpFrameDispatcher,
  runDeviceUplink,
  type PairedConnection
} from "@cotra/mcp/dist/device_uplink.js";
import type { KernelClient, RemoteDispatchContext } from "@cotra/mcp/dist/kernel.js";
import {
  generateRemoteConnectionId,
  generateRemotePrincipalId,
  generateTokenSigningKey,
  signAccessToken,
  verificationKeyOf,
  type AccessTokenClaims,
  type OAuthScope,
  type TokenSigningKey
} from "@cotra/mcp/dist/oauth_authorization.js";
import { DENIED_RELAY_CAPABILITIES } from "@cotra/mcp/dist/relay_contract.js";
import { CANONICAL_TOOL_NAMES } from "@cotra/mcp/dist/server.js";
import { createRelayServer, type RelayServer } from "./server.js";
import { MemoryRelayStore } from "./store.js";

const here = dirname(fileURLToPath(import.meta.url));
const ISSUER = "https://auth.cotra.example";
const CLIENT_ID = "https://provider.example/oauth/client.json";

async function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const probe = createNetServer();
    probe.once("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const address = probe.address();
      probe.close(() => resolve(typeof address === "object" && address !== null ? address.port : 0));
    });
  });
}

async function waitFor(condition: () => boolean, label: string, timeoutMs = 10_000): Promise<void> {
  const start = Date.now();
  while (!condition()) {
    if (Date.now() - start > timeoutMs) {
      throw new Error(`timed out waiting for ${label}`);
    }
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

interface Harness {
  readonly origin: string;
  readonly relay: RelayServer;
  readonly store: MemoryRelayStore;
  readonly key: TokenSigningKey;
  readonly deviceId: string;
  readonly principal: string;
  readonly connection: string;
  readonly kernelCalls: Array<{ capability: string; remote: RemoteDispatchContext }>;
  readonly stopUplink: () => void;
  close(): Promise<void>;
  token(overrides?: Partial<AccessTokenClaims> & { scopes?: readonly OAuthScope[] }): string;
}

async function startHarness(options: { connectDevice: boolean; allowedOrigins?: string[] }): Promise<Harness> {
  const port = await freePort();
  const origin = `http://127.0.0.1:${port}`;
  const store = new MemoryRelayStore();
  const key = generateTokenSigningKey();
  store.addVerificationKey(verificationKeyOf(key));
  const deviceId = generateDeviceId();
  const pair = generateDeviceKeyPair(deviceId, Date.now(), 1);
  store.putDevice(publicDeviceIdentity(pair));
  const principal = generateRemotePrincipalId();
  const connection = generateRemoteConnectionId();
  store.putConnection({
    principal,
    remoteConnectionId: connection,
    deviceId,
    clientId: CLIENT_ID,
    providerKind: "generic",
    scopeCeiling: ["cotra.read", "cotra.write"]
  });
  const relay = createRelayServer({
    publicOrigin: origin,
    issuer: ISSUER,
    allowedOrigins: options.allowedOrigins ?? [],
    store,
    pollWaitMs: 300
  });
  await new Promise<void>((resolve) => relay.server.listen(port, "127.0.0.1", resolve));
  const kernelCalls: Array<{ capability: string; remote: RemoteDispatchContext }> = [];
  const controller = new AbortController();
  let uplinkDone: Promise<void> = Promise.resolve();
  if (options.connectDevice) {
    const paired: PairedConnection = {
      principal,
      remoteConnectionId: connection,
      providerKind: "generic",
      clientProfileId: "profile-1",
      clientProfileRevision: 1,
      toolSurfaceProfile: "core",
      scopeCeiling: ["cotra.read", "cotra.write"]
    };
    const dispatcher = createMcpFrameDispatcher({
      defaultWorkspace: "default",
      kernelFactory: (remote) =>
        ({
          sessionId: "stub",
          remote,
          call: async (input: { capability: string }) => {
            kernelCalls.push({ capability: input.capability, remote });
            return {
              version: 1,
              request_id: "r",
              ok: false,
              error: { code: "REMOTE_SESSION_INACTIVE", message: "no active remote session lease" }
            };
          },
          close: () => {},
          pending: new Map(),
          child: {} as never
        }) as unknown as KernelClient
    });
    uplinkDone = runDeviceUplink(
      { relayOrigin: origin, identity: { deviceId, deviceEpoch: 1 }, privateKeyJwkBase64: pair.privateKeyJwkBase64, connections: [paired] },
      { fetch: (url, init) => fetch(url, init), dispatcher, revocations: () => [], signal: controller.signal, sleep: (ms) => new Promise((r) => setTimeout(r, Math.min(ms, 50))) }
    );
    await waitFor(() => relay.hub.isOnline(deviceId), "device channel");
  }
  const harness: Harness = {
    origin,
    relay,
    store,
    key,
    deviceId,
    principal,
    connection,
    kernelCalls,
    stopUplink: () => controller.abort(),
    async close() {
      controller.abort();
      relay.hub.closeChannel(deviceId);
      await uplinkDone;
      relay.server.closeAllConnections();
      await new Promise<void>((resolve) => relay.server.close(() => resolve()));
    },
    token(overrides = {}) {
      const now = Math.floor(Date.now() / 1000);
      const { scopes, ...claimOverrides } = overrides;
      const claims: AccessTokenClaims = {
        iss: ISSUER,
        sub: principal,
        aud: `${origin}/mcp`,
        exp: now + 300,
        nbf: now,
        iat: now,
        jti: "at-" + "1".repeat(31) + String(Math.floor(Math.random() * 10)),
        scope: (scopes ?? ["cotra.read"]).join(" "),
        client_id: CLIENT_ID,
        cotra_connection: connection,
        cotra_device: deviceId,
        cotra_epoch: 1,
        cotra_family: "rf-" + "2".repeat(32),
        ...claimOverrides
      };
      return signAccessToken(claims, key);
    }
  };
  return harness;
}

async function post(
  h: Harness,
  body: unknown,
  headers: Record<string, string> = {}
): Promise<{ status: number; headers: Headers; json: Record<string, unknown> | null; text: string }> {
  const response = await fetch(`${h.origin}/mcp`, {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json, text/event-stream", ...headers },
    body: typeof body === "string" ? body : JSON.stringify(body)
  });
  const text = await response.text();
  let json: Record<string, unknown> | null = null;
  try {
    json = text === "" ? null : (JSON.parse(text) as Record<string, unknown>);
  } catch {
    json = null;
  }
  return { status: response.status, headers: response.headers, json, text };
}

const INIT = {
  jsonrpc: "2.0",
  id: 1,
  method: "initialize",
  params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "relay-e2e", version: "1" } }
};

async function initialize(h: Harness, token: string): Promise<string> {
  const reply = await post(h, INIT, { authorization: `Bearer ${token}` });
  assert.equal(reply.status, 200, reply.text);
  const session = reply.headers.get("mcp-session-id");
  assert.ok(session !== null && session.length >= 43);
  const initialized = await post(
    h,
    { jsonrpc: "2.0", method: "notifications/initialized" },
    { authorization: `Bearer ${token}`, "mcp-session-id": session }
  );
  assert.equal(initialized.status, 202);
  return session;
}

test("an authenticated MCP client reaches the paired device through the outbound-only channel", async () => {
  const h = await startHarness({ connectDevice: true });
  try {
    const token = h.token();
    const session = await initialize(h, token);
    const auth = { authorization: `Bearer ${token}`, "mcp-session-id": session };
    const list = await post(h, { jsonrpc: "2.0", id: 2, method: "tools/list" }, auth);
    assert.equal(list.status, 200, list.text);
    const tools = ((list.json?.result as { tools: Array<{ name: string }> }).tools).map((t) => t.name).sort();
    assert.deepEqual(tools, [...CANONICAL_TOOL_NAMES].sort());
    const call = await post(h, { jsonrpc: "2.0", id: 3, method: "tools/call", params: { name: "fs_read", arguments: { path: "README.md" } } }, auth);
    assert.equal(call.status, 200, call.text);
    assert.ok(call.text.includes("REMOTE_SESSION_INACTIVE"), "the local lease decision reaches the remote client as a typed result");
    assert.equal(h.kernelCalls.length, 1);
    const remote = h.kernelCalls[0]?.remote;
    assert.equal(remote?.principal, h.principal);
    assert.equal(remote?.remoteConnectionId, h.connection);
    assert.equal(remote?.deviceId, h.deviceId);
    assert.deepEqual(remote?.scopes, ["cotra.read"]);
    const get = await fetch(`${h.origin}/mcp`, { headers: { authorization: `Bearer ${token}` } });
    assert.equal(get.status, 405);
    await get.text();
    const del = await fetch(`${h.origin}/mcp`, { method: "DELETE", headers: auth });
    assert.equal(del.status, 204);
    await del.text();
    const after = await post(h, { jsonrpc: "2.0", id: 4, method: "tools/list" }, auth);
    assert.equal(after.status, 404);
  } finally {
    await h.close();
  }
});

test("negative: missing, malformed, expired, wrong-audience, wrong-issuer, revoked, and stale-epoch tokens fail before framing", async () => {
  const h = await startHarness({ connectDevice: true });
  try {
    const missing = await post(h, INIT);
    assert.equal(missing.status, 401);
    const challenge = missing.headers.get("www-authenticate") ?? "";
    assert.ok(challenge.startsWith("Bearer "));
    assert.ok(challenge.includes(`resource_metadata="${h.origin}/.well-known/oauth-protected-resource/mcp"`));
    const now = Math.floor(Date.now() / 1000);
    for (const bad of [
      "not-a-token",
      h.token({ exp: now - 100, iat: now - 200, nbf: now - 200 }),
      h.token({ aud: "https://other.example/mcp" }),
      h.token({ iss: "https://evil.example" }),
      h.token({ cotra_epoch: 2 }),
      h.token({ scope: "cotra.read cotra.admin" })
    ]) {
      const reply = await post(h, INIT, { authorization: `Bearer ${bad}` });
      assert.equal(reply.status, 401, reply.text);
      assert.ok((reply.headers.get("www-authenticate") ?? "").includes('error="invalid_token"'));
    }
    const revokedToken = h.token({ jti: "at-" + "9".repeat(32) });
    h.store.revokeTokenId("at-" + "9".repeat(32));
    assert.equal((await post(h, INIT, { authorization: `Bearer ${revokedToken}` })).status, 401);
    h.store.revokeConnection(h.connection);
    assert.equal((await post(h, INIT, { authorization: `Bearer ${h.token()}` })).status, 401);
    assert.equal(h.kernelCalls.length, 0, "no rejected request reached the device kernel");
  } finally {
    await h.close();
  }
});

test("negative: missing scope and unknown tools are denied at the edge without device dispatch", async () => {
  const h = await startHarness({ connectDevice: true });
  try {
    const token = h.token({ scopes: ["cotra.read"] });
    const session = await initialize(h, token);
    const auth = { authorization: `Bearer ${token}`, "mcp-session-id": session };
    const write = await post(h, { jsonrpc: "2.0", id: 5, method: "tools/call", params: { name: "fs_write", arguments: {} } }, auth);
    assert.equal(write.status, 403);
    assert.ok((write.headers.get("www-authenticate") ?? "").includes('error="insufficient_scope"'));
    const unknown = await post(h, { jsonrpc: "2.0", id: 6, method: "tools/call", params: { name: "run_anything", arguments: {} } }, auth);
    assert.equal(unknown.status, 403);
    assert.ok(unknown.text.includes("TOOL_SURFACE_DENIED"));
    const narrower = await post(h, { jsonrpc: "2.0", id: 7, method: "tools/list" }, { authorization: `Bearer ${h.token({ scopes: ["cotra.write"] })}`, "mcp-session-id": session });
    assert.equal(narrower.status, 403, "a token without the session scopes cannot continue the session");
    assert.equal(h.kernelCalls.length, 0);
  } finally {
    await h.close();
  }
});

test("negative: sessions are bound to their route and bounded per route", async () => {
  const h = await startHarness({ connectDevice: true });
  try {
    const token = h.token();
    const first = await initialize(h, token);
    await initialize(h, token);
    const third = await post(h, INIT, { authorization: `Bearer ${token}` });
    assert.equal(third.status, 429);
    const otherConnection = generateRemoteConnectionId();
    h.store.putConnection({
      principal: h.principal,
      remoteConnectionId: otherConnection,
      deviceId: h.deviceId,
      clientId: CLIENT_ID,
      providerKind: "generic",
      scopeCeiling: ["cotra.read"]
    });
    const foreign = await post(
      h,
      { jsonrpc: "2.0", id: 8, method: "tools/list" },
      { authorization: `Bearer ${h.token({ cotra_connection: otherConnection })}`, "mcp-session-id": first }
    );
    assert.equal(foreign.status, 404, "a session never crosses to another route");
    const missingSession = await post(h, { jsonrpc: "2.0", id: 9, method: "tools/list" }, { authorization: `Bearer ${token}` });
    assert.equal(missingSession.status, 400);
  } finally {
    await h.close();
  }
});

test("negative: an offline device fails closed with DEVICE_OFFLINE and nothing is queued", async () => {
  const h = await startHarness({ connectDevice: false });
  try {
    const reply = await post(h, INIT, { authorization: `Bearer ${h.token()}` });
    assert.equal(reply.status, 503);
    assert.ok(reply.text.includes("DEVICE_OFFLINE"));
    assert.equal(h.relay.hub.pendingCount(), 0);
    assert.equal(h.relay.edge.sessionCount(), 0);
  } finally {
    await h.close();
  }
});

test("negative: malformed protocol shapes, origins, hosts, sizes, and paths are rejected", async () => {
  const h = await startHarness({ connectDevice: true });
  try {
    const auth = { authorization: `Bearer ${h.token()}` };
    assert.equal((await post(h, [INIT], auth)).status, 400, "batches are not accepted");
    assert.equal((await post(h, "{not json", auth)).status, 400);
    assert.equal((await post(h, { jsonrpc: "2.0", id: 1, result: {} }, auth)).status, 400);
    assert.equal((await post(h, INIT, { ...auth, "content-type": "text/plain" })).status, 415);
    assert.equal((await post(h, INIT, { ...auth, origin: "https://evil.example" })).status, 403);
    assert.equal((await post(h, INIT, { ...auth, "mcp-protocol-version": "1999-01-01" })).status, 400);
    const big = { jsonrpc: "2.0", id: 1, method: "ping", params: { pad: "x".repeat(1_200_000) } };
    assert.equal((await post(h, big, auth)).status, 413);
    const wrongHostStatus = await new Promise<number>((resolve, reject) => {
      const url = new URL(`${h.origin}/mcp`);
      const outgoing = httpRequest(
        { host: url.hostname, port: url.port, path: "/mcp", method: "POST", headers: { host: "relay.attacker.example", "content-type": "application/json", ...auth } },
        (response) => {
          response.resume();
          resolve(response.statusCode ?? 0);
        }
      );
      outgoing.on("error", reject);
      outgoing.end("{}");
    });
    assert.equal(wrongHostStatus, 421, "DNS-rebinding style Host values are refused");
    const unknown = await fetch(`${h.origin}/admin`);
    assert.equal(unknown.status, 404);
    await unknown.text();
    const metadata = await fetch(`${h.origin}/.well-known/oauth-protected-resource/mcp`);
    const document = (await metadata.json()) as Record<string, unknown>;
    assert.equal(document.resource, `${h.origin}/mcp`);
    assert.deepEqual(document.authorization_servers, [ISSUER]);
    assert.deepEqual(document.bearer_methods_supported, ["header"]);
    assert.equal(metadata.headers.get("access-control-allow-origin"), null, "no CORS is ever emitted");
  } finally {
    await h.close();
  }
});

test("negative: device channel endpoints refuse unproven devices, bad tokens, and forged responses", async () => {
  const h = await startHarness({ connectDevice: false });
  try {
    const call = async (path: string, body: unknown) => {
      const response = await fetch(`${h.origin}/device/v1/${path}`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
      return { status: response.status, json: (await response.json()) as Record<string, unknown> };
    };
    assert.equal((await call("challenge", { deviceId: generateDeviceId(), epoch: 1 })).status, 401);
    assert.equal((await call("challenge", { deviceId: h.deviceId, epoch: 2 })).status, 401);
    const issued = await call("challenge", { deviceId: h.deviceId, epoch: 1 });
    assert.equal(issued.status, 200);
    const attacker = generateDeviceKeyPair(h.deviceId, Date.now(), 1);
    const forged = signDeviceChallenge(attacker.privateKeyJwkBase64, issued.json.challenge as never);
    assert.equal((await call("session", { response: forged })).status, 401);
    assert.equal((await call("session", { response: forged })).status, 401, "a challenge is one-shot");
    assert.equal((await call("poll", { channelToken: "X".repeat(43) })).status, 401);
    assert.equal((await call("push", { channelToken: "X".repeat(43), frames: [] })).status, 401);
    h.store.revokeDevice(h.deviceId);
    assert.equal((await call("challenge", { deviceId: h.deviceId, epoch: 1 })).status, 401);
  } finally {
    await h.close();
  }
});

test("relay sources carry no generic proxy capability, executor, or tool registration", () => {
  for (const name of readdirSync(join(here, "..", "src"))) {
    if (!name.endsWith(".ts") || name.endsWith(".test.ts")) {
      continue;
    }
    const text = readFileSync(join(here, "..", "src", name), "utf8");
    for (const token of DENIED_RELAY_CAPABILITIES) {
      assert.ok(!text.toLowerCase().includes(token), `${name} must not contain ${token}`);
    }
    for (const forbidden of ["node:child_process", "registerTool(", "WebSocket", "node:net", "eval(", "new Function("]) {
      assert.ok(!text.includes(forbidden), `${name} must not reference ${forbidden}`);
    }
    assert.ok(!/fetch\(/.test(text), `${name} must not make outbound requests`);
  }
});
