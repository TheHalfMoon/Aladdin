/**
 * SG-000056 relay side of the SG-000055 outbound-only device channel.
 *
 * Devices connect out to these endpoints: challenge, session, long-poll,
 * and push. A device obtains a channel only by signing a fresh SG-000053
 * challenge with the key registered for its exact identifier and epoch.
 * The hub holds at most a bounded number of request frames per device in
 * memory, never persists them, refuses frames for a device without a live
 * channel, and accepts only well-formed response frames that correlate to a
 * pending request on a session bound to the same device.
 */
import { randomBytes, timingSafeEqual } from "node:crypto";
import { computeRequestDigest } from "@cotra/mcp/dist/device_uplink.js";
import {
  createDeviceChallenge,
  isDeviceId,
  verifyChallengeResponse,
  type ChallengeResponse,
  type DeviceChallenge
} from "@cotra/mcp/dist/device_identity.js";
import {
  RELAY_BOUNDS,
  RELAY_FAILURE_CODES,
  RELAY_PROTOCOL_VERSION,
  type RelayFailureCode
} from "@cotra/mcp/dist/relay_contract.js";
import type { RelayStore } from "./store.js";

export const CHANNEL_POLL_WAIT_MS = 25_000;
/** A device without a poll for this long is offline. */
export const CHANNEL_LIVENESS_MS = 35_000;
export const CHANNEL_MAX_NONCES = 8192;

const OUTBOUND_FIELDS = [
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
  "payload",
  "failure"
] as const;

export interface HubReply {
  readonly status: number;
  readonly json: unknown;
}

export type PendingOutcome =
  | { readonly kind: "response"; readonly payload: string }
  | { readonly kind: "failure"; readonly failure: RelayFailureCode };

interface Channel {
  readonly token: string;
  readonly deviceId: string;
  lastPollMs: number;
  frames: Record<string, unknown>[];
  waiter: ((frames: Record<string, unknown>[]) => void) | null;
}

interface Pending {
  readonly deviceId: string;
  readonly remoteConnectionId: string;
  readonly frame: Record<string, unknown>;
  readonly resolve: (outcome: PendingOutcome) => void;
  readonly timer: NodeJS.Timeout;
  delivered: boolean;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function equalAscii(left: string, right: string): boolean {
  const a = Buffer.from(left, "utf8");
  const b = Buffer.from(right, "utf8");
  return a.length === b.length && timingSafeEqual(a, b);
}

const DENIED: HubReply = { status: 401, json: { error: "device_channel_denied" } };

export class DeviceChannelHub {
  private readonly challenges = new Map<string, DeviceChallenge>();
  private readonly channels = new Map<string, Channel>();
  private readonly byToken = new Map<string, Channel>();
  private readonly pending = new Map<string, Pending>();
  private readonly nonces = new Map<string, number>();

  constructor(
    private readonly store: RelayStore,
    private readonly clock: () => number = Date.now,
    private readonly pollWaitMs: number = CHANNEL_POLL_WAIT_MS
  ) {}

  /** Step 1: issue a fresh challenge for a registered, unrevoked device. */
  challenge(body: unknown): HubReply {
    if (!isPlainObject(body) || typeof body.deviceId !== "string" || !isDeviceId(body.deviceId)) {
      return DENIED;
    }
    const record = this.store.device(body.deviceId);
    if (record === undefined || record.revoked || body.epoch !== record.identity.epoch) {
      return DENIED;
    }
    const challenge = createDeviceChallenge(record.identity.deviceId, record.identity.epoch, this.clock());
    this.challenges.set(record.identity.deviceId, challenge);
    return { status: 200, json: { challenge } };
  }

