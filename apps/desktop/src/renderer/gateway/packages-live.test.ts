import { describe, expect, it } from "vitest";
import type {
  DesktopGatewayRequestV1,
  DesktopGatewaySuccessValueV1,
} from "@vua/contracts";
import { createLivePackages } from "./packages-live.ts";
import type { PackagesView } from "./packages-port.ts";
import type { GatewayClient, GatewayResult } from "./gateway-client.ts";

/**
 * packages 读面 live 端口测试(024 P1 中间诚实态消费批;025 P2 读面消费
 * 批;025 v0.2 增量消费更新批):fake GatewayClient 编排 app.snapshot
 * 能力行与 packages.* wire 帧,钉死消费纪律——诚实缺席(not-connected)、
 * typed 失败照原词(不折叠空态)、帧窄化(行闭集,形状不符诚实失败)、
 * 区块可用性标注随能力行翻转(installed = packages.query;repos =
 * packages.listRepos;catalog = packages.packageCatalog)、订阅行
 * cached=false 诚实承载、目录按需查询 no_matching_package 独立空态码照
 * 原词上呈;目录族双版协商(v0.1 七键/v0.2 八键 cacheSourced 披露,盖
 * 戳族常量辨词面,缺键/发明键均形状不符)。
 */

type InvokeHandler = (
  request: DesktopGatewayRequestV1,
) => Promise<GatewayResult<DesktopGatewaySuccessValueV1>>;

function fakeClient(invoke: InvokeHandler): GatewayClient {
  return {
    invoke,
    subscribe: () => () => {},
  };
}

function asWire(value: unknown): DesktopGatewaySuccessValueV1 {
  return value as DesktopGatewaySuccessValueV1;
}

function appSnapshot(operations: readonly { operationId: string; availability: string }[]) {
  return asWire({
    contractVersion: "0.1",
    revision: 1,
    capabilities: { revision: 1, operations },
  });
}

const QUERY_AVAILABLE = [{ operationId: "packages.query", availability: "available" }];
const QUERY_ABSENT = [{ operationId: "task.list", availability: "available" }];
/** P2 读面齐全:三能力行全部 available */
const P2_AVAILABLE = [
  { operationId: "packages.query", availability: "available" },
  { operationId: "packages.listRepos", availability: "available" },
  { operationId: "packages.packageCatalog", availability: "available" },
];
/** P2 读面行存在但引擎后端未声明目录能力(availability 翻转前的诚实缺席) */
const P2_ROWS_UNAVAILABLE = [
  { operationId: "packages.query", availability: "available" },
  { operationId: "packages.listRepos", availability: "unavailable" },
  { operationId: "packages.packageCatalog", availability: "unavailable" },
];
/** F2 读面齐全:repoCatalogOps 行 available(027;环境覆写置真后的翻转态) */
const F2_AVAILABLE = [
  { operationId: "packages.query", availability: "available" },
  { operationId: "packages.listRepos", availability: "available" },
  { operationId: "packages.packageCatalog", availability: "available" },
  { operationId: "packages.repoCatalogOps", availability: "available" },
];

function installedFrame(rows: unknown[]) {
  return asWire({
    schemaVersion: "0.1",
    operation: "packages.listInstalled",
    result: {
      schemaVersion: "vua.packages-installed/v0.1",
      projectPath: "C:/proj",
      packages: rows,
    },
  });
}

function reposFrame(rows: unknown[]) {
  return asWire({
    schemaVersion: "0.1",
    operation: "packages.listRepos",
    result: {
      schemaVersion: "vua.packages-repos/v0.1",
      repos: rows,
    },
  });
}

function catalogFrame(result: unknown) {
  return asWire({
    schemaVersion: "0.1",
    operation: "packages.packageCatalog",
    result,
  });
}

function repoCatalogFrame(result: unknown) {
  return asWire({
    schemaVersion: "0.1",
    operation: "packages.repoCatalog",
    result,
  });
}

/** F5 模板枚举应答帧(027;族常量盖戳 + templates 行数组) */
function templatesFrame(result: unknown) {
  return asWire({
    schemaVersion: "0.1",
    operation: "packages.listTemplates",
    result,
  });
}

/** F5 读面齐全:templatesOps 行 available(027;环境覆写置真后的翻转态) */
const F5_AVAILABLE = [
  { operationId: "packages.query", availability: "available" },
  { operationId: "packages.templatesOps", availability: "available" },
];

/** F5 模板行集:服务端冻结 id 升序呈现事实(name = id 冻结同值显示投影;
 *  description/sourceRoot 刻意缺席,发明即非法) */
const VALID_TEMPLATE_ROWS = [
  { id: "com.vrchat.avatars", name: "com.vrchat.avatars" },
  { id: "com.vrchat.base", name: "com.vrchat.base" },
  { id: "com.vrchat.worlds", name: "com.vrchat.worlds" },
];

const VALID_ROWS = [
  { packageId: "com.vrchat.avatars", version: "3.7.4", dependencies: ["com.vrchat.base"] },
  { packageId: "com.vrchat.base", version: "3.7.2", dependencies: [] },
];

/** 订阅面自身顺序(配置事实);含 null 投影行与 cached=false 诚实行 */
const VALID_REPO_ROWS = [
  {
    repoId: "official",
    name: "Official",
    url: "https://vpm.example/vpm.json",
    localPath: "C:/Users/demo/AppData/Local/VRChatCreatorCompanion/vpm-repos/official.json",
    cached: true,
  },
  { repoId: null, name: null, url: null, localPath: "C:/repos/local.json", cached: false },
];

/** F2 仓库级目录事实(027 冻结词面;族常量在窄化校验时消费,不外传):
 *  cacheSourced 必带信息性降级披露 + cached=false 诚实行(空 packages) */
const VALID_REPO_CATALOG_FACTS = {
  repos: [
    {
      repoId: "official",
      name: "Official",
      cached: true,
      packages: [
        {
          packageId: "com.anatawa12.avatar-optimizer",
          displayName: "Avatar Optimizer",
          description: null,
          latestVersion: "1.9.0",
          versionCount: 7,
        },
        {
          packageId: "com.example.legacy",
          displayName: null,
          description: "Legacy package",
          latestVersion: null,
          versionCount: 2,
        },
      ],
    },
    { repoId: "pending", name: "Pending", cached: false, packages: [] },
  ],
  cacheSourced: false,
};

/** 端口返回的七键事实(族常量 schemaVersion 在窄化校验时消费,不外传) */
const VALID_CATALOG_FACTS = {
  projectPath: "C:/proj",
  packageId: "com.anatawa12.avatar-optimizer",
  displayName: "Avatar Optimizer",
  source: "repo",
  installed: true,
  updateAvailable: null,
  versions: [
    { version: "1.8.0", yanked: false, compatible: true },
    { version: "1.9.0", yanked: false, compatible: null },
  ],
};

function applicationError(
  code: string,
  category:
    | "validation"
    | "conflict"
    | "permission"
    | "dependency"
    | "unavailable"
    | "timeout"
    | "cancelled"
    | "external_failure"
    | "internal",
): GatewayResult<DesktopGatewaySuccessValueV1> {
  return {
    ok: false,
    error: {
      kind: "application",
      error: {
        contractVersion: "0.1",
        code,
        category,
        messageKey: `errors.${code}`,
        recoverable: false,
        retryable: false,
        correlationId: "c",
      },
    },
  } as const;
}

const UNAVAILABLE_ERROR = applicationError("vua.packages.unavailable", "unavailable");

function clientWith(overrides: {
  snapshot?: Parameters<typeof appSnapshot>[0];
  listResult?: GatewayResult<DesktopGatewaySuccessValueV1>;
  reposResult?: GatewayResult<DesktopGatewaySuccessValueV1>;
  catalogResult?: GatewayResult<DesktopGatewaySuccessValueV1>;
  repoCatalogResult?: GatewayResult<DesktopGatewaySuccessValueV1>;
  templatesResult?: GatewayResult<DesktopGatewaySuccessValueV1>;
  onRepoCatalogRequest?: (request: DesktopGatewayRequestV1) => void;
  onListTemplatesRequest?: (request: DesktopGatewayRequestV1) => void;
} = {}): GatewayClient {
  return fakeClient(async (request) => {
    if (request.method === "app.snapshot") {
      return { ok: true, value: appSnapshot(overrides.snapshot ?? QUERY_AVAILABLE) };
    }
    if (request.method === "packages.listInstalled") {
      return overrides.listResult ?? { ok: true, value: installedFrame(VALID_ROWS) };
    }
    if (request.method === "packages.listRepos") {
      return overrides.reposResult ?? { ok: true, value: reposFrame(VALID_REPO_ROWS) };
    }
    if (request.method === "packages.packageCatalog") {
      // wire result 本体 = 族常量盖戳 + 七键事实(路由盖戳,后端事实逐字)
      return (
        overrides.catalogResult ?? {
          ok: true,
          value: catalogFrame({ schemaVersion: "vua.packages-catalog/v0.1", ...VALID_CATALOG_FACTS }),
        }
      );
    }
    if (request.method === "packages.repoCatalog") {
      overrides.onRepoCatalogRequest?.(request);
      return (
        overrides.repoCatalogResult ?? {
          ok: true,
          value: repoCatalogFrame({ schemaVersion: "vua.packages-repo-catalog/v0.1", ...VALID_REPO_CATALOG_FACTS }),
        }
      );
    }
    if (request.method === "packages.listTemplates") {
      overrides.onListTemplatesRequest?.(request);
      return (
        overrides.templatesResult ?? {
          ok: true,
          value: templatesFrame({ schemaVersion: "vua.packages-templates/v0.1", templates: [] }),
        }
      );
    }
    return { ok: false, error: { kind: "unavailable" } as const };
  });
}

describe("packages live port (024 P1 consumption)", () => {
  it("answers not-connected and packagesEngineMissing when the capability row is absent", async () => {
    // 真实 provider 语义:引擎未装配时读面答 typed unavailable(缺席臂)
    const port = createLivePackages(
      clientWith({
        snapshot: QUERY_ABSENT,
        listResult: UNAVAILABLE_ERROR,
      }),
    );
    const view = await port.snapshot();
    expect(view).toEqual({ schemaVersion: 1, kind: "not-connected" });
    const capability = await port.capability();
    expect(capability).toEqual({ state: "unavailable", detailKey: "packagesEngineMissing" });
    // 缺席下的直接读面同样诚实 unavailable
    expect(await port.listInstalled("C:/proj")).toEqual({ kind: "unavailable" });
  });

  it("projects a ready-p2 view with availability blocks and no selection facts (P2 rows unavailable = honest hidden sections)", async () => {
    const port = createLivePackages(clientWith());
    const view = await port.snapshot();
    expect(view).toEqual({
      schemaVersion: 1,
      kind: "ready-p2",
      blocks: { installed: true, repos: false, catalog: false, changes: false, installs: false, registers: false, repoWrites: false, creates: false, repoCatalog: false, templates: false, repoLifecycle: false },
      projectPath: null,
      installedPackages: [],
      repos: [],
    });
    expect(await port.capability()).toEqual({ state: "ready" });
  });

  it("carries the frozen three-key rows after selectProject, ascending order untouched", async () => {
    const port = createLivePackages(clientWith());
    await port.selectProject("C:/proj");
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.projectPath).toBe("C:/proj");
    expect(view.installedPackages).toEqual(VALID_ROWS);
    expect(view.loadError).toBeUndefined();
  });

  it("surfaces typed failures verbatim instead of folding them into an empty listing", async () => {
    const port = createLivePackages(
      clientWith({
        listResult: applicationError("vua.project.project_not_found", "validation"),
      }),
    );
    await port.selectProject("C:/proj");
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.installedPackages).toEqual([]);
    expect(view.loadError).toEqual({ code: "vua.project.project_not_found" });
    expect(await port.listInstalled("C:/proj")).toEqual({
      kind: "failed",
      code: "vua.project.project_not_found",
    });
  });

  it("answers shape violation when the wire envelope deviates from the frozen three-key face", async () => {
    const port = createLivePackages(
      clientWith({
        listResult: {
          ok: true,
          value: installedFrame([
            // 发明 P2 事实字段(updateAvailable)= 虚假断言防线,整帧形状不符
            { packageId: "com.x", version: "1.0", dependencies: [], updateAvailable: false },
          ]),
        },
      }),
    );
    await port.selectProject("C:/proj");
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.loadError).toEqual({ code: "packages_shape_violation" });
  });

  it("stays not-connected when the engine answers unavailable for the direct read", async () => {
    const port = createLivePackages(
      clientWith({ listResult: UNAVAILABLE_ERROR }),
    );
    await port.selectProject("C:/proj");
    expect(await port.snapshot()).toEqual({ schemaVersion: 1, kind: "not-connected" });
    expect(await port.listInstalled("C:/proj")).toEqual({ kind: "unavailable" });
  });
});

