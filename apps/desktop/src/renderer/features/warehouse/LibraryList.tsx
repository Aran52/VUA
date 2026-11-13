import { useEffect, useRef, useState, type CSSProperties, type MouseEvent } from "react";
import { Button } from "../../components/primitives/Button.tsx";
import { format, strings } from "../../i18n/index.ts";
import { formatDateOnly } from "../../i18n/date-time.ts";
import type { LibrarySort } from "@vua/contracts";
import type { LibraryCardFacts } from "../../gateway/index.ts";
import { headerSortState, readLibraryColumns, rememberLibraryColumns, shownVariant, type LibraryHeaderColumn } from "./library-view-model.ts";
import { sizeLabel } from "./library-text.ts";

const copy = strings.warehouse.listColumns;
const COLUMNS = ["title", "shop", "source", "size", "added"] as const;
const minimums = [220, 100, 88, 76, 96] as const;
const SORTABLE: Partial<Record<(typeof COLUMNS)[number], LibraryHeaderColumn>> = { title: "title", shop: "shop", size: "size", added: "added" };
const ARROWS = { ascending: "▲", descending: "▼", none: "" } as const;

/** Shared tracks for the header, account products and local imports. Widths the player drags are kept. */
export function useLibraryColumns(sort?: LibrarySort, onHeaderSort?: (column: LibraryHeaderColumn) => void) {
  const [weights, setWeights] = useState<readonly number[]>(readLibraryColumns);
  const cleanup = useRef<(() => void) | null>(null);
  useEffect(() => () => cleanup.current?.(), []);
  const style: CSSProperties = { gridTemplateColumns: weights.map((weight, i) => `minmax(${minimums[i]}px, ${weight}fr)`).join(" ") };
  const resize = (index: number, widths: number[], delta: number) => {
    const change = Math.max(minimums[index]! - widths[index]!, Math.min(delta, widths[index + 1]! - minimums[index + 1]!));
    const next = [...widths]; next[index] = widths[index]! + change; next[index + 1] = widths[index + 1]! - change;
    setWeights(next);
    rememberLibraryColumns(next);
  };
  const header = <div className="vua-library-list__header" style={style} role="row">
    {COLUMNS.map((column, index) => {
      const name = copy[column];
      const sortable = SORTABLE[column];
      const state = sortable !== undefined && sort !== undefined ? headerSortState(sortable, sort) : "none";
      return <div className="vua-library-list__heading" key={column} role="columnheader" data-column={column} aria-sort={sortable !== undefined ? state : undefined}>
        {sortable !== undefined && onHeaderSort ? <button type="button" className="vua-library-list__sort" data-nav-id={`warehouse-sort-${column}`} data-sorted={state !== "none" || undefined} onClick={() => onHeaderSort(sortable)}>
          {name}<span aria-hidden="true">{ARROWS[state]}</span>
        </button> : name}
        {index < COLUMNS.length - 1 ? <span role="separator" tabIndex={0} aria-orientation="vertical" aria-label={format(copy.resize, { name })}
          className="vua-library-list__resize"
          onPointerDown={event => {
            if (event.button !== 0) return;
            event.preventDefault(); event.stopPropagation(); cleanup.current?.();
            const header = event.currentTarget.parentElement!.parentElement!;
            const widths = [...header.children].map(node => node.getBoundingClientRect().width); const x = event.clientX;
            const move = (pointer: PointerEvent) => resize(index, widths, pointer.clientX - x);
            const finish = () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", finish); window.removeEventListener("pointercancel", finish); window.removeEventListener("blur", finish); cleanup.current = null; };
            cleanup.current = finish;
            window.addEventListener("pointermove", move); window.addEventListener("pointerup", finish); window.addEventListener("pointercancel", finish); window.addEventListener("blur", finish);
          }}
          onKeyDown={event => {
            if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
            event.preventDefault(); event.stopPropagation();
            resize(index, [...event.currentTarget.parentElement!.parentElement!.children].map(node => node.getBoundingClientRect().width), event.key === "ArrowRight" ? 20 : -20);
          }} /> : null}
      </div>;
    })}
  </div>;
  return { style, header };
}

export function libraryListSource(facts: LibraryCardFacts | undefined, origin: "product" | "imported_material" | "downloaded_material", transfer?: { active: boolean; paused: boolean }) {
  if (transfer?.active || facts?.operation?.state === "running" && !facts.operation.inspectRequired) return transfer?.paused ? "paused" : "downloading";
  if (["missing", "partial", "changed", "unreadable"].includes(facts?.storage.state ?? "") || facts?.operation?.state === "failed" || facts?.operation?.inspectRequired) return "error";
  if (origin === "imported_material") return "imported";
  return facts?.storage.presentCopies ? "downloaded" : "cloud";
}

/** One row: the variation reads as part of the name, and the numbers line up on the right. */
export function LibraryListRow({ id, productId, title, shop, variant, variantCount, source, sizeBytes, addedAt, selected, style, onSelect, onOpen, onMenu }: {
  id: string; productId?: string | undefined; title: string; shop?: string | null | undefined; variant?: string | null | undefined; variantCount?: number | undefined;
  source: ReturnType<typeof libraryListSource>; sizeBytes?: number | undefined; addedAt?: string | null | undefined; selected: boolean; style: CSSProperties; onSelect: (event: MouseEvent<HTMLElement>) => void; onOpen: () => void; onMenu: (event: MouseEvent<HTMLElement>) => void;
}) {
  const variation = (variantCount ?? 0) > 1 ? format(copy.variants, { count: variantCount! }) : shownVariant(title, variant);
  const size = (sizeBytes ?? 0) > 0 ? sizeLabel(sizeBytes!) : "—";
  return <div className="vua-warehouse-list-row" style={style} data-selected={selected || undefined} data-product-id={productId} data-selection-id={id}
    role="row" tabIndex={0} onClick={onSelect} onDoubleClick={onOpen} onContextMenu={onMenu}
    onKeyDown={event => { if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); onOpen(); } }}>
    <div className="vua-library-list__cell vua-library-list__title" role="cell">
      <span className="vua-library-list__name" title={variation === null ? title : `${title} · ${variation}`}>
        <span>{title}</span>{variation !== null ? <small>{variation}</small> : null}
      </span>
      <Button variant="subtle" aria-label={strings.warehouse.removeFiles.actions} aria-haspopup="menu" onClick={event => { event.stopPropagation(); onMenu(event); }}>⋯</Button>
    </div>
    <span className="vua-library-list__cell" role="cell" title={shop ?? undefined}>{shop || "—"}</span>
    <span className="vua-library-list__cell vua-library-list__source" role="cell" data-source={source}>{copy.states[source]}</span>
    <span className="vua-library-list__cell vua-library-list__number" role="cell">{size}</span>
    <span className="vua-library-list__cell vua-library-list__number" role="cell">{addedAt ? formatDateOnly(addedAt) : "—"}</span>
  </div>;
}
