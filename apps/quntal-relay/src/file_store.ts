/**
 * SG-000057 durable relay state.
 *
 * One JSON document in the relay state directory, written atomically
 * (temporary file plus rename) with owner-only permissions where the
 * platform supports them, protected by a SHA-256 integrity checksum, and
 * bounded in size. A missing file starts empty; a corrupt, tampered, or
 * oversized file makes the relay refuse to start rather than silently
 * forgetting revocations. The state holds route identifiers, device public
 * keys, client registrations, refresh-family hashes, revocations, and the
 * relay's own token signing key. It never holds MCP payloads, tool
 * arguments, results, bearer tokens, or device private keys.
 */
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { DeviceIdentity, RevocationRecord } from "@quntal/mcp/dist/device_identity.js";
import {
  generateTokenSigningKey,
  verificationKeyOf,
  type ClientRegistration,
  type RefreshFamily,
  type TokenSigningKey,
  type TokenVerificationKey
} from "@quntal/mcp/dist/oauth_authorization.js";
import type { RelayConnectionRecord, RelayDeviceRecord, RelayStore } from "./store.js";

export const STATE_SCHEMA = "quntal-relay-state/1";
export const STATE_FILE = "relay-state.json";
export const MAX_STATE_BYTES = 64 * 1024 * 1024;
export const STATE_CEILINGS = {
  devices: 100_000,
  connections: 200_000,
  clients: 100_000,
  families: 200_000,
  revokedTokenIds: 200_000,
  revokedFamilyIds: 400_000,
  revocations: 100_000
} as const;

export interface StoredClient extends ClientRegistration {
  readonly clientName: string;
  readonly createdAtMs: number;
}

interface StateBody {
  schema: string;
  signingKey: TokenSigningKey;
  devices: Array<{ identity: DeviceIdentity; revoked: boolean; registeredAtMs: number }>;
  connections: Array<RelayConnectionRecord & { createdAtMs: number }>;
  clients: StoredClient[];
  families: RefreshFamily[];
  revokedTokenIds: Array<{ tokenId: string; expiresAtMs: number }>;
  revokedFamilyIds: string[];
  revocations: RevocationRecord[];
}

interface StateFile extends StateBody {
  checksum: string;
}

function checksum(body: StateBody): string {
  return createHash("sha256").update(STATE_SCHEMA).update(JSON.stringify(body)).digest("hex");
}

export class RelayStateError extends Error {}

export class FileRelayStore implements RelayStore {
  private readonly path: string;
  private body: StateBody;

  constructor(stateDir: string, private readonly clock: () => number = Date.now) {
    this.path = join(stateDir, STATE_FILE);
    mkdirSync(stateDir, { recursive: true, mode: 0o700 });
    if (existsSync(this.path)) {
      this.body = FileRelayStore.load(this.path);
    } else {
      this.body = {
        schema: STATE_SCHEMA,
        signingKey: generateTokenSigningKey(),
        devices: [],
        connections: [],
        clients: [],
        families: [],
        revokedTokenIds: [],
        revokedFamilyIds: [],
        revocations: []
      };
      this.persist();
    }
  }

  private static load(path: string): StateBody {
    if (statSync(path).size > MAX_STATE_BYTES) {
      throw new RelayStateError("relay state exceeds its size bound; refusing to start");
    }
    let parsed: StateFile;
    try {
      parsed = JSON.parse(readFileSync(path, "utf8")) as StateFile;
    } catch {
      throw new RelayStateError("relay state is not valid JSON; refusing to start");
    }
    const { checksum: stored, ...body } = parsed;
    if (body.schema !== STATE_SCHEMA || typeof stored !== "string" || stored !== checksum(body)) {
      throw new RelayStateError("relay state failed its integrity check; refusing to start");
    }
    for (const [name, ceiling] of Object.entries(STATE_CEILINGS)) {
      const list = (body as unknown as Record<string, unknown[]>)[name];
      if (!Array.isArray(list) || list.length > ceiling) {
        throw new RelayStateError(`relay state ${name} is malformed or over its ceiling; refusing to start`);
      }
    }
    return body;
  }

  private persist(): void {
    const file: StateFile = { ...this.body, checksum: checksum(this.body) };
    const text = JSON.stringify(file);
    if (Buffer.byteLength(text, "utf8") > MAX_STATE_BYTES) {
      throw new RelayStateError("relay state would exceed its size bound");
    }
    const temp = `${this.path}.tmp-${process.pid}-${this.clock()}`;
    mkdirSync(dirname(this.path), { recursive: true, mode: 0o700 });
    writeFileSync(temp, text, { mode: 0o600 });
    try {
      renameSync(temp, this.path);
    } catch (error) {
      rmSync(temp, { force: true });
      throw error;
    }
  }

  private within(name: keyof typeof STATE_CEILINGS, length: number): void {
    if (length > STATE_CEILINGS[name]) {
      throw new RelayStateError(`relay state ${name} ceiling reached`);
    }
  }

  // ---- signing key -------------------------------------------------------

  signingKey(): TokenSigningKey {
    return this.body.signingKey;
  }

  verificationKeys(): readonly TokenVerificationKey[] {
    return [verificationKeyOf(this.body.signingKey)];
  }

  // ---- devices -----------------------------------------------------------

  device(deviceId: string): RelayDeviceRecord | undefined {
    const record = this.body.devices.find((entry) => entry.identity.deviceId === deviceId);
    return record === undefined ? undefined : { identity: record.identity, revoked: record.revoked };
  }

  deviceCount(): number {
    return this.body.devices.length;
  }