describe("packages live port (025 P2 consumption)", () => {
  it("carries repo subscription rows verbatim — cached=false honest rows and null projections preserved, order untouched", async () => {
    const port = createLivePackages(
      clientWith({ snapshot: P2_ROWS_UNAVAILABLE, reposResult: { ok: true, value: reposFrame(VALID_REPO_ROWS) } }),
    );
    // 能力行 unavailable:区块不渲染(repos block false),但快照不伪造
    const hidden = await port.snapshot();
    assertReadyP2(hidden);
    if (hidden.kind !== "ready-p2") return;
    expect(hidden.blocks).toEqual({ installed: true, repos: false, catalog: false, changes: false, installs: false, registers: false, repoWrites: false, creates: false, repoCatalog: false, templates: false, repoLifecycle: false });

    const port2 = createLivePackages(clientWith({ snapshot: P2_AVAILABLE }));
    const view = await port2.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.blocks).toEqual({ installed: true, repos: true, catalog: true, changes: false, installs: false, registers: false, repoWrites: false, creates: false, repoCatalog: false, templates: false, repoLifecycle: false });
    expect(view.repos).toEqual(VALID_REPO_ROWS);
    expect(view.reposError).toBeUndefined();
  });

  it("renders an honest zero-subscription empty array when the subscription face is empty", async () => {
    const port = createLivePackages(
      clientWith({ snapshot: P2_AVAILABLE, reposResult: { ok: true, value: reposFrame([]) } }),
    );
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.repos).toEqual([]);
    expect(view.reposError).toBeUndefined();
  });

  it("surfaces listRepos typed failures verbatim as reposError — never an empty masquerade", async () => {
    const port = createLivePackages(
      clientWith({
        snapshot: P2_AVAILABLE,
        reposResult: applicationError("vua.vpm.capability_missing", "unavailable"),
      }),
    );
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.repos).toEqual([]);
    expect(view.reposError).toEqual({ code: "vua.vpm.capability_missing" });
  });

  it("answers repos shape violation when a row invents a health field (health face is a frozen non-goal)", async () => {
    const port = createLivePackages(
      clientWith({
        snapshot: P2_AVAILABLE,
        reposResult: {
          ok: true,
          value: reposFrame([
            {
              repoId: "official",
              name: "Official",
              url: "https://vpm.example/vpm.json",
              localPath: "C:/repos/official.json",
              cached: true,
              health: "ok",
            },
          ]),
        },
      }),
    );
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.repos).toEqual([]);
    expect(view.reposError).toEqual({ code: "packages_shape_violation" });
  });

  it("answers packageCatalog with the frozen seven-key facts, null semantics preserved", async () => {
    const port = createLivePackages(clientWith({ snapshot: P2_AVAILABLE }));
    const outcome = await port.packageCatalog("C:/proj", "com.anatawa12.avatar-optimizer");
    expect(outcome).toEqual({ kind: "ok", result: VALID_CATALOG_FACTS });
  });

  it("surfaces no_matching_package verbatim for the own empty state — never folded into an error page or empty masquerade", async () => {
    const port = createLivePackages(
      clientWith({
        snapshot: P2_AVAILABLE,
        catalogResult: applicationError("vua.vpm.no_matching_package", "validation"),
      }),
    );
    expect(await port.packageCatalog("C:/proj", "com.missing.package")).toEqual({
      kind: "failed",
      code: "vua.vpm.no_matching_package",
    });
  });

  it("answers catalog shape violation when a version row invents changelogUrl (false-assertion guard)", async () => {
    const port = createLivePackages(
      clientWith({
        snapshot: P2_AVAILABLE,
        catalogResult: {
          ok: true,
          value: catalogFrame({
            schemaVersion: "vua.packages-catalog/v0.1",
            projectPath: "C:/proj",
            packageId: "com.x",
            displayName: null,
            source: "repo",
            installed: false,
            updateAvailable: null,
            versions: [{ version: "1.0.0", yanked: false, compatible: true, changelogUrl: "https://x" }],
          }),
        },
      }),
    );
    expect(await port.packageCatalog("C:/proj", "com.x")).toEqual({
      kind: "failed",
      code: "packages_shape_violation",
    });
  });

  it("answers packageCatalog honestly unavailable when the engine is absent", async () => {
    const port = createLivePackages(
      clientWith({ snapshot: P2_AVAILABLE, catalogResult: UNAVAILABLE_ERROR }),
    );
    expect(await port.packageCatalog("C:/proj", "com.x")).toEqual({ kind: "unavailable" });
  });
});

describe("packages live port (025 v0.2 increment consumption update)", () => {
  /** v0.2 = 冻结 v0.1 七键恰加必带 cacheSourced(025 v0.2 增量冻结批词面) */
  const VALID_CATALOG_FACTS_V02 = {
    projectPath: "C:/proj",
    packageId: "com.anatawa12.avatar-optimizer",
    displayName: "Avatar Optimizer",
    source: "repo",
    installed: true,
    updateAvailable: null,
    versions: [
      { version: "1.8.0", yanked: false, compatible: true },
      { version: "1.9.0", yanked: false, compatible: null },
    ],
    cacheSourced: true,
  };

  it("accepts a v0.2-stamped eight-key answer and carries the cacheSourced disclosure verbatim (true = served via cache degradation)", async () => {
    const port = createLivePackages(
      clientWith({
        snapshot: P2_AVAILABLE,
        catalogResult: {
          ok: true,
          value: catalogFrame({ schemaVersion: "vua.packages-catalog/v0.2", ...VALID_CATALOG_FACTS_V02 }),
        },
      }),
    );
    const outcome = await port.packageCatalog("C:/proj", "com.anatawa12.avatar-optimizer");
    expect(outcome.kind).toBe("ok");
    if (outcome.kind !== "ok") return;
    expect(outcome.result).toEqual(VALID_CATALOG_FACTS_V02);
    // 族判别:结果携带披露事实,页面据此呈现「缓存数据」信息标注(非失败)
    expect("cacheSourced" in outcome.result && outcome.result.cacheSourced).toBe(true);
  });

  it("accepts a v0.2 answer with cacheSourced=false (online-refreshed, no annotation)", async () => {
    const port = createLivePackages(
      clientWith({
        snapshot: P2_AVAILABLE,
        catalogResult: {
          ok: true,
          value: catalogFrame({
            schemaVersion: "vua.packages-catalog/v0.2",
            ...VALID_CATALOG_FACTS_V02,
            cacheSourced: false,
          }),
        },
      }),
    );
    const outcome = await port.packageCatalog("C:/proj", "com.anatawa12.avatar-optimizer");
    expect(outcome.kind).toBe("ok");
    if (outcome.kind !== "ok") return;
    expect("cacheSourced" in outcome.result && outcome.result.cacheSourced).toBe(false);
  });

  it("answers shape violation for a v0.2 stamp without the required cacheSourced key (version generation is pinned by the family constant, never guessed)", async () => {
    const { cacheSourced: _omitted, ...sevenKeyFacts } = VALID_CATALOG_FACTS_V02;
    const port = createLivePackages(
      clientWith({
        snapshot: P2_AVAILABLE,
        catalogResult: {
          ok: true,
          value: catalogFrame({ schemaVersion: "vua.packages-catalog/v0.2", ...sevenKeyFacts }),
        },
      }),
    );
    expect(await port.packageCatalog("C:/proj", "com.anatawa12.avatar-optimizer")).toEqual({
      kind: "failed",
      code: "packages_shape_violation",
    });
  });

  it("answers shape violation for a v0.1 answer inventing the disclosure field (frozen v0.1 closed set unchanged — no fabricated annotation source)", async () => {
    const port = createLivePackages(
      clientWith({
        snapshot: P2_AVAILABLE,
        catalogResult: {
          ok: true,
          value: catalogFrame({ schemaVersion: "vua.packages-catalog/v0.1", ...VALID_CATALOG_FACTS, cacheSourced: false }),
        },
      }),
    );
    expect(await port.packageCatalog("C:/proj", "com.anatawa12.avatar-optimizer")).toEqual({
      kind: "failed",
      code: "packages_shape_violation",
    });
  });
});


/* ---- A1 移除写面消费(026 冻结词面 d7f6a57;wire 接线批 41503a4 世代) ---- */

const VALID_PLAN = {
  schemaVersion: "vua.packages-ops/v0.1",
  kind: "plan",
  projectPath: "C:/proj",
  items: [{ kind: "remove", packageId: "com.vrchat.avatars", version: null, reason: null }],
  conflicts: ["com.example.dependent depends on com.vrchat.avatars"],
  removeLegacyFiles: [],
  removeLegacyFolders: ["Packages/com.vrchat.avatars_legacy"],
  destructive: true,
  digest: "fnv-1a-abc123",
};

const VALID_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.1",
  kind: "receipt",
  projectPath: "C:/proj",
  confirmedDigest: "fnv-1a-abc123",
  requestedPackageIds: ["com.vrchat.avatars"],
  removedItems: [{ kind: "remove", packageId: "com.vrchat.avatars", version: null, reason: null }],
};

const VALID_REJECTED = {
  schemaVersion: "vua.packages-ops/v0.1",
  kind: "rejected",
  guard: "preview_drift",
  code: "vua.packages.preview_drift",
  detail: "confirmed digest fnv-1a-abc123 does not match the re-computed preview digest fnv-1a-def456",
};

function previewRemoveFrame(result: unknown): DesktopGatewaySuccessValueV1 {
  return asWire({ schemaVersion: "0.1", operation: "packages.previewRemove", result });
}

function acceptedFrame(taskId: string): DesktopGatewaySuccessValueV1 {
  return asWire({ schemaVersion: "0.1", operation: "packages.applyRemove", taskId, correlationId: "c-1" });
}

function taskSnapshotValue(state: string, extra: { result?: unknown; error?: unknown } = {}): DesktopGatewaySuccessValueV1 {
  return asWire({
    contractVersion: "0.1",
    taskId: "t-1",
    correlationId: "c-1",
    revision: 2,
    state,
    cancellationRequested: false,
    recoveryDisposition: "none",
    updatedAt: "2026-09-19T00:00:00Z",
    ...extra,
  });
}

/** A1 流编排 client:preview/apply/task.get 分支可控;subscribe 支持
 * task.completed 事件派发(020 终态等待通道)。 */
function a1Client(overrides: {
  snapshot?: Parameters<typeof appSnapshot>[0];
  preview?: GatewayResult<DesktopGatewaySuccessValueV1>;
  apply?: GatewayResult<DesktopGatewaySuccessValueV1>;
  taskGet?: GatewayResult<DesktopGatewaySuccessValueV1>;
} = {}): { client: GatewayClient; emitCompleted: (taskId: string) => void } {
  const listeners = new Set<(event: unknown) => void>();
  const client: GatewayClient = {
    invoke: async (request) => {
      if (request.method === "app.snapshot") {
        return { ok: true, value: appSnapshot(overrides.snapshot ?? QUERY_AVAILABLE) };
      }
      if (request.method === "packages.previewRemove") {
        return overrides.preview ?? { ok: true, value: previewRemoveFrame(VALID_PLAN) };
      }
      if (request.method === "packages.applyRemove") {
        return overrides.apply ?? { ok: true, value: acceptedFrame("t-1") };
      }
      if (request.method === "task.get") {
        return overrides.taskGet ?? { ok: true, value: taskSnapshotValue("running") };
      }
      return { ok: false, error: { kind: "unavailable" } as const };
    },
    subscribe: (callback) => {
      listeners.add(callback as (event: unknown) => void);
      return () => {
        listeners.delete(callback as (event: unknown) => void);
      };
    },
  };
  return {
    client,
    emitCompleted: (taskId) => {
      for (const listener of listeners) {
        listener({ kind: "task.completed", taskId, payload: {} });
      }
    },
  };
}

