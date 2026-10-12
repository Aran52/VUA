/**
 * Warehouse 本地轨的纯函数模型(C-ACQUIRE,F4-6 条目模型)。
 * 大小展示:条目工件登记面必填字节数,本函数只处理已知字节数;
 * 单位键与 i18n strings.warehouse.acquire 的 sizeB/sizeKb/sizeMb/sizeGb 对应。
 * 卡片与模式行是页面的数据源纯函数:条目 × 工件展开保留图册卡片交互
 * (组件零重写),模式行(覆盖 or 跟随全局)是 F4-9 编辑入口的展示位。
 */

import type {
  WarehouseArtifact,
  WarehouseArtifactMode,
  WarehouseEntry,
} from "../../gateway/index.ts";

export type SizeUnitKey = "sizeB" | "sizeKb" | "sizeMb" | "sizeGb";

export interface SizeText {
  readonly amount: string;
  readonly unitKey: SizeUnitKey;
}

/** 字节数 → 本地化所需的 { amount, unitKey };不足 1024 进位,一位小数去尾零 */
export function sizeText(sizeBytes: number): SizeText {
  if (!Number.isFinite(sizeBytes) || sizeBytes < 0) return { amount: "0", unitKey: "sizeB" };
  const units: readonly SizeUnitKey[] = ["sizeB", "sizeKb", "sizeMb", "sizeGb"];
  let value = sizeBytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // B 为整数;KB/MB/GB 保留一位小数并去尾零(46.0 → "46",46.1 → "46.1")
  const amount =
    unit === 0 ? String(Math.round(value)) : String(Math.round(value * 10) / 10);
  return { amount, unitKey: units[unit] ?? "sizeB" };
}

/** 卡片墙数据源:条目 × 工件展开(工件卡携带条目上下文) */
export interface AcquireArtifactCard {
  /** 列表 key:条目身份 + 内容身份(同条目内 sha 唯一) */
  readonly key: string;
  readonly entry: WarehouseEntry;
  readonly artifact: WarehouseArtifact;
}

export function artifactCards(
  entries: readonly WarehouseEntry[],
): readonly AcquireArtifactCard[] {
  return entries.flatMap((entry) =>
    entry.artifacts.map((artifact) => ({
      key: `${entry.warehouseItemId}:${artifact.artifactSha256}`,
      entry,
      artifact,
    })),
  );
}

/** A user-selected material entry owns its files; no inferred package boundaries. */
export interface AcquireEntryCard { readonly key: string; readonly entry: WarehouseEntry }
export function entryCards(entries: readonly WarehouseEntry[]): readonly AcquireEntryCard[] {
  return entries.map(entry => ({ key: entry.warehouseItemId, entry }));
}
export function entryCardMatches(card: AcquireEntryCard, query: string): boolean {
  const text = query.trim().toLowerCase();
  return text === "" || [card.entry.displayName, card.entry.folderName, ...card.entry.artifacts.map(file => file.relativePath)].some(value => value.toLowerCase().includes(text));
}

/** 卡片搜索:显示名 / 文件夹名 / 相对路径子串匹配(大小写不敏感;空词全过) */
export function artifactCardMatches(card: AcquireArtifactCard, query: string): boolean {
  const text = query.trim().toLowerCase();
  if (text === "") return true;
  return (
    card.entry.displayName.toLowerCase().includes(text) ||
    card.entry.folderName.toLowerCase().includes(text) ||
    card.artifact.relativePath.toLowerCase().includes(text)
  );
}

/** 素材选择器行(029 A3 本地段,仓储读面投影):条目级搜索——显示名 /
 *  文件夹名子串匹配(大小写不敏感;空词全过)。只投影既有读面事实,不做
 *  任何筛选外的派生;工件级展开不在选择器面(选择对象是素材包条目)。 */
export function warehouseEntrySelectorRows(
  entries: readonly WarehouseEntry[],
  query: string,
): readonly WarehouseEntry[] {
  const text = query.trim().toLowerCase();
  if (text === "") return entries;
  return entries.filter(
    (entry) =>
      entry.displayName.toLowerCase().includes(text) ||
      entry.folderName.toLowerCase().includes(text),
  );
}

/** 产物模式行:覆盖与否 + 生效模式(覆盖 ?? 全局,读取面已动态解析) */
export interface EntryModeLine {
  readonly overridden: boolean;
  readonly effective: WarehouseArtifactMode;
}

export function entryModeLine(entry: WarehouseEntry): EntryModeLine {
  return { overridden: entry.artifactMode !== null, effective: entry.effectiveArtifactMode };
}