  /** Register a device public key; an existing identifier must keep its key. */
  registerDevice(identity: DeviceIdentity): "created" | "unchanged" | "conflict" | "revoked" {
    const existing = this.body.devices.find((entry) => entry.identity.deviceId === identity.deviceId);
    if (existing !== undefined) {
      if (existing.revoked) {
        return "revoked";
      }
      return existing.identity.publicKeyJwkBase64 === identity.publicKeyJwkBase64 &&
        existing.identity.epoch === identity.epoch
        ? "unchanged"
        : "conflict";
    }
    this.within("devices", this.body.devices.length + 1);
    this.body.devices.push({ identity, revoked: false, registeredAtMs: this.clock() });
    this.persist();
    return "created";
  }

  /** Revoke a device and delete its refresh families and route metadata. */
  revokeDevice(deviceId: string): void {
    const record = this.body.devices.find((entry) => entry.identity.deviceId === deviceId);
    if (record === undefined) {
      return;
    }
    record.revoked = true;
    for (const connection of this.body.connections) {
      if (connection.deviceId === deviceId) {
        (connection as { revoked: boolean }).revoked = true;
      }
    }
    this.deleteFamilies((family) => family.binding.deviceId === deviceId);
    this.persist();
  }

  deviceEpochs(): ReadonlyMap<string, number> {
    const epochs = new Map<string, number>();
    for (const entry of this.body.devices) {
      if (!entry.revoked) {
        epochs.set(entry.identity.deviceId, entry.identity.epoch);
      }
    }
    return epochs;
  }

  // ---- connections -------------------------------------------------------

  connection(remoteConnectionId: string): RelayConnectionRecord | undefined {
    return this.body.connections.find((entry) => entry.remoteConnectionId === remoteConnectionId);
  }

  putConnection(record: Omit<RelayConnectionRecord, "revoked">): void {
    this.within("connections", this.body.connections.length + 1);
    this.body.connections.push({ ...record, revoked: false, createdAtMs: this.clock() });
    this.persist();
  }

  revokeConnection(remoteConnectionId: string): void {
    const record = this.body.connections.find((entry) => entry.remoteConnectionId === remoteConnectionId);
    if (record === undefined) {
      return;
    }
    (record as { revoked: boolean }).revoked = true;
    this.deleteFamilies((family) => family.binding.connection === remoteConnectionId);
    this.persist();
  }

  // ---- clients -----------------------------------------------------------

  client(clientId: string): StoredClient | undefined {
    return this.body.clients.find((entry) => entry.clientId === clientId);
  }

  clients(): ReadonlyMap<string, ClientRegistration> {
    return new Map(this.body.clients.map((entry) => [entry.clientId, entry]));
  }

  clientCount(): number {
    return this.body.clients.length;
  }

  putClient(client: StoredClient): void {
    this.within("clients", this.body.clients.length + 1);
    this.body.clients.push(client);
    this.persist();
  }

  // ---- refresh families --------------------------------------------------

  family(familyId: string): RefreshFamily | undefined {
    return this.body.families.find((entry) => entry.familyId === familyId);
  }

  saveFamily(family: RefreshFamily): void {
    const index = this.body.families.findIndex((entry) => entry.familyId === family.familyId);
    if (index >= 0) {
      this.body.families[index] = family;
    } else {
      this.within("families", this.body.families.length + 1);
      this.body.families.push(family);
    }
    if (family.revoked && !this.body.revokedFamilyIds.includes(family.familyId)) {
      this.within("revokedFamilyIds", this.body.revokedFamilyIds.length + 1);
      this.body.revokedFamilyIds.push(family.familyId);
    }
    this.pruneFamilies();
    this.persist();
  }

  private deleteFamilies(predicate: (family: RefreshFamily) => boolean): void {
    for (const family of this.body.families) {
      if (predicate(family) && !this.body.revokedFamilyIds.includes(family.familyId)) {
        this.within("revokedFamilyIds", this.body.revokedFamilyIds.length + 1);
        this.body.revokedFamilyIds.push(family.familyId);
      }
    }
    this.body.families = this.body.families.filter((family) => !predicate(family));
  }

  private pruneFamilies(): void {
    const now = this.clock();
    this.body.families = this.body.families.filter(
      (family) => !family.revoked && family.absoluteExpiresAtMs > now
    );
  }

  // ---- revocation --------------------------------------------------------

  revokeTokenId(tokenId: string, expiresAtMs: number): void {
    const now = this.clock();
    this.body.revokedTokenIds = this.body.revokedTokenIds.filter((entry) => entry.expiresAtMs > now);
    if (!this.body.revokedTokenIds.some((entry) => entry.tokenId === tokenId)) {
      this.within("revokedTokenIds", this.body.revokedTokenIds.length + 1);
      this.body.revokedTokenIds.push({ tokenId, expiresAtMs });
    }
    this.persist();
  }

  revokeFamilyId(familyId: string): void {
    const family = this.family(familyId);
    if (family !== undefined) {
      this.saveFamily({ ...family, revoked: true });
      return;
    }
    if (!this.body.revokedFamilyIds.includes(familyId)) {
      this.within("revokedFamilyIds", this.body.revokedFamilyIds.length + 1);
      this.body.revokedFamilyIds.push(familyId);
      this.persist();
    }
  }

  addRevocation(record: RevocationRecord): void {
    this.within("revocations", this.body.revocations.length + 1);
    this.body.revocations.push(record);
    this.persist();
  }

  revokedTokenIds(): ReadonlySet<string> {
    return new Set(this.body.revokedTokenIds.map((entry) => entry.tokenId));
  }

  revokedFamilyIds(): ReadonlySet<string> {
    return new Set(this.body.revokedFamilyIds);
  }

  revocations(): readonly RevocationRecord[] {
    return this.body.revocations;
  }
}