describe("packages live port A1 removal write face (026 consumption)", () => {
  it("narrows the frozen nine-key plan verbatim (family const stamped, nulls projected as port facts)", async () => {
    const { client } = a1Client();
    const port = createLivePackages(client);
    const outcome = await port.previewRemove("C:/proj", ["com.vrchat.avatars"]);
    expect(outcome).toEqual({ kind: "ok", plan: VALID_PLAN });
  });

  it("answers unavailable on the absence arm and passes unknown-port codes verbatim as envelope errors", async () => {
    const absent = a1Client({
      preview: {
        ok: false,
        error: {
          kind: "application",
          error: {
            contractVersion: "0.1",
            code: "vua.packages.unavailable",
            category: "unavailable",
            messageKey: "errors.packages.unavailable",
            recoverable: false,
            retryable: false,
            correlationId: "c",
          },
        },
      },
    });
    expect(
      await createLivePackages(absent.client).previewRemove("C:/proj", ["com.vrchat.avatars"]),
    ).toEqual({ kind: "unavailable" });
    const notFound = a1Client({
      preview: applicationError("vua.packages.package_not_found", "validation"),
    });
    expect(
      await createLivePackages(notFound.client).previewRemove("C:/proj", ["com.vrchat.avatars"]),
    ).toEqual({ kind: "failed", code: "vua.packages.package_not_found" });
  });

  it("answers shape violation when the plan invents a field or the result carries a foreign kind", async () => {
    const invented = a1Client({
      preview: {
        ok: true,
        value: previewRemoveFrame({ ...VALID_PLAN, updateAvailable: false }),
      },
    });
    expect(
      await createLivePackages(invented.client).previewRemove("C:/proj", ["com.vrchat.avatars"]),
    ).toEqual({ kind: "failed", code: "packages_shape_violation" });
    const foreignKind = a1Client({
      preview: {
        ok: true,
        value: previewRemoveFrame({ ...VALID_PLAN, kind: "receipt" }),
      },
    });
    expect(
      await createLivePackages(foreignKind.client).previewRemove("C:/proj", ["com.vrchat.avatars"]),
    ).toEqual({ kind: "failed", code: "packages_shape_violation" });
  });

  it("flips blocks.changes with the served packages.removeOps row (absent row stays false)", async () => {
    const withRow = await createLivePackages(
      a1Client({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.removeOps", availability: "available" },
        ],
      }).client,
    ).snapshot();
    assertReadyP2(withRow);
    if (withRow.kind === "ready-p2") {
      expect(withRow.blocks.changes).toBe(true);
    }
    const withoutRow = await createLivePackages(a1Client().client).snapshot();
    assertReadyP2(withoutRow);
    if (withoutRow.kind === "ready-p2") {
      expect(withoutRow.blocks.changes).toBe(false);
    }
  });

  it("rides the task loop: acceptance -> completed event -> succeeded snapshot with the receipt payload", async () => {
    // task.get 序列:首取 running(非终态) -> task.completed 事件后重取 succeeded 携收据
    const taskGetSequence: GatewayResult<DesktopGatewaySuccessValueV1>[] = [
      { ok: true, value: taskSnapshotValue("running") },
      {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.1", operation: "packages.applyRemove", result: VALID_RECEIPT },
        }),
      },
    ];
    let taskGetCalls = 0;
    let subscribed = false;
    const listeners = new Set<(event: unknown) => void>();
    const client: GatewayClient = {
      invoke: async (request) => {
        if (request.method === "packages.applyRemove") {
          return { ok: true, value: acceptedFrame("t-1") };
        }
        if (request.method === "task.get") {
          const answer: GatewayResult<DesktopGatewaySuccessValueV1> =
            taskGetSequence[Math.min(taskGetCalls, taskGetSequence.length - 1)] ?? {
              ok: false,
              error: { kind: "unavailable" },
            };
          taskGetCalls += 1;
          return answer;
        }
        return { ok: false, error: { kind: "unavailable" } as const };
      },
      subscribe: (callback) => {
        listeners.add(callback as (event: unknown) => void);
        subscribed = true;
        return () => {
          listeners.delete(callback as (event: unknown) => void);
        };
      },
    };
    const port = createLivePackages(client);
    const pending = port.applyRemove("C:/proj", ["com.vrchat.avatars"], "fnv-1a-abc123");
    // 端口订阅建立后才派发 task.completed(020 事件通道语义:订阅先于事件)
    while (!subscribed) await new Promise((resolve) => setTimeout(resolve, 1));
    for (const listener of listeners) listener({ kind: "task.completed", taskId: "t-1", payload: {} });
    expect(await pending).toEqual({ kind: "ok", receipt: VALID_RECEIPT });
  });

  it("surfaces a rejected guard refusal (drift) as the typed rejection, never an error", async () => {
    const flow = a1Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.1", operation: "packages.applyRemove", result: VALID_REJECTED },
        }),
      },
    });
    const port = createLivePackages(flow.client);
    expect(await port.applyRemove("C:/proj", ["com.vrchat.avatars"], "fnv-1a-abc123")).toEqual({
      kind: "rejected",
      rejection: VALID_REJECTED,
    });
  });

  it("reports a non-succeeded terminal state verbatim (task truth lives in the task center)", async () => {
    const flow = a1Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("failed", {
          error: {
            contractVersion: "0.1",
            code: "vua.provider.persistence_failed",
            category: "internal",
            messageKey: "errors.provider.persistence",
            recoverable: false,
            retryable: false,
            correlationId: "c-1",
          },
        }),
      },
    });
    const port = createLivePackages(flow.client);
    expect(await port.applyRemove("C:/proj", ["com.vrchat.avatars"], "fnv-1a-abc123")).toEqual({
      kind: "failed",
      code: "vua.provider.persistence_failed",
    });
  });

  it("answers acceptance shape violation and unavailable honestly (never fabricates a receipt)", async () => {
    const badAcceptance = a1Client({
      apply: { ok: true, value: asWire({ schemaVersion: "0.1", operation: "packages.applyRemove", taskId: "t-1" }) },
    });
    expect(
      await createLivePackages(badAcceptance.client).applyRemove("C:/proj", ["com.vrchat.avatars"], "d"),
    ).toEqual({ kind: "failed", code: "packages_apply_acceptance_shape" });
    const absent = a1Client({
      apply: applicationError("vua.packages.unavailable", "unavailable"),
    });
    expect(
      await createLivePackages(absent.client).applyRemove("C:/proj", ["com.vrchat.avatars"], "d"),
    ).toEqual({ kind: "unavailable" });
  });
});

/* ---- A2 安装/升级写面消费(026 packages-ops v0.2 冻结词面 8552d2c;wire
 * 接线批 61da51a + 钉法缺口收口 beb7d34 世代) ---- */

const VALID_INSTALL_PLAN = {
  schemaVersion: "vua.packages-ops/v0.2",
  kind: "plan",
  projectPath: "C:/proj",
  items: [
    { kind: "install", packageId: "com.vrchat.avatars", version: "3.7.4", reason: null },
    { kind: "remove", packageId: "com.example.conflict", version: null, reason: "conflict" },
  ],
  conflicts: ["com.example.conflict depends on com.vrchat.avatars < 3.7.0"],
  removeLegacyFiles: [],
  removeLegacyFolders: [],
  destructive: true,
  digest: "fnv-1a-def456",
};

const VALID_INSTALL_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.2",
  kind: "receipt",
  projectPath: "C:/proj",
  confirmedDigest: "fnv-1a-def456",
  requestedPackages: [{ packageId: "com.vrchat.avatars", version: null }],
  appliedItems: [
    { kind: "install", packageId: "com.vrchat.avatars", version: "3.7.5", reason: null },
  ],
};

const VALID_INSTALL_REJECTED = {
  schemaVersion: "vua.packages-ops/v0.2",
  kind: "rejected",
  guard: "preview_drift",
  code: "vua.packages.preview_drift",
  detail: "confirmed digest fnv-1a-def456 does not match the re-computed preview digest fnv-1a-abc123",
};

function previewInstallFrame(result: unknown): DesktopGatewaySuccessValueV1 {
  return asWire({ schemaVersion: "0.2", operation: "packages.previewInstall", result });
}

function acceptedInstallFrame(taskId: string): DesktopGatewaySuccessValueV1 {
  return asWire({ schemaVersion: "0.2", operation: "packages.applyInstall", taskId, correlationId: "c-2" });
}

/** A2 流编排 client:preview/apply/task.get 分支可控(与 A1 a1Client 分立,
 * 信封版本 0.2)。 */
function a2Client(overrides: {
  snapshot?: Parameters<typeof appSnapshot>[0];
  preview?: GatewayResult<DesktopGatewaySuccessValueV1>;
  apply?: GatewayResult<DesktopGatewaySuccessValueV1>;
  taskGet?: GatewayResult<DesktopGatewaySuccessValueV1>;
} = {}): { client: GatewayClient; emitCompleted: (taskId: string) => void } {
  const listeners = new Set<(event: unknown) => void>();
  const client: GatewayClient = {
    invoke: async (request) => {
      if (request.method === "app.snapshot") {
        return { ok: true, value: appSnapshot(overrides.snapshot ?? QUERY_AVAILABLE) };
      }
      if (request.method === "packages.previewInstall") {
        return overrides.preview ?? { ok: true, value: previewInstallFrame(VALID_INSTALL_PLAN) };
      }
      if (request.method === "packages.applyInstall") {
        return overrides.apply ?? { ok: true, value: acceptedInstallFrame("t-2") };
      }
      if (request.method === "task.get") {
        return overrides.taskGet ?? { ok: true, value: taskSnapshotValue("running") };
      }
      return { ok: false, error: { kind: "unavailable" } as const };
    },
    subscribe: (callback) => {
      listeners.add(callback as (event: unknown) => void);
      return () => {
        listeners.delete(callback as (event: unknown) => void);
      };
    },
  };
  return {
    client,
    emitCompleted: (taskId) => {
      for (const listener of listeners) {
        listener({ kind: "task.completed", taskId, payload: {} });
      }
    },
  };
}

