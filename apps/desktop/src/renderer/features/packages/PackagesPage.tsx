import { useEffect, useMemo, useRef, useState } from "react";
import { Badge } from "../../components/primitives/Badge.tsx";
import { toast } from "../../components/primitives/toast-store.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { EmptyState } from "../../components/primitives/EmptyState.tsx";
import { Skeleton } from "../../components/primitives/Skeleton.tsx";
import { Icon } from "@vua/design-system";
import { format, strings, termLabel } from "../../i18n/index.ts";
import {
  useDataSource,
  useGateway,
  usePackagesView,
  type CapabilityReport,
  type CatalogPackageFactsV01,
  type CatalogPackageFactsV02,
  type ChangeRequest,
  type InstalledPackageRowV01,
  type InstalledPackageRowV02,
  type PackageChangePreview,
  type PackageEntryResult,
  type PackageProject,
  type PackageRow,
  type PackageSource,
  type PackagesInstallPlanV02,
  type PackagesInstallReceiptV02,
  type PackagesOpsRejectedV02,
  type PackagesRegisterApplyOutcome,
  type PackagesRemovePlanV01,
  type PackagesRemoveReceiptV01,
  type PackagesRemoveRejectedV01,
  type PackagesTemplateItemV01,
  type RegisteredProjectRow,
  type RepoCatalogFactsV01,
  type RepoCatalogRepoRowV01,
  type RepoInfoRowV01,
  type RepoInfoRowV02,
} from "../../gateway/index.ts";
import { ChangesDialog } from "./ChangesDialog.tsx";
import { InstallConfirmDialog } from "./InstallConfirmDialog.tsx";
import { PackageDetailDrawer } from "./PackageDetailDrawer.tsx";
import { PackageTable } from "./PackageTable.tsx";
import { ProjectCompatSection } from "./ProjectCompatSection.tsx";
import { RemoveConfirmDialog } from "./RemoveConfirmDialog.tsx";
import { RepoSection } from "./RepoSection.tsx";
import {
  SEARCH_DEBOUNCE_MS,
  createEnvelopeErrorKey,
  createRefusalDetailKey,
  filterPackages,
  filterRepoCatalogPackages,
  installEnvelopeErrorKey,
  installLatestRequests,
  installedUpdateCellState,
  invalidReasonKey,
  isEmptyPreview,
  migrationSummaryKey,
  rangeSelect,
  registerEnvelopeErrorKey,
  removeEnvelopeErrorKey,
  removeGuardKey,
  repoEnvelopeErrorKey,
  repoLifecycleToggleAvailable,
  sortPackages,
  sortProjects,
  sourceTextKeys,
  type InstalledUpdateCellState,
} from "./packages-model.ts";
import "./packages.css";

const copy = strings.packages;

/**
 * 包管理页(S-XVI):Recipe 之外的手动 VPM 操作面。
 *
 * 状态机(诚实纪律):
 * - capability 查询中 → 骨架;unavailable/error → 整页诚实未接入空态
 *   (detailKey 文案来自 strings.capability.details);
 * - 视图 not-connected → EmptyState;无项目 → 空态 + 添加按钮(capability 门控);
 * - 视图 ready-p1(024 P1 中间诚实态)→ 项目选择器(013 注册清单)+
 *   已装包简表;repos 无词表行恒不渲染;移除写入口随 removeOps 能力行
 *   解锁(026 A1 消费批:previewRemove 确认链 + applyRemove 任务面);
 *   本地包注册区块随 registerOps 能力行解锁(026 A3 消费批:无确认链,
 *   用户显式提交即确认);
 * - 视图 ready-p2(025 P2 读面消费批)→ P1 布局 + 仓库订阅区块(能力行
 *   解锁,cached=false「已订阅·缓存未建立」诚实态,零健康拟态词)+
 *   行内目录查询入口(按需;compatible 绑定选中工程,无工程上下文不
 *   渲染入口;no_matching_package = 独立空态;updateAvailable null =
 *   更新行不渲染;v0.2 cacheSourced=true =「缓存数据」信息标注,v0.1
 *   无字段不虚构)+ 移除写入口(随 removeOps 能力行解锁,同 P1)+
 *   安装/升级写入口(026 A2 消费批:目录面板内「安装最新」/版本行
 *   「安装此版本」,随 installOps 能力行解锁;version null = 解析器选
 *   最新稳定版,string = 钉死精确版本——A2 词面不立 upgrade 动词)+
 *   批量多选安装(C 面自决,026 A2 消费面:已装表多选列 + 批量条随
 *   installOps 能力行解锁;批量行全部 version null = 解析器语义,与
 *   单包「安装/升级到最新」同语义;可装性不预判,权威在服务端)+
 *   本地包注册区块(026 A3 消费批:随 registerOps 能力行解锁,{packageRoot}
 *   单键手输 + 显式提交,无 preview 无确认链——注册是幂等集合添加,
 *   AlreadyAdded 折叠为同一个成功事实;register_capabilities 访问器
 *   翻转前能力行如实 unavailable,区块诚实缺席);
 * - A4 仓库订阅增删(026 v0.4 消费批,ready-p1/ready-p2 两视图):仓库
 *   订阅管理区块随 repoOps 能力行解锁(一行服务三方法,wire 门按方法
 *   绝不按面)——添加远端/本地仓库双键表单 + 显式提交(无 preview 无
 *   确认链,用户提交即确认;添加面不宣称幂等,拒绝如实呈现)+ 订阅
 *   行内移除两击确认(repoId 非 null 行;id 缺席行不在移除可达范围,
 *   不渲染入口);启停/重排不在任何已冻结词面内,桌面不发明入口;
 * - 有项目 → 项目头 + 迁移卡 + 工具栏 + 表格;切换项目时表格区骨架
 *   (stale-while-revalidate,其余区域不清空);
 * - demo 泛型变更链(fixture 面):previewChanges → ChangesDialog 确认 →
 *   applyChanges;破坏性预览的确认钮由 ChangesDialog 内 DelayedButton
 *   延迟解锁;live A1 词面链(026):previewRemove → RemoveConfirmDialog
 *   (conflicts 警示 + destructive 延迟确认)→ applyRemove(任务化)→
 *   receipt/rejected 终态内联呈现;live A2 词面链(026 v0.2):
 *   previewInstall → InstallConfirmDialog(同构,收据键集互斥)→
 *   applyInstall(任务化)→ 终态内联呈现;live A3 注册链(026 v0.3):
 *   registerLocalPackage 显式提交(无 preview 无确认链,用户提交即确认)
 *   → 任务化执行 → registered/rejected 行内呈现——各链分立互不污染。
 */

type PackagesSection = "packages" | "repos";

/* ---- 项目头:项目下拉(收藏置顶,无效禁用)+ 添加项目文件夹 ---- */

