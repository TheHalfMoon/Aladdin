import assert from "node:assert/strict";
import test from "node:test";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { createServer as createNetServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  createMcpFrameDispatcher,
  runDeviceUplink,
  type PairedConnection
} from "@qdral/mcp/dist/device_uplink.js";
import type { KernelClient } from "@qdral/mcp/dist/kernel.js";
import { generatePkceVerifier, pkceS256Challenge } from "@qdral/mcp/dist/oauth_authorization.js";
import { enableRemote, pairRemote } from "@qdral/mcp/dist/remote_enrollment.js";
import { loadDeviceKey } from "@qdral/mcp/dist/uplink_state.js";
import { CANONICAL_TOOL_NAMES } from "@qdral/mcp/dist/server.js";
import { REMOTE_TOOL_NAMES } from "@qdral/mcp/dist/tool_contract.js";
import { parseRelayConfig } from "./config.js";
import { FileRelayStore, RelayStateError, STATE_FILE } from "./file_store.js";
import { DEFAULT_QUOTAS, QUOTA_CEILINGS, validateQuotas } from "./quotas.js";
import { createRelayServer, type RelayServer } from "./server.js";

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

interface Relay {
  readonly origin: string;
  readonly port: number;
  readonly stateDir: string;
  readonly relay: RelayServer;
  readonly store: FileRelayStore;
  close(): Promise<void>;
}

async function startRelay(stateDir: string, port?: number, quotas = DEFAULT_QUOTAS): Promise<Relay> {
  const listenPort = port ?? (await freePort());
  const origin = `http://127.0.0.1:${listenPort}`;
  const store = new FileRelayStore(stateDir);
  const relay = createRelayServer({ publicOrigin: origin, issuer: origin, allowedOrigins: [], store, quotas, pollWaitMs: 300 });
  await new Promise<void>((resolve) => relay.server.listen(listenPort, "127.0.0.1", resolve));
  return {
    origin,
    port: listenPort,
    stateDir,
    relay,
    store,
    async close() {
      relay.server.closeAllConnections();
      await new Promise<void>((resolve) => relay.server.close(() => resolve()));
    }
  };
}

const fetchImpl = (url: string, init: RequestInit) => fetch(url, init);

async function form(origin: string, path: string, fields: Record<string, string>): Promise<{ status: number; json: Record<string, unknown> }> {
  const response = await fetch(origin + path, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams(fields).toString(),
    redirect: "manual"
  });
  const text = await response.text();
  const isJson = (response.headers.get("content-type") ?? "").includes("application/json");
  return { status: response.status, json: isJson && text !== "" ? (JSON.parse(text) as Record<string, unknown>) : {} };
}

interface Linked {
  readonly code: string;
  readonly verifier: string;
  readonly clientId: string;
  readonly redirect: string;
  readonly accessToken: string;
  readonly refreshToken: string;
  readonly paired: PairedConnection;
  readonly devicePaths: { key: string; uplink: string };
}

