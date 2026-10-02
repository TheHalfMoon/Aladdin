/**
 * SG-000056 relay HTTP server composition.
 *
 * Routes: RFC 9728 protected-resource metadata, the public `/mcp` edge, and
 * the four device channel endpoints. Every body is read with a hard byte
 * cap, unknown paths return 404, the Host header must match the configured
 * public origin, no CORS headers are ever emitted, and logs carry route and
 * status classes only.
 */
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import { buildProtectedResourceMetadata } from "@cotra/mcp/dist/oauth_authorization.js";
import { RELAY_BOUNDS } from "@cotra/mcp/dist/relay_contract.js";
import { DeviceChannelHub, type HubReply } from "./device_channel.js";
import { McpEdge, type EdgeConfig } from "./edge.js";
import type { RelayStore } from "./store.js";

/** `/mcp` body cap: one frame payload plus JSON framing slack. */
export const MCP_BODY_LIMIT = RELAY_BOUNDS.maxFrameBytes + 65_536;
/** Push body cap: the queued-frame bound of full frames. */
export const PUSH_BODY_LIMIT = RELAY_BOUNDS.maxQueuedFrames * (RELAY_BOUNDS.maxFrameBytes + 8_192) + 65_536;
export const SMALL_BODY_LIMIT = 16_384;

export interface RelayServerConfig extends EdgeConfig {
  readonly store: RelayStore;
  readonly clock?: () => number;
  readonly pollWaitMs?: number;
  readonly log?: (event: string) => void;
}

export interface RelayServer {
  readonly server: Server;
  readonly hub: DeviceChannelHub;
  readonly edge: McpEdge;
}

class BodyTooLarge extends Error {}

function readBody(request: IncomingMessage, limit: number): Promise<string> {
  return new Promise((resolve, reject) => {
    const declared = Number(request.headers["content-length"] ?? "0");
    if (Number.isFinite(declared) && declared > limit) {
      reject(new BodyTooLarge());
      return;
    }
    const chunks: Buffer[] = [];
    let total = 0;
    request.on("data", (chunk: Buffer) => {
      total += chunk.byteLength;
      if (total > limit) {
        reject(new BodyTooLarge());
        request.destroy();
        return;
      }
      chunks.push(chunk);
    });
    request.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    request.on("error", reject);
  });
}

function send(response: ServerResponse, status: number, headers: Record<string, string>, body: string): void {
  response.writeHead(status, {
    "cache-control": "no-store",
    "x-content-type-options": "nosniff",
    ...headers,
    "content-length": String(Buffer.byteLength(body, "utf8"))
  });
  response.end(body);
}

function sendJson(response: ServerResponse, reply: HubReply): void {
  send(response, reply.status, { "content-type": "application/json" }, JSON.stringify(reply.json));
}

function headerMap(request: IncomingMessage): Record<string, string | undefined> {
  const map: Record<string, string | undefined> = {};
  for (const [name, value] of Object.entries(request.headers)) {
    map[name] = Array.isArray(value) ? value.join(", ") : value;
  }
  return map;
}

export function createRelayServer(config: RelayServerConfig): RelayServer {
  const clock = config.clock ?? Date.now;
  const log = config.log ?? (() => {});
  const hub = new DeviceChannelHub(config.store, clock, config.pollWaitMs);
  const edge = new McpEdge(config, config.store, hub, clock);
  const metadata = JSON.stringify(buildProtectedResourceMetadata(edge.resource, config.issuer));
  const publicHost = new URL(config.publicOrigin).host;

  const server = createServer(async (request, response) => {
    const path = (request.url ?? "/").split("?")[0] ?? "/";
    try {
      if (request.headers.host !== publicHost) {
        log("host_rejected");
        send(response, 421, { "content-type": "application/json" }, JSON.stringify({ error: "misdirected_request" }));
        return;
      }
      if (
        request.method === "GET" &&
        (path === "/.well-known/oauth-protected-resource" || path === "/.well-known/oauth-protected-resource/mcp")
      ) {
        send(response, 200, { "content-type": "application/json" }, metadata);
        return;
      }
      if (path === "/mcp") {
        const body = request.method === "POST" ? await readBody(request, MCP_BODY_LIMIT) : "";
        const reply = await edge.handle({ method: request.method ?? "", headers: headerMap(request), body });
        log(`mcp_${reply.status}`);
        send(response, reply.status, reply.headers, reply.body);
        return;
      }
      if (request.method === "POST" && path.startsWith("/device/v1/")) {
        const limit = path === "/device/v1/push" ? PUSH_BODY_LIMIT : SMALL_BODY_LIMIT;
        let parsed: unknown;
        try {
          parsed = JSON.parse(await readBody(request, limit));
        } catch (error) {
          if (error instanceof BodyTooLarge) {
            throw error;
          }
          sendJson(response, { status: 400, json: { error: "malformed" } });
          return;
        }
        let reply: HubReply;
        switch (path) {
          case "/device/v1/challenge":
            reply = hub.challenge(parsed);
            break;
          case "/device/v1/session":
            reply = hub.session(parsed);
            break;
          case "/device/v1/poll": {
            const abort = new AbortController();
            response.on("close", () => abort.abort());
            reply = await hub.poll(parsed, abort.signal);
            break;
          }
          case "/device/v1/push":
            reply = hub.push(parsed);
            break;
          default:
            reply = { status: 404, json: { error: "not_found" } };
        }
        log(`device_${path.slice("/device/v1/".length)}_${reply.status}`);
        sendJson(response, reply);
        return;
      }
      send(response, 404, { "content-type": "application/json" }, JSON.stringify({ error: "not_found" }));
    } catch (error) {
      if (error instanceof BodyTooLarge) {
        log("body_too_large");
        if (!response.headersSent) {
          send(response, 413, { "content-type": "application/json" }, JSON.stringify({ error: "payload_too_large" }));
        }
        return;
      }
      log("internal_error");
      if (!response.headersSent) {
        send(response, 500, { "content-type": "application/json" }, JSON.stringify({ error: "internal" }));
      }
    }
  });
  server.headersTimeout = 10_000;
  server.requestTimeout = 70_000;
  server.keepAliveTimeout = 5_000;
  server.maxHeadersCount = 64;
  return { server, hub, edge };
}