/* ---- F4-9 条目动作(呈现层,遵守 U8⑤ 裁决 2026-09-09) ----
 * 「生成 VPM 包替代」全局开关(服务端全局默认模式)是条目级产物模式编辑与
 * 条目动作的呈现总闸:关闭时整组不呈现(U8⑤ 分支 a 呈现层屏蔽,W14 词表零
 * 变更,服务端守卫语义不变);开启时入口可见性回归条目事实镜像(生效模式与
 * 工件条件,与协议语义一致)。
 * 「删除原始素材」手动入口已按用户裁定从仓储条目 UI 移除——删除只由导入
 * 链按「生成后删除原始素材文件」偏好触发(app 层 delete-originals-auto);
 * 协议动作 deleteOriginals 保留(导入链自动消费),wire 词表零变更。
 */

export type WarehouseEntryAction = "generateVpm";

/**
 * 总闸呈现门控(U8⑤ 分支 a):全局默认模式为 use_original_unitypackage
 * (即「生成 VPM 包替代」关)时,条目级产物模式编辑与条目动作整组不呈现。
 * 全局状态推断不出(unknown)时不屏蔽——不猜测总闸状态,保持既有呈现。
 */
export function entrySurfacesVisible(globalDefault: GlobalDefaultInference): boolean {
  return !(globalDefault.kind === "known" && globalDefault.mode !== "generate_vpm");
}

export function entryActions(
  entry: WarehouseEntry,
  globalDefault: GlobalDefaultInference = { kind: "unknown" },
): readonly WarehouseEntryAction[] {
  if (!entrySurfacesVisible(globalDefault)) return [];
  if (entry.effectiveArtifactMode !== "generate_vpm") return [];
  const hasOriginal = entry.artifacts.some((artifact) => artifact.role === "original");
  const hasGenerated = entry.artifacts.some((artifact) => artifact.role === "generated_vpm");
  // 生成入口:生效模式 generate_vpm + 持有原始素材 + 尚无生成副本
  // (生成副本永不静默替换)
  return hasOriginal && !hasGenerated ? ["generateVpm"] : [];
}

/**
 * 写命令错误 → 本地化文案查表(仓储抽屉与设置页共用):code 是协议冻结面,
 * 键为点号转下划线;未知码回落通用失败文案;传输面三态回落服务未接入。
 */
export function commandErrorText(
  error: { kind: "unavailable" | "request_rejected" | "application"; code?: string },
  commandErrors: Readonly<Record<string, string>>,
): string {
  if (error.kind === "application" && typeof error.code === "string") {
    return commandErrors[error.code.replaceAll(".", "_")] ?? commandErrors.fallback!;
  }
  return commandErrors.vua_warehouse_unavailable!;
}

/* ---- W15 重做:全局默认模式的读面推断(设置页全局开关的初值) ----
 * wire 无独立的全局默认查询;条目读面中无覆盖条目的生效模式即
 * 「覆盖 ?? composed 全局默认」的动态解析结果,可据此推断。全部条目
 * 均有覆盖(或空仓库)时不可知——如实 unknown,UI 标注而非猜测初值。
 */

export type GlobalDefaultInference =
  | { readonly kind: "known"; readonly mode: WarehouseArtifactMode }
  | { readonly kind: "unknown" };

export function inferGlobalDefaultMode(
  entries: readonly WarehouseEntry[],
): GlobalDefaultInference {
  const probe = entries.find((entry) => entry.artifactMode === null);
  return probe === undefined
    ? { kind: "unknown" }
    : { kind: "known", mode: probe.effectiveArtifactMode };
}

/** 条目当前持有生成副本的身份集合(008 路径 a 接线的信号源) */
export function generatedVpmEntryIds(entries: readonly WarehouseEntry[]): ReadonlySet<string> {
  const ids = new Set<string>();
  for (const entry of entries) {
    if (entry.artifacts.some((artifact) => artifact.role === "generated_vpm")) {
      ids.add(entry.warehouseItemId);
    }
  }
  return ids;
}

/**
 * 新完成生成的条目身份(008 路径 a 桌面接线,W19):当前快照持有生成副本、
 * 而先前快照没有——即两次快照之间完成了生成。条目事实等价于生成 Done
 * (任务面快照不携带条目身份,条目读面是域内可得的等价信号);删除的守卫
 * 与审计仍在服务端,桌面只是发起时机。
 */
export function newlyGeneratedEntryIds(
  previous: ReadonlySet<string>,
  entries: readonly WarehouseEntry[],
): readonly string[] {
  const current = generatedVpmEntryIds(entries);
  const fresh: string[] = [];
  for (const id of current) {
    if (!previous.has(id)) fresh.push(id);
  }
  return fresh;
}