function ProjectHeader({
  projects,
  selectedProjectId,
  capable,
  onSelect,
  onAdd,
}: {
  projects: readonly PackageProject[];
  selectedProjectId: string | null;
  capable: boolean;
  onSelect: (projectId: string) => void;
  onAdd: () => void;
}) {
  const ordered = sortProjects(projects);
  const invalid = ordered.filter((project) => !project.valid);
  return (
    <div className="vua-packages__main">
      <div className="vua-packages__project-header">
        <select
          className="vua-packages__project-select"
          aria-label={copy.projects.selectorAria}
          value={selectedProjectId ?? ""}
          onChange={(event) => onSelect(event.target.value)}
        >
          {selectedProjectId === null ? (
            <option value="" disabled>
              {copy.projects.selectorAria}
            </option>
          ) : null}
          {ordered.map((project) => (
            <option key={project.id} value={project.id} disabled={!project.valid}>
              {project.unityVersion
                ? `${project.name} · Unity ${project.unityVersion}`
                : project.name}
            </option>
          ))}
        </select>
        {capable ? (
          <Button variant="default" onClick={onAdd}>
            <Icon name="folder" size={16} />
            {copy.projects.addProject}
          </Button>
        ) : null}
      </div>
      {/* 无效项目:禁用 + 解释(图标 + 文字,原因可发现) */}
      {invalid.length > 0 ? (
        <ul className="vua-packages__invalid-projects">
          {invalid.map((project) => (
            <li key={project.id}>
              <Icon name="warning" size={16} />
              <span className="vua-caption">
                {format(copy.projects.invalidLine, {
                  name: project.name,
                  reason:
                    copy.projects.invalidReasons[invalidReasonKey(project.invalidReasonKey)],
                })}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

/* ---- 迁移建议卡:存在即出现;真实迁移未接入 → 诚实说明,无假按钮 ---- */

function MigrationHintCard({ summaryKey }: { summaryKey: string }) {
  const key = migrationSummaryKey(summaryKey);
  // 词表外键:不渲染卡片,不编造文案
  if (key === null) return null;
  return (
    <div className="vua-packages__migration">
      <Icon name="question" size={16} />
      <div>
        <p>{copy.migration.summaries[key]}</p>
        <p className="vua-caption vua-text-secondary">{copy.migration.note}</p>
      </div>
    </div>
  );
}

/* ---- 工具栏:搜索(防抖)+ 来源筛选 + 导入本地包 + 预发布开关 ---- */

function PackageToolbar({
  searchText,
  source,
  showPrereleases,
  capable,
  onSearch,
  onSource,
  onTogglePrereleases,
  onImport,
}: {
  searchText: string;
  source: PackageSource | "";
  showPrereleases: boolean;
  capable: boolean;
  onSearch: (text: string) => void;
  onSource: (source: PackageSource | "") => void;
  onTogglePrereleases: (checked: boolean) => void;
  onImport: () => void;
}) {
  return (
    <div className="vua-packages__toolbar" role="search">
      <input
        type="search"
        className="vua-packages__search"
        placeholder={copy.toolbar.searchPlaceholder}
        aria-label={copy.toolbar.searchAria}
        value={searchText}
        onChange={(event) => onSearch(event.target.value)}
      />
      <select
        className="vua-packages__filter"
        aria-label={copy.toolbar.sourceFilterAria}
        value={source}
        onChange={(event) => onSource(event.target.value as PackageSource | "")}
      >
        <option value="">{copy.toolbar.allSources}</option>
        {(Object.keys(sourceTextKeys) as PackageSource[]).map((key) => (
          <option key={key} value={key}>
            {copy.sources[key]}
          </option>
        ))}
      </select>
      {capable ? (
        <Button variant="default" onClick={onImport}>
          {copy.toolbar.importLocal}
        </Button>
      ) : null}
      <label className="vua-packages__prerelease">
        <input
          type="checkbox"
          checked={showPrereleases}
          onChange={(event) => onTogglePrereleases(event.target.checked)}
        />
        <span className="vua-caption">{copy.toolbar.showPrereleases}</span>
      </label>
    </div>
  );
}

/* ---- P1 中间诚实态(024 冻结批消费批):「已安装可看、变更面不可用」。
 * 区块可用性标注(ready-p1 blocks)驱动渲染:repos/changes 类型级恒 false
 * = 无词表无事实源,分区切换器与一切写入口不渲染;包行三键
 * packageId/version/dependencies 照实显示,更新语义列与版本枚举 UI 无
 * 事实源不渲染(虚假断言防线);displayName 无生产者字段,以 packageId
 * 兼任显示(024 核心裁决 3,桌面表态既定走向)。 ---- */

function P1Notice({ changesOpen }: { changesOpen: boolean }) {
  return (
    <div className="vua-packages__migration">
      <Icon name="question" size={16} />
      <p>{changesOpen ? copy.p1.noticeChangesOpen : copy.p1.notice}</p>
    </div>
  );
}

function P2Notice({ changesOpen }: { changesOpen: boolean }) {
  return (
    <div className="vua-packages__migration">
      <Icon name="question" size={16} />
      <p>{changesOpen ? copy.p2.noticeChangesOpen : copy.p2.notice}</p>
    </div>
  );
}

/** P1 项目选择器:清单来自 013 project.listProjects(同一注册事实,无第二
 * 项目身份,path 即词面 projectPath);登记路径缺失的行禁用可见,形状不
 * 符行以计数如实呈现(诚实纪律:缺席可见,不猜测内容)。 */
function P1ProjectPicker({
  projects,
  unreadable,
  selectedProjectPath,
  onSelect,
}: {
  projects: readonly RegisteredProjectRow[];
  unreadable: number | null;
  selectedProjectPath: string | null;
  onSelect: (projectPath: string) => void;
}) {
  const ordered = [...projects].sort((a, b) => a.name.localeCompare(b.name));
  return (
    <div className="vua-packages__main">
      <div className="vua-packages__project-header">
        <select
          className="vua-packages__project-select"
          aria-label={copy.projects.selectorAria}
          value={selectedProjectPath ?? ""}
          onChange={(event) => onSelect(event.target.value)}
        >
          {selectedProjectPath === null ? (
            <option value="" disabled>
              {copy.projects.selectorAria}
            </option>
          ) : null}
          {ordered.map((project) => (
            <option key={project.path} value={project.path} disabled={!project.pathPresent}>
              {project.unityVersion !== null
                ? `${project.name} · Unity ${project.unityVersion}`
                : project.name}
            </option>
          ))}
        </select>
      </div>
      {unreadable !== null && unreadable > 0 ? (
        <ul className="vua-packages__invalid-projects">
          <li>
            <Icon name="warning" size={16} />
            <span className="vua-caption">{format(copy.p1.unreadableProjects, { count: unreadable })}</span>
          </li>
        </ul>
      ) : null}
    </div>
  );
}

/** P1 已装包简表:loadError 存在时呈现 typed 失败(错误码原词),绝不以
 * 空态冒充;零已装包 = 合法空数组的诚实空态(两种形态严格区分)。
 * onShowCatalog 仅 P2 分支传入:目录事实(catalog)区块可用时行内提供
 * 按需目录查询入口,无工程上下文不由本表控制(页面级门控)。
 * onRemove 仅 blocks.changes(026 A1 removeOps 能力行)可用时传入:
 * 行内移除写入口,能力行缺席 = 入口不渲染(渲染层不伪造)。
 * installBulk 仅 blocks.installs(026 A2 installOps 能力行)可用时传入:
 * 批量多选安装面(C 面自决,026 A2 消费面)——选择列 + sticky 批量条,
 * 批量语义 = 选中行全部「安装/升级到最新」(version null = 解析器选
 * 最新稳定版,与单包入口同语义;钉版本粒度保留目录面板单包入口);
 * 能力行缺席 = 选择列与批量条不渲染(渲染层不伪造)。 */
function P1InstalledTable({
  rows,
  loadErrorCode,
  cacheSourced,
  onShowCatalog,
  onRemove,
  removeBusy,
  onUpdate,
  updateBusy,
  installBulk,
}: {
  rows: readonly (InstalledPackageRowV01 | InstalledPackageRowV02)[];
  loadErrorCode: string | null;
  /** 027 F3:仅 v0.2 族应答且 true 时呈现「缓存数据」信息标注(复用
   * catalog 措辞;v0.1 族应答无该字段绝不虚构标注——族常量判别) */
  cacheSourced?: boolean;
  onShowCatalog?: (packageId: string) => void;
  onRemove?: (packageId: string) => void;
  removeBusy?: boolean;
  /** 027 F3:行内升级键(随 blocks.installs 能力行解锁,渲染层不伪造);
   * 语义 = A2 安装面 version null(解析器选最新稳定版,零新升级动词) */
  onUpdate?: (packageId: string) => void;
  updateBusy?: boolean;
  installBulk?: {
    /** 当前选中 packageId 集(行序 = 用户勾选顺序,提交时照实透传) */
    selectedIds: readonly string[];
    busy: boolean;
    onToggleRow: (packageId: string, shiftKey: boolean) => void;
    onToggleAll: () => void;
    onClear: () => void;
    onRun: (packageIds: readonly string[]) => void;
  };
}) {
  if (loadErrorCode !== null) {
    return (
      <EmptyState
        title={copy.p1.loadFailedTitle}
        description={format(copy.p1.loadFailed, { code: loadErrorCode })}
      />
    );
  }
  if (rows.length === 0) {
    return (
      <EmptyState
        title={copy.empty.noPackagesTitle}
        description={copy.p1.emptyInstalledDescription}
      />
    );
  }
  const bulk = installBulk;
  const allSelected = rows.every((row) => bulk?.selectedIds.includes(row.packageId) === true);
  const someSelected = rows.some((row) => bulk?.selectedIds.includes(row.packageId) === true);
  return (
    <div className="vua-packages__table-wrap">
      {/* 027 F3 cacheSourced 信息性标注:仅 v0.2 族应答且 true 时呈现
       * (复用 catalog 措辞,信息性非失败);v0.1 族应答无该字段,绝不
       * 虚构标注(族常量判别,盖戳辨词面) */}
      {cacheSourced ? (
        <p className="vua-packages__cache-note" role="note">
          <Icon name="question" size={16} /> {copy.p2.catalogCachedData}
        </p>
      ) : null}
      <div className="vua-packages__table-scroll">
      <table className="vua-packages__table">
        <thead>
          <tr>
            {bulk ? (
              <th>
                <input
                  type="checkbox"
                  aria-label={copy.columns.selectAll}
                  checked={allSelected}
                  ref={(element) => {
                    if (element) element.indeterminate = !allSelected && someSelected;
                  }}
                  onChange={bulk.onToggleAll}
                />
              </th>
            ) : null}
            <th scope="col">{copy.columns.name}</th>
            <th scope="col">{copy.columns.installed}</th>
            <th scope="col">{copy.p1.updatableColumn}</th>
            <th scope="col">{copy.p1.dependenciesColumn}</th>
            {onShowCatalog ? <th scope="col">{copy.p2.catalogColumn}</th> : null}
            {onRemove ? <th scope="col">{copy.remove.column}</th> : null}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.packageId}>
              {bulk ? (
                <td>
                  <input
                    type="checkbox"
                    aria-label={format(copy.columns.selectRow, { name: row.packageId })}
                    checked={bulk.selectedIds.includes(row.packageId)}
                    onChange={(event) =>
                      bulk.onToggleRow(
                        row.packageId,
                        (event.nativeEvent as MouseEvent).shiftKey === true,
                      )
                    }
                  />
                </td>
              ) : null}
              <td data-column={copy.columns.name}>
                <div className="vua-packages__name">
                  <span className="vua-packages__name-title" title={row.packageId}>
                    {row.packageId}
                  </span>
                </div>
              </td>
              <td data-column={copy.columns.installed}>{row.version}</td>
              {/* 027 F3 三态呈现(纯函数 installedUpdateCellState):absent
               * = v0.1 族应答行无判定事实(该列诚实空显,绝不虚构);
               * notExecuted = updateAvailable null(判定未执行,如实空显
               * 携悬浮说明——null 绝不是「已最新」绝不默认 false,024 表
               * 态②);noneUnderFilter = false(精确语义「当前条件下无严
               * 格更新」,不泛化);available = true(呈现有更新＋行内升级
               * 键复用 A2 version=null 语义,随 installs 能力行解锁) */}
              <td data-column={copy.p1.updatableColumn}>
                {(() => {
                  const cell: InstalledUpdateCellState = installedUpdateCellState(row);
                  if (cell === "absent") return null;
                  if (cell === "notExecuted") {
                    return <span title={copy.p1.updateNotExecuted} />;
                  }
                  if (cell === "noneUnderFilter") {
                    return <span className="vua-packages__update-none">{copy.p1.updateNoneUnderFilter}</span>;
                  }
                  return (
                    <span className="vua-packages__update-cell">
                      {copy.states.updateAvailable}
                      {onUpdate ? (
                        <Button
                          variant="subtle"
                          disabled={updateBusy}
                          aria-label={format(copy.menu.updateToLatest, {}) + " — " + row.packageId}
                          onClick={() => onUpdate(row.packageId)}
                        >
                          {copy.menu.updateToLatest}
                        </Button>
                      ) : null}
                    </span>
                  );
                })()}
              </td>
              <td
                data-column={copy.p1.dependenciesColumn}
                title={row.dependencies.length > 0 ? row.dependencies.join(", ") : undefined}
              >
                {format(copy.p1.dependenciesCount, { count: row.dependencies.length })}
              </td>
              {onShowCatalog ? (
                <td data-column={copy.p2.catalogColumn}>
                  <Button variant="subtle" onClick={() => onShowCatalog(row.packageId)}>
                    {copy.p2.catalogColumn}
                  </Button>
                </td>
              ) : null}
              {onRemove ? (
                <td data-column={copy.remove.column}>
                  <Button
                    variant="subtle"
                    disabled={removeBusy}
                    onClick={() => onRemove(row.packageId)}
                  >
                    {copy.menu.remove}
                  </Button>
                </td>
              ) : null}
            </tr>
          ))}
        </tbody>
      </table>
      {/* 批量条:sticky 贴表格滚动区底部(与 demo 面同构),出现时不推挤
       * 上方内容;按钮 = 批量「安装/升级到最新」(version null 解析器语义,
       * 可装性权威在服务端预览,空变更以 toast 如实反馈) */}
      {bulk && bulk.selectedIds.length > 0 ? (
        <div className="vua-packages__bulkbar">
          <span className="vua-caption">
            {format(copy.bulk.selectedCount, { count: bulk.selectedIds.length })}
          </span>
          <Button
            variant="default"
            disabled={bulk.busy}
            onClick={() => bulk.onRun(bulk.selectedIds)}
          >
            {copy.install.latest}
          </Button>
          <Button variant="subtle" onClick={bulk.onClear}>
            {copy.bulk.clear}
          </Button>
        </div>
      ) : null}
      </div>
    </div>
  );
}

/* ---- P2 读面(025 冻结批消费批):仓库订阅清单 + 单包目录按需查询。
 * 健康面 = 词面非目标,零健康拟态词;cached=false =「已订阅·缓存未建
 * 立」独立诚实态(绝不呈现为空目录);目录查询需工程上下文(compatible
 * 绑定选中工程),无工程上下文不渲染入口;no_matching_package = 独立
 * 空态非错误页;updateAvailable null = 判定未执行,更新行不渲染不以
 * 默认值填充(P1 防线);yanked 事实仅版本行携带,本地包(versions 空
 * 数组)无 yanked 断言。 ---- */

/** P2 仓库订阅清单:行序 = 订阅面自身顺序(配置事实,不重排);
 * reposError = typed 失败照原词呈现,与空数组零订阅严格区分;
 * 移除入口(026 A4 消费批)仅 repoId 非 null 的行渲染——id 缺席行不
 * 在本词面移除可达范围(协议本载明的诚实边界),UI 不发明;移除 =
 * 行内两击确认(第一击进入确认态,再击执行;纯 UX 步骤,不发明词面
 * 事实——删除订阅行不删任何包文件,ADR-0006 延迟警示路径不适用)。 */
function P2ReposSection({
  repos,
  reposErrorCode,
  browseEnabled,
  gateway,
  onRemoveRequest,
  removeConfirmId,
  removeBusyId,
  removeOutcome,
  lifecycleEnabled,
  onLifecycle,
  lifecycleBusyAction,
  lifecycleBusyRepoId,
  lifecycleOutcome,
}: {
  repos: readonly (RepoInfoRowV01 | RepoInfoRowV02)[];
  reposErrorCode: string | null;
  /** F2 仓库浏览入口(blocks.repoCatalog 能力行)——false = 展开入口
   *  不渲染(诚实缺席);repoId 缺席行不在浏览可达范围(id 缺席 = 无
   *  稳定行柄,scoped 查询无从寻址——A4 移除入口同款纪律),UI 不发明 */
  browseEnabled?: boolean;
  gateway?: ReturnType<typeof useGateway>;
  onRemoveRequest?: (repoId: string) => void;
  removeConfirmId?: string | null;
  removeBusyId?: string | null;
  removeOutcome?: RepoRemoveOutcomeView | null;
  /** F4 生命周期控制(blocks.repoLifecycle 能力行)——false/缺席 =
   *  启停/刷新控制不渲染,订阅行照常呈现(降级非错误——F5 TemplatesFace
   *  constant-absence 同构);v0.1 族行无 enabled 位 = 启停控制仍不渲染
   *  (状态不可知不猜测),刷新控制不依赖 enabled 位可独立渲染;
   *  repoId null 行在词面可达范围之外(removeRepo 同边界),无任何控制 */
  lifecycleEnabled?: boolean;
  onLifecycle?: (action: "enable" | "disable" | "refresh", repoId: string) => void;
  lifecycleBusyAction?: "enable" | "disable" | "refresh" | null;
  lifecycleBusyRepoId?: string | null;
  lifecycleOutcome?: RepoLifecycleOutcomeView | null;
}) {
  const [browseRepoId, setBrowseRepoId] = useState<string | null>(null);
  if (reposErrorCode !== null) {
    return (
      <EmptyState
        title={copy.p2.reposLoadFailedTitle}
        description={format(copy.p2.reposLoadFailed, { code: reposErrorCode })}
      />
    );
  }
  if (repos.length === 0) {
    return (
      <EmptyState
        title={copy.p2.reposEmptyTitle}
        description={copy.p2.reposEmptyDescription}
      />
    );
  }
  return (
    <Card>
      <h2 className="vua-packages__section-title">{copy.p2.reposTitle}</h2>
      <ul className="vua-packages__repo-list">
        {repos.map((repo, index) => {
          const title = repo.name ?? repo.repoId ?? copy.p2.repoNoIdentifier;
          const location = repo.url ?? repo.localPath;
          const browseOpen = browseEnabled === true && browseRepoId !== null && browseRepoId === repo.repoId;
          // F4 生命周期:v0.2 行携带 enabled 位(词面读回权威);v0.1 行无
          // 此位 = 启停控制不渲染(状态不可知不猜测——行族分派经
          // repoLifecycleToggleAvailable,148 批反向审查修复:原实现仅按
          // blocks.repoLifecycle 门控,v0.1 行误渲染启停按钮且词面恒为
          // 「禁用」)。Rows without an identifier reserve a disabled refresh slot;
          // they never dispatch a lifecycle command. Disabled rows remain visible.
          const rowEnabled = "enabled" in repo ? repo.enabled : undefined;
          const lifecycleRow = lifecycleEnabled === true
            && onLifecycle
            && repo.repoId !== null;
          const lifecycleToggleRow = lifecycleRow && repoLifecycleToggleAvailable(repo);
          const lifecycleToggling = lifecycleBusyAction !== null
            && lifecycleBusyAction !== "refresh"
            && lifecycleBusyRepoId === repo.repoId;
          const lifecycleRefreshing = lifecycleBusyAction === "refresh"
            && lifecycleBusyRepoId === repo.repoId;
          const anyLifecycleBusy = lifecycleBusyAction !== null;
          return (
            <li key={`${repo.repoId ?? "repo"}-${index}`} className="vua-packages__repo-row">
              <div className="vua-packages__repo-main">
                <span className="vua-packages__name-title" title={repo.repoId ?? undefined}>
                  {title}
                </span>
                {rowEnabled === false ? (
                  <Badge tone="warning">{copy.repoWrite.lifecycle.disabledBadge}</Badge>
                ) : null}
                <Badge tone={repo.cached ? "neutral" : "warning"}>
                  {repo.cached ? copy.p2.repoCached : copy.p2.repoNotCached}
                </Badge>
                {browseEnabled && repo.repoId !== null && gateway ? (
                  <Button
                    variant="subtle"
                    aria-expanded={browseOpen}
                    aria-label={format(copy.p2.browseAria, { name: title })}
                    onClick={() => setBrowseRepoId(browseOpen ? null : (repo.repoId as string))}
                  >
                    {browseOpen ? copy.p2.browseClose : copy.p2.browseAction}
                  </Button>
                ) : null}
                {lifecycleToggleRow ? (
                  <Button
                    variant="subtle"
                    disabled={anyLifecycleBusy}
                    aria-label={format(
                      rowEnabled === false ? copy.repoWrite.lifecycle.enableAria : copy.repoWrite.lifecycle.disableAria,
                      { name: title },
                    )}
                    onClick={() => onLifecycle(rowEnabled === false ? "enable" : "disable", repo.repoId as string)}
                  >
                    {lifecycleToggling
                      ? rowEnabled === false
                        ? copy.repoWrite.lifecycle.enabling
                        : copy.repoWrite.lifecycle.disabling
                      : rowEnabled === false
                        ? copy.repoWrite.lifecycle.enableAction
                        : copy.repoWrite.lifecycle.disableAction}
                  </Button>
                ) : null}
                <Button
                  variant="subtle"
                  className="vua-packages__repo-refresh"
                  disabled={!lifecycleRow || anyLifecycleBusy}
                  aria-label={format(copy.repoWrite.lifecycle.refreshAria, { name: title })}
                  onClick={() => { if (lifecycleRow && repo.repoId !== null) onLifecycle?.("refresh", repo.repoId); }}
                >
                  {lifecycleRefreshing ? copy.repoWrite.lifecycle.refreshing : copy.repoWrite.lifecycle.refreshAction}
                </Button>
                {onRemoveRequest && repo.repoId !== null ? (
                  <Button
                    variant="subtle"
                    className="vua-packages__repo-remove"
                    disabled={removeBusyId != null}
                    aria-label={format(copy.repoWrite.removeAria, { name: title })}
                    onClick={() => onRemoveRequest(repo.repoId as string)}
                  >
                    {removeBusyId === repo.repoId
                      ? copy.repoWrite.removing
                      : removeConfirmId === repo.repoId
                        ? copy.repoWrite.removeConfirm
                        : copy.repoWrite.removeAction}
                  </Button>
                ) : null}
              </div>
              {location !== null && location !== undefined ? (
                <span className="vua-caption vua-text-secondary" title={location}>
                  {repo.url !== null ? `${copy.p2.repoUrlLabel}: ${repo.url}` : `${copy.p2.repoLocalPathLabel}: ${repo.localPath}`}
                </span>
              ) : null}
              {rowEnabled === false ? (
                <span className="vua-caption vua-text-secondary">
                  {copy.repoWrite.lifecycle.disabledNote}
                </span>
              ) : null}
              {browseOpen && repo.repoId !== null && gateway ? (
                <RepoCatalogPanel
                  repoId={repo.repoId}
                  repoName={title}
                  gateway={gateway}
                  onClose={() => setBrowseRepoId(null)}
                />
              ) : null}
            </li>
          );
        })}
      </ul>
      {lifecycleOutcome?.kind === "ok" ? (
        <div className="vua-packages__register-result" role="status">
          <Icon name="check" size={16} />
          <span className="vua-caption">
            {lifecycleOutcome.cacheUpdated === false
              ? /* 呈现锚二:cacheUpdated=false = etag 未变「已是最新」——
                 * 两臂皆成功,信息呈现绝非错误 */
                copy.repoWrite.lifecycle.upToDate
              : format(copy.repoWrite.lifecycle.doneLine, { repoId: lifecycleOutcome.repoId })}
          </span>
        </div>
      ) : null}
      {lifecycleOutcome?.kind === "rejected" ? (
        <div className="vua-packages__register-result vua-packages__register-result--rejected" role="alert">
          <Icon name="warning" size={16} />
          <span className="vua-caption">
            {copy.repoWrite.guards[removeGuardKey(lifecycleOutcome.guard)]}
            {" "}
            {format(copy.repoWrite.rejectedDetail, { detail: lifecycleOutcome.detail })}
          </span>
        </div>
      ) : null}
      {removeOutcome?.kind === "ok" ? (
        <div className="vua-packages__register-result" role="status">
          <Icon name="check" size={16} />
          <span className="vua-caption">
            {format(copy.repoWrite.removedLine, { repoId: removeOutcome.repoId })}
          </span>
        </div>
      ) : null}
      {removeOutcome?.kind === "rejected" ? (
        <div className="vua-packages__register-result vua-packages__register-result--rejected" role="alert">
          <Icon name="warning" size={16} />
          <span className="vua-caption">
            {copy.repoWrite.guards[removeGuardKey(removeOutcome.guard)]}
            {" "}
            {format(copy.repoWrite.rejectedDetail, { detail: removeOutcome.detail })}
          </span>
        </div>
      ) : null}
    </Card>
  );
}

/* ---- A3 本地包注册区块(026 v0.3 消费批):blocks.registers(packages.
 * registerOps 能力行)门控,false = 区块不渲染(诚实缺席)。族中唯一无
 * preview 对偶的写面——无确认链:用户显式提交即确认,无 DelayedButton
 * 无对话框(非破坏性,ADR-0006 破坏性警示路径不适用,本面不发明破坏性
 * 事实)。空输入 = 按钮禁用(词面 minLength 1,UI 不构造违例请求)。
 * ok(registered 三键回显)/rejected(guard 文案 + detail 原词)行内
 * 呈现;failed/unavailable 关闭为 toast 诚实说明——任务真实状态由任务
 * 中心呈现。幂等语义如实呈现:重复注册同一包根 = 同一个成功事实。 ---- */

/** A3 注册区块行内终态:ok 保留收据回显 / rejected 保留拒绝呈现;
 * failed/unavailable 不留行内状态( toast 说明后复位)。 */
type RegisterOutcomeView =
  | { readonly kind: "ok"; readonly packageRoot: string }
  | { readonly kind: "rejected"; readonly guard: string; readonly detail: string };

function RegisterSection({
  outcome,
  busy,
  path,
  onPathChange,
  onSubmit,
}: {
  outcome: RegisterOutcomeView | null;
  busy: boolean;
  path: string;
  onPathChange: (value: string) => void;
  onSubmit: () => void;
}) {
  const trimmed = path.trim();
  const usable = trimmed.length > 0 && !busy;
  return (
    <Card>
      <h2 className="vua-packages__section-title">{copy.register.title}</h2>
      <p className="vua-caption vua-text-secondary">{copy.register.description}</p>
      <div className="vua-packages__register-row">
        <input
          type="text"
          className="vua-packages__register-input"
          placeholder={copy.register.placeholder}
          aria-label={copy.register.inputAria}
          value={path}
          onChange={(event) => onPathChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && usable) onSubmit();
          }}
        />
        <Button variant="default" disabled={!usable} onClick={onSubmit}>
          <Icon name="folder" size={16} />
          {busy ? copy.register.submitting : copy.register.action}
        </Button>
      </div>
      {outcome?.kind === "ok" ? (
        <div className="vua-packages__register-result" role="status">
          <Icon name="check" size={16} />
          <span className="vua-caption">
            {format(copy.register.successLine, { packageRoot: outcome.packageRoot })}
          </span>
        </div>
      ) : null}
      {outcome?.kind === "rejected" ? (
        <div className="vua-packages__register-result vua-packages__register-result--rejected" role="alert">
          <Icon name="warning" size={16} />
          <span className="vua-caption">
            {copy.register.guards[removeGuardKey(outcome.guard)]}
            {" "}
            {format(copy.register.rejectedDetail, { detail: outcome.detail })}
          </span>
        </div>
      ) : null}
    </Card>
  );
}

/* ---- A4 仓库订阅增删区块(026 v0.4 消费批):blocks.repoWrites(
 * packages.repoOps 能力行,一行服务三方法)门控,false = 区块不渲染
 * (诚实缺席)。照 A3 同律无 preview 无确认链——用户显式提交即确认,
 * 无 DelayedButton 无对话框(添加与移除订阅行都是非破坏性:不删任何
 * 包文件与项目内容,ADR-0006 破坏性警示路径不适用,本面不发明破坏性
 * 事实);订阅行移除的行内两击确认是纯 UX 步骤。空输入 = 按钮禁用
 * (词面 minLength 1,UI 不构造违例请求)。**添加面不宣称幂等**——
 * 文案不写「重复安全」(与 A3 注册刻意不同),库面拒绝重复订阅时
 * rejected 如实行内呈现,不发明幂等成功。ok(repoReceipt 变体回显)/
 * rejected(guard 文案 + detail 原词)行内呈现;failed/unavailable
 * 关闭为 toast 诚实说明——任务真实状态由任务中心呈现。启停/重排
 * 不在任何已冻结词面内——本区块不发明入口。 ---- */

/** A4 添加表单行内终态:ok 保留收据回显行 / rejected 保留拒绝呈现;
 * failed/unavailable 不留行内状态(toast 说明后复位)。 */
type RepoAddOutcomeView =
  | { readonly kind: "ok"; readonly line: string }
  | { readonly kind: "rejected"; readonly guard: string; readonly detail: string };

/** A4 移除行内终态:ok 保留被删行 repoId 回显 / rejected 保留拒绝呈现。 */
type RepoRemoveOutcomeView =
  | { readonly kind: "ok"; readonly repoId: string }
  | { readonly kind: "rejected"; readonly guard: string; readonly detail: string };

/** F4 生命周期行内终态:ok 保留收据回显(cacheUpdated 仅 refreshed 收据
 * 携带——false = etag 未变「已是最新」,信息呈现非错误)/ rejected 保留
 * 拒绝呈现;failed/unavailable 不留行内状态(toast 说明后复位)。 */
type RepoLifecycleOutcomeView =
  | { readonly kind: "ok"; readonly repoId: string; readonly cacheUpdated: boolean | null }
  | { readonly kind: "rejected"; readonly guard: string; readonly detail: string };

function RepoWriteSection({
  busy,
  remoteUrl,
  remoteName,
  localPath,
  localName,
  remoteOutcome,
  localOutcome,
  onRemoteUrlChange,
  onRemoteNameChange,
  onLocalPathChange,
  onLocalNameChange,
  onAddRemote,
  onAddLocal,
}: {
  busy: "remote" | "local" | null;
  remoteUrl: string;
  remoteName: string;
  localPath: string;
  localName: string;
  remoteOutcome: RepoAddOutcomeView | null;
  localOutcome: RepoAddOutcomeView | null;
  onRemoteUrlChange: (value: string) => void;
  onRemoteNameChange: (value: string) => void;
  onLocalPathChange: (value: string) => void;
  onLocalNameChange: (value: string) => void;
  onAddRemote: () => void;
  onAddLocal: () => void;
}) {
  const remoteUsable =
    remoteUrl.trim().length > 0 && remoteName.trim().length > 0 && busy === null;
  const localUsable =
    localPath.trim().length > 0 && localName.trim().length > 0 && busy === null;
  return (
    <Card>
      <h2 className="vua-packages__section-title">{copy.repoWrite.title}</h2>
      <p className="vua-caption vua-text-secondary">{copy.repoWrite.description}</p>
      <div className="vua-packages__repowrite-group">
        <h3 className="vua-caption">{copy.repoWrite.remoteHeadline}</h3>
        <div className="vua-packages__register-row">
          <input
            type="text"
            className="vua-packages__register-input"
            placeholder={copy.repoWrite.remoteUrlPlaceholder}
            aria-label={copy.repoWrite.remoteUrlAria}
            value={remoteUrl}
            onChange={(event) => onRemoteUrlChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && remoteUsable) onAddRemote();
            }}
          />
          <input
            type="text"
            className="vua-packages__register-input vua-packages__repowrite-name"
            placeholder={copy.repoWrite.remoteNamePlaceholder}
            aria-label={copy.repoWrite.remoteNameAria}
            value={remoteName}
            onChange={(event) => onRemoteNameChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && remoteUsable) onAddRemote();
            }}
          />
          <Button variant="default" disabled={!remoteUsable} onClick={onAddRemote}>
            <Icon name="cloud" size={16} />
            {busy === "remote" ? copy.repoWrite.adding : copy.repoWrite.remoteAction}
          </Button>
        </div>
        {remoteOutcome?.kind === "ok" ? (
          <div className="vua-packages__register-result" role="status">
            <Icon name="check" size={16} />
            <span className="vua-caption">{remoteOutcome.line}</span>
          </div>
        ) : null}
        {remoteOutcome?.kind === "rejected" ? (
          <div className="vua-packages__register-result vua-packages__register-result--rejected" role="alert">
            <Icon name="warning" size={16} />
            <span className="vua-caption">
              {copy.repoWrite.guards[removeGuardKey(remoteOutcome.guard)]}
              {" "}
              {format(copy.repoWrite.rejectedDetail, { detail: remoteOutcome.detail })}
            </span>
          </div>
        ) : null}
      </div>
      <div className="vua-packages__repowrite-group">
        <h3 className="vua-caption">{copy.repoWrite.localHeadline}</h3>
        <div className="vua-packages__register-row">
          <input
            type="text"
            className="vua-packages__register-input"
            placeholder={copy.repoWrite.localPathPlaceholder}
            aria-label={copy.repoWrite.localPathAria}
            value={localPath}
            onChange={(event) => onLocalPathChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && localUsable) onAddLocal();
            }}
          />
          <input
            type="text"
            className="vua-packages__register-input vua-packages__repowrite-name"
            placeholder={copy.repoWrite.localNamePlaceholder}
            aria-label={copy.repoWrite.localNameAria}
            value={localName}
            onChange={(event) => onLocalNameChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && localUsable) onAddLocal();
            }}
          />
          <Button variant="default" disabled={!localUsable} onClick={onAddLocal}>
            <Icon name="folder" size={16} />
            {busy === "local" ? copy.repoWrite.adding : copy.repoWrite.localAction}
          </Button>
        </div>
        {localOutcome?.kind === "ok" ? (
          <div className="vua-packages__register-result" role="status">
            <Icon name="check" size={16} />
            <span className="vua-caption">{localOutcome.line}</span>
          </div>
        ) : null}
        {localOutcome?.kind === "rejected" ? (
          <div className="vua-packages__register-result vua-packages__register-result--rejected" role="alert">
            <Icon name="warning" size={16} />
            <span className="vua-caption">
              {copy.repoWrite.guards[removeGuardKey(localOutcome.guard)]}
              {" "}
              {format(copy.repoWrite.rejectedDetail, { detail: localOutcome.detail })}
            </span>
          </div>
        ) : null}
      </div>
    </Card>
  );
}

