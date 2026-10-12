import assert from "node:assert/strict";
import { test } from "vitest";
import {
  entryCards, entryCardMatches,
  artifactCardMatches,
  artifactCards,
  commandErrorText,
  entryActions,
  entryModeLine,
  entrySurfacesVisible,
  inferGlobalDefaultMode,
  sizeText,
  warehouseEntrySelectorRows,
  type AcquireArtifactCard,
} from "./acquire-model.ts";
import type { WarehouseEntry } from "../../gateway/index.ts";

test("sizeText: B 级整数,不进位", () => {
  assert.deepEqual(sizeText(0), { amount: "0", unitKey: "sizeB" });
  assert.deepEqual(sizeText(512), { amount: "512", unitKey: "sizeB" });
  assert.deepEqual(sizeText(1023), { amount: "1023", unitKey: "sizeB" });
});

test("sizeText: KB/MB/GB 进位与一位小数去尾零", () => {
  assert.deepEqual(sizeText(1024), { amount: "1", unitKey: "sizeKb" });
  assert.deepEqual(sizeText(1536), { amount: "1.5", unitKey: "sizeKb" });
  assert.deepEqual(sizeText(48_332_800), { amount: "46.1", unitKey: "sizeMb" });
  assert.deepEqual(sizeText(2 * 1024 * 1024 * 1024), { amount: "2", unitKey: "sizeGb" });
});

test("sizeText: 非法输入回落 0 B,不猜测", () => {
  assert.deepEqual(sizeText(Number.NaN), { amount: "0", unitKey: "sizeB" });
  assert.deepEqual(sizeText(-5), { amount: "0", unitKey: "sizeB" });
  assert.deepEqual(sizeText(Number.POSITIVE_INFINITY), { amount: "0", unitKey: "sizeB" });
});

/* ---- F4-6 条目模型:卡片展开 / 搜索 / 模式行 ---- */

function entryOf(partial: Partial<WarehouseEntry> & { warehouseItemId: string }): WarehouseEntry {
  return {
    folderName: partial.warehouseItemId,
    displayName: partial.warehouseItemId,
    kind: "imported_material",
    createdAt: "2026-09-01T10:20:00+08:00",
    artifactMode: null,
    effectiveArtifactMode: "use_original_unitypackage",
    artifacts: [
      {
        artifactSha256: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        relativePath: "pack.unitypackage",
        state: "clean",
        sizeBytes: 1024,
        role: "original",
      },
    ],
    ...partial,
  };
}

const entryA = entryOf({ warehouseItemId: "whentry-a" });
test("single-item cards stay at entry scope for companion files, generated copies and empty entries", () => {
  const multiple = entryOf({ warehouseItemId: "whentry-many", artifacts: [...entryA.artifacts, { ...entryA.artifacts[0]!, relativePath: "texture.psd" }] });
  const empty = entryOf({ warehouseItemId: "whentry-empty", artifacts: [] });
  const cards = entryCards([multiple, empty]);
  assert.deepEqual(cards.map(card => card.key), ["whentry-many", "whentry-empty"]);
  assert.equal(entryCardMatches(cards[0]!, "TEXTURE.PSD"), true);
  assert.equal(entryCardMatches(cards[1]!, "empty"), true);
});
const entryB = entryOf({
  warehouseItemId: "whentry-b",
  displayName: "巫女裙套装",
  kind: "downloaded_material",
});

test("artifactCards: 条目 × 工件展开,key 含条目身份与内容身份", () => {
  const twoArtifactEntry = entryOf({
    warehouseItemId: "whentry-two",
    artifacts: [
      {
        artifactSha256: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        relativePath: "orig/pack.unitypackage",
        state: "clean",
        sizeBytes: 1,
        role: "original",
      },
      {
        artifactSha256: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        relativePath: "generated/pack.vpm",
        state: "clean",
        sizeBytes: 2,
        role: "generated_vpm",
      },
    ],
  });
  const cards = artifactCards([entryA, twoArtifactEntry, entryB]);
  assert.equal(cards.length, 4);
  assert.deepEqual(
    cards.map((card) => card.key),
    [
      "whentry-a:sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "whentry-two:sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "whentry-two:sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "whentry-b:sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ],
  );
  // 每张卡携带条目上下文(卡片标题与 kind 徽标的来源)
  assert.equal(cards[2]?.entry.warehouseItemId, "whentry-two");
  assert.equal(cards[2]?.artifact.role, "generated_vpm");
});

