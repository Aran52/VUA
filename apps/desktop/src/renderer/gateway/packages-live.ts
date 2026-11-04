/**
 * Packages 读面 live 端口(024 P1 中间诚实态消费批,2026-09-17;025 P2
 * 读面消费批,2026-09-17;025 v0.2 增量消费更新批,2026-09-17;026 A1
 * 移除写面消费批,2026-09-19;026 A2 安装/升级写面消费批,2026-09-19;
 * 026 A3 本地包注册写面消费批,2026-09-19;027 F2 仓库级包目录读面消费
 * 批,2026-09-20;027 F5 模板条目枚举读面消费批,2026-09-20;027 F4 仓库
 * 生命周期写面消费批,2026-09-21):
 * 经 Desktop Gateway 消费 packages-query v0.1(单方法只读
 * packages.listInstalled)、025 P2 双族 v0.1(packages.listRepos 仓库订
 * 阅清单 + packages.packageCatalog 单包目录按需查询)、packages-ops v0.1
 * A1 移除写面(packages.previewRemove 同步只读变更预览 +
 * packages.applyRemove 九态任务化移除写命令)、packages-ops v0.2 A2
 * 安装/升级写面(packages.previewInstall 同步只读安装/升级预览 +
 * packages.applyInstall 九态任务化安装写命令——依赖解析可达仓库,在线
 * 刷新失败降级缓存;双摘要守卫同 A1:confirmedDigest 漂移即拒
 * preview_drift recoverable 冲突,重预览重确认绝不静默覆盖,诚实纪律 3;
 * 权威判定在服务端)与 packages-ops v0.3 A3 本地包注册写面
 * (packages.registerLocalPackage 九态任务化注册写命令——族中唯一无
 * preview 对偶:幂等集合添加,AlreadyAdded 折叠为一个成功事实;非破坏
 * 性;无 digest 无确认链,用户显式提交即确认;{packageRoot} 单键闭集,
 * 注册只动后端隔离环境)。目录族纯增量双版本协商——backend 未声明 v0.2 前
 * 以 v0.1 族应答(七键),声明后以 v0.2 族应答(冻结七键恰加必带
 * cacheSourced 披露)。消费纪律:
 * - 诚实缺席:引擎未装配/实现域未接线(vua.packages.unavailable)映射为
 *   not-connected 形态,绝不以空清单伪装(空态即终态);
 * - typed 失败照原词呈现:复用码 vua.project.project_not_found(选中项目
 *   已从 013 注册面消失)与合法空数组严格区分;vua.vpm.no_matching_package
 *   (词表外无此包)= 独立空态非错误页;失败不冒充空态(诚实纪律 2);
 * - 区块可用性标注权威事实源 = served_capabilities 能力行(app.snapshot
 *   operations):installed = packages.query 行;repos = packages.listRepos
 *   行;catalog = packages.packageCatalog 行;changes = packages.removeOps
 *   行(026 A1:随引擎后端 remove_packages 能力声明翻转,未声明即写入口
 *   诚实不渲染);installs = packages.installOps 行(026 A2:一位服务
 *   previewInstall/applyInstall 双方法,removeOps 先例;A1 逐面升级承诺
 *   = 纯增量新键,changes 语义与来源零变更);registers = packages.
 *   registerOps 行(026 A3:一行服务 registerLocalPackage,register_
 *   capabilities 访问器门控,default declared-none 翻转前如实不可用,
 *   同翻转纪律;既有键语义与来源零变更);repoWrites = packages.repoOps
 *   行(026 A4:一行服务 addRemoteRepo/addLocalRepo/removeRepo 三方法,
 *   repo_write_capabilities 三独立位门控——任一位声明即 available,
 *   wire 门按方法绝不按面,同翻转纪律;既有键语义与来源零变更);
 *   templates = packages.templatesOps 行(027 F5:一行服务 listTemplates,
 *   template_capabilities().list_templates 门控,default declared-none
 *   翻转前如实不可用,false = 模板下拉不渲染、创建表单回落手填,同翻转
 *   纪律;既有键语义与来源零变更);repoLifecycle = packages.
 *   repoLifecycleOps 行(027 F4:一行服务 enableRepo/disableRepo/
 *   refreshRepo 三方法,repo_lifecycle_capabilities 三独立位 ANY 即
 *   available——部分覆写后端不被面级行隐藏,wire 门按方法绝不按面,
 *   同翻转纪律;既有键语义与来源零变更);listRepos 结果族按盖戳族常
 *   量辨世代(F3 installed 双族先例同构):v0.1 族五键行无 enabled 位
 *   = 启停开关不渲染(状态不可知不猜测),v0.2 族六键行启停面读回权威
 *   (禁用在列不隐藏);
 * - 响应窄化按三键纪律:schemaVersion 信封("0.1"/"0.2" 各随其行)+
 *   operation + result(本体族常量 vua.packages-installed/v0.1、
 *   vua.packages-repos/v0.1、vua.packages-repos/v0.2、
 *   vua.packages-catalog/v0.1、
 *   vua.packages-templates/v0.1、vua.packages-ops/v0.1、
 *   vua.packages-ops/v0.2、vua.packages-ops/v0.4、
 *   vua.packages-ops/v0.6)
 *   组合定位,零字段猜测,
 *   行闭集校验(多余键/缺键/类型不符 = 形状不符诚实失败);行序为服务端
 *   冻结事实(installed 按 packageId 升序、repos 按订阅面自身顺序),
 *   客户端不重排不猜测。
 */
import { isTerminalTaskStateV01 } from "@vua/contracts";
import type {
  PackagesAddLocalRepoRequestV1,
  PackagesAddRemoteRepoRequestV1,
  PackagesApplyInstallRequestV1,
  PackagesApplyRemoveRequestV1,
  PackagesChangeItemV01,
  PackagesCreateProjectRequestV1,
  PackagesCreateRejectedV05,
  PackagesDisableRepoRequestV1,
  PackagesEnableRepoRequestV1,
  PackagesInstallPlanV02,
  PackagesInstallReceiptV02,
  PackagesListInstalledRequestV1,
  PackagesListReposRequestV1,
  PackagesListTemplatesRequestV1,
  PackagesLocalRepoAddedV04,
  PackagesOpsRejectedV02,
  PackagesPackageCatalogRequestV1,
  PackagesPackageRequestV02,
  PackagesPreviewInstallRequestV1,
  PackagesPreviewRemoveRequestV1,
  PackagesProjectCreatedV05,
  PackagesRefreshRepoRequestV1,
  PackagesRegisterLocalPackageRequestV1,
  PackagesRegisterReceiptV03,
  PackagesRegisterRejectedV03,
  PackagesRemoteRepoAddedV04,
  PackagesRemovePlanV01,
  PackagesRemoveReceiptV01,
  PackagesRemoveRejectedV01,
  PackagesRemoveRepoRequestV1,
  PackagesRepoCatalogRequestV1,
  PackagesRepoDisabledV06,
  PackagesRepoEnabledV06,
  PackagesRepoRefreshedV06,
  PackagesRepoRejectedV04,
  PackagesRepoRejectedV06,
  PackagesRepoRemovedV04,
} from "@vua/contracts";
import type { GatewayClient } from "./gateway-client.ts";
import { waitForTerminalTask } from "./project-ops-port.ts";
import type {
  CatalogPackageFactsV01,
  CatalogPackageFactsV02,
  CatalogVersionRowV01,
  InstalledListAnswer,
  InstalledPackageRowV01,
  InstalledPackageRowV02,
  PackagesCreateApplyOutcome,
  PackagesInstallApplyOutcome,
  PackagesPort,
  RepoCatalogFactsV01,
  RepoCatalogPackageRowV01,
  RepoCatalogRepoRowV01,
  TemplatesFactsV01,
  PackagesRegisterApplyOutcome,
  PackagesRemoveApplyOutcome,
  PackagesRepoAddApplyOutcome,
  PackagesRepoLifecycleApplyOutcome,
  PackagesRepoRemoveApplyOutcome,
  PackagesView,
  RepoInfoRowV01,
  RepoInfoRowV02,
  ReposListAnswer,
} from "./packages-port.ts";
import type { CapabilityReport, Unsubscribe } from "./types.ts";

const PACKAGES_OPERATION_ID = "packages.query";
const LIST_REPOS_OPERATION_ID = "packages.listRepos";
const CATALOG_OPERATION_ID = "packages.packageCatalog";
/** A1 移除写面 served 行(026 接线批申报;remove_packages 能力门控) */
const REMOVE_OPS_OPERATION_ID = "packages.removeOps";
/** A2 安装/升级写面 served 行(026 v0.2 接线批申报;一位服务双方法,
 * preview_install 能力门控——removeOps 先例) */
const INSTALL_OPS_OPERATION_ID = "packages.installOps";
/** A3 本地包注册写面 served 行(026 v0.3 接线批申报;一行一方法,
 * register_capabilities().register_local_package 门控——removeOps/
 * installOps 先例;default declared-none 翻转前如实 unavailable) */
const REGISTER_OPS_OPERATION_ID = "packages.registerOps";
/** A4 仓库订阅增删写面 served 行(026 v0.4 接线批申报;一行服务三方法,
 * repo_write_capabilities 三独立位门控——任一位声明即 available;wire
 * 门按方法绝不按面) */
const REPO_OPS_OPERATION_ID = "packages.repoOps";
/** A5 项目创建写面 served 行(026 v0.5 接线批申报;一行一方法,
 * registerOps/removeOps/repoOps 一行先例;行可用性 = 既有
 * VpmCapabilities.create_project 五联位——A5 零新 accessor,位先于冻
 * 结批在库双后端已声明 true,无 declared-none 缺省态) */
const CREATE_OPS_OPERATION_ID = "packages.createOps";
/** F2 仓库级包目录读面 served 行(027 接线批申报;一行一方法,
 * repo_catalog_capabilities 访问器门控——removeOps/installOps/
 * registerOps/repoOps/createOps 一行先例;default declared-none,
 * 环境覆写置真前如实 unavailable) */
const REPO_CATALOG_OPERATION_ID = "packages.repoCatalogOps";
/** F5 模板条目枚举读面 served 行(027 接线批申报;一行一方法,
 * template_capabilities().list_templates 门控——repoCatalogOps 一行
 * 先例;default declared-none,环境覆写置真前如实 unavailable) */
const TEMPLATES_OPERATION_ID = "packages.templatesOps";
/** F4 仓库生命周期写面 served 行(027 v0.6 接线批申报;一行服务三方法,
 * repo_lifecycle_capabilities 三独立位 ANY 即 available——repoOps 一行
 * 三方法先例,部分覆写后端不被面级行隐藏;每路由仍按自身位独立门;
 * default declared-none,环境覆写置真前如实 unavailable) */
const REPO_LIFECYCLE_OPERATION_ID = "packages.repoLifecycleOps";
/** packages-ops result 本体族常量(026 冻结批;盖戳辨词面永不猜测) */
const PACKAGES_OPS_SCHEMA_VERSION = "vua.packages-ops/v0.1";
/** packages-ops v0.2 result 本体族常量(A2 冻结批;与 v0.1 plan 同键集,
 * 消费窄化按字面量,不按键集) */
