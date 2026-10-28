import { randomBytes, timingSafeEqual } from "node:crypto";
import { chmod, mkdir, open, readFile, rename, stat, unlink } from "node:fs/promises";
import { join } from "node:path";
import { MAX_BODY_BYTES, normalizeCode, shareCode } from "./json.js";

export interface ShareStore {
  get(code: string): Promise<string | null>;
  publish(code: string, canonical: string, tokenHash: string): Promise<{ created: boolean }>;
  withdraw(code: string, tokenHash: string): Promise<boolean>;
}
export class CodeCollision extends Error {}
export class CapacityExceeded extends Error {}
interface Record { version: 1; canonical: string; tokenHashes: string[] }
const MAX_PUBLISHERS = 4096;
const MAX_RECORD_BYTES = MAX_BODY_BYTES + MAX_PUBLISHERS * 68 + 1024;

/** One writer process; each code's mutations serialize across awaited disk operations. */
abstract class RecordStore implements ShareStore {
  private readonly pending = new Map<string, Promise<unknown>>();
  protected abstract read(code: string): Promise<Record | null>;
  protected abstract write(code: string, record: Record): Promise<void>;
  protected abstract remove(code: string): Promise<void>;

  private async mutate<T>(code: string, action: () => Promise<T>): Promise<T> {
    if (normalizeCode(code) !== code) throw new Error("Invalid storage key");
    const next = (this.pending.get(code) ?? Promise.resolve()).catch(() => {}).then(action);
    this.pending.set(code, next);
    try { return await next; }
    finally { if (this.pending.get(code) === next) this.pending.delete(code); }
  }

  async get(code: string): Promise<string | null> {
    if (normalizeCode(code) !== code) throw new Error("Invalid storage key");
    return (await this.read(code))?.canonical ?? null;
  }

  async publish(code: string, canonical: string, tokenHash: string): Promise<{ created: boolean }> {
    return this.mutate(code, async () => {
      const record = await this.read(code);
      if (record && record.canonical !== canonical) throw new CodeCollision();
      if (record && record.tokenHashes.length >= MAX_PUBLISHERS) throw new CapacityExceeded();
      if (record?.tokenHashes.includes(tokenHash)) throw new Error("Duplicate credential");
      await this.write(code, { version: 1, canonical, tokenHashes: [...(record?.tokenHashes ?? []), tokenHash] });
      return { created: record === null };
    });
  }

  async withdraw(code: string, tokenHash: string): Promise<boolean> {
    return this.mutate(code, async () => {
      const record = await this.read(code);
      if (!record) return false;
      const target = record.tokenHashes.findIndex(hash => timingSafeEqual(Buffer.from(hash, "hex"), Buffer.from(tokenHash, "hex")));
      if (target < 0) return false;
      const tokenHashes = record.tokenHashes.filter((_, index) => index !== target);
      if (tokenHashes.length === 0) await this.remove(code);
      else await this.write(code, { ...record, tokenHashes });
      return true;
    });
  }
}

export class MemoryStore extends RecordStore {
  private readonly records = new Map<string, Record>();
  protected async read(code: string): Promise<Record | null> { return this.records.get(code) ?? null; }
  protected async write(code: string, record: Record): Promise<void> { this.records.set(code, record); }
  protected async remove(code: string): Promise<void> { this.records.delete(code); }
}

export class DirectoryStore extends RecordStore {
  private readonly ready: Promise<void>;
  constructor(private readonly directory: string) {
    super();
    this.ready = mkdir(directory, { recursive: true, mode: 0o700 }).then(() => chmod(directory, 0o700));
  }
  async initialize(): Promise<void> { await this.ready; }
  private path(code: string): string { return join(this.directory, `${code}.json`); }

  protected async read(code: string): Promise<Record | null> {
    await this.ready;
    try {
      if ((await stat(this.path(code))).size > MAX_RECORD_BYTES) throw new Error("Corrupt record");
      const value = JSON.parse(await readFile(this.path(code), "utf8")) as Record;
      if (value.version !== 1 || typeof value.canonical !== "string" || Buffer.byteLength(value.canonical) > MAX_BODY_BYTES
        || shareCode(value.canonical) !== code || !Array.isArray(value.tokenHashes) || value.tokenHashes.length < 1
        || value.tokenHashes.length > MAX_PUBLISHERS || value.tokenHashes.some(hash => typeof hash !== "string" || !/^[a-f0-9]{64}$/u.test(hash))) {
        throw new Error("Corrupt record");
      }
      return value;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
      throw error;
    }
  }

  protected async write(code: string, record: Record): Promise<void> {
    await this.ready;
    const temporary = join(this.directory, `.${code}.${randomBytes(12).toString("hex")}.tmp`);
    try {
      const file = await open(temporary, "wx", 0o600);
      try {
        await file.writeFile(JSON.stringify(record), "utf8");
        await file.sync();
      } finally { await file.close(); }
      await rename(temporary, this.path(code));
    }
    finally { await unlink(temporary).catch(error => { if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error; }); }
  }

  protected async remove(code: string): Promise<void> { await unlink(this.path(code)); }
}