test("artifactCardMatches: 显示名/文件夹名/相对路径子串,大小写不敏感,空词全过", () => {
  const cards: readonly AcquireArtifactCard[] = artifactCards([entryA, entryB]);
  const miko = cards[1];
  assert.ok(miko !== undefined);
  assert.equal(artifactCardMatches(miko, ""), true);
  assert.equal(artifactCardMatches(miko, "  "), true);
  assert.equal(artifactCardMatches(miko, "巫女"), true);
  assert.equal(artifactCardMatches(miko, "MIKO"), false); // displayName 为中文,拉丁词不命中
  assert.equal(artifactCardMatches(miko, "WHENTRY-B"), true); // folderName 大写不敏感命中
  assert.equal(artifactCardMatches(miko, "whentry-b"), true); // folderName = 条目 id
  assert.equal(artifactCardMatches(miko, "PACK.UNITYPACKAGE"), true); // 相对路径
  assert.equal(artifactCardMatches(miko, "summer"), false);
});

test("entryModeLine: 覆盖与跟随全局两形态,生效模式读取面已解析", () => {
  assert.deepEqual(entryModeLine(entryA), {
    overridden: false,
    effective: "use_original_unitypackage",
  });
  assert.deepEqual(
    entryModeLine(
      entryOf({
        warehouseItemId: "whentry-c",
        artifactMode: "generate_vpm",
        effectiveArtifactMode: "generate_vpm",
      }),
    ),
    { overridden: true, effective: "generate_vpm" },
  );
});

/* ---- F4-9 条目动作可见性(bdl-commands v0.1 入口条件镜像) ---- */

function withArtifacts(
  partial: Partial<WarehouseEntry> & { warehouseItemId: string },
  roles: ReadonlyArray<"original" | "generated_vpm">,
): WarehouseEntry {
  const entry = entryOf(partial);
  return {
    ...entry,
    artifacts: roles.map((role, index) => ({
      ...entry.artifacts[0]!,
      artifactSha256: `sha256:${role}-${index}-${"0".repeat(48)}`,
      role,
    })),
  };
}

test("entryActions: 总闸关(全局默认非 generate_vpm)时动作整组不呈现,U8⑤ 分支 a", () => {
  // 生效模式本可使入口成立,但总闸关闭=呈现层屏蔽(词表零变更,守卫不变)
  assert.deepEqual(
    entryActions(
      withArtifacts({ warehouseItemId: "whentry-gated", effectiveArtifactMode: "generate_vpm" }, [
        "original",
      ]),
      { kind: "known", mode: "use_original_unitypackage" },
    ),
    [],
  );
});

test("entryActions: 总闸状态推断不出(unknown)时不屏蔽,入口回归条目事实镜像", () => {
  assert.deepEqual(
    entryActions(
      withArtifacts({ warehouseItemId: "whentry-unk", effectiveArtifactMode: "generate_vpm" }, [
        "original",
      ]),
      { kind: "unknown" },
    ),
    ["generateVpm"],
  );
  // 省略参数 = unknown(默认),与旧调用形态兼容
  assert.deepEqual(
    entryActions(
      withArtifacts({ warehouseItemId: "whentry-def", effectiveArtifactMode: "generate_vpm" }, [
        "original",
      ]),
    ),
    ["generateVpm"],
  );
});

test("entryActions: 生成入口 = 总闸开 + 生效 generate_vpm + 有原始件 + 无生成副本", () => {
  assert.deepEqual(
    entryActions(
      withArtifacts({
        warehouseItemId: "whentry-gen",
        effectiveArtifactMode: "generate_vpm",
      }, ["original"]),
      { kind: "known", mode: "generate_vpm" },
    ),
    ["generateVpm"],
  );
  // 已有生成副本:生成入口消失(副本永不静默替换)
  assert.deepEqual(
    entryActions(
      withArtifacts({
        warehouseItemId: "whentry-both",
        effectiveArtifactMode: "generate_vpm",
      }, ["original", "generated_vpm"]),
      { kind: "known", mode: "generate_vpm" },
    ),
    [],
  );
});

test("entryActions: 删除入口已按用户裁定从仓储呈现移除(任何情况不出现)", () => {
  // 条目 2 裁决(2026-09-09):「删除原始素材」按钮任何情况下不在本地仓库
  // 出现;删除只由导入链按偏好触发,协议动作 deleteOriginals 保留(自动链
  // 消费),wire 词表零变更
  assert.deepEqual(
    entryActions(
      withArtifacts({
        warehouseItemId: "whentry-del",
        effectiveArtifactMode: "generate_vpm",
      }, ["generated_vpm"]),
      { kind: "known", mode: "generate_vpm" },
    ),
    [],
  );
});

/* ---- U8⑤ 总闸呈现门控(entrySurfacesVisible) ---- */

test("entrySurfacesVisible: 总闸关=不呈现,总闸开/推断不出=呈现", () => {
  assert.equal(entrySurfacesVisible({ kind: "known", mode: "generate_vpm" }), true);
  assert.equal(entrySurfacesVisible({ kind: "known", mode: "use_original_unitypackage" }), false);
  assert.equal(entrySurfacesVisible({ kind: "unknown" }), true);
});