  /** Step 2: verify the signed challenge once and open one channel per device. */
  session(body: unknown): HubReply {
    if (!isPlainObject(body) || !isPlainObject(body.response)) {
      return DENIED;
    }
    const response = body.response as unknown as ChallengeResponse;
    if (typeof response.deviceId !== "string") {
      return DENIED;
    }
    const challenge = this.challenges.get(response.deviceId);
    const record = this.store.device(response.deviceId);
    this.challenges.delete(response.deviceId);
    if (challenge === undefined || record === undefined || record.revoked) {
      return DENIED;
    }
    const check = verifyChallengeResponse(record.identity, challenge, response, this.clock());
    if (!check.ok) {
      return DENIED;
    }
    this.closeChannel(record.identity.deviceId);
    const channel: Channel = {
      token: randomBytes(32).toString("base64url"),
      deviceId: record.identity.deviceId,
      lastPollMs: this.clock(),
      frames: [],
      waiter: null
    };
    this.channels.set(channel.deviceId, channel);
    this.byToken.set(channel.token, channel);
    return { status: 200, json: { channelToken: channel.token } };
  }

  private channelFor(body: unknown): Channel | null {
    if (!isPlainObject(body) || typeof body.channelToken !== "string") {
      return null;
    }
    const channel = this.byToken.get(body.channelToken);
    if (channel === undefined || !equalAscii(channel.token, body.channelToken)) {
      return null;
    }
    const record = this.store.device(channel.deviceId);
    if (record === undefined || record.revoked) {
      this.closeChannel(channel.deviceId);
      return null;
    }
    return channel;
  }

  /** Step 3: long-poll for request frames (bounded wait, bounded batch). */
  async poll(body: unknown, signal?: AbortSignal): Promise<HubReply> {
    const channel = this.channelFor(body);
    if (channel === null) {
      return DENIED;
    }
    channel.lastPollMs = this.clock();
    channel.waiter?.([]);
    channel.waiter = null;
    const now = this.clock();
    channel.frames = channel.frames.filter((frame) => (frame.expiresAt as number) > now);
    if (channel.frames.length > 0) {
      const ready = channel.frames.splice(0, RELAY_BOUNDS.maxQueuedFrames);
      this.markDelivered(ready);
      return { status: 200, json: { frames: ready } };
    }
    const frames = await new Promise<Record<string, unknown>[]>((resolve) => {
      const timer = setTimeout(() => {
        if (channel.waiter === finish) {
          channel.waiter = null;
        }
        resolve([]);
      }, this.pollWaitMs);
      const finish = (ready: Record<string, unknown>[]): void => {
        clearTimeout(timer);
        resolve(ready);
      };
      channel.waiter = finish;
      signal?.addEventListener("abort", () => {
        if (channel.waiter === finish) {
          channel.waiter = null;
        }
        finish([]);
      });
    });
    channel.lastPollMs = this.clock();
    return { status: 200, json: { frames } };
  }

  /** Step 4: accept response frames that correlate to pending requests. */
  push(body: unknown): HubReply {
    const channel = this.channelFor(body);
    if (channel === null) {
      return DENIED;
    }
    if (!isPlainObject(body) || !Array.isArray(body.frames) || body.frames.length > RELAY_BOUNDS.maxQueuedFrames) {
      return { status: 400, json: { error: "malformed_push" } };
    }
    let accepted = 0;
    for (const frame of body.frames) {
      if (this.acceptResponse(channel, frame)) {
        accepted += 1;
      }
    }
    return { status: 200, json: { accepted } };
  }

