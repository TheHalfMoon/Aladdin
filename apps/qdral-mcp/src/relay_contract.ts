/**
 * Frozen P15 relay protocol vocabulary (SG-000052, design-first).
 *
 * This module is pure data plus pure validation. It performs no I/O, opens
 * no sockets, imports no network modules, registers no MCP tools, and holds
 * no authority. It freezes the vocabulary from
 * `docs/security/RELAY_PROTOCOL_CONTRACT.md` so contract tests pin it
 * before any SG-000053 through SG-000057 implementation grain merges.
 *
 * TypeScript identifiers use camelCase. The canonical wire vocabulary uses
 * the snake_case spellings recorded in the contract document; the mapping
 * is asserted by `relay-contract.test.ts`. Transport identity strings stay
 * out of tool shapes per the transport contract suite.
 */

export const RELAY_PROTOCOL_VERSION = "qdral-relay/1";

export const RELAY_FRAME_KINDS = [
  "mcp_request",
  "mcp_response",
  "mcp_error",
  "cancel",
  "heartbeat"
] as const;

export type RelayFrameKind = (typeof RELAY_FRAME_KINDS)[number];

/** Required request and response envelope fields (wire order is irrelevant). */
export const RELAY_ENVELOPE_FIELDS = [
  "protocolVersion",
  "routeDeviceId",
  "remoteConnectionId",
  "connectionId",
  "sequence",
  "replayNonce",
  "correlationId",
  "messageKind",
  "payloadLength",
  "createdAt",
  "expiresAt",
  "channelAuth",
  "authorizationEnvelope"
] as const;

export type RelayEnvelopeField = (typeof RELAY_ENVELOPE_FIELDS)[number];

/** Required bindings inside the device-verifiable authorization envelope. */
export const RELAY_AUTHORIZATION_ENVELOPE_FIELDS = [
  "principal",
  "connection",
  "device",
  "requestDigest",
  "epoch",
  "sessionContext"
] as const;

export type RelayAuthorizationEnvelopeField =
  (typeof RELAY_AUTHORIZATION_ENVELOPE_FIELDS)[number];

/**
 * Frozen typed-failure vocabulary. These codes remain distinct from local
 * `CAPABILITY_DENIED`, `WORKSPACE_DENIED`, and approval failures.
 */
export const RELAY_FAILURE_CODES = [
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
] as const;

export type RelayFailureCode = (typeof RELAY_FAILURE_CODES)[number];

/**
 * Relay capabilities that are explicitly denied. These tokens must never
 * appear as capabilities, frame kinds, or payload shapes in relay sources.
 */
export const DENIED_RELAY_CAPABILITIES = [
  "tcp_forward",
  "socks",
  "http_connect",
  "http_forward",
  "websocket_tunnel",
  "destination_forward",
  "remote_shell",
  "generic_dispatch",
  "schema_fetch_execute",
  "hidden_subcommand"
] as const;

export type DeniedRelayCapability = (typeof DENIED_RELAY_CAPABILITIES)[number];

/** Frozen hard bounds from the relay protocol contract. */
export const RELAY_BOUNDS = {
  maxFrameBytes: 1048576,
  maxResultBytes: 1048576,
  maxConcurrentRequestsPerConnection: 4,
  maxConnectionsPerDevice: 2,
  maxQueuedFrames: 16,
  reconnectGraceSeconds: 60,
  queueLifetimeSeconds: 60,
  maxFrameLifetimeSeconds: 120,
  heartbeatSeconds: 30,
  idleChannelSuspendSeconds: 120,
  maxRequestsPerMinutePerDevice: 60,
  maxApprovalPromptsPerMinute: 6,
  maxAuthFailuresPerMinute: 5,
  maxPairingAttemptsPerCode: 5,
  logRetentionDays: 30,
  /** Cross-reference to REMOTE_SESSION_AUTHORIZATION.md section 5. */
  remoteSessionLeaseMaxMinutes: 15
} as const;

export function isRelayFailureCode(code: string): code is RelayFailureCode {
  return (RELAY_FAILURE_CODES as readonly string[]).includes(code);
}

export function isDeniedRelayCapability(token: string): boolean {
  return (DENIED_RELAY_CAPABILITIES as readonly string[]).includes(token);
}

/** Returns the required envelope fields absent from a candidate envelope. */
export function missingEnvelopeFields(
  candidate: Record<string, unknown>
): RelayEnvelopeField[] {
  return (RELAY_ENVELOPE_FIELDS as readonly string[]).filter(
    (field) => candidate[field] === undefined || candidate[field] === null
  ) as RelayEnvelopeField[];
}

/** Returns the required authorization bindings absent from a candidate. */
export function missingAuthorizationEnvelopeFields(
  candidate: Record<string, unknown>
): RelayAuthorizationEnvelopeField[] {
  return (RELAY_AUTHORIZATION_ENVELOPE_FIELDS as readonly string[]).filter(
    (field) => candidate[field] === undefined || candidate[field] === null
  ) as RelayAuthorizationEnvelopeField[];
}

export function isPayloadWithinRelayBound(bytes: number): boolean {
  return Number.isInteger(bytes) && bytes >= 0 && bytes <= RELAY_BOUNDS.maxFrameBytes;
}