/* ---- A5 项目创建区块(026 v0.5 消费批):blocks.creates(packages.createOps
 * 能力行,一行一方法)门控,false = 区块不渲染(诚实缺席)。照 A3/A4 同律
 * 无 preview 无确认链——全新项目目录无既有状态可 diff 无摘要可绑定,用户
 * 显式表单提交即确认。表单:parent 路径输入(不发明目录枚举/选择器)+
 * name;template 选填,留空 = null = 后端默认模板解析(冻结词面事实)。
 * 027 F5 消费批:blocks.templates(packages.templatesOps 能力行)为真时
 * 挂载即查 packages.listTemplates(环境级配置面,页面局部承载 F2 先例),
 * 枚举 ready = 模板下拉替换手填(显示行逐字用 name = id 冻结同值投影,
 * 绝不虚构更友好标签;选中项 value = id 作 createProject template 参数
 * 机器标识原样传递);回落纪律——能力行缺席/loading 外的枚举不可用
 * (空数组 = 诚实零模板应答,目录根缺失是事实非错误/typed 失败/unavailable)
 * 一律回落现行手填 + 留空 = 后端默认解析(026 A5 留白填面语义原样),
 * 空数组绝不渲染成错误、缺席绝不虚构模板清单;typed 失败(错误码原词)
 * 与能力缺席呈现严格区分,失败不冒充空清单。loading 期手填禁用(枚举
 * 即将就位,不制造「手填值遗留到下拉世界」的展示错位)。创建即在册(
 * 冻结端口事实)如实文案:成功即注册、在册列表刷新即见。**创建不幂等**:
 * 目标目录已存在等拒绝如实行内呈现(库路径四拒绝腿按 detail 原码呈现四
 * 语语义文案;CLI 腿与词外 detail 回落 guard 文案 + detail 原词,绝不合并
 * 词绝不猜测),不发明幂等成功。空输入 = 按钮禁用(词面 minLength 1,UI
 * 不构造违例请求;template 空串 = 词面形状违反,UI 只构造 null)。ok(
 * created 收据 projectPath = 注册路径身份回显)/rejected 行内呈现;
 * failed/unavailable 关闭为 toast 诚实说明——任务真实状态由任务中心呈
 * 现。 ---- */