/** Run the complete standards-based linking flow against a self-hosted relay. */
async function link(r: Relay, approve = true): Promise<Linked | { denied: string }> {
  const deviceDir = mkdtempSync(join(tmpdir(), "qdral-device-"));
  const devicePaths = { key: join(deviceDir, "device_key.json"), uplink: join(deviceDir, "uplink.json") };
  await enableRemote({ relayOrigin: r.origin, defaultWorkspace: "default", deviceKeyPath: devicePaths.key, uplinkPath: devicePaths.uplink, fetch: fetchImpl, nowMs: Date.now() });
  const redirect = "http://127.0.0.1:9/callback";
  const registered = await fetch(`${r.origin}/oauth/register`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ redirect_uris: [redirect], client_name: "Test client", token_endpoint_auth_method: "none", scope: "qdral.read qdral.write" })
  });
  assert.equal(registered.status, 201);
  const clientId = ((await registered.json()) as { client_id: string }).client_id;
  const verifier = generatePkceVerifier();
  const state = "state-" + "s".repeat(24);
  const authorize = new URL(`${r.origin}/oauth/authorize`);
  for (const [key, value] of Object.entries({
    response_type: "code",
    client_id: clientId,
    redirect_uri: redirect,
    code_challenge: pkceS256Challenge(verifier),
    code_challenge_method: "S256",
    resource: `${r.origin}/mcp`,
    scope: "qdral.read",
    state
  })) {
    authorize.searchParams.set(key, value);
  }
  const pageResponse = await fetch(authorize, { redirect: "manual" });
  assert.equal(pageResponse.status, 200);
  const pageText = await pageResponse.text();
  assert.ok(pageResponse.headers.get("content-security-policy")?.includes("default-src 'none'"));
  const transaction = /name="transaction" value="([^"]+)"/.exec(pageText)?.[1] ?? "";
  assert.ok(transaction.length >= 43);
  let codeEntered: Promise<void> = Promise.resolve();
  const paired = await pairRemote({
    deviceKeyPath: devicePaths.key,
    uplinkPath: devicePaths.uplink,
    fetch: fetchImpl,
    now: Date.now,
    sleep: (ms) => new Promise((resolve) => setTimeout(resolve, Math.min(ms, 50))),
    pollIntervalMs: 50,
    display: (code) => {
      codeEntered = form(r.origin, "/oauth/authorize", { transaction, pairing_code: code }).then((reply) => assert.equal(reply.status, 200));
    },
    confirm: async (request) => {
      await codeEntered;
      assert.equal(request.clientName, "Test client");
      assert.equal(request.redirectOrigin, "http://127.0.0.1:9");
      assert.deepEqual(request.scopes, ["qdral.read"]);
      return approve;
    }
  });
  const status = await fetch(`${r.origin}/oauth/authorize/status?transaction=${encodeURIComponent(transaction)}`, { redirect: "manual" });
  assert.equal(status.status, 302);
  const location = new URL(status.headers.get("location") ?? "");
  assert.equal(location.searchParams.get("state"), state);
  assert.equal(location.searchParams.get("iss"), r.origin);
  if (!approve) {
    assert.equal(paired, null);
    return { denied: location.searchParams.get("error") ?? "" };
  }
  assert.ok(paired !== null);
  const code = location.searchParams.get("code") ?? "";
  const tokens = await form(r.origin, "/oauth/token", {
    grant_type: "authorization_code",
    code,
    client_id: clientId,
    redirect_uri: redirect,
    code_verifier: verifier,
    resource: `${r.origin}/mcp`
  });
  assert.equal(tokens.status, 200, JSON.stringify(tokens.json));
  return {
    code,
    verifier,
    clientId,
    redirect,
    accessToken: String(tokens.json.access_token),
    refreshToken: String(tokens.json.refresh_token),
    paired,
    devicePaths
  };
}

function startUplink(r: Relay, linked: Linked): { stop: () => Promise<void>; calls: string[] } {
  const calls: string[] = [];
  const pair = loadDeviceKey(linked.devicePaths.key);
  const uplink = JSON.parse(readFileSync(linked.devicePaths.uplink, "utf8")) as { connections: PairedConnection[] };
  const controller = new AbortController();
  const done = runDeviceUplink(
    { relayOrigin: r.origin, identity: { deviceId: pair.deviceId, deviceEpoch: pair.epoch }, privateKeyJwkBase64: pair.privateKeyJwkBase64, connections: uplink.connections },
    {
      fetch: fetchImpl,
      dispatcher: createMcpFrameDispatcher({
        defaultWorkspace: "default",
        kernelFactory: (remote) =>
          ({
            sessionId: "stub",
            remote,
            call: async (input: { capability: string }) => {
              calls.push(input.capability);
              return { version: 1, request_id: "r", ok: false, error: { code: "REMOTE_SESSION_INACTIVE", message: "no lease" } };
            },
            close: () => {},
            pending: new Map(),
            child: {} as never
          }) as unknown as KernelClient
      }),
      revocations: () => [],
      signal: controller.signal,
      sleep: (ms) => new Promise((resolve) => setTimeout(resolve, Math.min(ms, 50)))
    }
  );
  return {
    calls,
    async stop() {
      controller.abort();
      r.relay.hub.closeChannel(pair.deviceId);
      await done;
    }
  };
}

async function mcp(r: Relay, token: string, body: unknown, session?: string): Promise<Response> {
  return fetch(`${r.origin}/mcp`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      accept: "application/json, text/event-stream",
      authorization: `Bearer ${token}`,
      ...(session === undefined ? {} : { "mcp-session-id": session })
    },
    body: JSON.stringify(body)
  });
}

