/**
 * SG-000057 self-host configuration.
 *
 * The relay is configured by one strict JSON file. Unknown fields fail, the
 * public origin must be https (or an exact loopback http origin for a relay
 * on the same machine), the listener defaults to loopback so a TLS reverse
 * proxy terminates public traffic, and quotas are bounded by hard ceilings.
 */
import { readFileSync } from "node:fs";
import { isIssuerIdentifier } from "@qdral/mcp/dist/oauth_authorization.js";
import { validateQuotas, type QuotaConfig } from "./quotas.js";

export interface RelayConfig {
  readonly publicOrigin: string;
  readonly listenHost: string;
  readonly listenPort: number;
  readonly stateDir: string;
  readonly allowedOrigins: readonly string[];
  readonly quotas: QuotaConfig;
  /** SG-000067 OpenAI domain-verification token, served as plain text. */
  readonly openaiAppsChallenge: string | null;
}

const FIELDS = new Set(["publicOrigin", "listenHost", "listenPort", "stateDir", "allowedOrigins", "quotas", "openaiAppsChallenge"]);

/**
 * A provider domain-verification token is a public, provider-issued value.
 * It is restricted to URL-safe characters so it can never inject markup or
 * headers into the plain-text response.
 */
export const APPS_CHALLENGE_PATTERN = /^[A-Za-z0-9._~-]{8,512}$/;

export function parseRelayConfig(text: string): RelayConfig {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    throw new Error("relay configuration is not valid JSON");
  }
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    throw new Error("relay configuration must be an object");
  }
  const config = raw as Record<string, unknown>;
  for (const key of Object.keys(config)) {
    if (!FIELDS.has(key)) {
      throw new Error(`unknown relay configuration field ${key}`);
    }
  }
  const origin = config.publicOrigin;
  if (typeof origin !== "string" || !isIssuerIdentifier(origin) || new URL(origin).pathname !== "/") {
    throw new Error("publicOrigin must be an https origin (or exact loopback http) with no path");
  }
  const listenHost = config.listenHost ?? "127.0.0.1";
  if (typeof listenHost !== "string" || !/^[0-9a-fA-F.:]+$/.test(listenHost)) {
    throw new Error("listenHost must be an IP address literal");
  }
  const listenPort = config.listenPort ?? 8787;
  if (typeof listenPort !== "number" || !Number.isInteger(listenPort) || listenPort < 1 || listenPort > 65535) {
    throw new Error("listenPort must be from 1 to 65535");
  }
  if (typeof config.stateDir !== "string" || config.stateDir.length === 0) {
    throw new Error("stateDir is required");
  }
  const allowedOrigins = config.allowedOrigins ?? [];
  if (!Array.isArray(allowedOrigins) || !allowedOrigins.every((entry) => typeof entry === "string" && /^https:\/\/[^/]+$/.test(entry))) {
    throw new Error("allowedOrigins must be a list of https origins");
  }
  const challenge = config.openaiAppsChallenge ?? null;
  if (challenge !== null && (typeof challenge !== "string" || !APPS_CHALLENGE_PATTERN.test(challenge))) {
    throw new Error("openaiAppsChallenge must be 8 to 512 URL-safe characters");
  }
  return {
    publicOrigin: origin,
    listenHost,
    listenPort,
    stateDir: config.stateDir,
    allowedOrigins: allowedOrigins as string[],
    quotas: validateQuotas(config.quotas),
    openaiAppsChallenge: challenge
  };
}

export function loadRelayConfig(path: string): RelayConfig {
  return parseRelayConfig(readFileSync(path, "utf8"));
}