describe("packages live port A2 install write face (026 v0.2 consumption)", () => {
  it("narrows the frozen nine-key v0.2 plan verbatim (family const stamped, version-selection semantics carried, conflict remove rows projected)", async () => {
    const { client } = a2Client();
    const port = createLivePackages(client);
    const outcome = await port.previewInstall("C:/proj", [{ packageId: "com.vrchat.avatars", version: null }]);
    expect(outcome).toEqual({ kind: "ok", plan: VALID_INSTALL_PLAN });
  });

  it("answers unavailable on the absence arm and passes the A2 preview_failed envelope code verbatim", async () => {
    const absent = a2Client({
      preview: applicationError("vua.packages.unavailable", "unavailable"),
    });
    expect(
      await createLivePackages(absent.client).previewInstall("C:/proj", [{ packageId: "com.a.b", version: null }]),
    ).toEqual({ kind: "unavailable" });
    const previewFailed = a2Client({
      preview: applicationError("vua.packages.preview_failed", "external_failure"),
    });
    expect(
      await createLivePackages(previewFailed.client).previewInstall("C:/proj", [{ packageId: "com.a.b", version: null }]),
    ).toEqual({ kind: "failed", code: "vua.packages.preview_failed" });
  });

  it("answers shape violation when a v0.1-stamped plan rides the v0.2 operation or a field is invented", async () => {
    // v0.1 戳冒充 v0.2 应答:消费窄化按字面量,v0.1 戳 = 形状不符
    const v01Stamp = a2Client({
      preview: {
        ok: true,
        value: previewInstallFrame({ ...VALID_INSTALL_PLAN, schemaVersion: "vua.packages-ops/v0.1" }),
      },
    });
    expect(
      await createLivePackages(v01Stamp.client).previewInstall("C:/proj", [{ packageId: "com.a.b", version: null }]),
    ).toEqual({ kind: "failed", code: "packages_shape_violation" });
    const invented = a2Client({
      preview: {
        ok: true,
        value: previewInstallFrame({ ...VALID_INSTALL_PLAN, cacheSourced: true }),
      },
    });
    expect(
      await createLivePackages(invented.client).previewInstall("C:/proj", [{ packageId: "com.a.b", version: null }]),
    ).toEqual({ kind: "failed", code: "packages_shape_violation" });
  });

  it("flips blocks.installs with the served packages.installOps row (one row serves both A2 methods; changes semantics untouched)", async () => {
    const withRow = await createLivePackages(
      a2Client({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.installOps", availability: "available" },
        ],
      }).client,
    ).snapshot();
    if (withRow.kind === "ready-p2") {
      expect(withRow.blocks.installs).toBe(true);
      expect(withRow.blocks.changes).toBe(false);
    }
    const withoutRow = await createLivePackages(a2Client().client).snapshot();
    if (withoutRow.kind === "ready-p2") {
      expect(withoutRow.blocks.installs).toBe(false);
    }
  });

  it("rides the task loop: v0.2 acceptance -> completed event -> succeeded snapshot with the install receipt payload", async () => {
    const taskGetSequence: GatewayResult<DesktopGatewaySuccessValueV1>[] = [
      { ok: true, value: taskSnapshotValue("running") },
      {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.2", operation: "packages.applyInstall", result: VALID_INSTALL_RECEIPT },
        }),
      },
    ];
    let taskGetCalls = 0;
    let subscribed = false;
    const listeners = new Set<(event: unknown) => void>();
    const client: GatewayClient = {
      invoke: async (request) => {
        if (request.method === "packages.applyInstall") {
          return { ok: true, value: acceptedInstallFrame("t-2") };
        }
        if (request.method === "task.get") {
          const answer: GatewayResult<DesktopGatewaySuccessValueV1> =
            taskGetSequence[Math.min(taskGetCalls, taskGetSequence.length - 1)] ?? {
              ok: false,
              error: { kind: "unavailable" },
            };
          taskGetCalls += 1;
          return answer;
        }
        return { ok: false, error: { kind: "unavailable" } as const };
      },
      subscribe: (callback) => {
        listeners.add(callback as (event: unknown) => void);
        subscribed = true;
        return () => {
          listeners.delete(callback as (event: unknown) => void);
        };
      },
    };
    const port = createLivePackages(client);
    const pending = port.applyInstall("C:/proj", [{ packageId: "com.vrchat.avatars", version: null }], "fnv-1a-def456");
    while (!subscribed) await new Promise((resolve) => setTimeout(resolve, 1));
    for (const listener of listeners) listener({ kind: "task.completed", taskId: "t-2", payload: {} });
    expect(await pending).toEqual({ kind: "ok", receipt: VALID_INSTALL_RECEIPT });
  });

  it("surfaces a rejected guard refusal (drift) as the typed rejection and never an error", async () => {
    const flow = a2Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.2", operation: "packages.applyInstall", result: VALID_INSTALL_REJECTED },
        }),
      },
    });
    const port = createLivePackages(flow.client);
    expect(
      await port.applyInstall("C:/proj", [{ packageId: "com.vrchat.avatars", version: null }], "fnv-1a-def456"),
    ).toEqual({ kind: "rejected", rejection: VALID_INSTALL_REJECTED });
  });

  it("answers a non-succeeded terminal verbatim and a v0.1-stamped acceptance as shape violation (never fabricates a receipt)", async () => {
    const failed = a2Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("failed", {
          error: {
            contractVersion: "0.1",
            code: "vua.provider.persistence_failed",
            category: "internal",
            messageKey: "errors.provider.persistence",
            recoverable: false,
            retryable: false,
            correlationId: "c-2",
          },
        }),
      },
    });
    expect(
      await createLivePackages(failed.client).applyInstall("C:/proj", [{ packageId: "com.a.b", version: null }], "d"),
    ).toEqual({ kind: "failed", code: "vua.provider.persistence_failed" });
    // 受理回执信封版本钉 0.2:0.1 戳 = 受理形状违规
    const badAcceptance = a2Client({
      apply: { ok: true, value: asWire({ schemaVersion: "0.1", operation: "packages.applyInstall", taskId: "t-2", correlationId: "c-2" }) },
    });
    expect(
      await createLivePackages(badAcceptance.client).applyInstall("C:/proj", [{ packageId: "com.a.b", version: null }], "d"),
    ).toEqual({ kind: "failed", code: "packages_apply_acceptance_shape" });
  });
});

/* ---- A3 本地包注册写面(026 v0.3 消费批) ---- */

const VALID_REGISTER_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.3",
  kind: "registered",
  packageRoot: "C:/LocalPackages/com.a.b-1.0.0",
};

const VALID_REGISTER_REJECTED = {
  schemaVersion: "vua.packages-ops/v0.3",
  kind: "rejected",
  guard: "execution_failed",
  code: "vua.packages.execution_failed",
  detail: "vua.vpm.local_package_register_failed: backend refused the set-add for C:/LocalPackages/com.a.b-1.0.0",
};

/** A3 流编排 client:register/task.get 分支可控(信封版本 0.3;与 A1/A2
 * 流编排 client 分立)。 */
function a3Client(overrides: {
  snapshot?: Parameters<typeof appSnapshot>[0];
  register?: GatewayResult<DesktopGatewaySuccessValueV1>;
  taskGet?: GatewayResult<DesktopGatewaySuccessValueV1>;
} = {}): GatewayClient {
  return fakeClient(async (request) => {
    if (request.method === "app.snapshot") {
      return { ok: true, value: appSnapshot(overrides.snapshot ?? QUERY_AVAILABLE) };
    }
    if (request.method === "packages.registerLocalPackage") {
      return overrides.register ?? { ok: true, value: acceptedRegisterFrame("t-3") };
    }
    if (request.method === "task.get") {
      return overrides.taskGet ?? { ok: true, value: taskSnapshotValue("running") };
    }
    return { ok: false, error: { kind: "unavailable" } as const };
  });
}

function acceptedRegisterFrame(taskId: string): DesktopGatewaySuccessValueV1 {
  return asWire({ schemaVersion: "0.3", operation: "packages.registerLocalPackage", taskId, correlationId: "c-3" });
}

describe("packages live port A3 register write face (026 v0.3 consumption)", () => {
  it("flips blocks.registers with the served packages.registerOps row (one row serves the one method; changes/installs semantics untouched)", async () => {
    const withRow = await createLivePackages(
      a3Client({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.installOps", availability: "available" },
          { operationId: "packages.registerOps", availability: "available" },
        ],
      }),
    ).snapshot();
    if (withRow.kind === "ready-p2") {
      expect(withRow.blocks.registers).toBe(true);
      expect(withRow.blocks.changes).toBe(false);
      expect(withRow.blocks.installs).toBe(true);
    }
    const withoutRow = await createLivePackages(a3Client()).snapshot();
    if (withoutRow.kind === "ready-p2") {
      expect(withoutRow.blocks.registers).toBe(false);
    }
    // 行存在但引擎后端未翻转访问器(availability unavailable)= 诚实缺席
    const rowUnavailable = await createLivePackages(
      a3Client({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.registerOps", availability: "unavailable" },
        ],
      }),
    ).snapshot();
    if (rowUnavailable.kind === "ready-p2") {
      expect(rowUnavailable.blocks.registers).toBe(false);
    }
  });

  it("rides the task loop: v0.3 acceptance -> completed event -> succeeded snapshot with the minimal registered receipt (idempotence = the same one success fact, no added flag no first/repeat distinction)", async () => {
    const taskGetSequence: GatewayResult<DesktopGatewaySuccessValueV1>[] = [
      { ok: true, value: taskSnapshotValue("running") },
      {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.3", operation: "packages.registerLocalPackage", result: VALID_REGISTER_RECEIPT },
        }),
      },
    ];
    let taskGetCalls = 0;
    let subscribed = false;
    const listeners = new Set<(event: unknown) => void>();
    const client: GatewayClient = {
      invoke: async (request) => {
        if (request.method === "packages.registerLocalPackage") {
          // 端口 verbatim 传输钉死:packageRoot 原样上呈,无 projectPath
          // 无 digest 位
          expect(request.params).toEqual({ packageRoot: "C:/LocalPackages/com.a.b-1.0.0" });
          return { ok: true, value: acceptedRegisterFrame("t-3") };
        }
        if (request.method === "task.get") {
          const answer: GatewayResult<DesktopGatewaySuccessValueV1> =
            taskGetSequence[Math.min(taskGetCalls, taskGetSequence.length - 1)] ?? {
              ok: false,
              error: { kind: "unavailable" },
            };
          taskGetCalls += 1;
          return answer;
        }
        return { ok: false, error: { kind: "unavailable" } as const };
      },
      subscribe: (callback) => {
        listeners.add(callback as (event: unknown) => void);
        subscribed = true;
        return () => {
          listeners.delete(callback as (event: unknown) => void);
        };
      },
    };
    const port = createLivePackages(client);
    const pending = port.registerLocalPackage("C:/LocalPackages/com.a.b-1.0.0");
    while (!subscribed) await new Promise((resolve) => setTimeout(resolve, 1));
    for (const listener of listeners) listener({ kind: "task.completed", taskId: "t-3", payload: {} });
    const first = await pending;
    expect(first).toEqual({ kind: "ok", receipt: VALID_REGISTER_RECEIPT });
    // 幂等第二轮(AlreadyAdded):收据形状与首轮同一成功事实
    const second = await port.registerLocalPackage("C:/LocalPackages/com.a.b-1.0.0");
    expect(second).toEqual(first);
  });

  it("surfaces the typed rejection (execution_failed with the original port code in detail) as the typed rejection and never an error", async () => {
    const client = a3Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.3", operation: "packages.registerLocalPackage", result: VALID_REGISTER_REJECTED },
        }),
      },
    });
    const outcome = await createLivePackages(client).registerLocalPackage("C:/LocalPackages/com.a.b-1.0.0");
    expect(outcome).toEqual({ kind: "rejected", rejection: VALID_REGISTER_REJECTED });
  });

  it("folds the capability-missing acceptance to failed verbatim, a non-succeeded terminal verbatim, and a 0.2-stamped acceptance as shape violation (never fabricates a receipt)", async () => {
    // 能力缺席在路由层答 vua.vpm.capability_missing(绝不进任务)——照
    // 原词 failed,缺席臂(vua.packages.unavailable)折叠 unavailable
    expect(
      await createLivePackages(
        a3Client({ register: applicationError("vua.vpm.capability_missing", "unavailable") }),
      ).registerLocalPackage("C:/x"),
    ).toEqual({ kind: "failed", code: "vua.vpm.capability_missing" });
    expect(
      await createLivePackages(
        a3Client({ register: applicationError("vua.packages.unavailable", "unavailable") }),
      ).registerLocalPackage("C:/x"),
    ).toEqual({ kind: "unavailable" });
    // 任务非成功终态:error.code 原词上呈(恢复非终态绝不隐式续传)
    const failed = a3Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("failed", {
          error: {
            contractVersion: "0.1",
            code: "vua.vpm.local_package_register_failed",
            category: "external_failure",
            messageKey: "errors.vpm.localPackageRegisterFailed",
            recoverable: false,
            retryable: false,
            correlationId: "c-3",
          },
        }),
      },
    });
    expect(
      await createLivePackages(failed).registerLocalPackage("C:/x"),
    ).toEqual({ kind: "failed", code: "vua.vpm.local_package_register_failed" });
    // 受理回执信封版本钉 0.3:0.2 戳 = 受理形状违规
    const badAcceptance = a3Client({
      register: { ok: true, value: asWire({ schemaVersion: "0.2", operation: "packages.registerLocalPackage", taskId: "t-3", correlationId: "c-3" }) },
    });
    expect(
      await createLivePackages(badAcceptance).registerLocalPackage("C:/x"),
    ).toEqual({ kind: "failed", code: "packages_apply_acceptance_shape" });
  });

  it("answers shape violation when the registered receipt invents a field (minimal honest audit shape — no timestamp, no package.json content, no env path) or the receipt is 0.2-stamped", async () => {
    const invented = a3Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: {
            schemaVersion: "0.3",
            operation: "packages.registerLocalPackage",
            result: { ...VALID_REGISTER_RECEIPT, registeredAt: "2026-09-19T00:00:00Z" },
          },
        }),
      },
    });
    expect(
      await createLivePackages(invented).registerLocalPackage("C:/x"),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
    const staleStamp = a3Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: {
            schemaVersion: "0.3",
            operation: "packages.registerLocalPackage",
            result: { ...VALID_REGISTER_RECEIPT, schemaVersion: "vua.packages-ops/v0.2" },
          },
        }),
      },
    });
    expect(
      await createLivePackages(staleStamp).registerLocalPackage("C:/x"),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
  });
});

/* ---- A4 仓库订阅增删写面(026 v0.4 消费批) ---- */

const VALID_REMOTE_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.4",
  kind: "repoReceipt",
  repoType: "remote",
  url: "https://vpm.example/index.json",
  name: "Example Repo",
};

const VALID_LOCAL_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.4",
  kind: "repoReceipt",
  repoType: "local",
  path: "C:/Repos/local-curations",
  name: "Local Curations",
};

const VALID_REMOVED_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.4",
  kind: "removed",
  repoId: "repo-example",
};

const VALID_REPO_REJECTED = {
  schemaVersion: "vua.packages-ops/v0.4",
  kind: "rejected",
  guard: "execution_failed",
  code: "vua.packages.execution_failed",
  detail: "vua.vpm.repo_invalid: backend refused the duplicate subscription for https://vpm.example/index.json",
};