const PACKAGES_OPS_SCHEMA_VERSION_V02 = "vua.packages-ops/v0.2";
/** packages-ops v0.2 wire 信封常量(A2 行;信封常设形状照 c914cf2 规矩) */
const PACKAGES_OPS_ENVELOPE_V02 = "0.2";
/** packages-ops v0.3 result 本体族常量(A3 冻结批;registered/rejected
 * 两成员盖戳,消费窄化按字面量) */
const PACKAGES_OPS_SCHEMA_VERSION_V03 = "vua.packages-ops/v0.3";
/** packages-ops v0.3 wire 信封常量(A3 行;接线批协议本 0.3.1 载明,
 * 桌面消费按落地面核对——形状核可登记的核对点就此闭合) */
const PACKAGES_OPS_ENVELOPE_V03 = "0.3";
/** packages-ops v0.4 result 本体族常量(A4 冻结批;repoReceipt/removed/
 * rejected 盖戳,消费窄化按字面量) */
const PACKAGES_OPS_SCHEMA_VERSION_V04 = "vua.packages-ops/v0.4";
/** packages-ops v0.4 wire 信封常量(A4 行;接线批协议本 0.4.1 载明,
 * 桌面消费按落地面核对——形状核可登记的核对点就此闭合) */
const PACKAGES_OPS_ENVELOPE_V04 = "0.4";
/** packages-ops v0.5 result 本体族常量(A5 冻结批;created/rejected 两
 * 成员盖戳,消费窄化按字面量) */
const PACKAGES_OPS_SCHEMA_VERSION_V05 = "vua.packages-ops/v0.5";
/** packages-ops v0.5 wire 信封常量(A5 行;接线批协议本 0.5.1 载明,
 * 桌面消费按落地面核对——形状核可登记的核对点就此闭合) */
const PACKAGES_OPS_ENVELOPE_V05 = "0.5";
/** packages-ops v0.6 result 本体族常量(F4 冻结批;enabled/disabled/
 * refreshed/rejected 盖戳,消费窄化按字面量) */
const PACKAGES_OPS_SCHEMA_VERSION_V06 = "vua.packages-ops/v0.6";
/** packages-ops v0.6 wire 信封常量(F4 行;接线批协议本 0.6.1 载明,
 * 桌面消费按落地面核对——形状核可登记的核对点就此闭合) */
const PACKAGES_OPS_ENVELOPE_V06 = "0.6";
/** packages-repos v0.2 result 本体族常量(F4 冻结批;双版本协商——
 * backend 未采纳 v0.2 前继续应答 v0.1 族,盖戳辨词面永不猜测) */
const PACKAGES_REPOS_SCHEMA_VERSION_V02 = "vua.packages-repos/v0.2";
/**
 * applyRemove/applyInstall 任务等待上界(本地文件操作,正常终态由
 * task.completed 事件驱动毫秒级到达;本界只防御事件丢失/断连后的无限
 * 挂起——超时是「无法确认结果」,任务本身仍在任务中心呈现真实状态;
 * 014 先例同界)。
 */
const PACKAGES_APPLY_TASK_WAIT_MS = 120_000;

/** wire 成功帧三键包裹:schemaVersion(信封 "0.1")+ operation + result */
interface PackagesWireEnvelope {
  readonly schemaVersion: unknown;
  readonly operation: unknown;
  readonly result: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** 词面可空字符串:null = 库面 Option 如实投影;非 null 空串非法(Schema minLength 1) */
function isNullableNonEmptyString(value: unknown): value is string | null {
  return value === null || (typeof value === "string" && value.length > 0);
}

/** 冻结包行三键闭集:多余键/缺键/类型不符即整行形状不符(诚实失败,不猜测) */
function isInstalledPackageRow(value: unknown): value is InstalledPackageRowV01 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  if (keys.length !== 3 || keys[0] !== "dependencies" || keys[1] !== "packageId" || keys[2] !== "version") {
    return false;
  }
  return typeof value.packageId === "string"
    && value.packageId.length > 0
    && typeof value.version === "string"
    && value.version.length > 0
    && Array.isArray(value.dependencies)
    && value.dependencies.every((dependency) => typeof dependency === "string");
}

/** F3 已装包行五键闭集(027 packages-installed v0.2 冻结词面;v0.1 三键
 * 零变动＋判定对必带可空——发明字段/缺判定对键 = 形状违规,负面向量
 * 钉死;updateAvailable 仅 true/false/null,数字零或缺键都不是合法词面) */
function isInstalledPackageRowV02(value: unknown): value is InstalledPackageRowV02 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = ["dependencies", "latestVersion", "packageId", "updateAvailable", "version"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return typeof value.packageId === "string"
    && value.packageId.length > 0
    && typeof value.version === "string"
    && value.version.length > 0
    && Array.isArray(value.dependencies)
    && value.dependencies.every((dependency) => typeof dependency === "string")
    && (value.latestVersion === null || (typeof value.latestVersion === "string" && value.latestVersion.length > 0))
    && (value.updateAvailable === null || typeof value.updateAvailable === "boolean");
}

/** result 本体:schemaVersion 族常量 + projectPath + packages 行数组;
 * 027 F3 起双族协商(v0.1 三键行族 / v0.2 五键行族＋cacheSourced 必带
 * 披露)——盖戳族常量按对应词面窄化,零字段猜测;两族盖戳不匹配各自键
 * 闭集 = 形状不符(catalog v0.2 双族先例同构) */
function isPackagesInstalledResultV01(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== "vua.packages-installed/v0.1") return false;
  if (typeof value.projectPath !== "string" || value.projectPath.length === 0) return false;
  return Array.isArray(value.packages) && value.packages.every(isInstalledPackageRow);
}

/** v0.2 族应答:五键行闭集＋cacheSourced 必带信息性披露(缺披露 = 形状
 * 违规——负面向量钉死) */
function isPackagesInstalledResultV02(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== "vua.packages-installed/v0.2") return false;
  if (typeof value.projectPath !== "string" || value.projectPath.length === 0) return false;
  return typeof value.cacheSourced === "boolean"
    && Array.isArray(value.packages)
    && value.packages.every(isInstalledPackageRowV02);
}

/** P2 仓库订阅行五键闭集(025 冻结词面;health/status 等发明字段 = 形状违规) */
function isRepoInfoRow(value: unknown): value is RepoInfoRowV01 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = ["cached", "localPath", "name", "repoId", "url"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return typeof value.cached === "boolean"
    && isNullableNonEmptyString(value.repoId)
    && isNullableNonEmptyString(value.name)
    && isNullableNonEmptyString(value.url)
    && isNullableNonEmptyString(value.localPath);
}

/** P2 仓库订阅行 v0.2 六键闭集(027 F4 冻结词面;v0.1 五键＋REQUIRED
 * enabled 位——VUA 自有启停状态读回;health/status/disabledAt 等发明
 * 字段 = 形状违规) */
function isRepoInfoRowV02(value: unknown): value is RepoInfoRowV02 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = ["cached", "enabled", "localPath", "name", "repoId", "url"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return typeof value.cached === "boolean"
    && typeof value.enabled === "boolean"
    && isNullableNonEmptyString(value.repoId)
    && isNullableNonEmptyString(value.name)
    && isNullableNonEmptyString(value.url)
    && isNullableNonEmptyString(value.localPath);
}

/** P2 目录版本行三键闭集(compatible boolean|null,null = 工程版本未知) */
function isCatalogVersionRow(value: unknown): value is CatalogVersionRowV01 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = ["compatible", "version", "yanked"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return typeof value.version === "string"
    && value.version.length > 0
    && typeof value.yanked === "boolean"
    && (typeof value.compatible === "boolean" || value.compatible === null);
}

/** P2 单包目录事实七键闭集(source 词表 repo|local;updateAvailable boolean|null) */
function isCatalogPackageFacts(value: unknown): value is CatalogPackageFactsV01 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = ["displayName", "installed", "packageId", "projectPath", "source", "updateAvailable", "versions"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return typeof value.projectPath === "string"
    && value.projectPath.length > 0
    && typeof value.packageId === "string"
    && value.packageId.length > 0
    && isNullableNonEmptyString(value.displayName)
    && (value.source === "repo" || value.source === "local")
    && typeof value.installed === "boolean"
    && (typeof value.updateAvailable === "boolean" || value.updateAvailable === null)
    && Array.isArray(value.versions)
    && value.versions.every(isCatalogVersionRow);
}

/** P2 v0.2 目录事实八键闭集 = 冻结七键恰加必带 cacheSourced(025 v0.2
 * 增量冻结批):缺键(七键盖 v0.2 戳)或多余键均形状不符诚实失败——
 * 版本世代由盖戳族常量钉死,消费端永不猜测 */
function isCatalogPackageFactsV02(value: unknown): value is CatalogPackageFactsV02 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = [
    "cacheSourced",
    "displayName",
    "installed",
    "packageId",
    "projectPath",
    "source",
    "updateAvailable",
    "versions",
  ];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return typeof value.cacheSourced === "boolean" && isCatalogPackageFacts({
    displayName: value.displayName,
    installed: value.installed,
    packageId: value.packageId,
    projectPath: value.projectPath,
    source: value.source,
    updateAvailable: value.updateAvailable,
    versions: value.versions,
  });
}

function isPackagesWireEnvelope(
  value: unknown,
  operation: string,
  schemaVersion: string,
): value is PackagesWireEnvelope {
  return isRecord(value)
    && value.schemaVersion === schemaVersion
    && value.operation === operation
    && isRecord(value.result);
}

/** F2 仓库级包行五键闭集(027 冻结词面;author/compatible 发明字段 =
 * 形状违规——环境考证 §1(b) 单事实源裁决,负面向量钉死) */
function isRepoCatalogPackageRow(value: unknown): value is RepoCatalogPackageRowV01 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = ["description", "displayName", "latestVersion", "packageId", "versionCount"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return typeof value.packageId === "string"
    && value.packageId.length > 0
    && isNullableNonEmptyString(value.displayName)
    && isNullableNonEmptyString(value.description)
    && isNullableNonEmptyString(value.latestVersion)
    && typeof value.versionCount === "number"
    && Number.isSafeInteger(value.versionCount)
    && value.versionCount >= 0;
}

/** F2 仓库行四键闭集(cached 必带 = 逐仓库缓存命中事实,false = 已订阅
 * 未刷新的诚实状态,不隐藏不伪造) */
function isRepoCatalogRepoRow(value: unknown): value is RepoCatalogRepoRowV01 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = ["cached", "name", "packages", "repoId"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return typeof value.cached === "boolean"
    && isNullableNonEmptyString(value.repoId)
    && isNullableNonEmptyString(value.name)
    && Array.isArray(value.packages)
    && value.packages.every(isRepoCatalogPackageRow);
}

/** result 本体:schemaVersion 族常量 + repos 行数组 + cacheSourced 必带
 * 信息性降级披露(027 冻结词面三键,出生即带——catalog v0.2 先例;
 * 空数组 = 诚实零仓库缓存应答) */
function isRepoCatalogResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== "vua.packages-repo-catalog/v0.1") return false;
  return typeof value.cacheSourced === "boolean"
    && Array.isArray(value.repos)
    && value.repos.every(isRepoCatalogRepoRow);
}

