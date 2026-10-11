import { useState } from "react";
import type { BoothSearchResultV01 } from "@vua/contracts";
import { boothSearchKeyword } from "./booth-match-model.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { ContentDialog } from "../../components/primitives/ContentDialog.tsx";
import { format, strings } from "../../i18n/index.ts";

export function BoothMatchDialog({ name, result, busy, notice, onConfirm, onSearch, onClose, onOpen }: {
  name: string; result: BoothSearchResultV01 | null; busy: boolean; notice: string | null;
  onConfirm: (productId: string) => void; onSearch: (query: string) => void; onClose: () => void; onOpen: (productId: string) => void;
}) {
  const copy = strings.warehouse.sourceSearch;
  const [query, setQuery] = useState(result?.query ?? boothSearchKeyword(name));
  return <ContentDialog open title={copy.title} closeLabel={strings.common.dialogClose} onClose={onClose}>
    <div className="vua-booth-match">
      <p className="vua-caption vua-text-secondary">{format(copy.description, { name })}</p>
      <form onSubmit={event => { event.preventDefault(); if (!busy && query.trim()) onSearch(query.trim()); }}>
        <label>{copy.keyword}<input value={query} maxLength={200} disabled={busy} onChange={event => setQuery(event.target.value)} /></label>
        <Button type="submit" disabled={busy || !query.trim()}>{copy.search}</Button>
      </form>
      {notice === null ? null : <p role="status">{notice}</p>}
      {result?.truncated ? <p className="vua-caption">{copy.truncated}</p> : null}
      {result?.items.length === 0 ? <p>{copy.empty}</p> : null}
      <ul>{result?.items.map(item => <li key={item.productId}>
        <div><strong>{item.title}</strong>{item.shopName ? <span className="vua-caption">{item.shopName}</span> : null}{item.sourceCategory ? <span className="vua-caption">{item.sourceCategory}</span> : null}</div>
        <Button disabled={busy} onClick={() => onOpen(item.productId)}>{copy.openBooth}</Button>
        <Button variant="primary" disabled={busy} onClick={() => onConfirm(item.productId)}>{copy.confirm}</Button>
      </li>)}</ul>
    </div>
  </ContentDialog>;
}