test("a standards OAuth client links through device pairing and reaches the device via a self-hosted relay", async () => {
  const stateDir = mkdtempSync(join(tmpdir(), "qdral-relay-state-"));
  const r = await startRelay(stateDir);
  let uplink: { stop: () => Promise<void>; calls: string[] } | null = null;
  try {
    const metadata = (await (await fetch(`${r.origin}/.well-known/oauth-authorization-server`)).json()) as Record<string, unknown>;
    assert.equal(metadata.issuer, r.origin);
    assert.deepEqual(metadata.code_challenge_methods_supported, ["S256"]);
    const linked = await link(r);
    assert.ok(!("denied" in linked));
    if ("denied" in linked) {
      return;
    }
    uplink = startUplink(r, linked);
    const deviceId = loadDeviceKey(linked.devicePaths.key).deviceId;
    await waitFor(() => r.relay.hub.isOnline(deviceId), "device channel");
    const init = await mcp(r, linked.accessToken, { jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "t", version: "1" } } });
    assert.equal(init.status, 200, await init.clone().text());
    const session = init.headers.get("mcp-session-id") ?? "";
    await init.text();
    assert.equal((await mcp(r, linked.accessToken, { jsonrpc: "2.0", method: "notifications/initialized" }, session)).status, 202);
    const list = await mcp(r, linked.accessToken, { jsonrpc: "2.0", id: 2, method: "tools/list" }, session);
    const tools = ((await list.json()) as { result: { tools: Array<{ name: string }> } }).result.tools.map((t) => t.name).sort();
    // SG-000065: remote discovery lists exactly the core profile.
    assert.deepEqual(tools, [...REMOTE_TOOL_NAMES].sort());
    assert.ok(CANONICAL_TOOL_NAMES.length > REMOTE_TOOL_NAMES.length);

    // Refresh requires a fresh device proof over the live channel and rotates.
    const refreshed = await form(r.origin, "/oauth/token", { grant_type: "refresh_token", refresh_token: linked.refreshToken, client_id: linked.clientId, resource: `${r.origin}/mcp` });
    assert.equal(refreshed.status, 200, JSON.stringify(refreshed.json));
    const rotated = String(refreshed.json.refresh_token);
    assert.notEqual(rotated, linked.refreshToken);
    // Replaying the rotated-away refresh token revokes the whole family.
    const replay = await form(r.origin, "/oauth/token", { grant_type: "refresh_token", refresh_token: linked.refreshToken, client_id: linked.clientId, resource: `${r.origin}/mcp` });
    assert.equal(replay.status, 400);
    assert.equal(replay.json.error_description, "refresh_replayed");
    const afterReplay = await form(r.origin, "/oauth/token", { grant_type: "refresh_token", refresh_token: rotated, client_id: linked.clientId, resource: `${r.origin}/mcp` });
    assert.equal(afterReplay.status, 400);
    assert.equal(afterReplay.json.error_description, "family_revoked");

    // Revoking an access token takes effect at the edge.
    assert.equal((await form(r.origin, "/oauth/revoke", { token: linked.accessToken, client_id: linked.clientId })).status, 200);
    assert.equal((await mcp(r, linked.accessToken, { jsonrpc: "2.0", id: 3, method: "tools/list" }, session)).status, 401);
  } finally {
    await uplink?.stop();
    await r.close();
  }
});

test("negative: authorization code replay is refused and revokes every token issued from the code", async () => {
  const r = await startRelay(mkdtempSync(join(tmpdir(), "qdral-relay-state-")));
  try {
    const linked = await link(r);
    if ("denied" in linked) {
      throw new Error("unexpected denial");
    }
    const replay = await form(r.origin, "/oauth/token", {
      grant_type: "authorization_code",
      code: linked.code,
      client_id: linked.clientId,
      redirect_uri: linked.redirect,
      code_verifier: linked.verifier,
      resource: `${r.origin}/mcp`
    });
    assert.equal(replay.status, 400);
    assert.equal(replay.json.error_description, "code_replayed");
    const refresh = await form(r.origin, "/oauth/token", { grant_type: "refresh_token", refresh_token: linked.refreshToken, client_id: linked.clientId, resource: `${r.origin}/mcp` });
    assert.equal(refresh.status, 400, "the family issued from a replayed code is revoked");
    const edge = await mcp(r, linked.accessToken, { jsonrpc: "2.0", id: 1, method: "initialize", params: {} });
    assert.equal(edge.status, 401, "access tokens from that family are rejected");
    await edge.text();
  } finally {
    await r.close();
  }
});