/** result 本体组合守卫:双族协商按盖戳族常量窄化(027 F3;catalog v0.2
 * 先例同构——盖戳辨词面永不猜测) */
function isPackagesInstalledResult(value: Record<string, unknown>): boolean {
  return isPackagesInstalledResultV01(value) || isPackagesInstalledResultV02(value);
}

/** result 本体:v0.1 族应答(schemaVersion 族常量 + repos 行数组;空数组
 *  = 诚实零订阅) */
function isPackagesReposResultV01(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== "vua.packages-repos/v0.1") return false;
  return Array.isArray(value.repos) && value.repos.every(isRepoInfoRow);
}

/** result 本体:v0.2 族应答(027 F4 增量;六键行 REQUIRED enabled——
 * id 缺席行 enabled 恒 true 的投影由 wire/端口层保证,桌面守卫只验行
 * 闭集与位类型) */
function isPackagesReposResultV02(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_REPOS_SCHEMA_VERSION_V02) return false;
  return Array.isArray(value.repos) && value.repos.every(isRepoInfoRowV02);
}

/** result 本体组合守卫:双族协商按盖戳族常量窄化(027 F4;installed
 * v0.2 双族先例同构——盖戳辨词面永不猜测,command 面逐字节 v0.1) */
function isPackagesReposResult(value: Record<string, unknown>): boolean {
  return isPackagesReposResultV01(value) || isPackagesReposResultV02(value);
}

/** 模板条目行:id = 非空机器标识(createProject template 参数原样传递),
 *  name = 非空显示投影(wire 层 name===id 同值锁由核心消费测试钉,桌面
 *  守卫只验诚实可显示性,不重审同值事实);description/sourceRoot 刻意
 *  缺席——多余键 = 形状不符(发明即非法,冻结负例向量钉死) */
function isTemplateItemRow(value: unknown): boolean {
  if (!isRecord(value)) return false;
  return Object.keys(value).length === 2
    && typeof value.id === "string"
    && value.id.length > 0
    && typeof value.name === "string"
    && value.name.length > 0;
}

/** result 本体:schemaVersion 族常量 + templates 行数组(空数组 = 诚实
 *  零模板应答;零网络面无 cacheSourced——恒常量信息字段不是事实) */
function isPackagesTemplatesResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== "vua.packages-templates/v0.1") return false;
  return Array.isArray(value.templates) && value.templates.every(isTemplateItemRow);
}

/** result 本体:schemaVersion 族常量 + 冻结七键目录事实(v0.1 冻结词面,
 * backend 未声明 v0.2 前的应答族) */
function isPackagesCatalogResultV01(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== "vua.packages-catalog/v0.1") return false;
  const { schemaVersion: _familyConst, ...facts } = value;
  return isCatalogPackageFacts(facts);
}

/** result 本体:schemaVersion 族常量 + 八键目录事实(冻结七键恰加必带
 * cacheSourced;声明 v0.2 的 backend 应答族)。双族协商:读盖戳族常量
 * 按对应词面窄化,零字段猜测;两族盖戳不匹配各自键闭集 = 形状不符 */
function isPackagesCatalogResultV02(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== "vua.packages-catalog/v0.2") return false;
  const { schemaVersion: _familyConst, ...facts } = value;
  return isCatalogPackageFactsV02(facts);
}

/** 目录族组合校验:盖戳族常量决定按 v0.1 或 v0.2 词面窄化(纯增量双
 * 版本协商);未知族常量 = 形状不符(消费端永不猜测词面) */
function isPackagesCatalogResult(value: Record<string, unknown>): boolean {
  return isPackagesCatalogResultV01(value) || isPackagesCatalogResultV02(value);
}

/* ---- A1 移除写面窄化(026 冻结词面;family const vua.packages-ops/v0.1) ---- */

/** 变更行四键闭集:kind 词表二值;version/reason 可空(null = 端口
 *  Option 如实投影,移除行即 null——非省略) */
function isRemoveChangeItem(value: unknown): value is PackagesChangeItemV01 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = ["kind", "packageId", "reason", "version"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return (value.kind === "install" || value.kind === "remove")
    && typeof value.packageId === "string"
    && value.packageId.length > 0
    && isNullableNonEmptyString(value.version)
    && (value.reason === null || typeof value.reason === "string");
}

function isStringArray(value: unknown): value is readonly string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string" && item.length > 0);
}

/** kind=plan 九键闭集(026 冻结词面):items/conflicts/
 * removeLegacyFiles/removeLegacyFolders 必为数组,destructive 必布尔,
 * digest 必非空串——收不齐即形状不符诚实失败(虚假断言防线) */
function isPackagesRemovePlanResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION) return false;
  const keys = Object.keys(value).sort();
  const expected = [
    "conflicts",
    "destructive",
    "digest",
    "items",
    "kind",
    "projectPath",
    "removeLegacyFiles",
    "removeLegacyFolders",
    "schemaVersion",
  ];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "plan"
    && typeof value.projectPath === "string"
    && value.projectPath.length > 0
    && Array.isArray(value.items)
    && value.items.every(isRemoveChangeItem)
    && isStringArray(value.conflicts)
    && isStringArray(value.removeLegacyFiles)
    && isStringArray(value.removeLegacyFolders)
    && typeof value.destructive === "boolean"
    && typeof value.digest === "string"
    && value.digest.length > 0;
}

/** kind=receipt 六键闭集(审计收据三半面:确认指纹回显＋请求清单＋
 * 实际移除行;无端口载体的发明事实在 Schema 即非法) */
function isPackagesRemoveReceiptResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION) return false;
  const keys = Object.keys(value).sort();
  const expected = [
    "confirmedDigest",
    "kind",
    "projectPath",
    "removedItems",
    "requestedPackageIds",
    "schemaVersion",
  ];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "receipt"
    && typeof value.projectPath === "string"
    && value.projectPath.length > 0
    && typeof value.confirmedDigest === "string"
    && value.confirmedDigest.length > 0
    && isStringArray(value.requestedPackageIds)
    && Array.isArray(value.removedItems)
    && value.removedItems.every(isRemoveChangeItem);
}

/** kind=rejected 五键闭集:guard 三值闭集 + code 锁 vua.packages. 族
 * (013 复用码永不入 rejected 文档) + detail 非空 */
function isPackagesRemoveRejectedResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION) return false;
  const keys = Object.keys(value).sort();
  const expected = ["code", "detail", "guard", "kind", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "rejected"
    && (value.guard === "preview_drift"
      || value.guard === "package_not_found"
      || value.guard === "execution_failed")
    && typeof value.code === "string"
    && value.code.startsWith("vua.packages.")
    && typeof value.detail === "string"
    && value.detail.length > 0;
}

/** applyRemove wire 受理回执窄化(import-copy 同构四键:schemaVersion
 * 信封 + operation + taskId + correlationId;收不齐 = 形状不符) */
function isApplyRemoveAccepted(value: unknown): value is { taskId: string; correlationId: string } {
  if (!isRecord(value)) return false;
  return value.schemaVersion === "0.1"
    && value.operation === "packages.applyRemove"
    && typeof value.taskId === "string"
    && value.taskId.length > 0
    && typeof value.correlationId === "string"
    && value.correlationId.length > 0;
}

/* ---- A2 安装/升级写面窄化(026 packages-ops v0.2 冻结词面;family const
 * vua.packages-ops/v0.2——与 v0.1 plan 同键集,消费窄化按字面量) ---- */

/** 安装请求行二键闭集:packageId 非空 + version string|null(version
 * 必填可空,null = 解析器选最新稳定版;空串/数字 = 形状违规) */
function isInstallRequestRow(value: unknown): value is PackagesPackageRequestV02 {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value).sort();
  if (keys.length !== 2 || keys[0] !== "packageId" || keys[1] !== "version") return false;
  return typeof value.packageId === "string"
    && value.packageId.length > 0
    && (value.version === null
      || (typeof value.version === "string" && value.version.length > 0));
}

/** kind=plan 九键闭集(v0.2 戳):与 v0.1 removePlan 同键集,items 可含
 * install 与 remove 行(PackagesChangeItemV01 全闭集);schemaVersion
 * 字面量互异 = 消费窄化按字面量成立,收不齐即形状不符诚实失败 */
function isPackagesInstallPlanResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V02) return false;
  const keys = Object.keys(value).sort();
  const expected = [
    "conflicts",
    "destructive",
    "digest",
    "items",
    "kind",
    "projectPath",
    "removeLegacyFiles",
    "removeLegacyFolders",
    "schemaVersion",
  ];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "plan"
    && typeof value.projectPath === "string"
    && value.projectPath.length > 0
    && Array.isArray(value.items)
    && value.items.every(isRemoveChangeItem)
    && isStringArray(value.conflicts)
    && isStringArray(value.removeLegacyFiles)
    && isStringArray(value.removeLegacyFolders)
    && typeof value.destructive === "boolean"
    && typeof value.digest === "string"
    && value.digest.length > 0;
}

/** kind=receipt 六键闭集(installReceipt 变体,与 v0.1 removeReceipt 键集
 * 互斥:requestedPackages 请求行 verbatim 携版本选择语义 + appliedItems
 * = 端口 {applied: items} verbatim;诚实空数组按空数组传输,不发明下限;
 * 无端口载体的发明事实在 Schema 即非法) */
function isPackagesInstallReceiptResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V02) return false;
  const keys = Object.keys(value).sort();
  const expected = [
    "appliedItems",
    "confirmedDigest",
    "kind",
    "projectPath",
    "requestedPackages",
    "schemaVersion",
  ];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "receipt"
    && typeof value.projectPath === "string"
    && value.projectPath.length > 0
    && typeof value.confirmedDigest === "string"
    && value.confirmedDigest.length > 0
    && Array.isArray(value.requestedPackages)
    && value.requestedPackages.every(isInstallRequestRow)
    && Array.isArray(value.appliedItems)
    && value.appliedItems.every(isRemoveChangeItem);
}

/** kind=rejected 五键闭集(v0.2 戳):guard 三值闭集复用 A1 零新增 +
 * code 锁 vua.packages. 族(013 复用码永不入 rejected 文档) + detail
 * 非空 */
function isPackagesInstallRejectedResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V02) return false;
  const keys = Object.keys(value).sort();
  const expected = ["code", "detail", "guard", "kind", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "rejected"
    && (value.guard === "preview_drift"
      || value.guard === "package_not_found"
      || value.guard === "execution_failed")
    && typeof value.code === "string"
    && value.code.startsWith("vua.packages.")
    && typeof value.detail === "string"
    && value.detail.length > 0;
}

/** applyInstall wire 受理回执窄化(import-copy 同构四键,v0.2 信封:
 * schemaVersion "0.2" + operation + taskId + correlationId;收不齐 =
 * 形状不符) */
function isApplyInstallAccepted(value: unknown): value is { taskId: string; correlationId: string } {
  if (!isRecord(value)) return false;
  return value.schemaVersion === PACKAGES_OPS_ENVELOPE_V02
    && value.operation === "packages.applyInstall"
    && typeof value.taskId === "string"
    && value.taskId.length > 0
    && typeof value.correlationId === "string"
    && value.correlationId.length > 0;
}

