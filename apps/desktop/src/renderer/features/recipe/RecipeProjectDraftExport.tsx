import { useEffect, useState } from "react";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { ConfirmDialog } from "../../components/primitives/ConfirmDialog.tsx";
import { EmptyState } from "../../components/primitives/EmptyState.tsx";
import { Skeleton } from "../../components/primitives/Skeleton.tsx";
import {
  useGateway,
  type ProjectListProjectsOutcome,
  type RecipeProjectDraft,
} from "../../gateway/index.ts";
import { format, strings } from "../../i18n/index.ts";
import { WarehouseEntrySelector } from "../warehouse/WarehouseEntrySelector.tsx";
import { useRecipeExportSave } from "../../app/recipe-export-save-chain.ts";
import {
  recipeExportAddItem,
  recipeExportConfirmOpened,
  recipeExportRemoveAddition,
  recipeExportSaveBlocked,
  recipeExportTitleEdited,
  recipeExportUnityConstraintEdited,
  type RecipeExportConfirmState,
} from "./recipe-export-draft-model.ts";

const copy = strings.recipe;

/**
 * 从工程导出配方草稿(proposal 029 B 面环 4 桌面消费批;recipe-export v0.1
 * 冻结词表消费,配方页 hero 内弹窗):
 *
 * - 拾取段:入口限定 VUA 已注册工程集(project.listProjects 013 聚合读面),
 *   不开放任意路径输入;陈旧登记(路径不存在)如实标注并禁用;服务未连接/
 *   无登记按诚实空态呈现;
 * - 确认段:草稿六事实键如实呈现——来源(路径/名称可空/身份三态)、环境
 *   版本 verbatim(不可读时由用户补全)、声明依赖 verbatim(空数组=诚实
 *   空态;锁定钉定只作呈现不入文档)、缺失维度清单照单渲染(导出不宣称
 *   还原设计意图);转正补全三件 = 标题(草稿无 title,用户显式给)+ 至少
 *   一条素材(草稿零关系面,与 A2 同一仓储读面投影选择器)+ 环境约束
 *   (仅不可读时);
 * - 转正:走既有 recipe.save 保存链(容器层 useRecipeExportSave——同一
 *   线形状、同一守卫集:忙碌守卫/D5 查重/回执分类),「已保存」仅在持久化
 *   回执后呈现,失败如实、内容保留、重试显式;草稿绝不静默转正。
 */

