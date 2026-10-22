import { useMemo, useState } from "react";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { ConfirmDialog } from "../../components/primitives/ConfirmDialog.tsx";
import { EmptyState } from "../../components/primitives/EmptyState.tsx";
import { useAcquireView } from "../../gateway/index.ts";
import { format, strings } from "../../i18n/index.ts";
import {
  composeAddItemAction,
  composeRemoveItemAction,
  composeSetNameHintAction,
  composeUndoAction,
  useComposeDraft,
} from "../../app/compose-draft-store.ts";
import { composeSaveBlocked, useComposeSave } from "../../app/compose-save-chain.ts";
import { composeSourceLines } from "./compose-source-model.ts";
import { ProductionChainSection } from "./ProductionChainSection.tsx";

/**
 * 搭配草稿(019 批 B,桌面切片;2026-09-20 导航重构起由配方页经
 * ContentDialog 挂载为页内弹窗,不再是独立页):项目无关的素材搭配草稿
 * ——从素材库
 * (warehouse 读面)直接把素材加入当前搭配,无须先创建 Unity 项目(UI-03)。
 *
 * - 共享草稿状态在容器层(app/compose-draft-store signal),跨 UI 根切换
 *   保留;本页只是其呈现/操作面之一;
 * - 保存链(批 B 保存链;D-3 起为容器层共享 hook app/compose-save-chain):
 *   两套 UI 消费同一保存链——同一线形状(recipe.save v1)、同一守卫、
 *   同一回执对齐;「已保存」仅在持久化回执后显示,失败保留内容并提供
 *   重试(UI-03/06);
 * - 撤销只回退本地未提交编辑,不反向执行已提交命令。
 */
const copy = strings.compose;