/* ---- A3 本地包注册写面窄化(026 packages-ops v0.3 冻结词面;family
 * const vua.packages-ops/v0.3——registered/rejected 盖戳按字面量) ---- */

/** kind=registered 三键闭集(最小诚实审计形状:端口答 unit 无载荷,
 * 收据只携请求回显 packageRoot,别无他物——注册时间戳/package.json
 * 内容/环境文件路径等发明事实 = 形状违规,负例
 * invalid-register-invented-field 同形;AlreadyAdded 幂等折叠 = 无
 * added 布尔无首次/重复事实,一个成功事实) */
function isPackagesRegisterReceiptResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V03) return false;
  const keys = Object.keys(value).sort();
  const expected = ["kind", "packageRoot", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "registered"
    && typeof value.packageRoot === "string"
    && value.packageRoot.length > 0;
}

/** kind=rejected 五键闭集(v0.3 戳):guard 三值闭集复用 A1/A2 零新增 +
 * code 锁 vua.packages. 族(013 复用码 vua.vpm.* 永不入 rejected 文档,
 * 原端口码在 detail 原词溯源) + detail 非空 */
function isPackagesRegisterRejectedResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V03) return false;
  const keys = Object.keys(value).sort();
  const expected = ["code", "detail", "guard", "kind", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "rejected"
    && (value.guard === "preview_drift"
      || value.guard === "package_not_found"
      || value.guard === "execution_failed")
    && typeof value.code === "string"
    && value.code.startsWith("vua.packages.")
    && typeof value.detail === "string"
    && value.detail.length > 0;
}

/** registerLocalPackage wire 受理回执窄化(import-copy 同构四键,v0.3
 * 信封:schemaVersion "0.3" + operation + taskId + correlationId;收不齐
 * = 形状不符) */
function isApplyRegisterAccepted(value: unknown): value is { taskId: string; correlationId: string } {
  if (!isRecord(value)) return false;
  return value.schemaVersion === PACKAGES_OPS_ENVELOPE_V03
    && value.operation === "packages.registerLocalPackage"
    && typeof value.taskId === "string"
    && value.taskId.length > 0
    && typeof value.correlationId === "string"
    && value.correlationId.length > 0;
}

/* ---- A4 仓库订阅增删写面窄化(026 packages-ops v0.4 冻结词面;family
 * const vua.packages-ops/v0.4——repoReceipt/removed/rejected 盖戳按字
 * 面量) ---- */

/** kind=repoReceipt remote 变体五键闭集(最小诚实审计形状:端口答
 * Result<(),_> 无载荷,收据只携请求回显 url＋name,别无他物——添加
 * 时间戳/行位/清单内容等发明事实 = 形状违规,负例
 * invalid-result-invented-field 同形) */
function isPackagesRemoteRepoReceiptResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V04) return false;
  const keys = Object.keys(value).sort();
  const expected = ["kind", "name", "repoType", "schemaVersion", "url"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "repoReceipt"
    && value.repoType === "remote"
    && typeof value.url === "string"
    && value.url.length > 0
    && typeof value.name === "string"
    && value.name.length > 0;
}

/** kind=repoReceipt local 变体五键闭集(path＋name 回显;与 remote 变体
 * 键集互斥——repoType 字面量分立) */
function isPackagesLocalRepoReceiptResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V04) return false;
  const keys = Object.keys(value).sort();
  const expected = ["kind", "name", "path", "repoType", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "repoReceipt"
    && value.repoType === "local"
    && typeof value.path === "string"
    && value.path.length > 0
    && typeof value.name === "string"
    && value.name.length > 0;
}

/** kind=removed 三键闭集(被删行 repoId 回显——回显即审计链;被删行快
 * 照 = 发明事实,形状违规) */
function isPackagesRepoRemovedResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V04) return false;
  const keys = Object.keys(value).sort();
  const expected = ["kind", "repoId", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "removed"
    && typeof value.repoId === "string"
    && value.repoId.length > 0;
}

/** kind=rejected 五键闭集(v0.4 戳):guard 三值闭集复用 A1/A2/A3 零新
 * 增(A4 不加 guard)+ code 锁 vua.packages. 族(013 复用码 vua.vpm.*
 * 永不入 rejected 文档,原端口码 repo_invalid/repo_not_found/
 * repo_fetch_failed/repo_write_failed 在 detail 原词溯源) + detail 非空 */
function isPackagesRepoRejectedResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V04) return false;
  const keys = Object.keys(value).sort();
  const expected = ["code", "detail", "guard", "kind", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "rejected"
    && (value.guard === "preview_drift"
      || value.guard === "package_not_found"
      || value.guard === "execution_failed")
    && typeof value.code === "string"
    && value.code.startsWith("vua.packages.")
    && typeof value.detail === "string"
    && value.detail.length > 0;
}

/** A4 wire 受理回执窄化(import-copy 同构四键,v0.4 信封:schemaVersion
 * "0.4" + operation(三方法分立) + taskId + correlationId;收不齐 =
 * 形状不符;0.3/0.2/0.1 戳 = 受理形状违规) */
function isApplyRepoAccepted(
  value: unknown,
  operation: "packages.addRemoteRepo" | "packages.addLocalRepo" | "packages.removeRepo",
): value is { taskId: string; correlationId: string } {
  if (!isRecord(value)) return false;
  return value.schemaVersion === PACKAGES_OPS_ENVELOPE_V04
    && value.operation === operation
    && typeof value.taskId === "string"
    && value.taskId.length > 0
    && typeof value.correlationId === "string"
    && value.correlationId.length > 0;
}

/* ---- A5 项目创建写面窄化(026 packages-ops v0.5 冻结词面;family
 * const vua.packages-ops/v0.5——created/rejected 盖戳按字面量) ---- */

/** kind=created 四键闭集(A5 创建收据 = 端口 ProjectRef {id, root} 投
 * 影——packages-ops 族唯一有实际载荷的收据:projectId = 端口铸造事实回
 * 显〔信息性标识非 013 身份键〕,projectPath = 新项目根目录 = 注册路径
 * 身份〔创建即在册冻结端口事实〕;发明创建时间戳/复制统计/包清单 =
 * 形状违规,负例 invalid-result-invented-field 同形) */
function isPackagesProjectCreatedResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V05) return false;
  const keys = Object.keys(value).sort();
  const expected = ["kind", "projectId", "projectPath", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "created"
    && typeof value.projectId === "string"
    && value.projectId.length > 0
    && typeof value.projectPath === "string"
    && value.projectPath.length > 0;
}

/** kind=rejected 五键闭集(v0.5 戳):guard 三值闭集复用 A1–A4 零新增
 * (A5 不加 guard)+ code 锁 vua.packages. 族(013 复用码 vua.vpm.*
 * 永不入 rejected 文档,原端口码 template_missing/apply_failed/
 * backend_unavailable 在 detail 原词溯源;双后端拒绝形状不同构如实折
 * 叠,桌面按 detail 原码呈现不合并词) + detail 非空 */
function isPackagesCreateRejectedResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V05) return false;
  const keys = Object.keys(value).sort();
  const expected = ["code", "detail", "guard", "kind", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "rejected"
    && (value.guard === "preview_drift"
      || value.guard === "package_not_found"
      || value.guard === "execution_failed")
    && typeof value.code === "string"
    && value.code.startsWith("vua.packages.")
    && typeof value.detail === "string"
    && value.detail.length > 0;
}

/** A5 wire 受理回执窄化(import-copy 同构四键,v0.5 信封:schemaVersion
 * "0.5" + operation + taskId + correlationId;收不齐 = 形状不符;0.4/
 * 0.3/0.2/0.1 戳 = 受理形状违规) */
function isApplyCreateAccepted(
  value: unknown,
  operation: "packages.createProject",
): value is { taskId: string; correlationId: string } {
  if (!isRecord(value)) return false;
  return value.schemaVersion === PACKAGES_OPS_ENVELOPE_V05
    && value.operation === operation
    && typeof value.taskId === "string"
    && value.taskId.length > 0
    && typeof value.correlationId === "string"
    && value.correlationId.length > 0;
}

/* ---- F4 仓库生命周期写面窄化(027 packages-ops v0.6 冻结词面;family
 * const vua.packages-ops/v0.6——enabled/disabled/refreshed/rejected 盖
 * 戳按字面量) ---- */

/** kind=enabled 三键闭集(enabled/disabled 同构:端口答 Result<(),_> 无
 * 载荷,repoId 回显即审计链;新状态经 repos v0.2 订阅面读回——发明切换
 * 时间戳/前状态回显 = 形状违规,负例向量钉死) */
function isPackagesRepoEnabledResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V06) return false;
  const keys = Object.keys(value).sort();
  const expected = ["kind", "repoId", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "enabled"
    && typeof value.repoId === "string"
    && value.repoId.length > 0;
}

/** kind=disabled 三键闭集(与 enabled 同构,kind 字面量分立) */
function isPackagesRepoDisabledResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V06) return false;
  const keys = Object.keys(value).sort();
  const expected = ["kind", "repoId", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "disabled"
    && typeof value.repoId === "string"
    && value.repoId.length > 0;
}

/** kind=refreshed 四键闭集(A4 收据族唯一新增事实):cacheUpdated REQUIRED
 * 必带(库面 update_cache 两臂结果——true = etag 条件抓取写入新缓存,
 * false = etag 未变「已是最新」;两臂皆成功,无新数据是结果绝非错误);
 * 发明字节计数/包清单 = 形状违规,缺 cacheUpdated = 形状违规(负例向量
 * invalid-result-refreshed-missing-cacheupdated 钉死) */
function isPackagesRepoRefreshedResult(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V06) return false;
  const keys = Object.keys(value).sort();
  const expected = ["cacheUpdated", "kind", "repoId", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "refreshed"
    && typeof value.repoId === "string"
    && value.repoId.length > 0
    && typeof value.cacheUpdated === "boolean";
}

/** kind=rejected 五键闭集(v0.6 戳):guard 三值闭集复用 A1–A5 零新增
 * (F4 不加 guard)+ code 锁 vua.packages. 族(复用码 vua.vpm.*
 * repo_not_found/repo_write_failed/repo_fetch_failed 永不入 rejected
 * 文档,原端口码在 detail 原词溯源) + detail 非空 */
function isPackagesRepoRejectedResultV06(value: Record<string, unknown>): boolean {
  if (value.schemaVersion !== PACKAGES_OPS_SCHEMA_VERSION_V06) return false;
  const keys = Object.keys(value).sort();
  const expected = ["code", "detail", "guard", "kind", "schemaVersion"];
  if (keys.length !== expected.length) return false;
  for (let index = 0; index < expected.length; index += 1) {
    if (keys[index] !== expected[index]) return false;
  }
  return value.kind === "rejected"
    && (value.guard === "preview_drift"
      || value.guard === "package_not_found"
      || value.guard === "execution_failed")
    && typeof value.code === "string"
    && value.code.startsWith("vua.packages.")
    && typeof value.detail === "string"
    && value.detail.length > 0;
}

/** F4 wire 受理回执窄化(import-copy 同构四键,v0.6 信封:schemaVersion
 * "0.6" + operation(三方法分立) + taskId + correlationId;收不齐 =
 * 形状不符;0.5/0.4/0.3/0.2/0.1 戳 = 受理形状违规) */