test("commandErrorText: 已知码查表,未知码回落 fallback,传输面回落服务未接入", () => {
  const table = {
    vua_warehouse_invalid_state: "模式不符",
    vua_warehouse_unavailable: "服务未接入",
    fallback: "操作未能完成",
  };
  assert.equal(
    commandErrorText(
      { kind: "application", code: "vua.warehouse.invalid_state" },
      table,
    ),
    "模式不符",
  );
  assert.equal(
    commandErrorText({ kind: "application", code: "vua.warehouse.unknown_code" }, table),
    "操作未能完成",
  );
  assert.equal(commandErrorText({ kind: "unavailable" }, table), "服务未接入");
  assert.equal(commandErrorText({ kind: "request_rejected" }, table), "服务未接入");
});

/* ---- W15 重做:全局默认读面推断(设置页全局开关初值) ---- */

test("inferGlobalDefaultMode: 无覆盖条目的生效模式即全局默认;不可知如实 unknown", () => {
  // 无覆盖条目:生效模式 = composed 全局默认
  const known = inferGlobalDefaultMode([
    entryOf({ warehouseItemId: "whentry-i1", artifactMode: "generate_vpm", effectiveArtifactMode: "generate_vpm" }),
    entryOf({ warehouseItemId: "whentry-i2" }),
  ]);
  assert.deepEqual(known, { kind: "known", mode: "use_original_unitypackage" });

  // 全部条目有覆盖:不可知,不猜测
  const unknown = inferGlobalDefaultMode([
    entryOf({ warehouseItemId: "whentry-i3", artifactMode: "generate_vpm", effectiveArtifactMode: "generate_vpm" }),
  ]);
  assert.deepEqual(unknown, { kind: "unknown" });

  // 空仓库:不可知
  assert.deepEqual(inferGlobalDefaultMode([]), { kind: "unknown" });
});

/* ---- W19 008 路径 a 接线:生成副本出现检测 ---- */

import { generatedVpmEntryIds, newlyGeneratedEntryIds } from "./acquire-model.ts";

test("generatedVpmEntryIds: 仅收集持有生成副本的条目身份", () => {
  const ids = generatedVpmEntryIds([
    withArtifacts({ warehouseItemId: "whentry-ga", effectiveArtifactMode: "generate_vpm" }, ["original"]),
    withArtifacts({ warehouseItemId: "whentry-gb", effectiveArtifactMode: "generate_vpm" }, ["original", "generated_vpm"]),
  ]);
  assert.deepEqual([...ids].sort(), ["whentry-gb"]);
});

test("newlyGeneratedEntryIds: 只报两次快照之间新完成生成的条目", () => {
  const before = generatedVpmEntryIds([
    withArtifacts({ warehouseItemId: "whentry-n1" }, ["original", "generated_vpm"]),
  ]);
  const after = [
    withArtifacts({ warehouseItemId: "whentry-n1" }, ["original", "generated_vpm"]),
    withArtifacts({ warehouseItemId: "whentry-n2" }, ["original", "generated_vpm"]),
  ];
  assert.deepEqual(newlyGeneratedEntryIds(before, after), ["whentry-n2"]);
  // 再一轮:无新增
  const mid = generatedVpmEntryIds(after);
  assert.deepEqual(newlyGeneratedEntryIds(mid, after), []);
});

/* ---- 029 A3 本地段:素材选择器(仓储读面投影)行源 ---- */

test("warehouseEntrySelectorRows: 条目级搜索(显示名/文件夹名,大小写不敏感,空词全过,trim)", () => {
  const entries = [
    entryOf({ warehouseItemId: "whentry-s1", displayName: "Liltoon Shader" }),
    entryOf({ warehouseItemId: "whentry-s2", displayName: "Outfit Pack", folderName: "Summer-Outfit" }),
    entryOf({ warehouseItemId: "whentry-s3", displayName: "Hair A" }),
  ];
  // 空词全过
  assert.equal(warehouseEntrySelectorRows(entries, "").length, 3);
  assert.equal(warehouseEntrySelectorRows(entries, "   ").length, 3);
  // 显示名子串,大小写不敏感
  assert.deepEqual(
    warehouseEntrySelectorRows(entries, "shader").map((entry) => entry.warehouseItemId),
    ["whentry-s1"],
  );
  // 文件夹名命中
  assert.deepEqual(
    warehouseEntrySelectorRows(entries, "summer").map((entry) => entry.warehouseItemId),
    ["whentry-s2"],
  );
  // 无匹配 = 空行集(诚实空态,不猜测)
  assert.deepEqual(warehouseEntrySelectorRows(entries, "不存在"), []);
  // 只投影,不派生:行集是读面条目本体,事实原样
  const rows = warehouseEntrySelectorRows(entries, "");
  assert.equal(rows, entries);
});