/** F5 模板枚举读面形态(null = blocks.templates false,未查询不虚构):
 * loading / ready(枚举行集,服务端冻结 id 升序呈现事实原样)/ empty
 * (诚实零模板)/ failed(typed 错误码原词)/ unavailable(能力缺席) */
type TemplatesFace =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly templates: readonly PackagesTemplateItemV01[] }
  | { readonly kind: "empty" }
  | { readonly kind: "failed"; readonly code: string }
  | { readonly kind: "unavailable" };

/** A5 创建表单行内终态:ok 保留注册路径回显 / rejected 保留拒绝呈现;
 * failed/unavailable 不留行内状态(toast 说明后复位)。 */
type CreateOutcomeView =
  | { readonly kind: "ok"; readonly projectPath: string }
  | { readonly kind: "rejected"; readonly guard: string; readonly detail: string };

function CreateSection({
  busy,
  parent,
  name,
  template,
  outcome,
  templatesBlock,
  gateway,
  onParentChange,
  onNameChange,
  onTemplateChange,
  onSubmit,
}: {
  busy: boolean;
  parent: string;
  name: string;
  template: string;
  outcome: CreateOutcomeView | null;
  /** F5 能力行(blocks.templates):false = 模板下拉不渲染,创建表单
   *  回落手填(渲染层不伪造) */
  templatesBlock: boolean;
  gateway: ReturnType<typeof useGateway>;
  onParentChange: (value: string) => void;
  onNameChange: (value: string) => void;
  onTemplateChange: (value: string) => void;
  onSubmit: () => void;
}) {
  const usable = parent.trim().length > 0 && name.trim().length > 0 && !busy;
  const refusalKey =
    outcome?.kind === "rejected" ? createRefusalDetailKey(outcome.detail) : null;
  // F5 模板枚举(027 消费批):能力行为真时挂载即查一次(环境级配置面,
  // 非随项目/随行变化;F2 页面局部承载先例)。查询只此一处,失败/缺席
  // 如实落形态,绝不重试轰炸绝不虚构清单
  const [templatesFace, setTemplatesFace] = useState<TemplatesFace | null>(null);
  useEffect(() => {
    if (!templatesBlock) return;
    let active = true;
    setTemplatesFace({ kind: "loading" });
    void gateway.packages.listTemplates().then((result) => {
      if (!active) return;
      if (result.kind === "ok") {
        setTemplatesFace(
          result.result.templates.length === 0
            ? { kind: "empty" }
            : { kind: "ready", templates: result.result.templates },
        );
      } else if (result.kind === "failed") {
        setTemplatesFace({ kind: "failed", code: result.code });
      } else {
        setTemplatesFace({ kind: "unavailable" });
      }
    });
    return () => {
      active = false;
    };
  }, [gateway, templatesBlock]);
  const templateSelector = templatesFace?.kind === "ready" ? templatesFace.templates : null;
  return (
    <Card>
      <h2 className="vua-packages__section-title">{copy.create.title}</h2>
      <p className="vua-caption vua-text-secondary">{copy.create.description}</p>
      <div className="vua-packages__repowrite-group">
        <div className="vua-packages__register-row">
          <input
            type="text"
            className="vua-packages__register-input"
            placeholder={copy.create.parentPlaceholder}
            aria-label={copy.create.parentAria}
            value={parent}
            onChange={(event) => onParentChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && usable) onSubmit();
            }}
          />
          <input
            type="text"
            className="vua-packages__register-input vua-packages__repowrite-name"
            placeholder={copy.create.namePlaceholder}
            aria-label={copy.create.nameAria}
            value={name}
            onChange={(event) => onNameChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && usable) onSubmit();
            }}
          />
          <Button variant="default" disabled={!usable} onClick={onSubmit}>
            <Icon name="folder" size={16} />
            {busy ? copy.create.submitting : copy.create.action}
          </Button>
        </div>
        <div className="vua-packages__register-row">
          {templateSelector !== null ? (
            <select
              className="vua-packages__register-input vua-packages__create-template-select"
              aria-label={copy.create.templateSelectAria}
              value={template}
              onChange={(event) => onTemplateChange(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && usable) onSubmit();
              }}
            >
              <option value="">{copy.create.templateDefaultOption}</option>
              {templateSelector.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          ) : (
            <input
              type="text"
              className="vua-packages__register-input"
              placeholder={copy.create.templatePlaceholder}
              aria-label={copy.create.templateAria}
              value={template}
              disabled={templatesFace?.kind === "loading"}
              onChange={(event) => onTemplateChange(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && usable) onSubmit();
              }}
            />
          )}
        </div>
        {templatesFace?.kind === "loading" ? (
          <p className="vua-caption vua-text-secondary" role="status">
            {copy.create.templatesLoading}
          </p>
        ) : null}
        {templatesFace?.kind === "empty" ? (
          <p className="vua-caption vua-text-secondary">
            {copy.create.templatesEmptyNote}
          </p>
        ) : null}
        {templatesFace?.kind === "failed" ? (
          <p className="vua-caption vua-packages__register-result--rejected" role="alert">
            {format(copy.create.templatesFailedNote, { code: templatesFace.code })}
          </p>
        ) : null}
        {templatesFace?.kind === "unavailable" ? (
          <p className="vua-caption vua-text-secondary">
            {copy.create.templatesUnavailableNote}
          </p>
        ) : null}
      </div>
      {outcome?.kind === "ok" ? (
        <div className="vua-packages__register-result" role="status">
          <Icon name="check" size={16} />
          <span className="vua-caption">
            {format(copy.create.successLine, { projectPath: outcome.projectPath })}
          </span>
        </div>
      ) : null}
      {outcome?.kind === "rejected" ? (
        <div className="vua-packages__register-result vua-packages__register-result--rejected" role="alert">
          <Icon name="warning" size={16} />
          <span className="vua-caption">
            {copy.create.guards[removeGuardKey(outcome.guard)]}
            {refusalKey !== null && refusalKey !== "unknown" ? (
              <> {copy.create.refusals[refusalKey]}</>
            ) : null}
            {" "}
            {format(copy.create.rejectedDetail, { detail: outcome.detail })}
          </span>
        </div>
      ) : null}
    </Card>
  );
}

/** P2 目录事实面板:挂载即按需查询(双键闭集);五种形态严格区分——
 * 加载骨架 / typed 失败(码原词) / no_matching_package 独立空态 /
 * 目录事实行闭集呈现(displayName null 以 packageId 兼任;source 二态
 * × installed 组合呈现;updateAvailable null 时更新行不渲染) /
 * v0.2 cacheSourced=true「缓存数据」信息性标注(非失败;v0.1 应答无
 * 此字段不虚构标注——双族协商,盖戳族常量辨词面永不猜测)。 */
/** P2 单包目录面板(按需查询):安装入口仅 p2.blocks.installs(packages.
 * installOps 能力行,026 A2 消费批)可用时传入 onInstall——能力行缺席 =
 * 入口不渲染(渲染层不伪造);onInstall(packageId, null) = 解析器选最新
 * 稳定版,onInstall(packageId, version) = 钉死精确版本(升级/降级同语
 * 法,A2 词面不立 upgrade 动词);yanked/compatible 事实照实标注,权威判
 * 定在服务端(桌面不预判可装性)。 */
