import type { UpdateCheckResultV1, UpdateCheckStateV1 } from "@vua/contracts";

/**
 * 版本检测(2026-09-19 用户裁决:默认开启、设置可关;仅只读探测——
 * 下载/应用更新属 Phase C,单独立提案,本模块不承载):
 * - 比对源 = GitHub latest release;URL 与渲染层 app/app-meta.ts 的
 *   repoUrl 同源(electron 边界不可导入渲染层模块,此处常量须与其保持同步);
 * - 诚实口径:网络异常/HTTP 非 2xx/负载不可解析/缺 tag_name 一律
 *   check-failed,绝不猜态、绝不回退缓存值;
 * - 版本比较是纯三段数字比较(去 v 前缀);不可解析的版本串比较结果
 *   按「无法证明更高」= up-to-date 之外不升级,落 check-failed。
 */

/** 与 app/app-meta.ts repoUrl 同源(旧仓公开归档;迁仓后两边同步更新) */
export const LATEST_RELEASE_URL = "https://api.github.com/repos/VUA-Project/VUA/releases/latest";

/** 三段数字版本比较:a>b 正,a<b 负,相等 0;不可解析返回 null(不猜测) */
export function compareVersions(a: string, b: string): number | null {
  const parse = (text: string): number[] | null => {
    const trimmed = text.trim().replace(/^v/i, "");
    const parts = trimmed.split(".");
    if (parts.length === 0 || parts.length > 4) return null;
    const numbers: number[] = [];
    for (const part of parts) {
      if (!/^\d+$/.test(part)) return null;
      numbers.push(Number.parseInt(part, 10));
    }
    return numbers;
  };
  const pa = parse(a);
  const pb = parse(b);
  if (pa === null || pb === null) return null;
  const length = Math.max(pa.length, pb.length);
  for (let index = 0; index < length; index += 1) {
    const diff = (pa[index] ?? 0) - (pb[index] ?? 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

export interface LatestReleasePayload {
  readonly tag_name?: unknown;
  readonly html_url?: unknown;
}

/** 注入式 fetcher(测试无网络):返回解析后的 JSON 负载;失败抛错 */
export type ReleaseFetcher = (url: string) => Promise<unknown>;

/**
 * 检测最新发布:任何失败路径恒落 check-failed(不抛出)——版本检测是
 * 装饰性信号,失败如实呈现,绝不阻断启动或猜造「已最新」。
 */
export async function checkLatestRelease(
  currentVersion: string,
  fetchJson: ReleaseFetcher,
  checkedAt: string = new Date().toISOString(),
): Promise<UpdateCheckResultV1> {
  const failed = (): UpdateCheckResultV1 => ({
    schemaVersion: 1,
    state: "check-failed",
    currentVersion,
    latestVersion: null,
    releaseUrl: null,
    checkedAt,
  });
  let payload: unknown;
  try {
    payload = await fetchJson(LATEST_RELEASE_URL);
  } catch {
    return failed();
  }
  if (payload === null || typeof payload !== "object" || Array.isArray(payload)) return failed();
  const record = payload as Record<string, unknown>;
  if (typeof record.tag_name !== "string" || record.tag_name.length === 0) return failed();
  const latestVersion = record.tag_name;
  const releaseUrl = typeof record.html_url === "string" && record.html_url.length > 0
    ? record.html_url
    : null;
  const comparison = compareVersions(latestVersion, currentVersion);
  if (comparison === null) return failed();
  const state: UpdateCheckStateV1 = comparison > 0 ? "newer-available" : "up-to-date";
  return {
    schemaVersion: 1,
    state,
    currentVersion,
    latestVersion,
    releaseUrl,
    checkedAt,
  };
}