/** A4 流编排 client:三写方法/task.get 分支可控(信封版本 0.4;与 A1/A2/
 * A3 流编排 client 分立)。 */
function a4Client(overrides: {
  snapshot?: Parameters<typeof appSnapshot>[0];
  addRemote?: GatewayResult<DesktopGatewaySuccessValueV1>;
  addLocal?: GatewayResult<DesktopGatewaySuccessValueV1>;
  remove?: GatewayResult<DesktopGatewaySuccessValueV1>;
  taskGet?: GatewayResult<DesktopGatewaySuccessValueV1>;
} = {}): GatewayClient {
  return fakeClient(async (request) => {
    if (request.method === "app.snapshot") {
      return { ok: true, value: appSnapshot(overrides.snapshot ?? QUERY_AVAILABLE) };
    }
    if (request.method === "packages.addRemoteRepo") {
      return overrides.addRemote ?? { ok: true, value: acceptedRepoFrame("t-4", "packages.addRemoteRepo") };
    }
    if (request.method === "packages.addLocalRepo") {
      return overrides.addLocal ?? { ok: true, value: acceptedRepoFrame("t-4", "packages.addLocalRepo") };
    }
    if (request.method === "packages.removeRepo") {
      return overrides.remove ?? { ok: true, value: acceptedRepoFrame("t-4", "packages.removeRepo") };
    }
    if (request.method === "task.get") {
      return overrides.taskGet ?? { ok: true, value: taskSnapshotValue("running") };
    }
    return { ok: false, error: { kind: "unavailable" } as const };
  });
}

function acceptedRepoFrame(taskId: string, operation: string): DesktopGatewaySuccessValueV1 {
  return asWire({ schemaVersion: "0.4", operation, taskId, correlationId: "c-4" });
}

describe("packages live port A4 repo write face (026 v0.4 consumption)", () => {
  it("flips blocks.repoWrites with the served packages.repoOps row (one row serves the three methods; changes/installs/registers semantics untouched)", async () => {
    const withRow = await createLivePackages(
      a4Client({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.installOps", availability: "available" },
          { operationId: "packages.registerOps", availability: "available" },
          { operationId: "packages.repoOps", availability: "available" },
        ],
      }),
    ).snapshot();
    if (withRow.kind === "ready-p2") {
      expect(withRow.blocks.repoWrites).toBe(true);
      expect(withRow.blocks.changes).toBe(false);
      expect(withRow.blocks.installs).toBe(true);
      expect(withRow.blocks.registers).toBe(true);
    }
    const withoutRow = await createLivePackages(a4Client()).snapshot();
    if (withoutRow.kind === "ready-p2") {
      expect(withoutRow.blocks.repoWrites).toBe(false);
    }
    // 行存在但引擎后端三独立位均未声明(availability unavailable)= 诚实缺席
    const rowUnavailable = await createLivePackages(
      a4Client({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.repoOps", availability: "unavailable" },
        ],
      }),
    ).snapshot();
    if (rowUnavailable.kind === "ready-p2") {
      expect(rowUnavailable.blocks.repoWrites).toBe(false);
    }
  });

  it("rides the task loop for all three methods: v0.4 acceptance -> succeeded snapshot with the exact-five-keys remote/local receipts and the exact-three-keys removed receipt (key sets mutually exclusive; verbatim params; ok rides a refresh broadcast)", async () => {
    const taskGetSequence: GatewayResult<DesktopGatewaySuccessValueV1>[] = [
      { ok: true, value: taskSnapshotValue("running") },
      {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.4", operation: "packages.addRemoteRepo", result: VALID_REMOTE_RECEIPT },
        }),
      },
    ];
    let taskGetCalls = 0;
    let broadcasts = 0;
    const listeners = new Set<(event: unknown) => void>();
    const client: GatewayClient = {
      invoke: async (request) => {
        if (request.method === "packages.addRemoteRepo") {
          // 端口 verbatim 传输钉死:params 恰 {url, name},无 projectPath
          // 无 digest 位
          expect(request.params).toEqual({ url: "https://vpm.example/index.json", name: "Example Repo" });
          return { ok: true, value: acceptedRepoFrame("t-4", "packages.addRemoteRepo") };
        }
        if (request.method === "task.get") {
          const answer: GatewayResult<DesktopGatewaySuccessValueV1> =
            taskGetSequence[Math.min(taskGetCalls, taskGetSequence.length - 1)] ?? {
              ok: false,
              error: { kind: "unavailable" },
            };
          taskGetCalls += 1;
          return answer;
        }
        return { ok: false, error: { kind: "unavailable" } as const };
      },
      subscribe: (callback) => {
        listeners.add(callback as (event: unknown) => void);
        return () => {
          listeners.delete(callback as (event: unknown) => void);
        };
      },
    };
    const port = createLivePackages(client);
    port.subscribe(() => {
      broadcasts += 1;
    });
    const pending = port.addRemoteRepo("https://vpm.example/index.json", "Example Repo");
    while (listeners.size === 0) await new Promise((resolve) => setTimeout(resolve, 1));
    for (const listener of listeners) listener({ kind: "task.completed", taskId: "t-4", payload: {} });
    // 收据逐键钉死:五键闭集 {schemaVersion, kind, repoType, url, name},
    // 无时间戳无行位无清单内容
    expect(await pending).toEqual({ kind: "ok", receipt: VALID_REMOTE_RECEIPT });
    expect(Object.keys(VALID_REMOTE_RECEIPT).sort()).toEqual(
      ["kind", "name", "repoType", "schemaVersion", "url"],
    );
    // ok 收据骑刷新广播(订阅面已变,列表按新事实重取);广播链异步,
    // 先排空任务队列再断言
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(broadcasts).toBeGreaterThan(0);

    // local 变体(键集与 remote 互斥:repoType local + path)与 removed
    // 三键收据
    const localClient = a4Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.4", operation: "packages.addLocalRepo", result: VALID_LOCAL_RECEIPT },
        }),
      },
    });
    expect(await createLivePackages(localClient).addLocalRepo("C:/Repos/local-curations", "Local Curations"))
      .toEqual({ kind: "ok", receipt: VALID_LOCAL_RECEIPT });
    expect(Object.keys(VALID_LOCAL_RECEIPT).sort()).toEqual(
      ["kind", "name", "path", "repoType", "schemaVersion"],
    );
    const removeClient = a4Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.4", operation: "packages.removeRepo", result: VALID_REMOVED_RECEIPT },
        }),
      },
    });
    expect(await createLivePackages(removeClient).removeRepo("repo-example"))
      .toEqual({ kind: "ok", receipt: VALID_REMOVED_RECEIPT });
    expect(Object.keys(VALID_REMOVED_RECEIPT).sort()).toEqual(["kind", "repoId", "schemaVersion"]);
  });

  it("surfaces the typed rejection (execution_failed with the original port code in detail) as the typed rejection and never an error; the add face claims NO idempotence - the duplicate refusal travels as the rejection", async () => {
    const client = a4Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.4", operation: "packages.addRemoteRepo", result: VALID_REPO_REJECTED },
        }),
      },
    });
    const outcome = await createLivePackages(client).addRemoteRepo("https://vpm.example/index.json", "Example Repo");
    expect(outcome).toEqual({ kind: "rejected", rejection: VALID_REPO_REJECTED });
  });

  it("folds the capability-missing acceptance to failed verbatim, the absence arm to unavailable, a non-succeeded terminal verbatim, and a 0.3-stamped acceptance as shape violation (never fabricates a receipt)", async () => {
    // 能力缺席在路由层按方法答 vua.vpm.capability_missing(绝不进任务)
    // ——照原词 failed;缺席臂(vua.packages.unavailable)折叠 unavailable
    expect(
      await createLivePackages(
        a4Client({ addRemote: applicationError("vua.vpm.capability_missing", "unavailable") }),
      ).addRemoteRepo("https://vpm.example/index.json", "Example Repo"),
    ).toEqual({ kind: "failed", code: "vua.vpm.capability_missing" });
    expect(
      await createLivePackages(
        a4Client({ remove: applicationError("vua.packages.unavailable", "unavailable") }),
      ).removeRepo("repo-example"),
    ).toEqual({ kind: "unavailable" });
    // 任务非成功终态:error.code 原词上呈(恢复非终态绝不隐式续传)
    const failed = a4Client({
      remove: { ok: true, value: acceptedRepoFrame("t-4", "packages.removeRepo") },
      taskGet: {
        ok: true,
        value: taskSnapshotValue("failed", {
          error: {
            contractVersion: "0.1",
            code: "vua.vpm.repo_write_failed",
            category: "external_failure",
            messageKey: "errors.vpm.repoWriteFailed",
            recoverable: false,
            retryable: false,
            correlationId: "c-4",
          },
        }),
      },
    });
    expect(
      await createLivePackages(failed).removeRepo("repo-example"),
    ).toEqual({ kind: "failed", code: "vua.vpm.repo_write_failed" });
    // 受理回执信封版本钉 0.4:0.3 戳 = 受理形状违规
    const badAcceptance = a4Client({
      addLocal: { ok: true, value: asWire({ schemaVersion: "0.3", operation: "packages.addLocalRepo", taskId: "t-4", correlationId: "c-4" }) },
    });
    expect(
      await createLivePackages(badAcceptance).addLocalRepo("C:/Repos", "Local"),
    ).toEqual({ kind: "failed", code: "packages_apply_acceptance_shape" });
  });

  it("answers shape violation when a receipt invents a field (no timestamps, no removed-row snapshots) or the receipt variant does not match the method (remote answered with the local variant)", async () => {
    // 发明字段(时间戳)= 形状违规(最小诚实审计形状)
    const invented = a4Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: {
            schemaVersion: "0.4",
            operation: "packages.addRemoteRepo",
            result: { ...VALID_REMOTE_RECEIPT, addedAt: "2026-09-19T00:00:00Z" },
          },
        }),
      },
    });
    expect(
      await createLivePackages(invented).addRemoteRepo("https://vpm.example/index.json", "Example Repo"),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
    // 被删行快照 = 发明事实(removed 回显即审计链,三键之外一字段即违规)
    const snapshotInvented = a4Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: {
            schemaVersion: "0.4",
            operation: "packages.removeRepo",
            result: { ...VALID_REMOVED_RECEIPT, removedRow: { repoId: "repo-example", name: "Example Repo" } },
          },
        }),
      },
    });
    expect(
      await createLivePackages(snapshotInvented).removeRepo("repo-example"),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
    // 收据变体与请求方法不对应(remote 请求答 local 变体)= 服务端词面
    // 违反,诚实降级不冒充成功
    const wrongVariant = a4Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.4", operation: "packages.addRemoteRepo", result: VALID_LOCAL_RECEIPT },
        }),
      },
    });
    expect(
      await createLivePackages(wrongVariant).addRemoteRepo("https://vpm.example/index.json", "Example Repo"),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
    // 旧戳收据(v0.3)= 形状违规
    const staleStamp = a4Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: {
            schemaVersion: "0.4",
            operation: "packages.removeRepo",
            result: { ...VALID_REMOVED_RECEIPT, schemaVersion: "vua.packages-ops/v0.3" },
          },
        }),
      },
    });
    expect(
      await createLivePackages(staleStamp).removeRepo("repo-example"),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
  });
});

/* ---- A5 项目创建写面(026 v0.5 消费批) ---- */

const VALID_CREATED_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.5",
  kind: "created",
  projectId: "proj-new-1",
  projectPath: "C:/Users/me/VRC projects/New World",
};

/** 重复目录执行拒绝:guard 三值闭集 execution_failed + 原端口码
 * (vua.vpm.template_missing 库路径腿)在 detail 原词溯源,code 锁族 */
const VALID_CREATE_REJECTED = {
  schemaVersion: "vua.packages-ops/v0.5",
  kind: "rejected",
  guard: "execution_failed",
  code: "vua.packages.execution_failed",
  detail: "errors.vpm.projectExists: the target path already exists",
};

function acceptedCreateFrame(taskId: string, operation: string): DesktopGatewaySuccessValueV1 {
  return asWire({ schemaVersion: "0.5", operation, taskId, correlationId: "c-5" });
}

/** A5 流编排 client:单写方法/task.get 分支可控(信封版本 0.5;与 A1–A4
 * 流编排 client 分立)。 */
