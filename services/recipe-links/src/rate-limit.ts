import { createHmac, randomBytes } from "node:crypto";

const HOUR = 60 * 60 * 1000;
const DAY = 24 * HOUR;
export interface RateDecision { allowed: boolean; retryAfterSeconds: number }

/** Only daily salted hashes and expiring timestamps survive a request. Never raw IPs. */
export class PublishLimiter {
  private readonly salts = new Map<number, Buffer>();
  private readonly buckets = new Map<string, number[]>();
  constructor(
    private readonly limit = 20,
    private readonly now: () => number = Date.now,
    private readonly maxSources = 10_000,
  ) {
    if (!Number.isSafeInteger(limit) || limit < 1 || !Number.isSafeInteger(maxSources) || maxSources < 1) throw new Error("Invalid rate limit");
  }

  take(source: string): RateDecision {
    const now = this.now();
    const day = Math.floor(now / DAY);
    for (const [key, times] of this.buckets) {
      const live = times.filter(time => time > now - HOUR);
      if (live.length) this.buckets.set(key, live);
      else this.buckets.delete(key);
    }
    const liveDays = new Set([...this.buckets.keys()].map(key => Number(key.split(":")[0])));
    for (const saltDay of this.salts.keys()) if (saltDay !== day && !liveDays.has(saltDay)) this.salts.delete(saltDay);
    if (!this.salts.has(day)) this.salts.set(day, randomBytes(32));
    const keys = [...this.salts].map(([saltDay, salt]) => ({
      day: saltDay, key: `${saltDay}:${createHmac("sha256", salt).update(source).digest("hex")}`,
    }));
    const times = keys.flatMap(({ key }) => this.buckets.get(key) ?? []);
    if (times.length >= this.limit) return { allowed: false, retryAfterSeconds: Math.max(1, Math.ceil((Math.min(...times) + HOUR - now) / 1000)) };
    const key = keys.find(entry => entry.day === day)!.key;
    if (!this.buckets.has(key) && this.buckets.size >= this.maxSources) return { allowed: false, retryAfterSeconds: 60 };
    this.buckets.set(key, [...(this.buckets.get(key) ?? []), now]);
    return { allowed: true, retryAfterSeconds: 0 };
  }
}
