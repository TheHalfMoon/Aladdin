/**
 * SG-000055 protected uplink state loading.
 *
 * Reads the SG-000053 device key record and the local route revocation list
 * from protected Quntal state. A missing or malformed device key fails
 * closed with PAIRING_REQUIRED. A missing revocation list means no
 * revocation; an unreadable or malformed one fails closed as an emergency
 * revocation so no route stays usable on corrupt state.
 */
import { existsSync, readFileSync } from "node:fs";
import {
  emergencyRevoke,
  isDeviceId,
  type DeviceKeyPair,
  type RevocationKind,
  type RevocationRecord
} from "./device_identity.js";

const REVOCATION_KINDS: readonly RevocationKind[] = [
  "hard_device",
  "principal_wide",
  "all_route",
  "emergency"
];

export function loadDeviceKey(path: string): DeviceKeyPair {
  let parsed: unknown;
  try {
    parsed = JSON.parse(readFileSync(path, "utf8"));
  } catch {
    throw new Error("PAIRING_REQUIRED: device key is missing or unreadable");
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error("PAIRING_REQUIRED: device key record is malformed");
  }
  const record = parsed as Record<string, unknown>;
  if (
    typeof record.deviceId !== "string" ||
    !isDeviceId(record.deviceId) ||
    typeof record.publicKeyJwkBase64 !== "string" ||
    typeof record.privateKeyJwkBase64 !== "string" ||
    record.privateKeyJwkBase64.length === 0 ||
    typeof record.createdAtMs !== "number" ||
    !Number.isSafeInteger(record.createdAtMs) ||
    typeof record.epoch !== "number" ||
    !Number.isSafeInteger(record.epoch) ||
    record.epoch < 1
  ) {
    throw new Error("PAIRING_REQUIRED: device key record is malformed");
  }
  return {
    deviceId: record.deviceId,
    publicKeyJwkBase64: record.publicKeyJwkBase64,
    privateKeyJwkBase64: record.privateKeyJwkBase64,
    createdAtMs: record.createdAtMs,
    epoch: record.epoch
  };
}

function isRevocationRecord(value: unknown): value is RevocationRecord {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const record = value as Record<string, unknown>;
  return (
    typeof record.kind === "string" &&
    (REVOCATION_KINDS as readonly string[]).includes(record.kind) &&
    typeof record.createdAtMs === "number" &&
    typeof record.epoch === "number" &&
    (record.deviceId === null || typeof record.deviceId === "string") &&
    (record.principal === null || typeof record.principal === "string") &&
    typeof record.reason === "string"
  );
}

export function loadRevocations(path: string): readonly RevocationRecord[] {
  if (!existsSync(path)) {
    return [];
  }
  try {
    const parsed: unknown = JSON.parse(readFileSync(path, "utf8"));
    if (Array.isArray(parsed) && parsed.length <= 1024 && parsed.every(isRevocationRecord)) {
      return parsed;
    }
  } catch {
    // fall through to fail closed
  }
  return [emergencyRevoke(0, Date.now(), "revocation list unreadable; failing closed")];
}