test("negative: a stolen refresh token cannot renew while the bound device is offline", async () => {
  const r = await startRelay(mkdtempSync(join(tmpdir(), "qdral-relay-state-")));
  try {
    const linked = await link(r);
    if ("denied" in linked) {
      throw new Error("unexpected denial");
    }
    const started = Date.now();
    const offline = await form(r.origin, "/oauth/token", { grant_type: "refresh_token", refresh_token: linked.refreshToken, client_id: linked.clientId, resource: `${r.origin}/mcp` });
    assert.equal(offline.status, 400);
    assert.equal(offline.json.error_description, "device_proof_missing");
    assert.ok(Date.now() - started < 5_000, "an offline device is refused immediately");
  } finally {
    await r.close();
  }
});

test("negative: the device can decline, and the client receives access_denied", async () => {
  const r = await startRelay(mkdtempSync(join(tmpdir(), "qdral-relay-state-")));
  try {
    const result = await link(r, false);
    assert.deepEqual(result, { denied: "access_denied" });
  } finally {
    await r.close();
  }
});

test("negative: authorization requests and pairing codes are strictly validated", async () => {
  const r = await startRelay(mkdtempSync(join(tmpdir(), "qdral-relay-state-")));
  try {
    const unknown = await fetch(`${r.origin}/oauth/authorize?client_id=nope&redirect_uri=https://evil.example/cb`, { redirect: "manual" });
    assert.equal(unknown.status, 400, "unknown clients never redirect");
    await unknown.text();
    const registered = await fetch(`${r.origin}/oauth/register`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ redirect_uris: ["https://client.example/cb"] }) });
    const clientId = ((await registered.json()) as { client_id: string }).client_id;
    for (const bad of [
      { redirect_uris: ["http://client.example/cb"] },
      { redirect_uris: ["https://client.example/cb"], token_endpoint_auth_method: "client_secret_basic" },
      { redirect_uris: ["https://client.example/cb"], grant_types: ["password"] },
      { redirect_uris: ["https://client.example/cb"], scope: "qdral.root" },
      { redirect_uris: ["https://client.example/cb"], jwks_uri: "https://evil.example" }
    ]) {
      const reply = await fetch(`${r.origin}/oauth/register`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(bad) });
      assert.equal(reply.status, 400, JSON.stringify(bad));
      await reply.text();
    }
    const authorize = new URL(`${r.origin}/oauth/authorize`);
    for (const [key, value] of Object.entries({
      response_type: "code",
      client_id: clientId,
      redirect_uri: "https://client.example/cb",
      code_challenge: pkceS256Challenge(generatePkceVerifier()),
      code_challenge_method: "S256",
      resource: `${r.origin}/mcp`,
      scope: "qdral.read",
      state: "s".repeat(32)
    })) {
      authorize.searchParams.set(key, value);
    }
    const wrongResource = new URL(authorize);
    wrongResource.searchParams.set("resource", "https://other.example/mcp");
    const redirected = await fetch(wrongResource, { redirect: "manual" });
    assert.equal(redirected.status, 302);
    assert.equal(new URL(redirected.headers.get("location") ?? "").searchParams.get("error"), "invalid_request");
    const pageText = await (await fetch(authorize, { redirect: "manual" })).text();
    const transaction = /name="transaction" value="([^"]+)"/.exec(pageText)?.[1] ?? "";
    for (let attempt = 1; attempt <= 5; attempt += 1) {
      const reply = await fetch(`${r.origin}/oauth/authorize`, {
        method: "POST",
        headers: { "content-type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams({ transaction, pairing_code: "0".repeat(64) }).toString()
      });
      assert.equal(reply.status, 400);
      await reply.text();
    }
    const locked = await fetch(`${r.origin}/oauth/authorize`, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({ transaction, pairing_code: "0".repeat(64) }).toString()
    });
    assert.ok((await locked.text()).includes("Too many attempts"));
    const token = await form(r.origin, "/oauth/token", { grant_type: "password", client_id: clientId, username: "u", password: "p" });
    assert.equal(token.json.error, "unsupported_grant_type");
    const forged = await form(r.origin, "/oauth/token", { grant_type: "authorization_code", code: "A".repeat(43), client_id: clientId });
    assert.equal(forged.json.error, "invalid_grant");
  } finally {
    await r.close();
  }
});