function isApplyLifecycleAccepted(
  value: unknown,
  operation: "packages.enableRepo" | "packages.disableRepo" | "packages.refreshRepo",
): value is { taskId: string; correlationId: string } {
  if (!isRecord(value)) return false;
  return value.schemaVersion === PACKAGES_OPS_ENVELOPE_V06
    && value.operation === operation
    && typeof value.taskId === "string"
    && value.taskId.length > 0
    && typeof value.correlationId === "string"
    && value.correlationId.length > 0;
}

type TypedOutcome<T> =
  | { readonly kind: "ok"; readonly result: T }
  | { readonly kind: "failed"; readonly code: string }
  | { readonly kind: "unavailable" };

export function createLivePackages(client: GatewayClient): PackagesPort {
  /**
   * served_capabilities 能力行读取(区块标注权威事实源):app.snapshot
   * 一次取十一行——packages.query(installed)/packages.listRepos(repos)/
   * packages.packageCatalog(catalog)/packages.removeOps(changes)/
   * packages.installOps(installs)/packages.registerOps(registers)/
   * packages.repoOps(repoWrites)/packages.createOps(creates)/
   * packages.repoCatalogOps(repoCatalog,027 F2)/packages.templatesOps
   * (templates,027 F5);行缺席或 availability 非 available = 该区块
   * 诚实不可渲染(渲染层不伪造)。
   */
  const readCapabilityRows = async (): Promise<{
    installed: boolean;
    repos: boolean;
    catalog: boolean;
    changes: boolean;
    installs: boolean;
    registers: boolean;
    repoWrites: boolean;
    creates: boolean;
    repoCatalog: boolean;
    templates: boolean;
    repoLifecycle: boolean;
  }> => {
    const result = await client.invoke({
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "app.snapshot",
      params: {},
    });
    const availability = (operationId: string): boolean => {
      const value: unknown = result.ok ? result.value : null;
      if (!isRecord(value)) return false;
      const capabilities = value.capabilities;
      if (!isRecord(capabilities) || !Array.isArray(capabilities.operations)) return false;
      return capabilities.operations.some(
        (operation) =>
          isRecord(operation)
          && operation.operationId === operationId
          && operation.availability === "available",
      );
    };
    return {
      installed: availability(PACKAGES_OPERATION_ID),
      repos: availability(LIST_REPOS_OPERATION_ID),
      catalog: availability(CATALOG_OPERATION_ID),
      changes: availability(REMOVE_OPS_OPERATION_ID),
      installs: availability(INSTALL_OPS_OPERATION_ID),
      registers: availability(REGISTER_OPS_OPERATION_ID),
      repoWrites: availability(REPO_OPS_OPERATION_ID),
      creates: availability(CREATE_OPS_OPERATION_ID),
      repoCatalog: availability(REPO_CATALOG_OPERATION_ID),
      templates: availability(TEMPLATES_OPERATION_ID),
      repoLifecycle: availability(REPO_LIFECYCLE_OPERATION_ID),
    };
  };

  const invokeTyped = async (
    request:
      | PackagesListInstalledRequestV1
      | PackagesListReposRequestV1
      | PackagesPackageCatalogRequestV1
      | PackagesRepoCatalogRequestV1
      | PackagesListTemplatesRequestV1
      | PackagesPreviewRemoveRequestV1
      | PackagesPreviewInstallRequestV1,
    operation: string,
    schemaVersion: string,
    validateResult: (value: Record<string, unknown>) => boolean,
  ): Promise<TypedOutcome<Record<string, unknown>>> => {
    const result = await client.invoke(request);
    if (!result.ok) {
      if (result.error.kind === "application") {
        // 缺席臂(vua.packages.unavailable)= 引擎未装配,诚实 unavailable;
        // 其余 typed 码(project_not_found/no_matching_package/
        // capability_missing/package_not_found/preview_failed 等)照原词
        // 上呈,不折叠为空态
        if (result.error.error.code === "vua.packages.unavailable") {
          return { kind: "unavailable" };
        }
        return { kind: "failed", code: result.error.error.code };
      }
      return { kind: "unavailable" };
    }
    if (!isPackagesWireEnvelope(result.value, operation, schemaVersion)) {
      return { kind: "failed", code: "packages_shape_violation" };
    }
    const resultBody = result.value.result;
    if (!validateResult(resultBody)) {
      return { kind: "failed", code: "packages_shape_violation" };
    }
    return { kind: "ok", result: resultBody };
  };

  /** P1 词面消费(027 F3 消费批起双族协商):组合校验已按族通过,剥信封
   * 后按盖戳族常量窄化返回对应词面行集(族常量字面量判别,永不猜测;
   * catalog v0.2 双族先例同构) */
  const listInstalledRaw = async (projectPath: string): Promise<TypedOutcome<InstalledListAnswer>> => {
    const request: PackagesListInstalledRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.listInstalled",
      params: { projectPath },
    };
    const outcome = await invokeTyped(request, "packages.listInstalled", "0.1", isPackagesInstalledResult);
    if (outcome.kind !== "ok") return outcome;
    const { schemaVersion: familyConst, projectPath: _stamped, ...rest } = outcome.result as {
      schemaVersion: string;
      projectPath: string;
      packages: readonly unknown[];
      cacheSourced?: boolean;
    };
    if (familyConst === "vua.packages-installed/v0.2") {
      return {
        kind: "ok",
        result: {
          family: familyConst,
          rows: rest.packages as readonly InstalledPackageRowV02[],
          cacheSourced: rest.cacheSourced === true,
        },
      };
    }
    return {
      kind: "ok",
      result: { family: "vua.packages-installed/v0.1", rows: rest.packages as readonly InstalledPackageRowV01[] },
    };
  };

  /** P2 订阅清单:行序 = 订阅面自身顺序(配置事实),空数组 = 诚实零订
   *  阅。027 F4:双族协商消费(027 F3 installed 双族先例同构)——盖戳族
   *  常量辨世代,backend 未采纳 v0.2 前继续应答 v0.1 族(command 面
   *  逐字节不变);v0.2 族六键行 REQUIRED enabled = 启停面读回权威 */
  const listReposRaw = async (): Promise<TypedOutcome<ReposListAnswer>> => {
    const request: PackagesListReposRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.listRepos",
      params: {},
    };
    const outcome = await invokeTyped(request, "packages.listRepos", "0.1", isPackagesReposResult);
    if (outcome.kind !== "ok") return outcome;
    const { schemaVersion: familyConst, repos } = outcome.result as {
      schemaVersion: string;
      repos: readonly unknown[];
    };
    if (familyConst === PACKAGES_REPOS_SCHEMA_VERSION_V02) {
      return {
        kind: "ok",
        result: { family: PACKAGES_REPOS_SCHEMA_VERSION_V02, rows: repos as readonly RepoInfoRowV02[] },
      };
    }
    return {
      kind: "ok",
      result: { family: "vua.packages-repos/v0.1", rows: repos as readonly RepoInfoRowV01[] },
    };
  };

  /** P2 目录事实:按需查询(双键闭集);typed 码照原词(no_matching_package
   * = 独立空态呈现,页面处理);族常量在窄化校验时消费并决定返回词面
   * (v0.1 七键 / v0.2 八键含 cacheSourced 披露),盖戳辨族永不猜测 */
  const packageCatalogRaw = async (projectPath: string, packageId: string): Promise<TypedOutcome<CatalogPackageFactsV01 | CatalogPackageFactsV02>> => {
    const request: PackagesPackageCatalogRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.packageCatalog",
      params: { projectPath, packageId },
    };
    const outcome = await invokeTyped(request, "packages.packageCatalog", "0.1", isPackagesCatalogResult);
    if (outcome.kind !== "ok") return outcome;
    const { schemaVersion: familyConst, ...facts } = outcome.result;
    // 组合校验已按族通过:剥信封键后按盖戳族断言对应词面事实
    const typedFacts = familyConst === "vua.packages-catalog/v0.2"
      ? (facts as unknown as CatalogPackageFactsV02)
      : (facts as unknown as CatalogPackageFactsV01);
    return { kind: "ok", result: typedFacts };
  };

  /** F2 仓库级目录事实(027 冻结词面):双键必带可空 params verbatim
   * 传输(repoId/packageIds null 原样上 wire;packageIds 非空 = 唯一非
   * 空 id 闭列,UI 不构造空数组);typed 码照原词(repo_not_found = 逐字
   * 透传不折叠,P2 读面零折叠);族常量在窄化校验时消费,剥信封键后
   * 返回事实文档,盖戳辨词面永不猜测 */
  const readRepoCatalog = async (
    repoId: string | null,
    packageIds: readonly string[] | null,
  ): Promise<TypedOutcome<RepoCatalogFactsV01>> => {
    const request: PackagesRepoCatalogRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.repoCatalog",
      params: {
        repoId,
        packageIds: packageIds === null ? null : [...packageIds],
      },
    };
    const outcome = await invokeTyped(request, "packages.repoCatalog", "0.1", isRepoCatalogResult);
    if (outcome.kind !== "ok") return outcome;
    const { schemaVersion: _familyConst, ...facts } = outcome.result;
    return { kind: "ok", result: facts as unknown as RepoCatalogFactsV01 };
  };
  const pendingCatalogs = new Map<string, Promise<TypedOutcome<RepoCatalogFactsV01>>>();
  const repoCatalogRaw = (repoId: string | null, packageIds: readonly string[] | null): Promise<TypedOutcome<RepoCatalogFactsV01>> => {
    const key = JSON.stringify([repoId, packageIds]);
    let pending = pendingCatalogs.get(key);
    if (!pending) {
      pending = readRepoCatalog(repoId, packageIds).finally(() => { pendingCatalogs.delete(key); });
      pendingCatalogs.set(key, pending);
    }
    return pending;
  };

  /** F5 模板条目枚举事实(027 冻结词面):params 空闭集(环境级配置面,
   *  listRepos 空闭集先例,任何键 = 词表外形状违反);零网络无
   *  cacheSourced(恒常量信息字段不是事实,repos v0.1 同律);typed 码
   *  照原词(unavailable 缺席臂折叠为 unavailable,其余 failed 照原词
   *  上呈);族常量在窄化校验时消费,剥信封键后返回事实文档,盖戳辨词
   *  面永不猜测;行序 = 服务端冻结 id 升序呈现事实,客户端不重排 */
  const listTemplatesRaw = async (): Promise<TypedOutcome<TemplatesFactsV01>> => {
    const request: PackagesListTemplatesRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.listTemplates",
      params: {},
    };
    const outcome = await invokeTyped(request, "packages.listTemplates", "0.1", isPackagesTemplatesResult);
    if (outcome.kind !== "ok") return outcome;
    const { schemaVersion: _familyConst, ...facts } = outcome.result;
    return { kind: "ok", result: facts as unknown as TemplatesFactsV01 };
  };

  /** A1 确认链第一步(026 冻结词面):同步只读预览,永不变更状态;
   * plan 九键闭集窄化,信封 typed 码照原词(unavailable 缺席臂折叠为
   * unavailable,其余 failed 照原词上呈) */
  const previewRemoveRaw = async (
    projectPath: string,
    packageIds: readonly string[],
  ): Promise<TypedOutcome<PackagesRemovePlanV01>> => {
    const request: PackagesPreviewRemoveRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.previewRemove",
      params: { projectPath, packageIds },
    };
    const outcome = await invokeTyped(request, "packages.previewRemove", "0.1", isPackagesRemovePlanResult);
    if (outcome.kind !== "ok") return outcome;
    return { kind: "ok", result: outcome.result as unknown as PackagesRemovePlanV01 };
  };

  /** A1 确认链第二步(026 冻结词面):任务化写命令(import-copy 同构,
   * 020 result 回流)——受理窄化→终态等待→Done payload 窄化。任务九态
   * 语义归应用契约任务面;任务真实状态由任务中心呈现,本端口只消费终
   * 态结果。超时/断连/形态不齐 = 诚实 unavailable,不猜测不伪造结果文
   * 档(014 先例);rejected 守卫拒绝是 Done payload(任务诚实完成、移
   * 除被拒),不是错误。 */
  const applyRemoveRaw = async (
    projectPath: string,
    packageIds: readonly string[],
    confirmedDigest: string,
  ): Promise<PackagesRemoveApplyOutcome> => {
    const request: PackagesApplyRemoveRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.applyRemove",
      params: { projectPath, packageIds, confirmedDigest },
    };
    const response = await client.invoke(request);
    if (!response.ok) {
      if (response.error.kind === "application") {
        // 受理阶段信封错误:缺席臂折叠 unavailable(引擎未装配/未接线),
        // 其余 typed 码(未注册 vua.project.project_not_found / 能力缺席
        // vua.vpm.capability_missing / 受理持久化失败
        // vua.provider.persistence_failed / invalid_params)照原词 failed
        if (response.error.error.code === "vua.packages.unavailable") {
          return { kind: "unavailable" };
        }
        return { kind: "failed", code: response.error.error.code };
      }
      return { kind: "unavailable" };
    }
    if (!isApplyRemoveAccepted(response.value)) {
      return { kind: "failed", code: "packages_apply_acceptance_shape" };
    }
    const snapshot = await waitForTerminalTask(client, response.value.taskId, PACKAGES_APPLY_TASK_WAIT_MS);
    if (snapshot === null) {
      return { kind: "unavailable" };
    }
    // 冻结不变量(020):result 仅成功终态出现;rejected 守卫拒绝也在成
    // 功终态的 Done payload 内(任务诚实完成)。非成功终态 = 移除未发生
    // (failed/cancelled;恢复非终态绝不隐式续传),error.code 原词上呈,
    // 收不齐 = 诚实降级码,不猜测
    if (snapshot.state !== "succeeded" && snapshot.state !== "succeeded_with_warnings") {
      const errorCode = isRecord(snapshot.error) && typeof snapshot.error.code === "string"
        ? snapshot.error.code
        : "packages_task_not_succeeded";
      return { kind: "failed", code: errorCode };
    }
    const payload = isRecord(snapshot.result) ? snapshot.result : null;
    const body = payload === null ? null : isRecord(payload.result) ? payload.result : null;
    if (body === null) {
      return { kind: "failed", code: "packages_apply_result_shape" };
    }
    if (body.kind === "receipt" && isPackagesRemoveReceiptResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesRemoveReceiptV01 };
    }
    if (body.kind === "rejected" && isPackagesRemoveRejectedResult(body)) {
      return { kind: "rejected", rejection: body as unknown as PackagesRemoveRejectedV01 };
    }
    return { kind: "failed", code: "packages_apply_result_shape" };
  };

  /** A2 确认链第一步(026 v0.2 冻结词面):同步只读安装/升级预览,永不
   * 变更状态;plan 九键闭集窄化(v0.2 戳),信封 typed 码照原词(缺席臂
   * 折叠 unavailable;vua.packages.preview_failed = A2 信封新码,其余
   * failed 照原词上呈) */
  const previewInstallRaw = async (
    projectPath: string,
    packages: readonly PackagesPackageRequestV02[],
  ): Promise<TypedOutcome<PackagesInstallPlanV02>> => {
    const request: PackagesPreviewInstallRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.previewInstall",
      params: { projectPath, packages: packages.map((row) => ({ ...row })) },
    };
    const outcome = await invokeTyped(
      request,
      "packages.previewInstall",
      PACKAGES_OPS_ENVELOPE_V02,
      isPackagesInstallPlanResult,
    );
    if (outcome.kind !== "ok") return outcome;
    return { kind: "ok", result: outcome.result as unknown as PackagesInstallPlanV02 };
  };

  /** A2 确认链第二步(026 v0.2 冻结词面):任务化安装写命令(import-copy
   * 同构,020 result 回流)——受理窄化→终态等待→Done payload 窄化。任
   * 务九态语义归应用契约任务面;任务真实状态由任务中心呈现,本端口只消
   * 费终态结果。apply 段仓库加载不降级(在线加载失败即任务如实失败)。
   * 超时/断连/形态不齐 = 诚实 unavailable,不猜测不伪造结果文档(014 先
   * 例);rejected 守卫拒绝是 Done payload(任务诚实完成、安装被拒),
   * 不是错误。 */
  const applyInstallRaw = async (
    projectPath: string,
    packages: readonly PackagesPackageRequestV02[],
    confirmedDigest: string,
  ): Promise<PackagesInstallApplyOutcome> => {
    const request: PackagesApplyInstallRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.applyInstall",
      params: {
        projectPath,
        packages: packages.map((row) => ({ ...row })),
        confirmedDigest,
      },
    };
    const response = await client.invoke(request);
    if (!response.ok) {
      if (response.error.kind === "application") {
        // 受理阶段信封错误:缺席臂折叠 unavailable(引擎未装配/未接线),
        // 其余 typed 码(未注册 vua.project.project_not_found / 能力缺席
        // vua.vpm.capability_missing / 受理持久化失败
        // vua.provider.persistence_failed / invalid_params)照原词 failed
        if (response.error.error.code === "vua.packages.unavailable") {
          return { kind: "unavailable" };
        }
        return { kind: "failed", code: response.error.error.code };
      }
      return { kind: "unavailable" };
    }
    if (!isApplyInstallAccepted(response.value)) {
      return { kind: "failed", code: "packages_apply_acceptance_shape" };
    }
    const snapshot = await waitForTerminalTask(client, response.value.taskId, PACKAGES_APPLY_TASK_WAIT_MS);
    if (snapshot === null) {
      return { kind: "unavailable" };
    }
    // 冻结不变量(020):result 仅成功终态出现;rejected 守卫拒绝也在成
    // 功终态的 Done payload 内(任务诚实完成)。非成功终态 = 安装未发生
    // (failed/cancelled;恢复非终态绝不隐式续传),error.code 原词上呈,
    // 收不齐 = 诚实降级码,不猜测
    if (snapshot.state !== "succeeded" && snapshot.state !== "succeeded_with_warnings") {
      const errorCode = isRecord(snapshot.error) && typeof snapshot.error.code === "string"
        ? snapshot.error.code
        : "packages_task_not_succeeded";
      return { kind: "failed", code: errorCode };
    }
    const payload = isRecord(snapshot.result) ? snapshot.result : null;
    const body = payload === null ? null : isRecord(payload.result) ? payload.result : null;
    if (body === null) {
      return { kind: "failed", code: "packages_apply_result_shape" };
    }
    if (body.kind === "receipt" && isPackagesInstallReceiptResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesInstallReceiptV02 };
    }
    if (body.kind === "rejected" && isPackagesInstallRejectedResult(body)) {
      return { kind: "rejected", rejection: body as unknown as PackagesOpsRejectedV02 };
    }
    return { kind: "failed", code: "packages_apply_result_shape" };
  };

  /** A3 写命令(026 v0.3 冻结词面):任务化本地包注册(import-copy/A1/A2
   * 同构,020 result 回流)——受理窄化→终态等待→Done payload 窄化。族
   * 中唯一无 preview 对偶:无 digest 无确认链,用户显式提交即确认;幂等
   * 集合添加,AlreadyAdded 折叠为同一个成功事实。任务九态语义归应用契
   * 约任务面;任务真实状态由任务中心呈现,本端口只消费终态结果。超时/
   * 断连/形态不齐 = 诚实 unavailable,不猜测不伪造结果文档(014 先例);
   * rejected 守卫拒绝是 Done payload(任务诚实完成、注册被拒),不是错
   * 误。 */
  const registerLocalPackageRaw = async (
    packageRoot: string,
  ): Promise<PackagesRegisterApplyOutcome> => {
    const request: PackagesRegisterLocalPackageRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.registerLocalPackage",
      params: { packageRoot },
    };
    const response = await client.invoke(request);
    if (!response.ok) {
      if (response.error.kind === "application") {
        // 受理阶段信封错误:缺席臂折叠 unavailable(引擎未装配/未接线),
        // 其余 typed 码(能力缺席 vua.vpm.capability_missing 在路由层答
        // ——绝不进任务/受理持久化失败 vua.provider.persistence_failed/
        // invalid_params)照原词 failed
        if (response.error.error.code === "vua.packages.unavailable") {
          return { kind: "unavailable" };
        }
        return { kind: "failed", code: response.error.error.code };
      }
      return { kind: "unavailable" };
    }
    if (!isApplyRegisterAccepted(response.value)) {
      return { kind: "failed", code: "packages_apply_acceptance_shape" };
    }
    const snapshot = await waitForTerminalTask(client, response.value.taskId, PACKAGES_APPLY_TASK_WAIT_MS);
    if (snapshot === null) {
      return { kind: "unavailable" };
    }
    // 冻结不变量(020):result 仅成功终态出现;rejected 守卫拒绝也在成
    // 功终态的 Done payload 内(任务诚实完成)。非成功终态 = 注册未发生
    // (failed/cancelled;恢复非终态绝不隐式续传),error.code 原词上呈,
    // 收不齐 = 诚实降级码,不猜测
    if (snapshot.state !== "succeeded" && snapshot.state !== "succeeded_with_warnings") {
      const errorCode = isRecord(snapshot.error) && typeof snapshot.error.code === "string"
        ? snapshot.error.code
        : "packages_task_not_succeeded";
      return { kind: "failed", code: errorCode };
    }
    const payload = isRecord(snapshot.result) ? snapshot.result : null;
    const body = payload === null ? null : isRecord(payload.result) ? payload.result : null;
    if (body === null) {
      return { kind: "failed", code: "packages_apply_result_shape" };
    }
    if (body.kind === "registered" && isPackagesRegisterReceiptResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesRegisterReceiptV03 };
    }
    if (body.kind === "rejected" && isPackagesRegisterRejectedResult(body)) {
      return { kind: "rejected", rejection: body as unknown as PackagesRegisterRejectedV03 };
    }
    return { kind: "failed", code: "packages_apply_result_shape" };
  };

  /** A4 写命令共享任务环(026 v0.4 冻结词面;import-copy/A1/A2/A3 同构,
   * 020 result 回流)——受理窄化→终态等待→Done payload 窄化,三方法
   * (addRemoteRepo/addLocalRepo/removeRepo)分立薄封装,收据按 kind 字
   * 面量分派(repoReceipt 双互斥变体/removed/rejected)。照 A3 同律无
   * preview 对偶:无 digest 无确认链,用户显式提交即确认。任务九态语
   * 义归应用契约任务面;任务真实状态由任务中心呈现,本端口只消费终态
   * 结果。超时/断连/形态不齐 = 诚实 unavailable,不猜测不伪造结果文
   * 档(014 先例);rejected 守卫拒绝是 Done payload(任务诚实完成、
   * 写操作被拒),不是错误。 */
  const repoWriteViaTask = async (
    operation: "packages.addRemoteRepo" | "packages.addLocalRepo" | "packages.removeRepo",
    request: PackagesAddRemoteRepoRequestV1 | PackagesAddLocalRepoRequestV1 | PackagesRemoveRepoRequestV1,
  ): Promise<PackagesRepoAddApplyOutcome | PackagesRepoRemoveApplyOutcome> => {
    const response = await client.invoke(request);
    if (!response.ok) {
      if (response.error.kind === "application") {
        // 受理阶段信封错误:缺席臂折叠 unavailable(引擎未装配/未接线),
        // 其余 typed 码(能力缺席 vua.vpm.capability_missing 在路由层答
        // ——绝不进任务/受理持久化失败 vua.provider.persistence_failed/
        // invalid_params)照原词 failed
        if (response.error.error.code === "vua.packages.unavailable") {
          return { kind: "unavailable" };
        }
        return { kind: "failed", code: response.error.error.code };
      }
      return { kind: "unavailable" };
    }
    if (!isApplyRepoAccepted(response.value, operation)) {
      return { kind: "failed", code: "packages_apply_acceptance_shape" };
    }
    const snapshot = await waitForTerminalTask(client, response.value.taskId, PACKAGES_APPLY_TASK_WAIT_MS);
    if (snapshot === null) {
      return { kind: "unavailable" };
    }
    // 冻结不变量(020):result 仅成功终态出现;rejected 守卫拒绝也在成
    // 功终态的 Done payload 内(任务诚实完成、写操作被拒)。非成功终态
    // = 写操作未发生(failed/cancelled;恢复非终态绝不隐式续传),
    // error.code 原词上呈,收不齐 = 诚实降级码,不猜测
    if (snapshot.state !== "succeeded" && snapshot.state !== "succeeded_with_warnings") {
      const errorCode = isRecord(snapshot.error) && typeof snapshot.error.code === "string"
        ? snapshot.error.code
        : "packages_task_not_succeeded";
      return { kind: "failed", code: errorCode };
    }
    const payload = isRecord(snapshot.result) ? snapshot.result : null;
    const body = payload === null ? null : isRecord(payload.result) ? payload.result : null;
    if (body === null) {
      return { kind: "failed", code: "packages_apply_result_shape" };
    }
    if (body.kind === "repoReceipt" && isPackagesRemoteRepoReceiptResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesRemoteRepoAddedV04 };
    }
    if (body.kind === "repoReceipt" && isPackagesLocalRepoReceiptResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesLocalRepoAddedV04 };
    }
    if (body.kind === "removed" && isPackagesRepoRemovedResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesRepoRemovedV04 };
    }
    if (body.kind === "rejected" && isPackagesRepoRejectedResult(body)) {
      return { kind: "rejected", rejection: body as unknown as PackagesRepoRejectedV04 };
    }
    return { kind: "failed", code: "packages_apply_result_shape" };
  };

  /** A4 添加远端订阅(026 v0.4 冻结词面):params 双键闭集 {url, name}
   * verbatim 传输;收据按 remote 变体收窄——方法与收据变体不对应 =
   * 服务端词面违反,诚实降级不冒充成功;成功后广播新快照(订阅面已
   * 变,列表按新事实重取) */
  const addRemoteRepoRaw = async (url: string, name: string): Promise<PackagesRepoAddApplyOutcome> => {
    const request: PackagesAddRemoteRepoRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.addRemoteRepo",
      params: { url, name },
    };
    const outcome = await repoWriteViaTask("packages.addRemoteRepo", request);
    if (outcome.kind === "ok") {
      const { receipt } = outcome;
      if (receipt.kind !== "repoReceipt" || receipt.repoType !== "remote") {
        return { kind: "failed", code: "packages_apply_result_shape" };
      }
      broadcast();
      return { kind: "ok", receipt };
    }
    return outcome;
  };

  /** A4 添加本地目录订阅(026 v0.4 冻结词面):params 双键闭集
   * {path, name} verbatim 传输;收据按 local 变体收窄(同上诚实纪律);
   * 成功后广播新快照 */
  const addLocalRepoRaw = async (path: string, name: string): Promise<PackagesRepoAddApplyOutcome> => {
    const request: PackagesAddLocalRepoRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.addLocalRepo",
      params: { path, name },
    };
    const outcome = await repoWriteViaTask("packages.addLocalRepo", request);
    if (outcome.kind === "ok") {
      const { receipt } = outcome;
      if (receipt.kind !== "repoReceipt" || receipt.repoType !== "local") {
        return { kind: "failed", code: "packages_apply_result_shape" };
      }
      broadcast();
      return { kind: "ok", receipt };
    }
    return outcome;
  };

  /** A4 移除订阅(026 v0.4 冻结词面):params 单键闭集 {repoId} verbatim
   * 传输(稳定行柄;未知 id = 执行时端口答 repo_not_found 折 rejected,
   * 移除面不宣称幂等);收据按 removed 变体收窄(同上诚实纪律);成功
   * 后广播新快照 */
  const removeRepoRaw = async (repoId: string): Promise<PackagesRepoRemoveApplyOutcome> => {
    const request: PackagesRemoveRepoRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.removeRepo",
      params: { repoId },
    };
    const outcome = await repoWriteViaTask("packages.removeRepo", request);
    if (outcome.kind === "ok") {
      const { receipt } = outcome;
      if (receipt.kind !== "removed") {
        return { kind: "failed", code: "packages_apply_result_shape" };
      }
      broadcast();
      return { kind: "ok", receipt };
    }
    return outcome;
  };

  /** F4 生命周期任务环(027 v0.6 冻结词面;A4 removeRepo 同构,020
   * result 回流)——受理窄化→终态等待→Done payload 窄化,三方法薄封装
   * 共用,收据按 kind 字面量分派(enabled/disabled/refreshed/rejected)。
   * 照 A3/A4 同律无 preview 对偶且根在端口:无 digest 无确认链,用户显
   * 式提交即确认。任务九态语义归应用契约任务面;任务真实状态由任务中心
   * 呈现,本端口只消费终态结果。超时/断连/形态不齐 = 诚实 unavailable,
   * 不猜测不伪造结果文档(014 先例);rejected 守卫拒绝是 Done payload
   * (任务诚实完成、操作被拒),不是错误——重复启停不宣称幂等。 */
  const lifecycleViaTask = async (
    operation: "packages.enableRepo" | "packages.disableRepo" | "packages.refreshRepo",
    request: PackagesEnableRepoRequestV1 | PackagesDisableRepoRequestV1 | PackagesRefreshRepoRequestV1,
  ): Promise<PackagesRepoLifecycleApplyOutcome> => {
    const response = await client.invoke(request);
    if (!response.ok) {
      if (response.error.kind === "application") {
        // 受理阶段信封错误:缺席臂折叠 unavailable(引擎未装配/未接线),
        // 其余 typed 码(能力缺席 vua.vpm.capability_missing 在路由层答
        // ——绝不进任务/受理持久化失败/invalid_params)照原词 failed
        if (response.error.error.code === "vua.packages.unavailable") {
          return { kind: "unavailable" };
        }
        return { kind: "failed", code: response.error.error.code };
      }
      return { kind: "unavailable" };
    }
    if (!isApplyLifecycleAccepted(response.value, operation)) {
      return { kind: "failed", code: "packages_apply_acceptance_shape" };
    }
    const snapshot = await waitForTerminalTask(client, response.value.taskId, PACKAGES_APPLY_TASK_WAIT_MS);
    if (snapshot === null) {
      return { kind: "unavailable" };
    }
    // 冻结不变量(020):result 仅成功终态出现;rejected 守卫拒绝也在成
    // 功终态的 Done payload 内(任务诚实完成、操作被拒)。非成功终态 =
    // 操作未发生(failed/cancelled;恢复非终态绝不隐式续传),error.code
    // 原词上呈,收不齐 = 诚实降级码,不猜测
    if (snapshot.state !== "succeeded" && snapshot.state !== "succeeded_with_warnings") {
      const errorCode = isRecord(snapshot.error) && typeof snapshot.error.code === "string"
        ? snapshot.error.code
        : "packages_task_not_succeeded";
      return { kind: "failed", code: errorCode };
    }
    const payload = isRecord(snapshot.result) ? snapshot.result : null;
    const body = payload === null ? null : isRecord(payload.result) ? payload.result : null;
    if (body === null) {
      return { kind: "failed", code: "packages_apply_result_shape" };
    }
    if (body.kind === "enabled" && isPackagesRepoEnabledResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesRepoEnabledV06 };
    }
    if (body.kind === "disabled" && isPackagesRepoDisabledResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesRepoDisabledV06 };
    }
    if (body.kind === "refreshed" && isPackagesRepoRefreshedResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesRepoRefreshedV06 };
    }
    if (body.kind === "rejected" && isPackagesRepoRejectedResultV06(body)) {
      return { kind: "rejected", rejection: body as unknown as PackagesRepoRejectedV06 };
    }
    return { kind: "failed", code: "packages_apply_result_shape" };
  };

  /** F4 启用(027 v0.6 冻结词面):params 单键闭集 {repoId} verbatim;
   * 方法与收据变体不对应 = 服务端词面违反,诚实降级不冒充成功;成功后
   * 广播新快照(订阅面 enabled 位已变,列表按新事实重取——禁用在列不
   * 隐藏,新状态读回权威在 v0.2 行) */
  const enableRepoRaw = async (repoId: string): Promise<PackagesRepoLifecycleApplyOutcome> => {
    const request: PackagesEnableRepoRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.enableRepo",
      params: { repoId },
    };
    const outcome = await lifecycleViaTask("packages.enableRepo", request);
    if (outcome.kind === "ok") {
      const { receipt } = outcome;
      if (receipt.kind !== "enabled") {
        return { kind: "failed", code: "packages_apply_result_shape" };
      }
      broadcast();
      return { kind: "ok", receipt };
    }
    return outcome;
  };

  /** F4 禁用(027 v0.6 冻结词面):禁用行离开包集合世界但保留订阅面在
   * 列;同律收窄与广播 */
  const disableRepoRaw = async (repoId: string): Promise<PackagesRepoLifecycleApplyOutcome> => {
    const request: PackagesDisableRepoRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.disableRepo",
      params: { repoId },
    };
    const outcome = await lifecycleViaTask("packages.disableRepo", request);
    if (outcome.kind === "ok") {
      const { receipt } = outcome;
      if (receipt.kind !== "disabled") {
        return { kind: "failed", code: "packages_apply_result_shape" };
      }
      broadcast();
      return { kind: "ok", receipt };
    }
    return outcome;
  };

  /** F4 刷新(027 v0.6 冻结词面):该行自身 localPath 缓存 etag 条件抓
   * 取;refreshed 收据 REQUIRED cacheUpdated 两臂皆成功——false = etag
   * 未变「已是最新」,结果非错误,呈现层如实呈现(缓存面已变与否均广
   * 播刷新,订阅行 cached 位按新事实重取) */
  const refreshRepoRaw = async (repoId: string): Promise<PackagesRepoLifecycleApplyOutcome> => {
    const request: PackagesRefreshRepoRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.refreshRepo",
      params: { repoId },
    };
    const outcome = await lifecycleViaTask("packages.refreshRepo", request);
    if (outcome.kind === "ok") {
      const { receipt } = outcome;
      if (receipt.kind !== "refreshed") {
        return { kind: "failed", code: "packages_apply_result_shape" };
      }
      broadcast();
      return { kind: "ok", receipt };
    }
    return outcome;
  };

  /** A5 写命令任务环(026 v0.5 冻结词面;import-copy/A1–A4 同构,020
   * result 回流)——受理窄化→终态等待→Done payload 窄化,单方法
   * createProject 薄封装,收据按 kind 字面量分派(created/rejected)。
   * 照 A3/A4 同律无 preview 对偶且根在端口:无 digest 无确认链,用户显
   * 式表单提交即确认。任务九态语义归应用契约任务面;任务真实状态由任
   * 务中心呈现,本端口只消费终态结果。超时/断连/形态不齐 = 诚实
   * unavailable,不猜测不伪造结果文档(014 先例);rejected 守卫拒绝是
   * Done payload(任务诚实完成、创建被拒),不是错误——创建不幂等,重
   * 复目录执行时拒绝如实上呈。 */
  const createViaTask = async (
    request: PackagesCreateProjectRequestV1,
  ): Promise<PackagesCreateApplyOutcome> => {
    const response = await client.invoke(request);
    if (!response.ok) {
      if (response.error.kind === "application") {
        // 受理阶段信封错误:缺席臂折叠 unavailable(引擎未装配/未接线),
        // 其余 typed 码(能力缺席 vua.vpm.capability_missing 在路由层答
        // ——绝不进任务/受理持久化失败 vua.provider.persistence_failed/
        // invalid_params)照原词 failed
        if (response.error.error.code === "vua.packages.unavailable") {
          return { kind: "unavailable" };
        }
        return { kind: "failed", code: response.error.error.code };
      }
      return { kind: "unavailable" };
    }
    if (!isApplyCreateAccepted(response.value, "packages.createProject")) {
      return { kind: "failed", code: "packages_apply_acceptance_shape" };
    }
    const snapshot = await waitForTerminalTask(client, response.value.taskId, PACKAGES_APPLY_TASK_WAIT_MS);
    if (snapshot === null) {
      return { kind: "unavailable" };
    }
    // 冻结不变量(020):result 仅成功终态出现;rejected 守卫拒绝也在成
    // 功终态的 Done payload 内(任务诚实完成、创建被拒)。非成功终态 =
    // 创建未发生(failed/cancelled;恢复非终态绝不隐式续传),error.code
    // 原词上呈,收不齐 = 诚实降级码,不猜测
    if (snapshot.state !== "succeeded" && snapshot.state !== "succeeded_with_warnings") {
      const errorCode = isRecord(snapshot.error) && typeof snapshot.error.code === "string"
        ? snapshot.error.code
        : "packages_task_not_succeeded";
      return { kind: "failed", code: errorCode };
    }
    const payload = isRecord(snapshot.result) ? snapshot.result : null;
    const body = payload === null ? null : isRecord(payload.result) ? payload.result : null;
    if (body === null) {
      return { kind: "failed", code: "packages_apply_result_shape" };
    }
    if (body.kind === "created" && isPackagesProjectCreatedResult(body)) {
      return { kind: "ok", receipt: body as unknown as PackagesProjectCreatedV05 };
    }
    if (body.kind === "rejected" && isPackagesCreateRejectedResult(body)) {
      return { kind: "rejected", rejection: body as unknown as PackagesCreateRejectedV05 };
    }
    return { kind: "failed", code: "packages_apply_result_shape" };
  };

  /** A5 项目创建(026 v0.5 冻结词面):params 三键闭集 {parent, name,
   * template} verbatim 传输(template REQUIRED-nullable:null = 后端默
   * 认解析,非空串 = verbatim);收据按 created 变体收窄——非 created
   * 结果冒充创建成功 = 服务端词面违反,诚实降级不冒充成功;创建即在册
   * (冻结端口事实),成功后广播新快照,在册列表刷新即见 */
  const createProjectRaw = async (
    parent: string,
    name: string,
    template: string | null,
  ): Promise<PackagesCreateApplyOutcome> => {
    const request: PackagesCreateProjectRequestV1 = {
      schemaVersion: 1,
      requestId: crypto.randomUUID(),
      method: "packages.createProject",
      params: { parent, name, template },
    };
    const outcome = await createViaTask(request);
    if (outcome.kind === "ok") {
      const { receipt } = outcome;
      if (receipt.kind !== "created") {
        return { kind: "failed", code: "packages_apply_result_shape" };
      }
      broadcast();
      return { kind: "ok", receipt };
    }
    return outcome;
  };

  // 无事件推送源:快照按需聚合(选中项目变化或 capability.changed 驱动
  // 重取),订阅仅作能力行翻转的通知通道
  let selectedProjectPath: string | null = null;

  /** repos 面装配:能力行 false = 区块不渲染(repos: []);调用 unavailable
   * = 引擎缺席/断连(与 installed 同源)→ 整页 not-connected;typed 失败
   * = reposError 照原词(失败不冒充空态) */
  const fetchView = async (): Promise<PackagesView> => {
    const capabilityRows = await readCapabilityRows();
    if (!capabilityRows.installed) return { schemaVersion: 1, kind: "not-connected" };
    const reposOutcome = capabilityRows.repos ? await listReposRaw() : null;
    if (reposOutcome !== null && reposOutcome.kind === "unavailable") {
      return { schemaVersion: 1, kind: "not-connected" };
    }
    const reposFace: { repos: readonly (RepoInfoRowV01 | RepoInfoRowV02)[]; reposWordFace?: "vua.packages-repos/v0.2" } & { reposError?: { code: string } } =
      reposOutcome === null
        ? { repos: [] }
        : reposOutcome.kind === "ok"
          ? reposOutcome.result.family === PACKAGES_REPOS_SCHEMA_VERSION_V02
            ? { repos: reposOutcome.result.rows, reposWordFace: PACKAGES_REPOS_SCHEMA_VERSION_V02 }
            : { repos: reposOutcome.result.rows }
          : { repos: [], reposError: { code: reposOutcome.code } };
    if (selectedProjectPath === null) {
      return {
        schemaVersion: 1,
        kind: "ready-p2",
        blocks: {
          installed: true,
          repos: capabilityRows.repos,
          catalog: capabilityRows.catalog,
          changes: capabilityRows.changes,
          installs: capabilityRows.installs,
          registers: capabilityRows.registers,
          repoWrites: capabilityRows.repoWrites,
          creates: capabilityRows.creates,
          repoCatalog: capabilityRows.repoCatalog,
          templates: capabilityRows.templates,
          repoLifecycle: capabilityRows.repoLifecycle,
        },
        projectPath: null,
        installedPackages: [],
        ...reposFace,
      };
    }
    const outcome = await listInstalledRaw(selectedProjectPath);
    if (outcome.kind === "unavailable") {
      return { schemaVersion: 1, kind: "not-connected" };
    }
    return {
      schemaVersion: 1,
      kind: "ready-p2",
      blocks: {
        installed: true,
        repos: capabilityRows.repos,
        catalog: capabilityRows.catalog,
        changes: capabilityRows.changes,
        installs: capabilityRows.installs,
        registers: capabilityRows.registers,
        repoWrites: capabilityRows.repoWrites,
        creates: capabilityRows.creates,
        repoCatalog: capabilityRows.repoCatalog,
        templates: capabilityRows.templates,
        repoLifecycle: capabilityRows.repoLifecycle,
      },
      projectPath: selectedProjectPath,
      // 027 F3 双族组装:v0.2 族应答携判定行集＋cacheSourced 披露;
      // v0.1 族应答零新增字段(既有 P1 呈现零回归,绝不虚构标注)
      ...(outcome.kind === "ok" && outcome.result.family === "vua.packages-installed/v0.2"
        ? {
            installedPackages: outcome.result.rows,
            installedCacheSourced: outcome.result.cacheSourced,
          }
        : { installedPackages: outcome.kind === "ok" ? outcome.result.rows : [] }),
      ...(outcome.kind === "failed" ? { loadError: { code: outcome.code } } : {}),
      ...reposFace,
    };
  };

  const listeners = new Set<(view: PackagesView) => void>();
  const broadcast = () => {
    void fetchView().then(
      (view) => {
        for (const callback of listeners) callback(view);
      },
      () => {
        /* 断连期间保留上一视图(environment 端口同纪律) */
      },
    );
  };

  return {
    snapshot: fetchView,
    subscribe(callback) {
      listeners.add(callback);
      return (() => {
        listeners.delete(callback);
      }) as Unsubscribe;
    },
    async selectProject(projectPath) {
      // 视图的项目身份 = 013 注册路径;能力行不可用时保持 not-connected
      const capabilityRows = await readCapabilityRows();
      if (capabilityRows.installed) {
        selectedProjectPath = projectPath;
        broadcast();
      }
      return fetchView();
    },
    listInstalled: listInstalledRaw,
    packageCatalog: packageCatalogRaw,
    repoCatalog: repoCatalogRaw,
    listTemplates: listTemplatesRaw,
    async previewRemove(projectPath, packageIds) {
      const outcome = await previewRemoveRaw(projectPath, packageIds);
      return outcome.kind === "ok"
        ? { kind: "ok", plan: outcome.result }
        : outcome;
    },
    applyRemove: applyRemoveRaw,
    async previewInstall(projectPath, packages) {
      const outcome = await previewInstallRaw(projectPath, packages);
      return outcome.kind === "ok"
        ? { kind: "ok", plan: outcome.result }
        : outcome;
    },
    applyInstall: applyInstallRaw,
    registerLocalPackage: registerLocalPackageRaw,
    addRemoteRepo: addRemoteRepoRaw,
    addLocalRepo: addLocalRepoRaw,
    removeRepo: removeRepoRaw,
    createProject: createProjectRaw,
    async addProject() {
      return { kind: "unavailable" };
    },
    async importLocalPackage() {
      return { kind: "unavailable" };
    },
    async previewChanges() {
      return { kind: "unavailable" };
    },
    async applyChanges() {
      return { kind: "unavailable" };
    },
    // F4 仓库生命周期写面(027 v0.6 消费批):enableRepo/disableRepo/
    // refreshRepo 三方法走 gateway 真实面——原 setRepoEnabled 本地假翻转
    // (仅重取视图,状态从未变更)就此退役,本地翻转绝不冒充 wire 写面
    enableRepo: enableRepoRaw,
    disableRepo: disableRepoRaw,
    refreshRepo: refreshRepoRaw,
    async capability(): Promise<CapabilityReport> {
      const capabilityRows = await readCapabilityRows();
      return capabilityRows.installed
        ? { state: "ready" }
        : { state: "unavailable", detailKey: "packagesEngineMissing" };
    },
  };
}