export function ComposePage() {
  const view = useAcquireView();
  const draft = useComposeDraft();
  const [sourceIndex, setSourceIndex] = useState(0);
  // 挂载名称高级展开(D3):默认隐藏——自动派生后无须用户填写
  const [advancedOpen, setAdvancedOpen] = useState(false);
  // 保存链:容器层共享 hook(D-3 提取;recipe.save 线形状与回执对齐不变;
  // D5 查重命中确认框亦由本 hook 承载——两套 UI 同一规则)
  const { saveState, duplicate, saveDraft, confirmDuplicateSave, cancelDuplicateSave } =
    useComposeSave();

  const entries = view.kind === "entries" ? view.entries : [];
  const canUndo = draft.undoStack.length > 0;
  const source = entries[sourceIndex];

  const sourceLines = useMemo(
    () => composeSourceLines(view, draft.items),
    [view, draft.items],
  );

  return (
    <div className="vua-page">
      <section className="vua-page__hero">
        <h1 className="vua-title">{strings.nav.pages.composePage}</h1>
      </section>

      <Card>
        <div className="vua-page__stack">
          <section>
            <h3 className="vua-warehouse-detail__section-title">{copy.draftTitle}</h3>
            {draft.items.length === 0 ? (
              <EmptyState title={copy.draftEmptyTitle} description={copy.draftEmptyDesc} />
            ) : (
              <>
                {/* D3(用户裁定 2026-09-20「不该让用户填写」):挂载名称加入草稿
                    时已自动派生自条目 displayName,输入框收进可选高级展开——
                    默认隐藏,覆盖仅高级用户按需展开 */}
                <div className="vua-project-compat__row">
                  <Button
                    variant="subtle"
                    aria-expanded={advancedOpen}
                    onClick={() => setAdvancedOpen((open) => !open)}
                  >
                    {copy.advancedMountToggle}
                  </Button>
                </div>
                <ul className="vua-project-compat__specs">
                  {draft.items.map((item) => (
                    <li key={item.warehouseItemId}>
                      <strong>{item.title}</strong>{" "}
                      <span className="vua-caption vua-text-secondary">
                        {item.role === null ? "" : format(copy.roleLine, { role: item.role })}
                      </span>{" "}
                      {advancedOpen ? (
                        <input
                          type="text"
                          value={item.nameHint ?? ""}
                          placeholder={copy.nameHintPlaceholder}
                          aria-label={format(copy.nameHintAria, { title: item.title })}
                          onChange={(event) =>
                            composeSetNameHintAction(item.warehouseItemId, event.target.value)
                          }
                        />
                      ) : null}
                      <Button
                        variant="subtle"
                        aria-label={format(copy.removeItemAria, { title: item.title })}
                        onClick={() => composeRemoveItemAction(item.warehouseItemId)}
                      >
                        {copy.removeCta}
                      </Button>
                    </li>
                  ))}
                </ul>
              </>
            )}
            <div className="vua-project-compat__row">
              <Button variant="subtle" disabled={!canUndo} onClick={composeUndoAction}>
                {copy.undoCta}
              </Button>
              <Button
                variant="primary"
                disabled={
                  draft.items.length === 0 ||
                  saveState === "checking" ||
                  saveState === "saving" ||
                  composeSaveBlocked(draft.items)
                }
                onClick={saveDraft}
              >
                {saveState === "saving" ? copy.savingCta : copy.saveCta}
              </Button>
            </div>
            {saveState === "failed" ? (
              <p className="vua-caption vua-text-secondary" role="alert">
                {copy.saveFailedNote}
              </p>
            ) : null}
            {draft.saved !== null ? (
              <p className="vua-caption vua-text-secondary" role="status">
                {format(copy.savedNote, { revision: String(draft.saved.revision) })}
              </p>
            ) : null}
            {draft.dirty && draft.items.length > 0 ? (
              <p className="vua-caption vua-text-secondary" role="status">
                {copy.unsavedNote}
              </p>
            ) : null}
          </section>
        </div>
      </Card>

      <Card>
        <div className="vua-page__stack">
          <section>
            <h3 className="vua-warehouse-detail__section-title">{copy.sourceTitle}</h3>
            {entries.length === 0 ? (
              <EmptyState title={copy.sourceEmptyTitle} description={copy.sourceEmptyDesc} />
            ) : null}
            <ul className="vua-project-compat__specs">
              {sourceLines.map((line, index) => (
                <li key={line.id}>
                  <button
                    type="button"
                    className="vua-select-row__trigger"
                    aria-pressed={index === sourceIndex}
                    onClick={() => setSourceIndex(index)}
                  >
                    <strong>{line.title}</strong>
                  </button>{" "}
                  {line.added ? <Badge tone="success">{copy.inDraftBadge}</Badge> : null}{" "}
                  {index === sourceIndex ? (
                    <Button
                      variant="default"
                      onClick={() =>
                        composeAddItemAction({
                          warehouseItemId: line.id,
                          title: line.title,
                          role: null,
                          nameHint: null,
                        })
                      }
                    >
                      {copy.addCta}
                    </Button>
                  ) : null}
                </li>
              ))}
            </ul>
          </section>
        </div>
      </Card>

      {/* 019 批 C:生产链段(保存后推进——解析/计划/任务/记录;无保存事实时
          自行不渲染);身份与请求状态在共享容器层,跨 UI 根保留 */}
      <ProductionChainSection />

      {/* D5(用户裁定 2026-09-20「点 N 次存 N 版应先查重询问」):保存前查重
          命中完全一致内容时,须用户确认才提交新修订 */}
      <ConfirmDialog
        open={duplicate !== null}
        title={copy.dedupTitle}
        cancelLabel={copy.dedupCancelCta}
        confirmLabel={copy.dedupConfirmCta}
        onCancel={cancelDuplicateSave}
        onConfirm={confirmDuplicateSave}
      >
        <p className="vua-text-secondary">
          {duplicate === null
            ? null
            : format(copy.dedupBody, {
                recipeId: duplicate.recipeId,
                revision: String(duplicate.revision),
              })}
        </p>
      </ConfirmDialog>
    </div>
  );
}