export function RecipeProjectDraftExport() {
  const gateway = useGateway();
  const [stage, setStage] = useState<"pick" | "confirm">("pick");
  const [projects, setProjects] = useState<ProjectListProjectsOutcome | "loading">("loading");
  const [exportingPath, setExportingPath] = useState<string | null>(null);
  const [exportError, setExportError] = useState<"unavailable" | "request_rejected" | null>(null);
  const [confirm, setConfirm] = useState<RecipeExportConfirmState | null>(null);
  const {
    saveState,
    savedReceipt,
    duplicate,
    saveRecipe,
    confirmDuplicateSave,
    cancelDuplicateSave,
  } = useRecipeExportSave(confirm);

  useEffect(() => {
    if (stage !== "pick") return;
    let alive = true;
    setProjects("loading");
    void gateway.projectOps.listProjects().then((outcome) => {
      if (alive) setProjects(outcome);
    });
    return () => {
      alive = false;
    };
  }, [gateway, stage]);

  const pickProject = (projectPath: string) => {
    if (exportingPath !== null) return;
    setExportError(null);
    setExportingPath(projectPath);
    void gateway.recipeExport
      .exportProjectDraft(projectPath)
      .then((outcome) => {
        if (outcome.ok) {
          setConfirm(recipeExportConfirmOpened(outcome.draft));
          setStage("confirm");
        } else {
          setExportError(outcome.error.kind);
        }
      })
      .finally(() => setExportingPath(null));
  };

  if (stage === "pick") {
    return (
      <div className="vua-page__stack">
        <p className="vua-caption vua-text-secondary">{copy.exportPickTitle}</p>
        {projects === "loading" ? (
          <>
            <Skeleton width="60%" />
            <Skeleton width="40%" />
          </>
        ) : !projects.ok ? (
          <EmptyState title={copy.exportPickUnavailable} />
        ) : projects.projects.length === 0 ? (
          <EmptyState title={copy.exportPickEmpty} />
        ) : (
          <ul
            className="vua-project-compat__specs"
            role="listbox"
            aria-label={copy.exportPickAria}
          >
            {projects.projects.map((project) => {
              const exporting = exportingPath === project.path;
              return (
                <li key={project.path}>
                  <button
                    type="button"
                    className="vua-select-row__trigger"
                    disabled={!project.pathPresent || exportingPath !== null}
                    aria-pressed={exporting}
                    onClick={() => pickProject(project.path)}
                  >
                    <strong>{project.name}</strong>
                  </button>{" "}
                  <span className="vua-caption vua-text-secondary">{project.path}</span>{" "}
                  {!project.pathPresent ? (
                    <Badge tone="warning">{copy.exportStaleBadge}</Badge>
                  ) : null}{" "}
                  {exporting ? (
                    <span className="vua-caption vua-text-secondary" role="status">
                      {copy.exportExporting}
                    </span>
                  ) : null}
                </li>
              );
            })}
          </ul>
        )}
        {exportError !== null ? (
          <p className="vua-caption vua-text-secondary" role="alert">
            {exportError === "unavailable" ? copy.exportFailedUnavailable : copy.exportFailedRejected}
          </p>
        ) : null}
      </div>
    );
  }

  if (confirm === null) return null;
  const draft: RecipeProjectDraft = confirm.draft;
  const blocked = recipeExportSaveBlocked(confirm);
  const addedIds = new Set(confirm.additions.map((item) => item.warehouseItemId));

  return (
    <div className="vua-page__stack">
      <div className="vua-page__actions" role="group" aria-label={copy.exportDraftBadge}>
        <Badge tone="warning">{copy.exportDraftBadge}</Badge>
      </div>
      {/* 来源工程(013 聚合观察 verbatim;名称可空 = 诚实缺席;身份三态只是
          适用边界事实,不是门,非 VUA 差异提示候未决项 2 裁决) */}
      <section>
        <h3 className="vua-warehouse-detail__section-title">{copy.exportOriginSection}</h3>
        <ul className="vua-project-compat__specs">
          <li>
            <span className="vua-caption vua-text-secondary">{copy.exportPathLabel}: </span>
            {draft.origin.projectPath}
          </li>
          <li>
            <span className="vua-caption vua-text-secondary">{copy.exportNameLabel}: </span>
            {draft.origin.projectName ?? copy.exportNameAbsent}
          </li>
          <li>
            <span className="vua-caption vua-text-secondary">{copy.exportIdentityLabel}: </span>
            {draft.origin.vuaIdentityStatus === "present"
              ? copy.exportIdentityPresent
              : draft.origin.vuaIdentityStatus === "absent"
                ? copy.exportIdentityAbsent
                : copy.exportIdentityUnreadable}
          </li>
        </ul>
      </section>
      {/* 环境(盘上观察 verbatim 不迁移;null = 不可读,由用户显式补全——
          系统绝不代填) */}
      <section>
        <h3 className="vua-warehouse-detail__section-title">{copy.exportUnityLabel}</h3>
        {draft.environment.unityVersionConstraint !== null ? (
          <p>{draft.environment.unityVersionConstraint}</p>
        ) : (
          <>
            <p className="vua-caption vua-text-secondary" role="note">
              {copy.exportUnityUnreadable}
            </p>
            <input
              type="text"
              value={confirm.unityConstraintInput ?? ""}
              placeholder={copy.exportUnityPlaceholder}
              aria-label={copy.exportUnityInputAria}
              onChange={(event) =>
                setConfirm(recipeExportUnityConstraintEdited(confirm, event.target.value))
              }
            />
          </>
        )}
      </section>
      {/* 依赖(声明集 verbatim;空数组 = 诚实应答;锁定钉定只作呈现——配方
          文档的 locked 块由保存/解析链权威铸造,草稿绝不伪造) */}
      <section>
        <h3 className="vua-warehouse-detail__section-title">{copy.exportDepsTitle}</h3>
        {draft.dependencies.length === 0 ? (
          <p className="vua-caption vua-text-secondary">{copy.exportDepsEmpty}</p>
        ) : (
          <ul className="vua-project-compat__specs">
            {draft.dependencies.map((row) => (
              <li key={row.packageId}>
                <strong>{row.packageId}</strong>{" "}
                <span className="vua-caption vua-text-secondary">{row.versionConstraint}</span>{" "}
                {row.lockedVersion !== undefined ? (
                  <Badge tone="neutral">
                    {format(copy.exportDepsLocked, { version: row.lockedVersion })}
                  </Badge>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </section>
      {/* 缺失维度清单(导出的诚实核心:照单呈现「缺什么」,九维恒在,版本
          不可读时第十维在场;不宣称还原设计意图) */}
      <section>
        <h3 className="vua-warehouse-detail__section-title">{copy.exportMissingTitle}</h3>
        <ul className="vua-project-compat__specs">
          {draft.missing.map((dimension) => (
            <li key={dimension}>{copy.missingDims[dimension]}</li>
          ))}
        </ul>
        <p className="vua-caption vua-text-secondary" role="note">
          {copy.exportHonestyNote}
        </p>
      </section>
      {/* 转正补全三件:标题(用户显式给;预填自工程名时注明来源)+ 素材
          (选择器只是仓储读面投影,A3 同一面)+ 环境(仅不可读时,见上) */}
      <section>
        <h3 className="vua-warehouse-detail__section-title">{copy.exportTitleLabel}</h3>
        <input
          type="text"
          value={confirm.title}
          aria-label={copy.exportTitleAria}
          onChange={(event) => setConfirm(recipeExportTitleEdited(confirm, event.target.value))}
        />
      </section>
      <section>
        <h3 className="vua-warehouse-detail__section-title">{copy.addMaterialCta}</h3>
        <WarehouseEntrySelector
          addedIds={addedIds}
          onPick={(entry) =>
            setConfirm(
              recipeExportAddItem(
                confirm,
                {
                  warehouseItemId: entry.warehouseItemId,
                  title: entry.displayName,
                  role: null,
                  nameHint: null,
                },
                new Date().toISOString(),
              ),
            )
          }
        />
      </section>
      {confirm.additions.length > 0 ? (
        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.editSectionTitle}</h3>
          <ul className="vua-project-compat__specs">
            {confirm.additions.map((item) => (
              <li key={item.warehouseItemId}>
                <strong>{item.title}</strong>{" "}
                <Button
                  variant="subtle"
                  aria-label={format(copy.editRemoveAria, { title: item.title })}
                  onClick={() =>
                    setConfirm(recipeExportRemoveAddition(confirm, item.warehouseItemId))
                  }
                >
                  {strings.compose.removeCta}
                </Button>
              </li>
            ))}
          </ul>
          <p className="vua-caption vua-text-secondary" role="status">
            {copy.editDirtyNote}
          </p>
        </section>
      ) : null}
      {/* 转正保存(既有 recipe.save 保存链:同一线形状同一守卫集;「已保存」
          仅在持久化回执后呈现;失败如实、内容保留、重试显式) */}
      <div className="vua-page__actions">
        <Button
          variant="primary"
          disabled={blocked || saveState === "checking" || saveState === "saving"}
          onClick={saveRecipe}
        >
          {saveState === "saving" ? copy.savingEditCta : copy.exportSaveCta}
        </Button>
      </div>
      {blocked && saveState === "idle" ? (
        <p className="vua-caption vua-text-secondary">{copy.exportBlockedNote}</p>
      ) : null}
      {saveState === "failed" ? (
        <p className="vua-caption vua-text-secondary" role="alert">
          {copy.editFailedNote}
        </p>
      ) : null}
      {savedReceipt !== null ? (
        <p className="vua-caption vua-text-secondary" role="status">
          {format(strings.compose.savedNote, { revision: String(savedReceipt.revision) })}
        </p>
      ) : null}
      {/* D5 同一守卫:查重命中完全一致内容 → 同一确认框,用户确认才提交 */}
      <ConfirmDialog
        open={duplicate !== null}
        title={strings.compose.dedupTitle}
        cancelLabel={strings.compose.dedupCancelCta}
        confirmLabel={strings.compose.dedupConfirmCta}
        onCancel={cancelDuplicateSave}
        onConfirm={confirmDuplicateSave}
      >
        <p className="vua-text-secondary">
          {duplicate === null
            ? null
            : format(strings.compose.dedupBody, {
                recipeId: duplicate.recipeId,
                revision: String(duplicate.revision),
              })}
        </p>
      </ConfirmDialog>
    </div>
  );
}