function a5Client(overrides: {
  snapshot?: Parameters<typeof appSnapshot>[0];
  create?: GatewayResult<DesktopGatewaySuccessValueV1>;
  taskGet?: GatewayResult<DesktopGatewaySuccessValueV1>;
} = {}): GatewayClient {
  return fakeClient(async (request) => {
    if (request.method === "app.snapshot") {
      return { ok: true, value: appSnapshot(overrides.snapshot ?? QUERY_AVAILABLE) };
    }
    if (request.method === "packages.createProject") {
      return overrides.create ?? { ok: true, value: acceptedCreateFrame("t-5", "packages.createProject") };
    }
    if (request.method === "task.get") {
      return overrides.taskGet ?? { ok: true, value: taskSnapshotValue("running") };
    }
    return { ok: false, error: { kind: "unavailable" } as const };
  });
}

describe("packages live port A5 create write face (026 v0.5 consumption)", () => {
  it("flips blocks.creates with the served packages.createOps row (one row serves the one method; changes/installs/registers/repoWrites semantics untouched; absent or unavailable row stays false)", async () => {
    const withRow = await createLivePackages(
      a5Client({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.installOps", availability: "available" },
          { operationId: "packages.registerOps", availability: "available" },
          { operationId: "packages.repoOps", availability: "available" },
          { operationId: "packages.createOps", availability: "available" },
        ],
      }),
    ).snapshot();
    if (withRow.kind === "ready-p2") {
      expect(withRow.blocks.creates).toBe(true);
      expect(withRow.blocks.changes).toBe(false);
      expect(withRow.blocks.installs).toBe(true);
      expect(withRow.blocks.registers).toBe(true);
      expect(withRow.blocks.repoWrites).toBe(true);
    }
    const withoutRow = await createLivePackages(a5Client()).snapshot();
    if (withoutRow.kind === "ready-p2") {
      expect(withoutRow.blocks.creates).toBe(false);
    }
    // 行存在但引擎后端 create 位未声明(availability unavailable)= 诚实缺席
    const rowUnavailable = await createLivePackages(
      a5Client({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.createOps", availability: "unavailable" },
        ],
      }),
    ).snapshot();
    if (rowUnavailable.kind === "ready-p2") {
      expect(rowUnavailable.blocks.creates).toBe(false);
    }
  });

  it("rides the task loop: v0.5 acceptance -> succeeded snapshot with the exact-four-keys created receipt (ProjectRef projection; verbatim three-key params with the REQUIRED-nullable template on both arms; ok rides a refresh broadcast)", async () => {
    const taskGetSequence: GatewayResult<DesktopGatewaySuccessValueV1>[] = [
      { ok: true, value: taskSnapshotValue("running") },
      {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.5", operation: "packages.createProject", result: VALID_CREATED_RECEIPT },
        }),
      },
    ];
    let taskGetCalls = 0;
    let broadcasts = 0;
    const listeners = new Set<(event: unknown) => void>();
    const client: GatewayClient = {
      invoke: async (request) => {
        if (request.method === "packages.createProject") {
          // 端口 verbatim 传输钉死:params 恰 {parent, name, template},
          // null 原样透传,无 projectPath 无 digest 位
          expect(request.params).toEqual({
            parent: "C:/Users/me/VRC projects",
            name: "New World",
            template: null,
          });
          return { ok: true, value: acceptedCreateFrame("t-5", "packages.createProject") };
        }
        if (request.method === "task.get") {
          const answer: GatewayResult<DesktopGatewaySuccessValueV1> =
            taskGetSequence[Math.min(taskGetCalls, taskGetSequence.length - 1)] ?? {
              ok: false,
              error: { kind: "unavailable" },
            };
          taskGetCalls += 1;
          return answer;
        }
        return { ok: false, error: { kind: "unavailable" } as const };
      },
      subscribe: (callback) => {
        listeners.add(callback as (event: unknown) => void);
        return () => {
          listeners.delete(callback as (event: unknown) => void);
        };
      },
    };
    const port = createLivePackages(client);
    port.subscribe(() => {
      broadcasts += 1;
    });
    const pending = port.createProject("C:/Users/me/VRC projects", "New World", null);
    while (listeners.size === 0) await new Promise((resolve) => setTimeout(resolve, 1));
    for (const listener of listeners) listener({ kind: "task.completed", taskId: "t-5", payload: {} });
    // 收据逐键钉死:四键闭集 {schemaVersion, kind, projectId, projectPath}
    // ——无创建时间戳无复制统计无包清单
    expect(await pending).toEqual({ kind: "ok", receipt: VALID_CREATED_RECEIPT });
    expect(Object.keys(VALID_CREATED_RECEIPT).sort()).toEqual(
      ["kind", "projectId", "projectPath", "schemaVersion"],
    );
    // ok 收据骑刷新广播(创建即在册,在册列表按新事实重取);广播链异步,
    // 先排空任务队列再断言
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(broadcasts).toBeGreaterThan(0);

    // 非空 template verbatim 臂
    const withTemplate = a5Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.5", operation: "packages.createProject", result: VALID_CREATED_RECEIPT },
        }),
      },
    });
    expect(
      await createLivePackages(withTemplate).createProject("C:/Users/me/VRC projects", "New Avatar", "Avatar"),
    ).toEqual({ kind: "ok", receipt: VALID_CREATED_RECEIPT });
  });

  it("surfaces the typed rejection (execution_failed with the original port code in detail) as the typed rejection and never an error; creation claims NO idempotence - the duplicate-directory refusal travels as the rejection", async () => {
    const client = a5Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.5", operation: "packages.createProject", result: VALID_CREATE_REJECTED },
        }),
      },
    });
    const outcome = await createLivePackages(client).createProject(
      "C:/Users/me/VRC projects",
      "New World",
      null,
    );
    expect(outcome).toEqual({ kind: "rejected", rejection: VALID_CREATE_REJECTED });
  });

  it("folds the capability-missing acceptance to failed verbatim, the absence arm to unavailable, a non-succeeded terminal verbatim, and a 0.4-stamped acceptance as shape violation (never fabricates a receipt)", async () => {
    // 能力缺席在路由层答通用 vua.vpm.capability_missing(create 位假绝不
    // 进任务)——照原词 failed;缺席臂(vua.packages.unavailable)折叠
    // unavailable
    expect(
      await createLivePackages(
        a5Client({ create: applicationError("vua.vpm.capability_missing", "unavailable") }),
      ).createProject("C:/p", "New World", null),
    ).toEqual({ kind: "failed", code: "vua.vpm.capability_missing" });
    expect(
      await createLivePackages(
        a5Client({ create: applicationError("vua.packages.unavailable", "unavailable") }),
      ).createProject("C:/p", "New World", null),
    ).toEqual({ kind: "unavailable" });
    // 任务非成功终态:error.code 原词上呈(恢复非终态绝不隐式续传)
    const failed = a5Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("failed", {
          error: {
            contractVersion: "0.1",
            code: "vua.vpm.backend_unavailable",
            category: "external_failure",
            messageKey: "errors.vpm.backendUnavailable",
            recoverable: false,
            retryable: false,
            correlationId: "c-5",
          },
        }),
      },
    });
    expect(
      await createLivePackages(failed).createProject("C:/p", "New World", null),
    ).toEqual({ kind: "failed", code: "vua.vpm.backend_unavailable" });
    // 受理回执信封版本钉 0.5:0.4 戳 = 受理形状违规
    const badAcceptance = a5Client({
      create: { ok: true, value: asWire({ schemaVersion: "0.4", operation: "packages.createProject", taskId: "t-5", correlationId: "c-5" }) },
    });
    expect(
      await createLivePackages(badAcceptance).createProject("C:/p", "New World", null),
    ).toEqual({ kind: "failed", code: "packages_apply_acceptance_shape" });
  });

  it("answers shape violation when the receipt invents a field (no timestamps, no copy statistics, no package manifest), the rejected guard leaves the three-value closed set, the code leaves the vua.packages. family, or the receipt is 0.4-stamped", async () => {
    // 发明创建时间戳 = 形状违规
    const invented = a5Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: {
            schemaVersion: "0.5",
            operation: "packages.createProject",
            result: { ...VALID_CREATED_RECEIPT, createdAt: "2026-09-19T00:00:00Z" },
          },
        }),
      },
    });
    expect(
      await createLivePackages(invented).createProject("C:/p", "New World", null),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
    // guard 词外值 = 形状违规(三值闭集复用 A1–A4 零新增)
    const foreignGuard = a5Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { ...VALID_CREATE_REJECTED, guard: "digest_mismatch" },
        }),
      },
    });
    expect(
      await createLivePackages(foreignGuard).createProject("C:/p", "New World", null),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
    // code 锁族 ^vua\.packages\.(原端口码 vua.vpm.* 只在 detail 溯源)
    const foreignCode = a5Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { ...VALID_CREATE_REJECTED, code: "vua.vpm.template_missing" },
        }),
      },
    });
    expect(
      await createLivePackages(foreignCode).createProject("C:/p", "New World", null),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
    // 旧戳收据(v0.4)= 形状违规
    const staleStamp = a5Client({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: {
            schemaVersion: "0.5",
            operation: "packages.createProject",
            result: { ...VALID_CREATED_RECEIPT, schemaVersion: "vua.packages-ops/v0.4" },
          },
        }),
      },
    });
    expect(
      await createLivePackages(staleStamp).createProject("C:/p", "New World", null),
    ).toEqual({ kind: "failed", code: "packages_apply_result_shape" });
  });
});

function assertReadyP2(view: PackagesView): void {
  expect(view.kind).toBe("ready-p2");
}

describe("packages live port (027 F2 consumption)", () => {
  it("shares overlapping catalog reads without retaining stale facts or merging different scopes", async () => {
    const requests: DesktopGatewayRequestV1[] = [];
    const port = createLivePackages(clientWith({ snapshot: F2_AVAILABLE, onRepoCatalogRequest: request => { requests.push(request); } }));
    const [first, second] = await Promise.all([port.repoCatalog(null, null), port.repoCatalog(null, null), port.repoCatalog("official", null)]);
    expect(first).toEqual(second);
    expect(requests).toHaveLength(2);
    await port.repoCatalog(null, null);
    expect(requests).toHaveLength(3);
  });
  it("flips the repoCatalog block from the repoCatalogOps served row (declared-none stays honestly hidden)", async () => {
    const hidden = await createLivePackages(clientWith()).snapshot();
    assertReadyP2(hidden);
    if (hidden.kind !== "ready-p2") return;
    expect(hidden.blocks.repoCatalog).toBe(false);

    const port = createLivePackages(clientWith({ snapshot: F2_AVAILABLE }));
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.blocks.repoCatalog).toBe(true);
  });

  it("carries the frozen per-repo rows verbatim after stripping the family const (cacheSourced and cached=false preserved)", async () => {
    const port = createLivePackages(clientWith({ snapshot: F2_AVAILABLE }));
    const outcome = await port.repoCatalog(null, null);
    expect(outcome.kind).toBe("ok");
    if (outcome.kind !== "ok") return;
    expect(outcome.result).toEqual(VALID_REPO_CATALOG_FACTS);
    expect(outcome.result).not.toHaveProperty("schemaVersion");
  });

  it("transports the two-key REQUIRED-nullable params verbatim (null browse and scoped+filtered shapes)", async () => {
    let seen: unknown = null;
    const base = {
      onRepoCatalogRequest: (request: DesktopGatewayRequestV1) => {
        seen = (request as { params: unknown }).params;
      },
    };
    await createLivePackages(clientWith({ ...base, snapshot: F2_AVAILABLE })).repoCatalog(null, null);
    expect(seen).toEqual({ repoId: null, packageIds: null });

    seen = null;
    await createLivePackages(
      clientWith({ ...base, snapshot: F2_AVAILABLE }),
    ).repoCatalog("official", ["com.anatawa12.avatar-optimizer", "com.vrchat.avatars"]);
    expect(seen).toEqual({
      repoId: "official",
      packageIds: ["com.anatawa12.avatar-optimizer", "com.vrchat.avatars"],
    });
  });

  it("surfaces the port refusal verbatim (repo_not_found travels, no read-face fold)", async () => {
    const port = createLivePackages(
      clientWith({
        snapshot: F2_AVAILABLE,
        repoCatalogResult: applicationError("vua.vpm.repo_not_found", "validation"),
      }),
    );
    expect(await port.repoCatalog("ghost", null)).toEqual({
      kind: "failed",
      code: "vua.vpm.repo_not_found",
    });
  });

  it("answers unavailable for the absence arm and shape violation for invented/missing facts", async () => {
    const absent = createLivePackages(
      clientWith({ snapshot: F2_AVAILABLE, repoCatalogResult: UNAVAILABLE_ERROR }),
    );
    expect(await absent.repoCatalog(null, null)).toEqual({ kind: "unavailable" });

    const invented = createLivePackages(
      clientWith({
        snapshot: F2_AVAILABLE,
        repoCatalogResult: {
          ok: true,
          value: repoCatalogFrame({
            schemaVersion: "vua.packages-repo-catalog/v0.1",
            repos: [
              {
                repoId: "official",
                name: "Official",
                cached: true,
                packages: [
                  // author = 冻结词面刻意缺席(单事实源裁决);发明即形状不符
                  {
                    packageId: "com.x",
                    displayName: null,
                    description: null,
                    latestVersion: null,
                    versionCount: 1,
                    author: "someone",
                  },
                ],
              },
            ],
            cacheSourced: false,
          }),
        },
      }),
    );
    expect(await invented.repoCatalog(null, null)).toEqual({
      kind: "failed",
      code: "packages_shape_violation",
    });

    const missingDisclosure = createLivePackages(
      clientWith({
        snapshot: F2_AVAILABLE,
        repoCatalogResult: {
          ok: true,
          value: repoCatalogFrame({
            schemaVersion: "vua.packages-repo-catalog/v0.1",
            repos: [],
          }),
        },
      }),
    );
    expect(await missingDisclosure.repoCatalog(null, null)).toEqual({
      kind: "failed",
      code: "packages_shape_violation",
    });
  });
});