  private acceptResponse(channel: Channel, raw: unknown): boolean {
    if (!isPlainObject(raw)) {
      return false;
    }
    const keys = Object.keys(raw);
    if (keys.length !== OUTBOUND_FIELDS.length || !OUTBOUND_FIELDS.every((key) => Object.hasOwn(raw, key))) {
      return false;
    }
    const now = this.clock();
    if (
      raw.protocolVersion !== RELAY_PROTOCOL_VERSION ||
      raw.routeDeviceId !== channel.deviceId ||
      typeof raw.channelAuth !== "string" ||
      !equalAscii(raw.channelAuth, channel.token) ||
      typeof raw.connectionId !== "string" ||
      typeof raw.correlationId !== "string" ||
      typeof raw.remoteConnectionId !== "string" ||
      typeof raw.replayNonce !== "string" ||
      raw.replayNonce.length < 16 ||
      raw.replayNonce.length > 128 ||
      typeof raw.sequence !== "number" ||
      !Number.isSafeInteger(raw.sequence) ||
      typeof raw.expiresAt !== "number" ||
      raw.expiresAt < now ||
      typeof raw.payloadLength !== "number"
    ) {
      return false;
    }
    if (this.nonces.has(raw.replayNonce)) {
      return false;
    }
    const key = `${raw.connectionId}:${raw.correlationId}`;
    const pending = this.pending.get(key);
    if (
      pending === undefined ||
      pending.deviceId !== channel.deviceId ||
      pending.remoteConnectionId !== raw.remoteConnectionId
    ) {
      return false;
    }
    // Responses are correlated one-shot and carry single-use nonces; pushes
    // may arrive concurrently, so response order is not required.
    if (raw.sequence < 1) {
      return false;
    }
    let outcome: PendingOutcome;
    if (raw.messageKind === "mcp_response") {
      if (
        typeof raw.payload !== "string" ||
        raw.failure !== null ||
        Buffer.byteLength(raw.payload, "utf8") !== raw.payloadLength ||
        raw.payloadLength > RELAY_BOUNDS.maxResultBytes
      ) {
        return false;
      }
      outcome = { kind: "response", payload: raw.payload };
    } else if (raw.messageKind === "mcp_error") {
      if (
        raw.payload !== null ||
        raw.payloadLength !== 0 ||
        typeof raw.failure !== "string" ||
        !(RELAY_FAILURE_CODES as readonly string[]).includes(raw.failure)
      ) {
        return false;
      }
      outcome = { kind: "failure", failure: raw.failure as RelayFailureCode };
    } else {
      return false;
    }
    if (this.nonces.size >= CHANNEL_MAX_NONCES) {
      for (const [nonce, expiry] of this.nonces) {
        if (expiry < now) {
          this.nonces.delete(nonce);
        }
      }
      if (this.nonces.size >= CHANNEL_MAX_NONCES) {
        return false;
      }
    }
    this.nonces.set(raw.replayNonce, raw.expiresAt);
    this.pending.delete(key);
    clearTimeout(pending.timer);
    pending.resolve(outcome);
    return true;
  }

  private markDelivered(frames: readonly Record<string, unknown>[]): void {
    for (const frame of frames) {
      const pending = this.pending.get(`${String(frame.connectionId)}:${String(frame.correlationId)}`);
      if (pending !== undefined) {
        pending.delivered = true;
      }
    }
  }

  /**
   * Turn an undelivered request frame into a `cancel` for the same
   * correlation, keeping its sequence so the connection stays contiguous
   * while guaranteeing the request is never executed.
   */
  private cancelQueued(deviceId: string, frame: Record<string, unknown>): boolean {
    const channel = this.channels.get(deviceId);
    if (channel === undefined) {
      return false;
    }
    const index = channel.frames.findIndex(
      (queued) => queued.connectionId === frame.connectionId && queued.correlationId === frame.correlationId
    );
    if (index < 0) {
      return false;
    }
    const envelope = frame.authorizationEnvelope as Record<string, unknown>;
    const now = this.clock();
    channel.frames[index] = {
      ...frame,
      channelAuth: channel.token,
      replayNonce: randomBytes(24).toString("base64url"),
      messageKind: "cancel",
      payloadLength: 0,
      payload: "",
      createdAt: now,
      expiresAt: now + RELAY_BOUNDS.maxFrameLifetimeSeconds * 1000,
      authorizationEnvelope: {
        ...envelope,
        requestDigest: computeRequestDigest({
          principal: String(envelope.principal),
          remoteConnectionId: String(frame.remoteConnectionId),
          connectionId: String(frame.connectionId),
          deviceId: String(frame.routeDeviceId),
          correlationId: String(frame.correlationId),
          sequence: Number(frame.sequence),
          method: "",
          payload: ""
        })
      }
    };
    return true;
  }

