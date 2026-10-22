import { useState } from "react";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { EmptyState } from "../../components/primitives/EmptyState.tsx";
import { useAcquireView, type WarehouseEntry } from "../../gateway/index.ts";
import { format, strings } from "../../i18n/index.ts";
import { warehouseEntrySelectorRows } from "./acquire-model.ts";

const acquireCopy = strings.warehouse.acquire;
const copy = strings.warehouse.selector;

/**
 * 素材选择器(proposal 029 A3 本地段,消费切片三;仓储读面投影):配方页
 * 「添加素材」步的本地素材仓库选择面——只是仓储读面(acquire entries)的
 * 投影,不是素材入库入口:
 * - 素材入库仍走素材导入页既有两路径(内嵌浏览/系统拾取 → 任务中心),
 *   本选择器不立第三导入入口(§8.3 纪律照旧;本面零导入词面);
 * - 云端素材接入(未决项 3 = BOARD #46 / proposal 030 / U18 联动)裁决前
 *   诚实缺席:选择器只呈现本地仓储条目事实,不虚构云端入口;
 * - 诚实状态:读面未接入/空仓库/无匹配按既有诚实空态呈现;已在配方中的
 *   条目如实标注并禁用重复添加(添加幂等由编辑模型双层守卫)。
 * 添加语义:挑选即回调 onPick(条目事实原样上抛),是否入库由消费方保存链
 * 决定——本组件不持有配方状态。
 */
export function WarehouseEntrySelector({
  addedIds,
  onPick,
}: {
  /** 已在本配方中的条目 id(底稿 assets ∪ 待保存新增;如实标注 + 禁用) */
  addedIds: ReadonlySet<string>;
  /** 挑选回调(条目事实原样上抛;幂等守卫在编辑模型) */
  onPick: (entry: WarehouseEntry) => void;
}) {
  const view = useAcquireView();
  const [text, setText] = useState("");

  if (view.kind === "not-connected") {
    return (
      <EmptyState title={acquireCopy.states.notConnectedTitle} />
    );
  }

  const rows = warehouseEntrySelectorRows(view.entries, text);

  return (
    <div className="vua-page__stack">
      <div className="vua-warehouse__toolbar" role="search">
        <input
          type="search"
          className="vua-warehouse__search"
          placeholder={strings.warehouse.searchPlaceholder}
          aria-label={strings.warehouse.searchAria}
          value={text}
          onChange={(event) => setText(event.target.value)}
        />
        <span className="vua-caption vua-text-secondary">
          {format(acquireCopy.entryCount, { count: rows.length })}
        </span>
      </div>
      <p className="vua-caption vua-text-secondary" role="note">
        {copy.localOnlyNote}
      </p>
      {view.entries.length === 0 ? (
        <EmptyState title={acquireCopy.entriesTitle} description={acquireCopy.entriesEmpty} />
      ) : rows.length === 0 ? (
        <EmptyState title={strings.warehouse.states.emptyResultTitle} />
      ) : (
        <ul className="vua-project-compat__specs" role="listbox" aria-label={copy.listAria}>
          {rows.map((entry) => {
            const added = addedIds.has(entry.warehouseItemId);
            return (
              <li key={entry.warehouseItemId}>
                <button
                  type="button"
                  className="vua-select-row__trigger"
                  aria-pressed={added}
                  onClick={() => {
                    if (!added) onPick(entry);
                  }}
                >
                  <strong>{entry.displayName}</strong>
                </button>{" "}
                <span className="vua-caption vua-text-secondary">{entry.folderName}</span>{" "}
                {added ? (
                  <Badge tone="success">{copy.addedBadge}</Badge>
                ) : (
                  <Button variant="default" onClick={() => onPick(entry)}>
                    {copy.pickCta}
                  </Button>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