/** ---- 027 F5 消费批:packages.listTemplates 模板条目枚举读面 ---- */

describe("packages live port (027 F5 consumption)", () => {
  it("flips the templates block from the templatesOps served row (declared-none stays honestly hidden)", async () => {
    const hidden = await createLivePackages(clientWith()).snapshot();
    assertReadyP2(hidden);
    if (hidden.kind !== "ready-p2") return;
    expect(hidden.blocks.templates).toBe(false);

    const port = createLivePackages(clientWith({ snapshot: F5_AVAILABLE }));
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.blocks.templates).toBe(true);
  });

  it("carries the frozen id-ascending rows verbatim after stripping the family const (order untouched, no invented fields)", async () => {
    const port = createLivePackages(
      clientWith({
        snapshot: F5_AVAILABLE,
        templatesResult: {
          ok: true,
          value: templatesFrame({
            schemaVersion: "vua.packages-templates/v0.1",
            templates: VALID_TEMPLATE_ROWS,
          }),
        },
      }),
    );
    const outcome = await port.listTemplates();
    expect(outcome.kind).toBe("ok");
    if (outcome.kind !== "ok") return;
    expect(outcome.result).toEqual({ templates: VALID_TEMPLATE_ROWS });
    expect(outcome.result).not.toHaveProperty("schemaVersion");
    // 冻结呈现事实:id 升序由服务端投影,客户端不重排
    expect(outcome.result.templates.map((row) => row.id)).toEqual([
      "com.vrchat.avatars",
      "com.vrchat.base",
      "com.vrchat.worlds",
    ]);
  });

  it("answers an honest empty array for zero templates (missing directory root is a fact, not an error)", async () => {
    const port = createLivePackages(clientWith({ snapshot: F5_AVAILABLE }));
    const outcome = await port.listTemplates();
    expect(outcome).toEqual({ kind: "ok", result: { templates: [] } });
  });

  it("transports the empty-closed-set params (environment-level face, any key would be a wire violation)", async () => {
    let seen: unknown = null;
    const port = createLivePackages(
      clientWith({
        snapshot: F5_AVAILABLE,
        onListTemplatesRequest: (request) => {
          seen = (request as { params: unknown }).params;
        },
      }),
    );
    await port.listTemplates();
    // 空闭集 = 环境级配置面(listRepos 先例);live 构造恒 {} 绝不带键
    expect(seen).toEqual({});
  });

  it("surfaces typed refusals verbatim and the absence arm as unavailable (never folded into an empty listing)", async () => {
    const refused = createLivePackages(
      clientWith({
        snapshot: F5_AVAILABLE,
        templatesResult: applicationError("vua.vpm.capability_missing", "validation"),
      }),
    );
    expect(await refused.listTemplates()).toEqual({
      kind: "failed",
      code: "vua.vpm.capability_missing",
    });

    const absent = createLivePackages(
      clientWith({ snapshot: F5_AVAILABLE, templatesResult: UNAVAILABLE_ERROR }),
    );
    expect(await absent.listTemplates()).toEqual({ kind: "unavailable" });
  });

  it("answers shape violation for invented or empty-name rows (frozen two-key closed set, wire lock stays authoritative)", async () => {
    const invented = createLivePackages(
      clientWith({
        snapshot: F5_AVAILABLE,
        templatesResult: {
          ok: true,
          value: templatesFrame({
            schemaVersion: "vua.packages-templates/v0.1",
            // description = 冻结词面刻意缺席(无 v0.1 生产者);发明即形状不符
            templates: [{ id: "com.x", name: "com.x", description: "friendly" }],
          }),
        },
      }),
    );
    expect(await invented.listTemplates()).toEqual({
      kind: "failed",
      code: "packages_shape_violation",
    });

    const emptyName = createLivePackages(
      clientWith({
        snapshot: F5_AVAILABLE,
        templatesResult: {
          ok: true,
          value: templatesFrame({
            schemaVersion: "vua.packages-templates/v0.1",
            templates: [{ id: "com.x", name: "" }],
          }),
        },
      }),
    );
    expect(await emptyName.listTemplates()).toEqual({
      kind: "failed",
      code: "packages_shape_violation",
    });
  });
});

/** ---- 027 F3 消费批:packages.listInstalled 双族协商(v0.1/v0.2) ---- */

/** v0.2 族应答帧:路由盖戳族常量 vua.packages-installed/v0.2,行 = 五键
 *  闭集(v0.1 三键零变动＋判定对必带可空),cacheSourced 必带披露 */
function installedFrameV02(result: unknown) {
  return asWire({
    schemaVersion: "0.1",
    operation: "packages.listInstalled",
    result,
  });
}

const VALID_ROWS_V02 = [
  {
    packageId: "com.vrchat.avatars",
    version: "3.7.4",
    dependencies: ["com.vrchat.base"],
    latestVersion: "3.7.6",
    updateAvailable: true,
  },
  {
    packageId: "com.vrchat.base",
    version: "3.7.2",
    dependencies: [],
    latestVersion: null,
    updateAvailable: null,
  },
];

describe("packages live port (027 F3 consumption)", () => {
  it("carries v0.2-family judgment rows and the cacheSourced disclosure into the view (family const consumed, never guessed)", async () => {
    const port = createLivePackages(
      clientWith({
        listResult: {
          ok: true,
          value: installedFrameV02({
            schemaVersion: "vua.packages-installed/v0.2",
            projectPath: "C:/proj",
            packages: VALID_ROWS_V02,
            cacheSourced: true,
          }),
        },
      }),
    );
    await port.selectProject("C:/proj");
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.installedPackages).toEqual(VALID_ROWS_V02);
    expect(view.installedCacheSourced).toBe(true);

    const outcome = await port.listInstalled("C:/proj");
    expect(outcome.kind).toBe("ok");
    if (outcome.kind !== "ok") return;
    expect(outcome.result).toEqual({
      family: "vua.packages-installed/v0.2",
      rows: VALID_ROWS_V02,
      cacheSourced: true,
    });
    expect(outcome.result).not.toHaveProperty("schemaVersion");
  });

  it("keeps the v0.1-family answer at the existing P1 rendering with zero regression (no cacheSourced field invented)", async () => {
    const port = createLivePackages(clientWith());
    await port.selectProject("C:/proj");
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.installedPackages).toEqual(VALID_ROWS);
    expect("installedCacheSourced" in view).toBe(false);

    const outcome = await port.listInstalled("C:/proj");
    expect(outcome.kind).toBe("ok");
    if (outcome.kind !== "ok") return;
    expect(outcome.result).toEqual({ family: "vua.packages-installed/v0.1", rows: VALID_ROWS });
  });

  it("propagates cacheSourced=false as the online-refresh fact (present, not fabricated, not a failure)", async () => {
    const port = createLivePackages(
      clientWith({
        listResult: {
          ok: true,
          value: installedFrameV02({
            schemaVersion: "vua.packages-installed/v0.2",
            projectPath: "C:/proj",
            packages: [
              {
                packageId: "com.vrchat.avatars",
                version: "3.7.4",
                dependencies: [],
                latestVersion: "3.7.4",
                updateAvailable: false,
              },
            ],
            cacheSourced: false,
          }),
        },
      }),
    );
    await port.selectProject("C:/proj");
    const view = await port.snapshot();
    assertReadyP2(view);
    if (view.kind !== "ready-p2") return;
    expect(view.installedCacheSourced).toBe(false);
  });

  it("rejects shape violations machine-honestly (missing judgment pair or missing cacheSourced under the v0.2 stamp)", async () => {
    const missingPair = createLivePackages(
      clientWith({
        listResult: {
          ok: true,
          value: installedFrameV02({
            schemaVersion: "vua.packages-installed/v0.2",
            projectPath: "C:/proj",
            packages: VALID_ROWS,
            cacheSourced: true,
          }),
        },
      }),
    );
    expect(await missingPair.listInstalled("C:/proj")).toEqual({
      kind: "failed",
      code: "packages_shape_violation",
    });

    const missingDisclosure = createLivePackages(
      clientWith({
        listResult: {
          ok: true,
          value: installedFrameV02({
            schemaVersion: "vua.packages-installed/v0.2",
            projectPath: "C:/proj",
            packages: VALID_ROWS_V02,
          }),
        },
      }),
    );
    expect(await missingDisclosure.listInstalled("C:/proj")).toEqual({
      kind: "failed",
      code: "packages_shape_violation",
    });
  });
});

/* ---- F4 仓库生命周期写面(027 v0.6 消费批) ---- */

const VALID_ENABLED_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.6",
  kind: "enabled",
  repoId: "repo-example",
};

const VALID_DISABLED_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.6",
  kind: "disabled",
  repoId: "repo-example",
};

const VALID_REFRESHED_RECEIPT = {
  schemaVersion: "vua.packages-ops/v0.6",
  kind: "refreshed",
  repoId: "repo-example",
  cacheUpdated: true,
};

const VALID_LIFECYCLE_REJECTED = {
  schemaVersion: "vua.packages-ops/v0.6",
  kind: "rejected",
  guard: "execution_failed",
  code: "vua.packages.execution_failed",
  detail: "vua.vpm.repo_write_failed: backend refused the toggle write",
};

/** F4 流编排 client:三生命周期方法/task.get 分支可控(信封版本 0.6;
 * 与 A4 流编排 client 分立互不污染)。 */
function lifecycleClient(overrides: {
  snapshot?: Parameters<typeof appSnapshot>[0];
  enable?: GatewayResult<DesktopGatewaySuccessValueV1>;
  disable?: GatewayResult<DesktopGatewaySuccessValueV1>;
  refresh?: GatewayResult<DesktopGatewaySuccessValueV1>;
  taskGet?: GatewayResult<DesktopGatewaySuccessValueV1>;
} = {}): GatewayClient {
  return fakeClient(async (request) => {
    if (request.method === "app.snapshot") {
      return { ok: true, value: appSnapshot(overrides.snapshot ?? QUERY_AVAILABLE) };
    }
    if (request.method === "packages.enableRepo") {
      return overrides.enable ?? { ok: true, value: acceptedLifecycleFrame("t-6", "packages.enableRepo") };
    }
    if (request.method === "packages.disableRepo") {
      return overrides.disable ?? { ok: true, value: acceptedLifecycleFrame("t-6", "packages.disableRepo") };
    }
    if (request.method === "packages.refreshRepo") {
      return overrides.refresh ?? { ok: true, value: acceptedLifecycleFrame("t-6", "packages.refreshRepo") };
    }
    if (request.method === "task.get") {
      return overrides.taskGet ?? { ok: true, value: taskSnapshotValue("running") };
    }
    return { ok: false, error: { kind: "unavailable" } as const };
  });
}

function acceptedLifecycleFrame(taskId: string, operation: string): DesktopGatewaySuccessValueV1 {
  return asWire({ schemaVersion: "0.6", operation, taskId, correlationId: "c-6" });
}

/** listRepos 单方法 client(应答体测试内给定;v0.2 协商消费专用) */
function reposReadClient(reposAnswer: unknown): GatewayClient {
  return fakeClient(async (request) => {
    if (request.method === "app.snapshot") {
      return {
        ok: true,
        value: appSnapshot([
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.listRepos", availability: "available" },
        ]),
      };
    }
    if (request.method === "packages.listRepos") {
      return {
        ok: true,
        value: asWire({
          schemaVersion: "0.1",
          operation: "packages.listRepos",
          result: reposAnswer,
        }),
      };
    }
    return { ok: false, error: { kind: "unavailable" } as const };
  });
}