function P2CatalogPanel({
  projectPath,
  packageId,
  gateway,
  onInstall,
  installBusy,
  onClose,
}: {
  projectPath: string;
  packageId: string;
  gateway: ReturnType<typeof useGateway>;
  onInstall?: (packageId: string, version: string | null) => void;
  installBusy?: boolean;
  onClose: () => void;
}) {
  const [outcome, setOutcome] = useState<
    | { kind: "loading" }
    | { kind: "ok"; facts: CatalogPackageFactsV01 | CatalogPackageFactsV02 }
    | { kind: "failed"; code: string }
    | { kind: "unavailable" }
  >({ kind: "loading" });

  useEffect(() => {
    let active = true;
    setOutcome({ kind: "loading" });
    void gateway.packages.packageCatalog(projectPath, packageId).then((result) => {
      if (!active) return;
      if (result.kind === "ok") setOutcome({ kind: "ok", facts: result.result });
      else if (result.kind === "failed") setOutcome({ kind: "failed", code: result.code });
      else setOutcome({ kind: "unavailable" });
    });
    return () => {
      active = false;
    };
  }, [gateway, projectPath, packageId]);

  const facts = outcome.kind === "ok" ? outcome.facts : null;
  const displayName = facts === null ? null : (facts.displayName ?? facts.packageId);
  // v0.2 披露标注:仅盖戳 v0.2 且 cacheSourced=true 时呈现「缓存数据」
  // 信息标注(信息性非失败);v0.1 应答无此字段,绝不虚构标注(双族协商)
  const cacheSourcedLine =
    facts === null || !("cacheSourced" in facts) || !facts.cacheSourced
      ? null
      : copy.p2.catalogCachedData;
  const updateLine =
    facts === null || facts.updateAvailable === null
      ? null
      : facts.updateAvailable
        ? copy.p2.updateAvailableYes
        : copy.p2.updateAvailableNo;
  const sourceLine =
    facts === null
      ? null
      : facts.source === "repo"
        ? facts.installed
          ? copy.p2.sourceRepoInstalled
          : copy.p2.sourceRepoNotInstalled
        : facts.installed
          ? copy.p2.sourceLocalInstalled
          : copy.p2.sourceLocalNotInstalled;

  return (
    <Card>
      <div className="vua-packages__catalog-head">
        <h2 className="vua-packages__section-title">
          {format(copy.p2.catalogTitle, { packageId })}
        </h2>
        <Button variant="subtle" onClick={onClose}>
          {copy.p2.catalogClose}
        </Button>
      </div>
      {outcome.kind === "loading" ? (
        <div className="vua-page__stack">
          <Skeleton width="45%" />
          <Skeleton width="70%" />
        </div>
      ) : outcome.kind === "failed" ? (
        outcome.code === "vua.vpm.no_matching_package" ? (
          <EmptyState
            title={copy.p2.catalogNotFoundTitle}
            description={copy.p2.catalogNotFoundDescription}
          />
        ) : (
          <EmptyState
            title={copy.p2.catalogFailedTitle}
            description={format(copy.p2.catalogQueryFailed, { code: outcome.code })}
          />
        )
      ) : outcome.kind === "unavailable" ? (
        <EmptyState
          title={copy.p2.catalogFailedTitle}
          description={copy.p2.catalogUnavailable}
        />
      ) : (
        <div className="vua-packages__catalog-body">
          <p>
            <span className="vua-caption vua-text-secondary">{copy.columns.name}: </span>
            {displayName}
          </p>
          <p>
            <span className="vua-caption vua-text-secondary">{copy.p2.sourceLabel}: </span>
            {sourceLine}
          </p>
          {cacheSourcedLine !== null ? (
            <p className="vua-caption vua-text-secondary">
              <Icon name="question" size={16} /> {cacheSourcedLine}
            </p>
          ) : null}
          {updateLine !== null ? (
            <p>
              <span className="vua-caption vua-text-secondary">{copy.p2.updateAvailableLabel}: </span>
              {updateLine}
            </p>
          ) : null}
          {onInstall ? (
            <p className="vua-packages__install-latest">
              <Button
                variant="default"
                disabled={installBusy}
                onClick={() => onInstall(packageId, null)}
              >
                {copy.install.latest}
              </Button>
              <span className="vua-caption vua-text-secondary">{copy.install.latestHint}</span>
            </p>
          ) : null}
          <h3 className="vua-packages__section-title">{copy.p2.versionsTitle}</h3>
          {outcome.facts.versions.length === 0 ? (
            <p className="vua-caption vua-text-secondary">{copy.p2.versionsEmpty}</p>
          ) : (
            <ul className="vua-packages__repo-list">
              {outcome.facts.versions.map((version) => (
                <li key={version.version} className="vua-packages__repo-row">
                  <span className="vua-packages__name-title">{version.version}</span>
                  <span className="vua-caption">
                    {version.yanked ? copy.p2.versionYanked : null}
                    {version.compatible === true ? copy.p2.compatibleYes : null}
                    {version.compatible === false ? copy.p2.compatibleNo : null}
                    {version.compatible === null ? copy.p2.compatibleUnknown : null}
                  </span>
                  {onInstall ? (
                    <Button
                      variant="subtle"
                      disabled={installBusy}
                      onClick={() => onInstall(packageId, version.version)}
                    >
                      {copy.install.versionAction}
                    </Button>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </Card>
  );
}

/* ---- F2 仓库浏览面板(027 消费批):行内展开即该仓库可装包浏览面
 * (IA 表态 2)。同一事实源——repoCatalog(repoId, null) 不过滤形态,
 * 搜索框是呈现层过滤(filterRepoCatalogPackages 纯函数),不另立查询
 * 形状(027 设计约束 1)。诚实纪律:cached=false 行 =「已订阅·缓存未建
 * 立」空态;latestVersion null = 如实「无合资格版本」,绝不渲染「已最
 * 新」类断言(024 表态②防线);cacheSourced=true =「缓存数据」信息标注
 * 非失败(复用目录面板同款词面);typed 失败照原词(repo_not_found 逐字
 * 透传——P2 读面零折叠);scoped 应答缺行 = 形状不符如实呈现,无证据
 * 不发明行。 ---- */

function RepoCatalogPanel({
  repoId,
  repoName,
  gateway,
  onClose,
}: {
  repoId: string;
  repoName: string;
  gateway: ReturnType<typeof useGateway>;
  onClose: () => void;
}) {
  const [outcome, setOutcome] = useState<
    | { kind: "loading" }
    | { kind: "ok"; cacheSourced: boolean; row: RepoCatalogRepoRowV01 }
    | { kind: "failed"; code: string }
    | { kind: "unavailable" }
  >({ kind: "loading" });
  const [search, setSearch] = useState("");

  useEffect(() => {
    let active = true;
    setOutcome({ kind: "loading" });
    setSearch("");
    void gateway.packages.repoCatalog(repoId, null).then((result) => {
      if (!active) return;
      if (result.kind === "ok") {
        const row = result.result.repos.find((repo) => repo.repoId === repoId);
        if (row === undefined) {
          setOutcome({ kind: "failed", code: "packages_shape_violation" });
        } else {
          setOutcome({ kind: "ok", cacheSourced: result.result.cacheSourced, row });
        }
      } else if (result.kind === "failed") setOutcome({ kind: "failed", code: result.code });
      else setOutcome({ kind: "unavailable" });
    });
    return () => {
      active = false;
    };
  }, [gateway, repoId]);

  const row = outcome.kind === "ok" ? outcome.row : null;
  const filtered = row === null ? [] : filterRepoCatalogPackages(row.packages, search);

  return (
    <div className="vua-packages__repo-browse">
      <div className="vua-packages__repo-browse-head">
        <h3 className="vua-packages__section-title">
          {format(copy.p2.browseTitle, { name: repoName })}
        </h3>
        <Button variant="subtle" aria-expanded onClick={onClose}>
          {copy.p2.browseClose}
        </Button>
      </div>
      {outcome.kind === "loading" ? (
        <div className="vua-page__stack">
          <Skeleton width="45%" />
          <Skeleton width="70%" />
        </div>
      ) : outcome.kind === "failed" ? (
        <EmptyState
          title={copy.p2.browseFailedTitle}
          description={format(copy.p2.browseQueryFailed, { code: outcome.code })}
        />
      ) : outcome.kind === "unavailable" ? (
        <EmptyState
          title={copy.p2.browseFailedTitle}
          description={copy.p2.browseUnavailable}
        />
      ) : (
        <>
          {outcome.cacheSourced ? (
            <p className="vua-caption vua-text-secondary">
              <Icon name="question" size={16} /> {copy.p2.catalogCachedData}
            </p>
          ) : null}
          {row !== null && !row.cached ? (
            <EmptyState
              title={copy.p2.repoNotCached}
              description={copy.p2.browseNotCachedDescription}
            />
          ) : row !== null && row.packages.length === 0 ? (
            <EmptyState
              title={copy.p2.browseEmptyTitle}
              description={copy.p2.browseEmptyDescription}
            />
          ) : (
            <>
              <input
                type="search"
                className="vua-packages__search"
                placeholder={copy.p2.browseSearchPlaceholder}
                aria-label={copy.p2.browseSearchAria}
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
              {filtered.length === 0 ? (
                <EmptyState
                  title={copy.empty.noResultTitle}
                  description={copy.empty.noResultDescription}
                />
              ) : (
                <ul className="vua-packages__repo-list">
                  {filtered.map((entry) => (
                    <li key={entry.packageId} className="vua-packages__repo-browse-row">
                      <div className="vua-packages__repo-main">
                        <span className="vua-packages__name-title" title={entry.packageId}>
                          {entry.displayName ?? entry.packageId}
                        </span>
                        <span className="vua-caption vua-text-secondary">{entry.packageId}</span>
                      </div>
                      {entry.description !== null ? (
                        <span className="vua-caption vua-text-secondary">{entry.description}</span>
                      ) : null}
                      <div className="vua-packages__repo-browse-facts">
                        <span className="vua-caption">
                          {entry.latestVersion !== null
                            ? `${copy.columns.latest}: ${entry.latestVersion}`
                            : copy.p2.browseLatestNone}
                        </span>
                        <span className="vua-caption vua-text-secondary">
                          {format(copy.p2.browseVersionCount, { count: entry.versionCount })}
                        </span>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}

/* ---- 页面 ---- */

export function PackagesPage() {
  const gateway = useGateway();
  const dataSource = useDataSource();
  const view = usePackagesView();

  const [capability, setCapability] = useState<CapabilityReport | null>(null);
  const [section, setSection] = useState<PackagesSection>("packages");
  // 工具栏:搜索原始输入立即受控,防抖后再参与筛选
  const [searchText, setSearchText] = useState("");
  const [debouncedText, setDebouncedText] = useState("");
  const [sourceFilter, setSourceFilter] = useState<PackageSource | "">("");
  const [showPrereleases, setShowPrereleases] = useState(false);
  // 预发布警告:每次会话开启前弹一次确认(用户裁决的一次性确认,不持久化)
  const [prereleaseAcked, setPrereleaseAcked] = useState(false);
  const [prereleasePrompt, setPrereleasePrompt] = useState(false);
  // 选择 / 详情抽屉 / 项目切换 / P2 目录面板(按需查询目标包)
  const [selectedIds, setSelectedIds] = useState<readonly string[]>([]);
  const [anchorId, setAnchorId] = useState<string | null>(null);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [catalogTarget, setCatalogTarget] = useState<string | null>(null);
  const [pendingProjectId, setPendingProjectId] = useState<string | null>(null);
  // 批量多选安装选择(live 链,C 面自决 026 A2 消费面;与 demo 泛型链的
  // selectedIds 分立互不污染):anchor 供 Shift 范围选
  const [installSelectedIds, setInstallSelectedIds] = useState<readonly string[]>([]);
  const [installAnchorId, setInstallAnchorId] = useState<string | null>(null);
  // 变更两阶段
  const [preview, setPreview] = useState<PackageChangePreview | null>(null);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [applying, setApplying] = useState(false);
  // A1 移除确认链(026 消费批;live 词面与 demo 泛型链分立互不污染):
  // confirm(预览确认)→ applying(任务化执行,任务中心呈现真实状态)→
  // receipt(审计收据)/rejected(守卫拒绝)终态内联呈现
  const [removeFlow, setRemoveFlow] = useState<{
    phase: "confirm" | "applying" | "receipt" | "rejected";
    plan: PackagesRemovePlanV01;
    requestedPackageIds: readonly string[];
    receipt: PackagesRemoveReceiptV01 | null;
    rejection: PackagesRemoveRejectedV01 | null;
  } | null>(null);
  const [removePreviewBusy, setRemovePreviewBusy] = useState(false);
  // A2 安装/升级确认链(026 v0.2 消费批;与 A1 移除链分立——收据键集互
  // 斥,呈现互不污染):confirm(预览确认)→ applying(任务化执行,任务
  // 中心呈现真实状态)→ receipt(审计收据)/rejected(守卫拒绝)终态内
  // 联呈现
  const [installFlow, setInstallFlow] = useState<{
    phase: "confirm" | "applying" | "receipt" | "rejected";
    plan: PackagesInstallPlanV02;
    requestedPackages: readonly { packageId: string; version: string | null }[];
    receipt: PackagesInstallReceiptV02 | null;
    rejection: PackagesOpsRejectedV02 | null;
  } | null>(null);
  const [installPreviewBusy, setInstallPreviewBusy] = useState(false);
  // A3 本地包注册(026 v0.3 消费批;与 A1/A2 确认链分立):无 preview 无
  // 确认链——用户显式提交即确认;ok(registered 回显)/rejected 行内呈现,
  // failed/unavailable toast 后复位
  const [registerRoot, setRegisterRoot] = useState("");
  const [registerBusy, setRegisterBusy] = useState(false);
  const [registerOutcome, setRegisterOutcome] = useState<RegisterOutcomeView | null>(null);
  // A4 仓库订阅增删(026 v0.4 消费批;与 A1/A2/A3 各链分立):无 preview
  // 无确认链——用户显式提交即确认;添加双键表单(remote url+name/local
  // path+name),移除 = 订阅行内两击确认;ok(收据回显)/rejected 行内呈
  // 现,failed/unavailable toast 后复位。添加面不宣称幂等——文案不写
  // 「重复安全」,拒绝如实呈现
  const [repoRemoteUrl, setRepoRemoteUrl] = useState("");
  const [repoRemoteName, setRepoRemoteName] = useState("");
  const [repoLocalPath, setRepoLocalPath] = useState("");
  const [repoLocalName, setRepoLocalName] = useState("");
  const [repoBusy, setRepoBusy] = useState<"remote" | "local" | "remove" | null>(null);
  const [repoRemoteOutcome, setRepoRemoteOutcome] = useState<RepoAddOutcomeView | null>(null);
  const [repoLocalOutcome, setRepoLocalOutcome] = useState<RepoAddOutcomeView | null>(null);
  const [repoRemoveConfirmId, setRepoRemoveConfirmId] = useState<string | null>(null);
  const [repoRemoveOutcome, setRepoRemoveOutcome] = useState<RepoRemoveOutcomeView | null>(null);
  // F4 仓库生命周期(027 v0.6 消费批;与 A1–A5 各链分立):启停/刷新三
  // 方法任务化写命令,行内单操作 busy;ok(refreshed cacheUpdated=false =
  // 「已是最新」信息呈现非错误)/rejected 行内呈现,failed/unavailable
  // toast 后复位。重复启停不宣称幂等——拒绝如实呈现
  const [lifecycleBusy, setLifecycleBusy] = useState<"enable" | "disable" | "refresh" | null>(null);
  const [lifecycleBusyRepoId, setLifecycleBusyRepoId] = useState<string | null>(null);
  const [lifecycleOutcome, setLifecycleOutcome] = useState<RepoLifecycleOutcomeView | null>(null);
  // A5 项目创建(026 v0.5 消费批;与 A1–A4 各链分立):无 preview 无确认
  // 链——用户显式表单提交即确认;三键表单(parent/name 必填,template 选
  // 填留空 = null = 后端默认模板解析);ok(created 收据回显)/rejected
  // 行内呈现,failed/unavailable toast 后复位。创建不幂等——重复目录拒
  // 绝如实呈现,不发明幂等成功
  const [createParent, setCreateParent] = useState("");
  const [createName, setCreateName] = useState("");
  const [createTemplate, setCreateTemplate] = useState("");
  const [createBusy, setCreateBusy] = useState(false);
  const [createOutcome, setCreateOutcome] = useState<CreateOutcomeView | null>(null);

  useEffect(() => {
    let active = true;
    gateway.packages.capability().then(
      (report) => {
        if (active) setCapability(report);
      },
      () => {
        // capability 查询本身失败:按未接入诚实呈现,不猜测原因
        if (active) setCapability({ state: "unavailable", detailKey: "packagesEngineMissing" });
      },
    );
    return () => {
      active = false;
    };
  }, [gateway]);

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedText(searchText), SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [searchText]);


  const ready = view.kind === "ready" ? view : null;
  const p1 = view.kind === "ready-p1" ? view : null;
  const p2 = view.kind === "ready-p2" ? view : null;
  const selectedProjectId = ready?.selectedProjectId ?? null;

  // P1/P2 注册项目清单(013 聚合,packages.listInstalled 的同一注册事实):
  // 进入中间诚实态时加载;清单不可用 = null(诚实注记),形状不符行计数上呈
  const [registeredProjects, setRegisteredProjects] = useState<readonly RegisteredProjectRow[] | null>(null);
  const [p1Unreadable, setP1Unreadable] = useState<number | null>(null);
  const p1AutoSelected = useRef(false);

  useEffect(() => {
    if (view.kind !== "ready-p1" && view.kind !== "ready-p2") return;
    let active = true;
    void gateway.projectOps.listProjects().then((outcome) => {
      if (!active) return;
      if (outcome.ok) {
        setRegisteredProjects(outcome.projects);
        setP1Unreadable(outcome.unreadable);
      } else {
        setRegisteredProjects(null);
        setP1Unreadable(null);
      }
    });
    return () => {
      active = false;
    };
  }, [gateway, view.kind]);

  // P1/P2 自动初始选择:清单首个可用项目(一次;此后尊重用户显式选择)
  useEffect(() => {
    if ((view.kind !== "ready-p1" && view.kind !== "ready-p2") || registeredProjects === null || p1AutoSelected.current) return;
    const firstUsable = registeredProjects.find((project) => project.pathPresent);
    if (firstUsable !== undefined) {
      p1AutoSelected.current = true;
      void gateway.packages.selectProject(firstUsable.path);
    }
  }, [gateway, view.kind, registeredProjects]);

  // 项目切换收敛(render 期调整,React 推荐模式):清空选择/锚点/抽屉/
  // 目录面板;订阅广播到达后解除切换中骨架。ready 身份 = projectId,
  // P1/P2 身份 = 路径
  const selectedIdentity = ready?.selectedProjectId ?? p1?.projectPath ?? p2?.projectPath ?? null;
  const [lastProjectId, setLastProjectId] = useState(selectedIdentity);
  if (selectedIdentity !== lastProjectId) {
    setLastProjectId(selectedIdentity);
    setSelectedIds([]);
    setAnchorId(null);
    setDetailId(null);
    setCatalogTarget(null);
    setRemoveFlow(null);
    setRemovePreviewBusy(false);
    setInstallFlow(null);
    setInstallPreviewBusy(false);
    setInstallSelectedIds([]);
    setInstallAnchorId(null);
  }
  if (pendingProjectId !== null && pendingProjectId === selectedIdentity) {
    setPendingProjectId(null);
  }

  const capable = capability?.state === "ready";
  const switching = pendingProjectId !== null && pendingProjectId !== selectedIdentity;

  const visibleRows = useMemo(
    () =>
      sortPackages(
        filterPackages(ready?.packages ?? [], {
          text: debouncedText,
          source: sourceFilter,
          showPrereleases,
        }),
      ),
    [ready?.packages, debouncedText, sourceFilter, showPrereleases],
  );

  // P1 过滤:仅按 packageId 本地搜索;行序保持服务端 packageId 升序
  // (冻结的确定性呈现事实,客户端不重排)
  const p1Rows = useMemo(() => {
    if (p1 === null) return [] as readonly (InstalledPackageRowV01 | InstalledPackageRowV02)[];
    const text = debouncedText.trim().toLowerCase();
    if (text === "") return p1.installedPackages;
    return p1.installedPackages.filter((row) => row.packageId.toLowerCase().includes(text));
  }, [p1, debouncedText]);

  // P2 过滤:同 P1 纪律(packageId 本地搜索,行序不重排)
  const p2Rows = useMemo(() => {
    if (p2 === null) return [] as readonly (InstalledPackageRowV01 | InstalledPackageRowV02)[];
    const text = debouncedText.trim().toLowerCase();
    if (text === "") return p2.installedPackages;
    return p2.installedPackages.filter((row) => row.packageId.toLowerCase().includes(text));
  }, [p2, debouncedText]);

  const showToast = (text: string) => { toast(text); };

  /** 变更两阶段第一步:预览;空预览/未接入均以 toast 如实反馈,不弹空对话框 */
  const startChanges = (requests: readonly ChangeRequest[]) => {
    if (requests.length === 0 || previewBusy) return;
    setPreviewBusy(true);
    void gateway.packages.previewChanges(requests).then(
      (result) => {
        setPreviewBusy(false);
        if (result.kind === "unavailable") {
          showToast(copy.toasts.previewUnavailable);
          return;
        }
        if (isEmptyPreview(result.preview)) {
          showToast(copy.toasts.nothingToChange);
          return;
        }
        setPreview(result.preview);
      },
      () => {
        setPreviewBusy(false);
        showToast(copy.toasts.previewUnavailable);
      },
    );
  };

  /** 第二步:应用已确认的预览;结果 toast,成功后清空选择 */
  const confirmPreview = () => {
    if (preview === null || applying) return;
    const count = preview.items.length;
    setApplying(true);
    void gateway.packages.applyChanges(preview.id).then(
      (result) => {
        setApplying(false);
        setPreview(null);
        if (result.kind === "applied") {
          setSelectedIds([]);
          setAnchorId(null);
          showToast(format(copy.toasts.appliedSummary, { count }));
        } else {
          showToast(copy.toasts.applyFailed);
        }
      },
      () => {
        setApplying(false);
        setPreview(null);
        showToast(copy.toasts.applyFailed);
      },
    );
  };

  /** 添加入口(项目/本地包):added → toast;cancelled → 不打扰;unavailable → 诚实说明 */
  const runEntry = (action: () => Promise<PackageEntryResult>, addedText: string) => {
    void action().then(
      (result) => {
        if (result.kind === "added") showToast(addedText);
        else if (result.kind === "unavailable")
          showToast(strings.capability.details.packagesEngineMissing);
      },
      () => showToast(strings.capability.details.packagesEngineMissing),
    );
  };

  /** A1 确认链第一步(026 消费批):previewRemove 同步预览;typed 失败
   * 照词面文案(词外码原词插值),unavailable 诚实说明,绝不弹空对话框 */
  const startRemove = (packageId: string) => {
    const projectPath = p1?.projectPath ?? p2?.projectPath ?? null;
    if (projectPath === null || removePreviewBusy || removeFlow !== null) return;
    setRemovePreviewBusy(true);
    void gateway.packages.previewRemove(projectPath, [packageId]).then(
      (result) => {
        setRemovePreviewBusy(false);
        if (result.kind === "unavailable") {
          showToast(copy.remove.toasts.previewUnavailable);
          return;
        }
        if (result.kind === "failed") {
          const key = removeEnvelopeErrorKey(result.code);
          showToast(
            key === "unknown"
              ? format(copy.remove.toasts.previewFailedUnknown, { code: result.code })
              : copy.remove.envelopeErrors[key],
          );
          return;
        }
        if (result.plan.items.length === 0) {
          showToast(copy.remove.toasts.nothingToRemove);
          return;
        }
        setRemoveFlow({
          phase: "confirm",
          plan: result.plan,
          requestedPackageIds: [packageId],
          receipt: null,
          rejection: null,
        });
      },
      () => {
        setRemovePreviewBusy(false);
        showToast(copy.remove.toasts.previewUnavailable);
      },
    );
  };

  /** A1 确认链第二步:applyRemove 任务化执行(携 plan.digest 为
   * confirmedDigest);receipt/rejected 终态对话框内呈现,failed/unavailable
   * 关闭流以 toast 诚实说明——任务真实状态由任务中心呈现 */
  const confirmRemove = () => {
    if (removeFlow === null || removeFlow.phase !== "confirm") return;
    const { plan, requestedPackageIds } = removeFlow;
    setRemoveFlow({ ...removeFlow, phase: "applying" });
    void gateway.packages.applyRemove(plan.projectPath, requestedPackageIds, plan.digest).then(
      (result) => {
        if (result.kind === "ok") {
          setRemoveFlow((flow) =>
            flow === null ? flow : { ...flow, phase: "receipt", receipt: result.receipt },
          );
        } else if (result.kind === "rejected") {
          setRemoveFlow((flow) =>
            flow === null ? flow : { ...flow, phase: "rejected", rejection: result.rejection },
          );
        } else {
          setRemoveFlow(null);
          if (result.kind === "failed") {
            const key = removeEnvelopeErrorKey(result.code);
            showToast(
              key === "unknown"
                ? format(copy.remove.toasts.applyFailedUnknown, { code: result.code })
                : copy.remove.envelopeErrors[key],
            );
          } else {
            showToast(copy.remove.toasts.applyUnavailable);
          }
        }
      },
      () => {
        setRemoveFlow(null);
        showToast(copy.remove.toasts.applyUnavailable);
      },
    );
  };

  /** A2 确认链第一步(026 v0.2 消费批):previewInstall 同步预览(依赖解
   * 析可达仓库);请求行 = {packageId, version}:version null = 解析器选
   * 最新稳定版(「安装/升级到最新」,单包与批量多选入口同语义),string =
   * 钉死精确版本(版本行内入口);typed 失败照词面文案(词外码原词插值,
   * A2 新码 preview_failed 有专属文案),unavailable 诚实说明,绝不弹空
   * 对话框;空请求行集 = UI 层拒发(词面 minItems 1,不构造违例请求) */
  const startInstallRows = (
    requests: readonly { packageId: string; version: string | null }[],
  ) => {
    const projectPath = p2?.projectPath ?? null;
    if (projectPath === null || installPreviewBusy || installFlow !== null) return;
    if (requests.length === 0) {
      showToast(copy.install.toasts.nothingToInstall);
      return;
    }
    setInstallPreviewBusy(true);
    void gateway.packages.previewInstall(projectPath, requests).then(
      (result) => {
        setInstallPreviewBusy(false);
        if (result.kind === "unavailable") {
          showToast(copy.install.toasts.previewUnavailable);
          return;
        }
        if (result.kind === "failed") {
          const key = installEnvelopeErrorKey(result.code);
          showToast(
            key === "unknown"
              ? format(copy.install.toasts.previewFailedUnknown, { code: result.code })
              : copy.install.envelopeErrors[key],
          );
          return;
        }
        if (result.plan.items.length === 0) {
          showToast(copy.install.toasts.nothingToInstall);
          return;
        }
        setInstallFlow({
          phase: "confirm",
          plan: result.plan,
          requestedPackages: requests,
          receipt: null,
          rejection: null,
        });
      },
      () => {
        setInstallPreviewBusy(false);
        showToast(copy.install.toasts.previewUnavailable);
      },
    );
  };

  /** 单包安装入口(目录面板「安装最新」/版本行「安装此版本」) */
  const startInstall = (packageId: string, version: string | null) => {
    startInstallRows([{ packageId, version }]);
  };

  /** A2 确认链第二步:applyInstall 任务化执行(携 plan.digest 为
   * confirmedDigest);receipt/rejected 终态对话框内呈现,failed/unavailable
   * 关闭流以 toast 诚实说明——任务真实状态由任务中心呈现 */
  const confirmInstall = () => {
    if (installFlow === null || installFlow.phase !== "confirm") return;
    const { plan, requestedPackages } = installFlow;
    setInstallFlow({ ...installFlow, phase: "applying" });
    void gateway.packages
      .applyInstall(plan.projectPath, requestedPackages, plan.digest)
      .then(
        (result) => {
          if (result.kind === "ok") {
            setInstallFlow((flow) =>
              flow === null ? flow : { ...flow, phase: "receipt", receipt: result.receipt },
            );
          } else if (result.kind === "rejected") {
            setInstallFlow((flow) =>
              flow === null ? flow : { ...flow, phase: "rejected", rejection: result.rejection },
            );
          } else {
            setInstallFlow(null);
            if (result.kind === "failed") {
              const key = installEnvelopeErrorKey(result.code);
              showToast(
                key === "unknown"
                  ? format(copy.install.toasts.applyFailedUnknown, { code: result.code })
                  : copy.install.envelopeErrors[key],
              );
            } else {
              showToast(copy.install.toasts.applyUnavailable);
            }
          }
        },
        () => {
          setInstallFlow(null);
          showToast(copy.install.toasts.applyUnavailable);
        },
      );
  };

  const chooseProject = (projectId: string) => {
    // P1/P2 视图的项目身份 = 013 注册路径(projectPath);各模式同一选择通道
    const currentSelected = ready?.selectedProjectId ?? p1?.projectPath ?? p2?.projectPath ?? null;
    if (ready === null && p1 === null && p2 === null) return;
    if (projectId === currentSelected) return;
    setPendingProjectId(projectId);
    void gateway.packages.selectProject(projectId).then(
      () => setPendingProjectId(null),
      () => setPendingProjectId(null),
    );
  };

  const toggleRow = (packageId: string, shiftKey: boolean) => {
    if (shiftKey && anchorId !== null) {
      setSelectedIds(rangeSelect(visibleRows.map((row) => row.id), anchorId, packageId));
      return;
    }
    setAnchorId(packageId);
    setSelectedIds((prev) =>
      prev.includes(packageId)
        ? prev.filter((id) => id !== packageId)
        : [...prev, packageId],
    );
  };

  const toggleAll = () => {
    const allVisibleSelected =
      visibleRows.length > 0 && visibleRows.every((row) => selectedIds.includes(row.id));
    setSelectedIds(allVisibleSelected ? [] : visibleRows.map((row) => row.id));
    setAnchorId(null);
  };

  /* ---- 批量多选安装(live 链,C 面自决 026 A2 消费面;与 demo 泛型链的
   * toggleRow/toggleAll 分立互不污染)。行序 = 当前视图过滤后行序(服务
   * 端 packageId 升序,客户端不重排)。 ---- */

  const toggleInstallRow = (packageId: string, shiftKey: boolean) => {
    const orderedRows = p1 !== null ? p1Rows : p2Rows;
    if (shiftKey && installAnchorId !== null) {
      setInstallSelectedIds(
        rangeSelect(
          orderedRows.map((row) => row.packageId),
          installAnchorId,
          packageId,
        ),
      );
      return;
    }
    setInstallAnchorId(packageId);
    setInstallSelectedIds((prev) =>
      prev.includes(packageId)
        ? prev.filter((id) => id !== packageId)
        : [...prev, packageId],
    );
  };

  const toggleInstallAll = () => {
    const orderedRows = p1 !== null ? p1Rows : p2Rows;
    const allSelected =
      orderedRows.length > 0 &&
      orderedRows.every((row) => installSelectedIds.includes(row.packageId));
    setInstallSelectedIds(allSelected ? [] : orderedRows.map((row) => row.packageId));
    setInstallAnchorId(null);
  };

  const clearInstallSelection = () => {
    setInstallSelectedIds([]);
    setInstallAnchorId(null);
  };

  /** 批量入口:选中行全部「安装/升级到最新」(version null = 解析器选
   * 最新稳定版,与单包入口同语义;可装性/可升性不预判,权威判定在服务
   * 端 preview——空变更以 toast 如实反馈)。预览受理(进入确认链)即
   * 清空批量选择:确认链期间批量条退场;拒绝后的重试 = 重新勾选重预览
   * (digest 确认链机制本就要求重预览,绝不静默沿用旧清单,诚实纪律 3) */
  const runInstallLatest = (packageIds: readonly string[]) => {
    if (packageIds.length === 0) return;
    startInstallRows(installLatestRequests(packageIds));
    clearInstallSelection();
  };

  /* ---- A3 本地包注册(026 v0.3 消费批):无 preview 无确认链,用户显
   * 式提交即确认(词面无 digest 位,UI 不构造携 digest 请求)。空输入 =
   * 按钮禁用,提交函数再守卫一次(词面 minLength 1,UI 绝不构造违例请
   * 求)。ok/rejected 行内呈现;failed/unavailable 关闭为 toast 诚实说
   * 明——任务真实状态由任务中心呈现。幂等语义如实呈现:重复注册同一
   * 包根 = 同一个成功事实(收据形状相同)。 ---- */

  const startRegister = () => {
    if (registerBusy) return;
    const packageRoot = registerRoot.trim();
    if (packageRoot.length === 0) return;
    setRegisterBusy(true);
    setRegisterOutcome(null);
    void gateway.packages.registerLocalPackage(packageRoot).then(
      (result) => {
        setRegisterBusy(false);
        if (result.kind === "ok") {
          setRegisterOutcome({ kind: "ok", packageRoot: result.receipt.packageRoot });
          return;
        }
        if (result.kind === "rejected") {
          setRegisterOutcome({
            kind: "rejected",
            guard: result.rejection.guard,
            detail: result.rejection.detail,
          });
          return;
        }
        setRegisterOutcome(null);
        if (result.kind === "failed") {
          const key = registerEnvelopeErrorKey(result.code);
          showToast(
            key === "unknown"
              ? format(copy.register.toasts.failedUnknown, { code: result.code })
              : copy.register.envelopeErrors[key],
          );
        } else {
          showToast(copy.register.toasts.unavailable);
        }
      },
      () => {
        setRegisterBusy(false);
        setRegisterOutcome(null);
        showToast(copy.register.toasts.unavailable);
      },
    );
  };

  /* ---- A4 仓库订阅增删(026 v0.4 消费批):三方法各自任务化——受理→
   * 终态等待→收据/拒绝回流(端口内封装,骑共享 waitForTerminalTask)。
   * ok/rejected 行内呈现;failed/unavailable 关闭为 toast 诚实说明——
   * 任务真实状态由任务中心呈现。添加面不宣称幂等:重复订阅被拒如实
   * 呈现,不发明幂等成功。移除 = 行内两击确认(第一击进入确认态,再击
   * 执行);repoId 为 null 的行不提供移除入口(词面:id 缺席行不在移除
   * 可达范围)。ok 后订阅列表由端口广播刷新(列表按新事实重取)。 ---- */

  const startRepoAddRemote = () => {
    if (repoBusy !== null) return;
    const url = repoRemoteUrl.trim();
    const name = repoRemoteName.trim();
    if (url.length === 0 || name.length === 0) return;
    setRepoBusy("remote");
    setRepoRemoteOutcome(null);
    void gateway.packages.addRemoteRepo(url, name).then(
      (result) => {
        setRepoBusy(null);
        if (result.kind === "ok") {
          const receipt = result.receipt;
          if (receipt.kind === "repoReceipt" && receipt.repoType === "remote") {
            setRepoRemoteOutcome({
              kind: "ok",
              line: format(copy.repoWrite.remoteSuccessLine, { name: receipt.name, url: receipt.url }),
            });
          }
          return;
        }
        if (result.kind === "rejected") {
          setRepoRemoteOutcome({
            kind: "rejected",
            guard: result.rejection.guard,
            detail: result.rejection.detail,
          });
          return;
        }
        setRepoRemoteOutcome(null);
        if (result.kind === "failed") {
          const key = repoEnvelopeErrorKey(result.code);
          showToast(
            key === "unknown"
              ? format(copy.repoWrite.toasts.failedUnknown, { code: result.code })
              : copy.repoWrite.envelopeErrors[key],
          );
        } else {
          showToast(copy.repoWrite.toasts.unavailable);
        }
      },
      () => {
        setRepoBusy(null);
        setRepoRemoteOutcome(null);
        showToast(copy.repoWrite.toasts.unavailable);
      },
    );
  };

  const startRepoAddLocal = () => {
    if (repoBusy !== null) return;
    const path = repoLocalPath.trim();
    const name = repoLocalName.trim();
    if (path.length === 0 || name.length === 0) return;
    setRepoBusy("local");
    setRepoLocalOutcome(null);
    void gateway.packages.addLocalRepo(path, name).then(
      (result) => {
        setRepoBusy(null);
        if (result.kind === "ok") {
          const receipt = result.receipt;
          if (receipt.kind === "repoReceipt" && receipt.repoType === "local") {
            setRepoLocalOutcome({
              kind: "ok",
              line: format(copy.repoWrite.localSuccessLine, { name: receipt.name, path: receipt.path }),
            });
          }
          return;
        }
        if (result.kind === "rejected") {
          setRepoLocalOutcome({
            kind: "rejected",
            guard: result.rejection.guard,
            detail: result.rejection.detail,
          });
          return;
        }
        setRepoLocalOutcome(null);
        if (result.kind === "failed") {
          const key = repoEnvelopeErrorKey(result.code);
          showToast(
            key === "unknown"
              ? format(copy.repoWrite.toasts.failedUnknown, { code: result.code })
              : copy.repoWrite.envelopeErrors[key],
          );
        } else {
          showToast(copy.repoWrite.toasts.unavailable);
        }
      },
      () => {
        setRepoBusy(null);
        setRepoLocalOutcome(null);
        showToast(copy.repoWrite.toasts.unavailable);
      },
    );
  };

  /** 移除两击确认:第一击进入确认态;同 repoId 再击执行;其他操作繁忙
   * 时忽略。执行期间保留确认 repoId(行内按钮呈「正在移除…」并禁用),
   * 完成或失败后复位。 */
  const requestRemoveRepo = (repoId: string) => {
    if (repoBusy !== null) return;
    if (repoRemoveConfirmId !== repoId) {
      setRepoRemoveConfirmId(repoId);
      return;
    }
    setRepoBusy("remove");
    setRepoRemoveOutcome(null);
    void gateway.packages.removeRepo(repoId).then(
      (result) => {
        setRepoBusy(null);
        setRepoRemoveConfirmId(null);
        if (result.kind === "ok") {
          setRepoRemoveOutcome({ kind: "ok", repoId: result.receipt.repoId });
          return;
        }
        if (result.kind === "rejected") {
          setRepoRemoveOutcome({
            kind: "rejected",
            guard: result.rejection.guard,
            detail: result.rejection.detail,
          });
          return;
        }
        if (result.kind === "failed") {
          const key = repoEnvelopeErrorKey(result.code);
          showToast(
            key === "unknown"
              ? format(copy.repoWrite.toasts.failedUnknown, { code: result.code })
              : copy.repoWrite.envelopeErrors[key],
          );
        } else {
          showToast(copy.repoWrite.toasts.unavailable);
        }
      },
      () => {
        setRepoBusy(null);
        setRepoRemoveConfirmId(null);
        showToast(copy.repoWrite.toasts.unavailable);
      },
    );
  };

  /* ---- F4 仓库生命周期(027 v0.6 消费批):启停/刷新三方法任务化写命
   * 令——params 单键 {repoId} verbatim。Refresh keeps a disabled layout slot
   * when the identifier or lifecycle capability is absent; no command is sent.
   * Toggle availability still follows the returned lifecycle capability.
   * (订阅行照常呈现,降级非错误);v0.1 族行无 enabled 位 = 启停控制不
   * 渲染(状态不可知不猜测),刷新控制不依赖 enabled 位。禁用语义如实
   * 呈现:禁用行离开包集合世界但在列不隐藏(W25 裁决 (c):VUA 自有状态,
   * 绝不写共享 settings.json,区块说明词面照此口径)。ok(refreshed
   * cacheUpdated=false = 「已是最新」信息呈现非错误)/rejected 行内呈现;
   * failed/unavailable 关闭为 toast 诚实说明——任务真实状态由任务中心
   * 呈现。重复启停不宣称幂等,拒绝如实呈现。 ---- */

  const runRepoLifecycle = (action: "enable" | "disable" | "refresh", repoId: string) => {
    if (lifecycleBusy !== null) return;
    setLifecycleBusy(action);
    setLifecycleBusyRepoId(repoId);
    setLifecycleOutcome(null);
    const call =
      action === "enable"
        ? gateway.packages.enableRepo(repoId)
        : action === "disable"
          ? gateway.packages.disableRepo(repoId)
          : gateway.packages.refreshRepo(repoId);
    void call.then(
      (result) => {
        setLifecycleBusy(null);
        setLifecycleBusyRepoId(null);
        if (result.kind === "ok") {
          setLifecycleOutcome({
            kind: "ok",
            repoId: result.receipt.repoId,
            cacheUpdated: result.receipt.kind === "refreshed" ? result.receipt.cacheUpdated : null,
          });
          return;
        }
        if (result.kind === "rejected") {
          setLifecycleOutcome({
            kind: "rejected",
            guard: result.rejection.guard,
            detail: result.rejection.detail,
          });
          return;
        }
        if (result.kind === "failed") {
          const key = repoEnvelopeErrorKey(result.code);
          showToast(
            key === "unknown"
              ? format(copy.repoWrite.toasts.failedUnknown, { code: result.code })
              : copy.repoWrite.envelopeErrors[key],
          );
        } else {
          showToast(copy.repoWrite.toasts.unavailable);
        }
      },
      () => {
        setLifecycleBusy(null);
        setLifecycleBusyRepoId(null);
        showToast(copy.repoWrite.toasts.unavailable);
      },
    );
  };

  /* ---- A5 项目创建(026 v0.5 消费批):无 preview 无确认链——用户显式
   * 表单提交即确认(全新目录无既有状态可 diff 无摘要可绑定,UI 不构造携
   * digest/projectPath 请求)。template 输入留空 = null(后端默认模板解
   * 析),非空 = verbatim(027 F5 消费批:非空值来源 = 模板下拉选中的
   * id〔冻结机器标识,原样传递〕或枚举不可用时的手填,下拉显示行逐字用
   * name 绝不虚构标签);空串 = 词面形状违反,UI 只构造 null 绝不构造空
   * 串。创建不幂等:重复目录拒绝如实行内呈现,不发明幂等成功。ok(
   * created 收据 = ProjectRef 投影,projectPath = 注册路径身份;创建即在
   * 册,成功后在册列表由端口广播刷新)/rejected 行内呈现;failed/
   * unavailable 关闭为 toast 诚实说明——任务真实状态由任务中心呈现。 ---- */

  const startCreate = () => {
    if (createBusy) return;
    const parent = createParent.trim();
    const name = createName.trim();
    const template = createTemplate.trim();
    if (parent.length === 0 || name.length === 0) return;
    setCreateBusy(true);
    setCreateOutcome(null);
    void gateway.packages.createProject(parent, name, template.length === 0 ? null : template).then(
      (result) => {
        setCreateBusy(false);
        if (result.kind === "ok") {
          setCreateOutcome({ kind: "ok", projectPath: result.receipt.projectPath });
          return;
        }
        if (result.kind === "rejected") {
          setCreateOutcome({
            kind: "rejected",
            guard: result.rejection.guard,
            detail: result.rejection.detail,
          });
          return;
        }
        setCreateOutcome(null);
        if (result.kind === "failed") {
          const key = createEnvelopeErrorKey(result.code);
          showToast(
            key === "unknown"
              ? format(copy.create.toasts.failedUnknown, { code: result.code })
              : copy.create.envelopeErrors[key],
          );
        } else {
          showToast(copy.create.toasts.unavailable);
        }
      },
      () => {
        setCreateBusy(false);
        setCreateOutcome(null);
        showToast(copy.create.toasts.unavailable);
      },
    );
  };

  const togglePrereleases = (checked: boolean) => {
    if (checked && !prereleaseAcked) {
      setPrereleasePrompt(true);
      return;
    }
    setShowPrereleases(checked);
  };
  const detailRow =
    detailId === null
      ? null
      : (ready?.packages.find((row) => row.id === detailId) ?? null);

  const resolveName = (packageId: string) =>
    ready?.packages.find((row) => row.id === packageId)?.displayName ?? packageId;

  const engineDown = capability !== null && capability.state !== "ready";

  return (
    <div className="vua-page vua-packages">
      <section className="vua-page__hero">
        <h1 className="vua-title">
          {strings.nav.pages.packages}
          {dataSource === "fixture" ? (
            <>
              {" "}
              <Badge tone="warning">{strings.common.fixtureBadge}</Badge>
            </>
          ) : null}
        </h1>
        <p className="vua-text-secondary">{format(copy.subtitle, { recipe: termLabel("recipe") })}</p>
      </section>

      {capability === null ? (
        /* capability 查询中(极短):骨架占位 */
        <Card>
          <div className="vua-page__stack">
            <Skeleton width="40%" />
            <Skeleton width="80%" />
            <Skeleton width="60%" />
          </div>
        </Card>
      ) : engineDown ? (
        /* 能力未接入/异常:整页诚实空态,不渲染任何包数据入口 */
        <EmptyState
          title={copy.empty.engineTitle}
          description={
            capability.detailKey
              ? strings.capability.details[capability.detailKey]
              : strings.capability.states[capability.state]
          }
        />
      ) : p2 !== null ? (
        /* P2 读面诚实态(025)+ A1 写面(026):已装可看 + 订阅清单/目录/
         * 移除写入口按能力行解锁;repos/catalog/changes 区块 false 时不
         * 渲染对应区块与入口(无事实源不渲染,渲染层不伪造) */
        p2.blocks.installed ? (
          <>
            <P2Notice changesOpen={p2.blocks.changes || p2.blocks.installs} />
            <P1ProjectPicker
              projects={registeredProjects ?? []}
              unreadable={p1Unreadable}
              selectedProjectPath={p2.projectPath}
              onSelect={chooseProject}
            />
            {(p2.blocks.repos || p2.blocks.repoWrites || p2.blocks.repoCatalog) ? (
              /* F2 消费批(027)IA 表态 2 落地:仓库订阅与订阅管理两分区
               * 合并为单一「仓库」分区(纯呈现层重组——blocks 键与能力行
               * 照旧,入口随 repos＋repoWrites＋repoCatalog 能力事实行共
               * 同门控,无事实不渲染纪律不变);订阅列表行内展开即该仓浏
               * 览面(blocks.repoCatalog 门控) */
              <section className="vua-packages__repo-partition" aria-label={copy.p2.partitionAria}>
                <h2 className="vua-packages__partition-title">{copy.p2.partitionTitle}</h2>
                {p2.blocks.repos ? (
                  <P2ReposSection
                    repos={p2.repos}
                    reposErrorCode={p2.reposError?.code ?? null}
                    {...(p2.blocks.repoCatalog ? { browseEnabled: true, gateway } : {})}
                    {...(p2.blocks.repoWrites
                      ? {
                          onRemoveRequest: requestRemoveRepo,
                          removeConfirmId: repoRemoveConfirmId,
                          removeBusyId: repoBusy === "remove" ? repoRemoveConfirmId : null,
                          removeOutcome: repoRemoveOutcome,
                        }
                      : {})}
                    {...(p2.blocks.repoLifecycle
                      ? {
                          lifecycleEnabled: true,
                          onLifecycle: runRepoLifecycle,
                          lifecycleBusyAction: lifecycleBusy,
                          lifecycleBusyRepoId: lifecycleBusyRepoId,
                          lifecycleOutcome: lifecycleOutcome,
                        }
                      : {})}
                  />
                ) : null}
                {p2.blocks.repoWrites ? (
                  <RepoWriteSection
                    busy={repoBusy === "remote" ? "remote" : repoBusy === "local" ? "local" : null}
                    remoteUrl={repoRemoteUrl}
                    remoteName={repoRemoteName}
                    localPath={repoLocalPath}
                    localName={repoLocalName}
                    remoteOutcome={repoRemoteOutcome}
                    localOutcome={repoLocalOutcome}
                    onRemoteUrlChange={setRepoRemoteUrl}
                    onRemoteNameChange={setRepoRemoteName}
                    onLocalPathChange={setRepoLocalPath}
                    onLocalNameChange={setRepoLocalName}
                    onAddRemote={startRepoAddRemote}
                    onAddLocal={startRepoAddLocal}
                  />
                ) : null}
              </section>
            ) : null}
            {p2.blocks.registers ? (
              <RegisterSection
                outcome={registerOutcome}
                busy={registerBusy}
                path={registerRoot}
                onPathChange={setRegisterRoot}
                onSubmit={startRegister}
              />
            ) : null}
            {p2.blocks.creates ? (
              <CreateSection
                busy={createBusy}
                parent={createParent}
                name={createName}
                template={createTemplate}
                outcome={createOutcome}
                templatesBlock={p2.blocks.templates}
                gateway={gateway}
                onParentChange={setCreateParent}
                onNameChange={setCreateName}
                onTemplateChange={setCreateTemplate}
                onSubmit={startCreate}
              />
            ) : null}
            {registeredProjects !== null && registeredProjects.length === 0 ? (
              <EmptyState
                title={copy.empty.noProjectsTitle}
                description={copy.p1.noProjectsDescription}
              />
            ) : registeredProjects === null ? (
              <EmptyState
                title={copy.empty.noProjectsTitle}
                description={copy.p1.projectsUnavailable}
              />
            ) : p2.projectPath === null ? (
              <EmptyState
                title={copy.empty.noSelectionTitle}
                description={copy.empty.noSelectionDescription}
              />
            ) : (
              <div className="vua-packages__main">
                <div className="vua-packages__toolbar" role="search">
                  <input
                    type="search"
                    className="vua-packages__search"
                    placeholder={copy.toolbar.searchPlaceholder}
                    aria-label={copy.toolbar.searchAria}
                    value={searchText}
                    onChange={(event) => setSearchText(event.target.value)}
                  />
                </div>
                {switching ? (
                  <Card>
                    <div className="vua-page__stack">
                      <Skeleton width="55%" />
                      <Skeleton width="80%" />
                      <Skeleton width="45%" />
                    </div>
                  </Card>
                ) : (
                  <P1InstalledTable
                    rows={p2Rows}
                    loadErrorCode={p2.loadError?.code ?? null}
                    {...(p2.installedCacheSourced ? { cacheSourced: true } : {})}
                    {...(p2.blocks.installs
                      ? {
                          onUpdate: (packageId: string) =>
                            startInstallRows([{ packageId, version: null }]),
                          updateBusy: installPreviewBusy || installFlow !== null,
                        }
                      : {})}
                    {...(p2.blocks.catalog ? { onShowCatalog: setCatalogTarget } : {})}
                    {...(p2.blocks.changes
                      ? { onRemove: startRemove, removeBusy: removePreviewBusy || removeFlow !== null }
                      : {})}
                    {...(p2.blocks.installs
                      ? {
                          installBulk: {
                            selectedIds: installSelectedIds,
                            busy: installPreviewBusy || installFlow !== null,
                            onToggleRow: toggleInstallRow,
                            onToggleAll: toggleInstallAll,
                            onClear: clearInstallSelection,
                            onRun: runInstallLatest,
                          },
                        }
                      : {})}
                  />
                )}
                {catalogTarget !== null && p2.blocks.catalog && p2.projectPath !== null ? (
                  <P2CatalogPanel
                    projectPath={p2.projectPath}
                    packageId={catalogTarget}
                    gateway={gateway}
                    {...(p2.blocks.installs
                      ? {
                          onInstall: startInstall,
                          installBusy: installPreviewBusy || installFlow !== null,
                        }
                      : {})}
                    onClose={() => setCatalogTarget(null)}
                  />
                ) : null}
              </div>
            )}
          </>
        ) : (
          /* 能力行存在但不可用:已安装区块诚实不可渲染 */
          <EmptyState
            title={copy.empty.engineTitle}
            description={strings.capability.details.packagesEngineMissing}
          />
        )
      ) : p1 !== null ? (
        /* P1 中间诚实态(024)+ A1 写面(026):repos 无词表行恒 false,
         * 分区切换器不渲染;移除写入口随 removeOps 能力行解锁;清单来自
         * 013 注册面,包行三键照实显示 */
        <>
          {p1.blocks.installed ? (
            <>
              <P1Notice changesOpen={p1.blocks.changes} />
              <P1ProjectPicker
                projects={registeredProjects ?? []}
                unreadable={p1Unreadable}
                selectedProjectPath={p1.projectPath}
                onSelect={chooseProject}
              />
              {p1.blocks.repoWrites ? (
                <RepoWriteSection
                  busy={repoBusy === "remote" ? "remote" : repoBusy === "local" ? "local" : null}
                  remoteUrl={repoRemoteUrl}
                  remoteName={repoRemoteName}
                  localPath={repoLocalPath}
                  localName={repoLocalName}
                  remoteOutcome={repoRemoteOutcome}
                  localOutcome={repoLocalOutcome}
                  onRemoteUrlChange={setRepoRemoteUrl}
                  onRemoteNameChange={setRepoRemoteName}
                  onLocalPathChange={setRepoLocalPath}
                  onLocalNameChange={setRepoLocalName}
                  onAddRemote={startRepoAddRemote}
                  onAddLocal={startRepoAddLocal}
                />
              ) : null}
              {p1.blocks.registers ? (
                <RegisterSection
                  outcome={registerOutcome}
                  busy={registerBusy}
                  path={registerRoot}
                  onPathChange={setRegisterRoot}
                  onSubmit={startRegister}
                />
              ) : null}
              {p1.blocks.creates ? (
                <CreateSection
                  busy={createBusy}
                  parent={createParent}
                  name={createName}
                  template={createTemplate}
                  outcome={createOutcome}
                  templatesBlock={p1.blocks.templates}
                  gateway={gateway}
                  onParentChange={setCreateParent}
                  onNameChange={setCreateName}
                  onTemplateChange={setCreateTemplate}
                  onSubmit={startCreate}
                />
              ) : null}
              {registeredProjects !== null && registeredProjects.length === 0 ? (
                <EmptyState
                  title={copy.empty.noProjectsTitle}
                  description={copy.p1.noProjectsDescription}
                />
              ) : registeredProjects === null ? (
                <EmptyState
                  title={copy.empty.noProjectsTitle}
                  description={copy.p1.projectsUnavailable}
                />
              ) : p1.projectPath === null ? (
                <EmptyState
                  title={copy.empty.noSelectionTitle}
                  description={copy.empty.noSelectionDescription}
                />
              ) : (
                <div className="vua-packages__main">
                  <div className="vua-packages__toolbar" role="search">
                    <input
                      type="search"
                      className="vua-packages__search"
                      placeholder={copy.toolbar.searchPlaceholder}
                      aria-label={copy.toolbar.searchAria}
                      value={searchText}
                      onChange={(event) => setSearchText(event.target.value)}
                    />
                  </div>
                  {switching ? (
                    <Card>
                      <div className="vua-page__stack">
                        <Skeleton width="55%" />
                        <Skeleton width="80%" />
                        <Skeleton width="45%" />
                      </div>
                    </Card>
                  ) : (
                    <P1InstalledTable
                      rows={p1Rows}
                      loadErrorCode={p1.loadError?.code ?? null}
                      {...(p1.blocks.changes
                        ? { onRemove: startRemove, removeBusy: removePreviewBusy || removeFlow !== null }
                        : {})}
                      {...(p1.blocks.installs
                        ? {
                            installBulk: {
                              selectedIds: installSelectedIds,
                              busy: installPreviewBusy || installFlow !== null,
                              onToggleRow: toggleInstallRow,
                              onToggleAll: toggleInstallAll,
                              onClear: clearInstallSelection,
                              onRun: runInstallLatest,
                            },
                          }
                        : {})}
                    />
                  )}
                </div>
              )}
            </>
          ) : (
            /* 能力行存在但不可用:已安装区块诚实不可渲染 */
            <EmptyState
              title={copy.empty.engineTitle}
              description={strings.capability.details.packagesEngineMissing}
            />
          )}
        </>
      ) : ready === null ? (
        <EmptyState
          title={copy.empty.notConnectedTitle}
          description={copy.empty.notConnectedDescription}
        />
      ) : (
        <>
          <div
            className="vua-packages__sections"
            role="group"
            aria-label={copy.sections.switchAria}
          >
            <button
              type="button"
              className="vua-packages__section-tab"
              data-active={section === "packages" || undefined}
              aria-pressed={section === "packages"}
              onClick={() => setSection("packages")}
            >
              {copy.sections.packages}
            </button>
            <button
              type="button"
              className="vua-packages__section-tab"
              data-active={section === "repos" || undefined}
              aria-pressed={section === "repos"}
              onClick={() => setSection("repos")}
            >
              {copy.sections.repos}
            </button>
          </div>

          {section === "repos" ? (
            /* F4 消费批:旧演示视图启停 checkbox 退役——本地状态翻转绝不
             * 冒充 wire 写面;演示行启停位改为只读静态呈现,启停交互只在
             * live 装配的 P2 词面(blocks.repoLifecycle 门控)提供 */
            <RepoSection repos={ready.repos} />
          ) : (
            <>
              <ProjectHeader
                projects={ready.projects}
                selectedProjectId={selectedProjectId}
                capable={capable}
                onSelect={chooseProject}
                onAdd={() =>
                  runEntry(() => gateway.packages.addProject(), copy.toasts.projectAdded)
                }
              />
              {ready.migrationHint ? (
                <MigrationHintCard summaryKey={ready.migrationHint.summaryKey} />
              ) : null}

              {ready.projects.length === 0 ? (
                <EmptyState
                  title={copy.empty.noProjectsTitle}
                  description={copy.empty.noProjectsDescription}
                  action={
                    capable ? (
                      <Button
                        variant="primary"
                        onClick={() =>
                          runEntry(() => gateway.packages.addProject(), copy.toasts.projectAdded)
                        }
                      >
                        {copy.projects.addProject}
                      </Button>
                    ) : undefined
                  }
                />
              ) : selectedProjectId === null ? (
                <EmptyState
                  title={copy.empty.noSelectionTitle}
                  description={copy.empty.noSelectionDescription}
                />
              ) : (
                <div
                  className="vua-packages__content"
                  data-drawer-open={detailRow !== null || undefined}
                >
                  <div className="vua-packages__main">
                    <PackageToolbar
                      searchText={searchText}
                      source={sourceFilter}
                      showPrereleases={showPrereleases}
                      capable={capable}
                      onSearch={setSearchText}
                      onSource={setSourceFilter}
                      onTogglePrereleases={togglePrereleases}
                      onImport={() =>
                        runEntry(
                          () => gateway.packages.importLocalPackage(),
                          copy.toasts.importAdded,
                        )
                      }
                    />
                    {switching ? (
                      <PackageTable
                        rows={[]}
                        allRows={ready.packages}
                        switching
                        selectedIds={selectedIds}
                        showPrereleases={showPrereleases}
                        busy={previewBusy}
                        onToggleRow={toggleRow}
                        onToggleAll={toggleAll}
                        onClearSelection={() => {
                          setSelectedIds([]);
                          setAnchorId(null);
                        }}
                        onRequests={startChanges}
                        onShowDetail={(row) => setDetailId(row.id)}
                      />
                    ) : ready.packages.length === 0 ? (
                      <EmptyState
                        title={copy.empty.noPackagesTitle}
                        description={format(copy.empty.noPackagesDescription, {
                          recipe: termLabel("recipe"),
                        })}
                        action={
                          capable ? (
                            <Button
                              variant="default"
                              onClick={() =>
                                runEntry(
                                  () => gateway.packages.importLocalPackage(),
                                  copy.toasts.importAdded,
                                )
                              }
                            >
                              {copy.toolbar.importLocal}
                            </Button>
                          ) : undefined
                        }
                      />
                    ) : visibleRows.length === 0 ? (
                      /* 搜索/筛选无结果 ≠ 项目无包 */
                      <EmptyState
                        title={copy.empty.noResultTitle}
                        description={copy.empty.noResultDescription}
                      />
                    ) : (
                      <PackageTable
                        rows={visibleRows}
                        allRows={ready.packages}
                        switching={false}
                        selectedIds={selectedIds}
                        showPrereleases={showPrereleases}
                        busy={previewBusy}
                        onToggleRow={toggleRow}
                        onToggleAll={toggleAll}
                        onClearSelection={() => {
                          setSelectedIds([]);
                          setAnchorId(null);
                        }}
                        onRequests={startChanges}
                        onShowDetail={(row) => setDetailId(row.id)}
                      />
                    )}
                  </div>
                  {detailRow !== null ? (
                    <PackageDetailDrawer row={detailRow} onClose={() => setDetailId(null)} />
                  ) : null}
                </div>
              )}
            </>
          )}
        </>
      )}

      {/* 项目兼容分区(proposal 026 B,用户 2026-09-18 裁决):原「项目兼容」
       * 独立页并入本页尾部,恒渲染不挂 packages 引擎 capability 门控——
       * 分区消费面走 projectOps 独立通道(IA 并入＝选项卡级合并)。 */}
      <ProjectCompatSection />

      {preview !== null ? (
        <ChangesDialog
          preview={preview}
          applying={applying}
          resolveName={resolveName}
          onCancel={() => setPreview(null)}
          onConfirm={confirmPreview}
        />
      ) : null}

      {removeFlow !== null ? (
        <RemoveConfirmDialog
          plan={removeFlow.plan}
          requestedPackageIds={removeFlow.requestedPackageIds}
          phase={removeFlow.phase}
          receipt={removeFlow.receipt}
          rejection={removeFlow.rejection}
          onCancel={() => setRemoveFlow(null)}
          onConfirm={confirmRemove}
        />
      ) : null}

      {installFlow !== null ? (
        <InstallConfirmDialog
          plan={installFlow.plan}
          requestedPackages={installFlow.requestedPackages}
          phase={installFlow.phase}
          receipt={installFlow.receipt}
          rejection={installFlow.rejection}
          onCancel={() => setInstallFlow(null)}
          onConfirm={confirmInstall}
        />
      ) : null}

      {prereleasePrompt ? (
        <div
          className="vua-packages-dialog"
          role="dialog"
          aria-modal="true"
          aria-label={copy.toolbar.prereleaseTitle}
          onClick={() => setPrereleasePrompt(false)}
          onKeyDown={(event) => {
            if (event.key === "Escape") setPrereleasePrompt(false);
          }}
        >
          <div
            className="vua-packages-dialog__panel"
            onClick={(event) => event.stopPropagation()}
          >
            <h2 className="vua-packages-dialog__title">{copy.toolbar.prereleaseTitle}</h2>
            <div className="vua-packages__warning-strip">
              <Icon name="warning" size={16} />
              <p className="vua-caption">{copy.toolbar.prereleaseBody}</p>
            </div>
            <div className="vua-packages-dialog__footer">
              <Button variant="subtle" autoFocus onClick={() => setPrereleasePrompt(false)}>
                {copy.toolbar.prereleaseCancel}
              </Button>
              <Button
                variant="primary"
                onClick={() => {
                  setPrereleaseAcked(true);
                  setShowPrereleases(true);
                  setPrereleasePrompt(false);
                }}
              >
                {copy.toolbar.prereleaseConfirm}
              </Button>
            </div>
          </div>
        </div>
      ) : null}

    </div>
  );
}
