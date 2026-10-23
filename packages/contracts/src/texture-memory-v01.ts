/** Platform decoded texture memory. Values describe the texture category, not the overall SDK rank. */
export type TexturePlatform = "pc" | "android";
export type TextureRank = "excellent" | "good" | "medium" | "poor" | "very_poor" | "unknown";
export type TextureTargetRank = Exclude<TextureRank, "very_poor" | "unknown">;
export interface TextureInfo {
  readonly id: string; readonly path: string; readonly width: number; readonly height: number;
  readonly format: string; readonly mipmap: boolean; readonly faces: number; readonly estimatedBytes: number;
  readonly known: boolean; readonly maxSize: number; readonly editable: boolean; readonly reason: string;
}
export interface TextureMemoryReport {
  readonly platform: TexturePlatform; readonly textures: readonly TextureInfo[]; readonly totalBytes: number;
  readonly complete: boolean; readonly rank: TextureRank;
}
export interface TextureImportChange { readonly path: string; readonly platform: TexturePlatform; readonly maxSize: number; readonly format: string }
export interface TexturePlanEntry { readonly current: TextureInfo; readonly proposed: TextureImportChange; readonly estimatedBytes: number }
export interface TextureMemoryPlan {
  readonly platform: TexturePlatform; readonly targetRank: TextureTargetRank; readonly current: TextureMemoryReport;
  readonly entries: readonly TexturePlanEntry[]; readonly estimatedTotalBytes: number; readonly estimatedRank: TextureRank; readonly reachesTarget: boolean;
}
export interface AmfTextureMemoryPlan {
  readonly schemaVersion: "0.1"; readonly runId: string; readonly planDigest: string; readonly plan: TextureMemoryPlan;
}
const record = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const uint = (v: unknown, max = Number.MAX_SAFE_INTEGER): v is number => typeof v === "number" && Number.isSafeInteger(v) && v >= 0 && v <= max;
const text = (v: unknown, max: number): v is string => typeof v === "string" && v.length <= max;
const pot = (v: number) => v > 0 && (v & (v - 1)) === 0;
export const isTexturePlatform = (v: unknown): v is TexturePlatform => v === "pc" || v === "android";
export const isTextureTargetRank = (v: unknown): v is TextureTargetRank => ["excellent", "good", "medium", "poor"].includes(String(v));
const rank = (v: unknown): v is TextureRank => isTextureTargetRank(v) || v === "very_poor" || v === "unknown";
export function isTexturePath(v: unknown): v is string {
  return text(v, 4096) && v.startsWith("Assets/") && !/[\\:<>"|?*\u0000-\u001f\u007f]/u.test(v)
    && !v.toLowerCase().endsWith(".meta") && v.split("/").every(p => p !== "" && p !== "." && p !== ".." && !/[. ]$/u.test(p));
}
export function textureRank(bytes: number, platform: TexturePlatform): TextureRank {
  const index = (platform === "pc" ? [40, 75, 110, 150] : [10, 18, 25, 40]).findIndex(limit => bytes <= limit * 1024 * 1024);
  return (["excellent", "good", "medium", "poor"] as const)[index] ?? "very_poor";
}
function info(v: unknown): v is TextureInfo {
  return record(v) && text(v.id, 256) && v.id.length > 0 && text(v.path, 4096) && text(v.format, 64) && text(v.reason, 128)
    && uint(v.width, 16384) && uint(v.height, 16384) && uint(v.maxSize, 16384) && [1, 6].includes(Number(v.faces))
    && typeof v.mipmap === "boolean" && typeof v.known === "boolean" && typeof v.editable === "boolean" && uint(v.estimatedBytes)
    && (v.known ? v.width > 0 && v.height > 0 && v.estimatedBytes > 0 && v.format.length > 0 : v.estimatedBytes === 0)
    && (!v.editable || (v.known && isTexturePath(v.path) && v.faces === 1 && pot(v.width) && pot(v.height) && pot(v.maxSize) && v.maxSize >= 32));
}
export function isTextureMemoryReport(v: unknown): v is TextureMemoryReport {
  if (!record(v) || !isTexturePlatform(v.platform) || !Array.isArray(v.textures) || v.textures.length > 2048 || !v.textures.every(info)
    || !uint(v.totalBytes) || typeof v.complete !== "boolean" || !rank(v.rank)) return false;
  return new Set(v.textures.map(t => t.id)).size === v.textures.length && v.textures.every(t => t.known) === v.complete
    && v.totalBytes === v.textures.reduce((sum, t) => sum + t.estimatedBytes, 0) && v.rank === (v.complete ? textureRank(v.totalBytes, v.platform) : "unknown");
}
export function isTextureMemoryFacts(v: unknown): v is readonly TextureMemoryReport[] {
  return Array.isArray(v) && (v.length === 0 || (v.length === 2 && v.every(isTextureMemoryReport) && v[0]?.platform === "pc" && v[1]?.platform === "android"));
}
export function isAmfTextureMemoryPlan(v: unknown): v is AmfTextureMemoryPlan {
  if (!record(v) || v.schemaVersion !== "0.1" || !text(v.runId, 100) || !text(v.planDigest, 64) || !/^[a-f0-9]{64}$/u.test(v.planDigest) || !record(v.plan)) return false;
  const p = v.plan;
  if (!isTexturePlatform(p.platform) || !isTextureTargetRank(p.targetRank) || !isTextureMemoryReport(p.current) || p.current.platform !== p.platform
    || !Array.isArray(p.entries) || p.entries.length > 2048 || !uint(p.estimatedTotalBytes) || !rank(p.estimatedRank) || typeof p.reachesTarget !== "boolean") return false;
  return p.entries.every(e => record(e) && info(e.current) && e.current.editable && record(e.proposed) && isTexturePath(e.proposed.path)
    && e.proposed.path === e.current.path && e.proposed.platform === p.platform && uint(e.proposed.maxSize, 16384) && pot(e.proposed.maxSize)
    && e.proposed.maxSize >= 32 && (e.proposed.maxSize >= 512 || e.proposed.maxSize === e.current.maxSize)
    && e.proposed.maxSize <= e.current.maxSize && text(e.proposed.format, 64) && uint(e.estimatedBytes)
    && e.estimatedBytes < e.current.estimatedBytes)
    && new Set(p.entries.map(e => e.proposed.path)).size === p.entries.length
    && p.estimatedTotalBytes === p.current.totalBytes - p.entries.reduce((sum, e) => sum + e.current.estimatedBytes - e.estimatedBytes, 0)
    && p.estimatedRank === (p.current.complete ? textureRank(p.estimatedTotalBytes, p.platform) : "unknown")
    && p.reachesTarget === (p.current.complete && p.estimatedTotalBytes <= (p.platform === "pc" ? [40, 75, 110, 150] : [10, 18, 25, 40])[["excellent", "good", "medium", "poor"].indexOf(p.targetRank)]! * 1024 * 1024);
}