describe("packages live port F4 repo lifecycle write face (027 v0.6 consumption)", () => {
  it("flips blocks.repoLifecycle with the served packages.repoLifecycleOps row (one row serves the three methods; repoWrites/templates semantics untouched; declared-none stays false)", async () => {
    const withRow = await createLivePackages(
      lifecycleClient({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.repoOps", availability: "available" },
          { operationId: "packages.templatesOps", availability: "available" },
          { operationId: "packages.repoLifecycleOps", availability: "available" },
        ],
      }),
    ).snapshot();
    if (withRow.kind === "ready-p2") {
      expect(withRow.blocks.repoLifecycle).toBe(true);
      expect(withRow.blocks.repoWrites).toBe(true);
      expect(withRow.blocks.templates).toBe(true);
    }
    const withoutRow = await createLivePackages(lifecycleClient()).snapshot();
    if (withoutRow.kind === "ready-p2") {
      expect(withoutRow.blocks.repoLifecycle).toBe(false);
    }
    // 行存在但引擎后端三独立位均未声明(availability unavailable)= 诚实缺席
    const rowUnavailable = await createLivePackages(
      lifecycleClient({
        snapshot: [
          { operationId: "packages.query", availability: "available" },
          { operationId: "packages.repoLifecycleOps", availability: "unavailable" },
        ],
      }),
    ).snapshot();
    if (rowUnavailable.kind === "ready-p2") {
      expect(rowUnavailable.blocks.repoLifecycle).toBe(false);
    }
  });

  it("rides the task loop for all three methods: v0.6 acceptance -> succeeded snapshot with the exact-three-keys enabled/disabled receipts and the exact-four-keys refreshed receipt (cacheUpdated REQUIRED; verbatim single-key params; ok rides a refresh broadcast)", async () => {
    let broadcasts = 0;
    const listeners = new Set<(event: unknown) => void>();
    // task.get 序列:running(非终态,迫使 waitForTerminalTask 订阅事件
    // 通道)→ succeeded(A4 task loop 同构)
    const taskGetSequence: GatewayResult<DesktopGatewaySuccessValueV1>[] = [
      { ok: true, value: taskSnapshotValue("running") },
      {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.6", operation: "packages.enableRepo", result: VALID_ENABLED_RECEIPT },
        }),
      },
    ];
    let taskGetCalls = 0;
    const client: GatewayClient = {
      invoke: async (request) => {
        if (request.method === "packages.enableRepo") {
          // 端口 verbatim 传输钉死:params 恰 {repoId},无 confirmedDigest
          // 无 projectPath(闭集形状违反由 wire 层拒绝,UI 绝不构造)
          expect(request.params).toEqual({ repoId: "repo-example" });
          return { ok: true, value: acceptedLifecycleFrame("t-6", "packages.enableRepo") };
        }
        if (request.method === "task.get") {
          const answer: GatewayResult<DesktopGatewaySuccessValueV1> =
            taskGetSequence[Math.min(taskGetCalls, taskGetSequence.length - 1)] ?? {
              ok: false,
              error: { kind: "unavailable" },
            };
          taskGetCalls += 1;
          return answer;
        }
        return { ok: false, error: { kind: "unavailable" } as const };
      },
      subscribe: (callback) => {
        listeners.add(callback as (event: unknown) => void);
        return () => {
          listeners.delete(callback as (event: unknown) => void);
        };
      },
    };
    const port = createLivePackages(client);
    port.subscribe(() => {
      broadcasts += 1;
    });
    const pending = port.enableRepo("repo-example");
    while (listeners.size === 0) await new Promise((resolve) => setTimeout(resolve, 1));
    for (const listener of listeners) listener({ kind: "task.completed", taskId: "t-6", payload: {} });
    // 收据逐键钉死:enabled/disabled 三键闭集 {schemaVersion, kind, repoId}
    // ——回显即审计链,无时间戳无前状态回显;新状态经订阅面读回收据绝不重复
    expect(await pending).toEqual({ kind: "ok", receipt: VALID_ENABLED_RECEIPT });
    expect(Object.keys(VALID_ENABLED_RECEIPT).sort()).toEqual(["kind", "repoId", "schemaVersion"]);
    // ok 收据骑刷新广播(订阅面 enabled 位已变,列表按新事实重取)
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(broadcasts).toBeGreaterThan(0);

    // disabled 三键收据
    const disableClient = lifecycleClient({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.6", operation: "packages.disableRepo", result: VALID_DISABLED_RECEIPT },
        }),
      },
    });
    expect(await createLivePackages(disableClient).disableRepo("repo-example"))
      .toEqual({ kind: "ok", receipt: VALID_DISABLED_RECEIPT });
    expect(Object.keys(VALID_DISABLED_RECEIPT).sort()).toEqual(["kind", "repoId", "schemaVersion"]);

    // refreshed 四键收据:cacheUpdated REQUIRED(缺即形状违规)
    const refreshClient = lifecycleClient({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.6", operation: "packages.refreshRepo", result: VALID_REFRESHED_RECEIPT },
        }),
      },
    });
    expect(await createLivePackages(refreshClient).refreshRepo("repo-example"))
      .toEqual({ kind: "ok", receipt: VALID_REFRESHED_RECEIPT });
    expect(Object.keys(VALID_REFRESHED_RECEIPT).sort()).toEqual([
      "cacheUpdated",
      "kind",
      "repoId",
      "schemaVersion",
    ]);
  });

  it("carries cacheUpdated=false as an honest success (already up to date is an outcome, never an error) and surfaces the typed rejection (execution_failed with the original port code in detail) as the rejection, never an error", async () => {
    const upToDateClient = lifecycleClient({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: {
            schemaVersion: "0.6",
            operation: "packages.refreshRepo",
            result: { ...VALID_REFRESHED_RECEIPT, cacheUpdated: false },
          },
        }),
      },
    });
    // 呈现锚二:cacheUpdated=false = etag 未变「已是最新」——ok 臂,绝不折叠失败
    expect(await createLivePackages(upToDateClient).refreshRepo("repo-example")).toEqual({
      kind: "ok",
      receipt: { ...VALID_REFRESHED_RECEIPT, cacheUpdated: false },
    });

    const rejectedClient = lifecycleClient({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: { schemaVersion: "0.6", operation: "packages.disableRepo", result: VALID_LIFECYCLE_REJECTED },
        }),
      },
    });
    expect(await createLivePackages(rejectedClient).disableRepo("repo-example")).toEqual({
      kind: "rejected",
      rejection: VALID_LIFECYCLE_REJECTED,
    });
  });

  it("folds the capability-missing acceptance to failed verbatim, the absence arm to unavailable, a non-succeeded terminal verbatim, and a 0.4-stamped acceptance or invented receipt field as shape violation (never fabricates a receipt)", async () => {
    // 能力缺席在路由层答(绝不进任务)→ 折 failed 原词,与引擎缺席(unavailable)呈现区分
    const capabilityMissing = lifecycleClient({
      enable: {
        ok: false,
        error: {
          kind: "application",
          error: {
            contractVersion: "0.1",
            code: "vua.vpm.capability_missing",
            category: "unavailable",
            messageKey: "errors.vpm.capabilityMissing",
            recoverable: false,
            retryable: false,
            correlationId: "c-6",
          },
        },
      },
    });
    expect(await createLivePackages(capabilityMissing).enableRepo("repo-example")).toEqual({
      kind: "failed",
      code: "vua.vpm.capability_missing",
    });
    const absent = lifecycleClient({
      enable: { ok: false, error: { kind: "unavailable" } as const },
    });
    expect(await createLivePackages(absent).enableRepo("repo-example")).toEqual({ kind: "unavailable" });

    // 非成功终态:error.code 原词上呈(任务真实状态由任务中心呈现)
    const failedTerminal = lifecycleClient({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("failed", {
          error: {
            contractVersion: "0.1",
            code: "vua.provider.persistence_failed",
            category: "internal",
            messageKey: "errors.provider.persistenceFailed",
            recoverable: false,
            retryable: false,
            correlationId: "c-6",
          },
        }),
      },
    });
    expect(await createLivePackages(failedTerminal).enableRepo("repo-example")).toEqual({
      kind: "failed",
      code: "vua.provider.persistence_failed",
    });

    // 0.4 假盖戳 = 受理形状违规;发明收据字段(切换时间戳)= 形状违规
    const staleAcceptance = lifecycleClient({
      enable: { ok: true, value: acceptedRepoFrame("t-4", "packages.enableRepo") },
    });
    expect(await createLivePackages(staleAcceptance).enableRepo("repo-example")).toEqual({
      kind: "failed",
      code: "packages_apply_acceptance_shape",
    });
    const inventedReceipt = lifecycleClient({
      taskGet: {
        ok: true,
        value: taskSnapshotValue("succeeded", {
          result: {
            schemaVersion: "0.6",
            operation: "packages.enableRepo",
            result: { ...VALID_ENABLED_RECEIPT, disabledAt: "2026-09-21T00:00:00Z" },
          },
        }),
      },
    });
    expect(await createLivePackages(inventedReceipt).enableRepo("repo-example")).toEqual({
      kind: "failed",
      code: "packages_apply_result_shape",
    });
  });

  it("consumes the v0.2 repos family through the stamped family const (six-key rows with the REQUIRED enabled bit projected verbatim, disabled stays listed, word face carried) while the v0.1 family keeps answering without the word face", async () => {
    const v02Port = createLivePackages(
      reposReadClient({
        schemaVersion: "vua.packages-repos/v0.2",
        repos: [
          {
            repoId: "repo-live",
            name: "Live Repo",
            url: "https://vpm.example/index.json",
            localPath: null,
            cached: true,
            enabled: false,
          },
          {
            repoId: null,
            name: null,
            url: null,
            localPath: "C:/Repos/local",
            cached: false,
            enabled: true,
          },
        ],
      }),
    );
    const v02View = await v02Port.snapshot();
    if (v02View.kind === "ready-p2") {
      expect(v02View.reposWordFace).toBe("vua.packages-repos/v0.2");
      expect(v02View.repos).toEqual([
        {
          repoId: "repo-live",
          name: "Live Repo",
          url: "https://vpm.example/index.json",
          localPath: null,
          cached: true,
          enabled: false,
        },
        {
          repoId: null,
          name: null,
          url: null,
          localPath: "C:/Repos/local",
          cached: false,
          enabled: true,
        },
      ]);
    } else {
      throw new Error("expected a ready-p2 view");
    }

    // v0.1 族照常应答:五键行无 enabled 位,word face 不置位(消费端启停
    // 控制不渲染——状态不可知不猜测)
    const v01Port = createLivePackages(
      reposReadClient({
        schemaVersion: "vua.packages-repos/v0.1",
        repos: [
          {
            repoId: "repo-legacy",
            name: "Legacy Repo",
            url: "https://vpm.example/index.json",
            localPath: null,
            cached: true,
          },
        ],
      }),
    );
    const v01View = await v01Port.snapshot();
    if (v01View.kind === "ready-p2") {
      expect(v01View.reposWordFace).toBeUndefined();
      expect(v01View.repos).toEqual([
        {
          repoId: "repo-legacy",
          name: "Legacy Repo",
          url: "https://vpm.example/index.json",
          localPath: null,
          cached: true,
        },
      ]);
    } else {
      throw new Error("expected a ready-p2 view");
    }
  });

  it("answers shape violation when a v0.2-stamped row invents a field or drops the REQUIRED enabled bit (closed sets pinned, never guessed)", async () => {
    for (const badRow of [
      // 发明 health 位(虚假断言防线:health face 是冻结非目标)
      {
        repoId: "repo-x",
        name: "X",
        url: null,
        localPath: null,
        cached: true,
        enabled: true,
        health: "ok",
      },
      // 缺 REQUIRED enabled 位
      { repoId: "repo-x", name: "X", url: null, localPath: null, cached: true },
    ]) {
      const port = createLivePackages(
        reposReadClient({ schemaVersion: "vua.packages-repos/v0.2", repos: [badRow] }),
      );
      const view = await port.snapshot();
      if (view.kind === "ready-p2") {
        expect(view.reposError?.code).toBe("packages_shape_violation");
      } else {
        throw new Error("expected a ready-p2 view");
      }
    }
  });
});