  /** Why a request cannot be handed to the device now, or null. */
  refusal(deviceId: string): RelayFailureCode | null {
    const channel = this.channels.get(deviceId);
    if (channel === undefined || !this.isOnline(deviceId)) {
      return "DEVICE_OFFLINE";
    }
    if (channel.frames.length >= RELAY_BOUNDS.maxQueuedFrames) {
      return "REMOTE_RATE_LIMITED";
    }
    return null;
  }

  isOnline(deviceId: string): boolean {
    const channel = this.channels.get(deviceId);
    return channel !== undefined && this.clock() - channel.lastPollMs <= CHANNEL_LIVENESS_MS;
  }

  /**
   * Hand one request frame to an online device and wait for its response.
   * Offline devices fail immediately; nothing is queued for later. Callers
   * check `refusal` and build the frame in the same synchronous step so a
   * refused request never consumes a connection sequence. A request that
   * times out before delivery is converted to a `cancel` and reported as
   * `REMOTE_QUEUE_EXPIRED` (never executed); after delivery its outcome is
   * unknown and reported as `TRANSPORT_UNAVAILABLE`.
   */
  dispatch(
    deviceId: string,
    remoteConnectionId: string,
    frame: Record<string, unknown>,
    timeoutMs: number,
    expectResponse: boolean
  ): Promise<PendingOutcome> {
    const refused = this.refusal(deviceId);
    const channel = this.channels.get(deviceId);
    if (refused !== null || channel === undefined) {
      return Promise.resolve({ kind: "failure", failure: refused ?? "DEVICE_OFFLINE" });
    }
    const signed = { ...frame, channelAuth: channel.token };
    const delivered = new Promise<PendingOutcome>((resolve) => {
      if (!expectResponse) {
        resolve({ kind: "response", payload: "" });
        return;
      }
      const key = `${String(frame.connectionId)}:${String(frame.correlationId)}`;
      const timer = setTimeout(() => {
        const entry = this.pending.get(key);
        this.pending.delete(key);
        if (entry !== undefined && !entry.delivered && this.cancelQueued(deviceId, signed)) {
          resolve({ kind: "failure", failure: "REMOTE_QUEUE_EXPIRED" });
          return;
        }
        resolve({ kind: "failure", failure: "TRANSPORT_UNAVAILABLE" });
      }, timeoutMs);
      this.pending.set(key, { deviceId, remoteConnectionId, frame: signed, resolve, timer, delivered: false });
    });
    if (channel.waiter !== null) {
      const waiter = channel.waiter;
      channel.waiter = null;
      this.markDelivered([signed]);
      waiter([signed]);
    } else {
      channel.frames.push(signed);
    }
    return delivered;
  }

  pendingCount(): number {
    return this.pending.size;
  }

  closeChannel(deviceId: string): void {
    const channel = this.channels.get(deviceId);
    if (channel === undefined) {
      return;
    }
    this.channels.delete(deviceId);
    this.byToken.delete(channel.token);
    channel.waiter?.([]);
    channel.waiter = null;
    channel.frames = [];
    for (const [key, pending] of this.pending) {
      if (pending.deviceId === deviceId) {
        this.pending.delete(key);
        clearTimeout(pending.timer);
        // Undelivered requests were never executed; delivered ones have an
        // unknown outcome and are never reported as not executed.
        pending.resolve({ kind: "failure", failure: pending.delivered ? "TRANSPORT_UNAVAILABLE" : "DEVICE_OFFLINE" });
      }
    }
  }
}