test("relay state survives restart, keeps its signing key, and refuses corrupt state", async () => {
  const stateDir = mkdtempSync(join(tmpdir(), "qdral-relay-state-"));
  const first = await startRelay(stateDir);
  const linked = await link(first);
  if ("denied" in linked) {
    throw new Error("unexpected denial");
  }
  const port = first.port;
  await first.close();
  const second = await startRelay(stateDir, port);
  try {
    // Same signing key, client, route, and device: the token authenticates and
    // the request fails only because the device is offline.
    let reply: Response;
    try {
      reply = await mcp(second, linked.accessToken, { jsonrpc: "2.0", id: 1, method: "initialize", params: {} });
    } catch {
      // A pooled keep-alive socket to the stopped server is reset once.
      reply = await mcp(second, linked.accessToken, { jsonrpc: "2.0", id: 1, method: "initialize", params: {} });
    }
    assert.equal(reply.status, 503);
    assert.ok((await reply.text()).includes("DEVICE_OFFLINE"));
  } finally {
    await second.close();
  }
  const path = join(stateDir, STATE_FILE);
  const text = readFileSync(path, "utf8");
  writeFileSync(path, text.replace('"revoked":false', '"revoked":true'));
  assert.throws(() => new FileRelayStore(stateDir), RelayStateError);
  writeFileSync(path, "{");
  assert.throws(() => new FileRelayStore(stateDir), RelayStateError);
});

test("quotas are explicit, bounded by hard ceilings, and fail closed without overflow", async () => {
  assert.throws(() => validateQuotas({ dailyRequestBudget: QUOTA_CEILINGS.dailyRequestBudget + 1 }));
  assert.throws(() => validateQuotas({ autoscale: true }));
  assert.throws(() => validateQuotas({ registrationsPerHour: 0 }));
  const r = await startRelay(mkdtempSync(join(tmpdir(), "qdral-relay-state-")), undefined, { ...DEFAULT_QUOTAS, registrationsPerHour: 1 });
  try {
    const register = () =>
      fetch(`${r.origin}/oauth/register`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ redirect_uris: ["https://client.example/cb"] }) });
    assert.equal((await register()).status, 201);
    const limited = await register();
    assert.equal(limited.status, 429);
    assert.ok(Number(limited.headers.get("retry-after")) >= 1);
    assert.ok((await limited.text()).includes("REMOTE_RATE_LIMITED"));
  } finally {
    await r.close();
  }
  const budget = await startRelay(mkdtempSync(join(tmpdir(), "qdral-relay-state-")), undefined, { ...DEFAULT_QUOTAS, dailyRequestBudget: 2 });
  try {
    const register = () =>
      fetch(`${budget.origin}/oauth/register`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ redirect_uris: ["https://client.example/cb"] }) });
    assert.equal((await register()).status, 201);
    assert.equal((await register()).status, 201);
    assert.equal((await register()).status, 429, "the daily budget fails closed instead of overflowing");
  } finally {
    await budget.close();
  }
});

test("self-host configuration is strict", () => {
  const ok = parseRelayConfig(JSON.stringify({ publicOrigin: "https://relay.example.com", stateDir: "/var/lib/qdral-relay" }));
  assert.equal(ok.listenHost, "127.0.0.1");
  assert.equal(ok.listenPort, 8787);
  assert.deepEqual(ok.quotas, DEFAULT_QUOTAS);
  for (const bad of [
    { publicOrigin: "http://relay.example.com", stateDir: "/s" },
    { publicOrigin: "https://relay.example.com/path", stateDir: "/s" },
    { publicOrigin: "https://relay.example.com" },
    { publicOrigin: "https://relay.example.com", stateDir: "/s", billing: "auto" },
    { publicOrigin: "https://relay.example.com", stateDir: "/s", listenHost: "example.com" },
    { publicOrigin: "https://relay.example.com", stateDir: "/s", allowedOrigins: ["*"] }
  ]) {
    assert.throws(() => parseRelayConfig(JSON.stringify(bad)), JSON.stringify(bad));
  }
});
