/**
 * SG-000057 explicit relay quotas with fail-closed no-billing overflow.
 *
 * Every limit has a configured value and a hard ceiling the configuration
 * cannot exceed. Exhaustion always fails closed with the typed
 * `REMOTE_RATE_LIMITED` failure and a `Retry-After` hint; the relay never
 * scales out, queues durably, or overflows into paid capacity. Counters are
 * in memory and keyed only by opaque identifiers or coarse classes, never by
 * payload content.
 */

export interface QuotaConfig {
  readonly maxDevices: number;
  readonly maxClients: number;
  readonly registrationsPerHour: number;
  readonly authorizeAttemptsPerMinute: number;
  readonly tokenRequestsPerMinute: number;
  readonly mcpRequestsPerMinutePerRoute: number;
  readonly dailyRequestBudget: number;
}

/** Hard ceilings that no configuration may exceed. */
export const QUOTA_CEILINGS: QuotaConfig = {
  maxDevices: 100_000,
  maxClients: 100_000,
  registrationsPerHour: 10_000,
  authorizeAttemptsPerMinute: 600,
  tokenRequestsPerMinute: 6_000,
  mcpRequestsPerMinutePerRoute: 60,
  dailyRequestBudget: 10_000_000
};

/** Conservative defaults suitable for a free-tier or personal relay. */
export const DEFAULT_QUOTAS: QuotaConfig = {
  maxDevices: 1_000,
  maxClients: 1_000,
  registrationsPerHour: 60,
  authorizeAttemptsPerMinute: 30,
  tokenRequestsPerMinute: 120,
  mcpRequestsPerMinutePerRoute: 60,
  dailyRequestBudget: 100_000
};

/** No-billing overflow is not configurable: exhaustion always fails closed. */
export const NO_BILLING_OVERFLOW = true;

export function validateQuotas(input: unknown): QuotaConfig {
  const source = (input ?? {}) as Record<string, unknown>;
  if (typeof source !== "object" || Array.isArray(source)) {
    throw new Error("quotas must be an object");
  }
  for (const key of Object.keys(source)) {
    if (!Object.hasOwn(DEFAULT_QUOTAS, key)) {
      throw new Error(`unknown quota ${key}`);
    }
  }
  const result: Record<string, number> = {};
  for (const [key, fallback] of Object.entries(DEFAULT_QUOTAS)) {
    const value = source[key] ?? fallback;
    const ceiling = QUOTA_CEILINGS[key as keyof QuotaConfig];
    if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 1 || value > ceiling) {
      throw new Error(`quota ${key} must be an integer from 1 to ${ceiling}`);
    }
    result[key] = value;
  }
  return result as unknown as QuotaConfig;
}

export type QuotaDecision = { readonly ok: true } | { readonly ok: false; readonly retryAfterSeconds: number };

class Window {
  private readonly hits = new Map<string, number[]>();

  constructor(
    private readonly windowMs: number,
    private readonly maxKeys = 100_000
  ) {}

  take(key: string, limit: number, now: number): QuotaDecision {
    const kept = (this.hits.get(key) ?? []).filter((at) => now - at < this.windowMs);
    if (kept.length >= limit) {
      this.hits.set(key, kept);
      const oldest = kept[0] ?? now;
      return { ok: false, retryAfterSeconds: Math.max(1, Math.ceil((this.windowMs - (now - oldest)) / 1000)) };
    }
    if (!this.hits.has(key) && this.hits.size >= this.maxKeys) {
      for (const [other, times] of this.hits) {
        if (times.every((at) => now - at >= this.windowMs)) {
          this.hits.delete(other);
        }
      }
      if (this.hits.size >= this.maxKeys) {
        return { ok: false, retryAfterSeconds: 60 };
      }
    }
    kept.push(now);
    this.hits.set(key, kept);
    return { ok: true };
  }
}

export class QuotaGuard {
  private readonly registrations = new Window(60 * 60 * 1000);
  private readonly authorize = new Window(60 * 1000);
  private readonly tokens = new Window(60 * 1000);
  private readonly mcp = new Window(60 * 1000);
  private dayStart = 0;
  private dayCount = 0;

  constructor(
    readonly config: QuotaConfig,
    private readonly clock: () => number = Date.now
  ) {}

  private budget(now: number): QuotaDecision {
    const day = Math.floor(now / 86_400_000);
    if (day !== this.dayStart) {
      this.dayStart = day;
      this.dayCount = 0;
    }
    if (this.dayCount >= this.config.dailyRequestBudget) {
      return { ok: false, retryAfterSeconds: Math.ceil(((day + 1) * 86_400_000 - now) / 1000) };
    }
    this.dayCount += 1;
    return { ok: true };
  }

  registration(): QuotaDecision {
    const now = this.clock();
    const budget = this.budget(now);
    return budget.ok ? this.registrations.take("all", this.config.registrationsPerHour, now) : budget;
  }

  authorizeAttempt(): QuotaDecision {
    const now = this.clock();
    const budget = this.budget(now);
    return budget.ok ? this.authorize.take("all", this.config.authorizeAttemptsPerMinute, now) : budget;
  }

  tokenRequest(): QuotaDecision {
    const now = this.clock();
    const budget = this.budget(now);
    return budget.ok ? this.tokens.take("all", this.config.tokenRequestsPerMinute, now) : budget;
  }

  mcpRequest(remoteConnectionId: string): QuotaDecision {
    const now = this.clock();
    const budget = this.budget(now);
    return budget.ok ? this.mcp.take(remoteConnectionId, this.config.mcpRequestsPerMinutePerRoute, now) : budget;
  }
}
