/** Material choice from the local library: a search and a check per package, at most 64. */
import { useMemo, useState } from "react";
import type { AmfProductionSource } from "../../gateway/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { EmptyState } from "../../components/primitives/EmptyState.tsx";
import { format, strings } from "../../i18n/index.ts";

const copy = strings.amfProduction;

export function MaterialPicker({ sources, chosen, onChange, disabled }: {
  sources: readonly AmfProductionSource[]; chosen: readonly string[]; onChange: (ids: readonly string[]) => void; disabled: boolean;
}) {
  const [search, setSearch] = useState("");
  const [visible, setVisible] = useState(60);
  const filtered = useMemo(() => sources.filter(s => `${s.itemName} ${s.name}`.toLocaleLowerCase().includes(search.toLocaleLowerCase())), [sources, search]);
  return <>
    <div className="vua-n3__toolbar"><input type="search" value={search} placeholder={copy.search} aria-label={copy.search} onChange={e => { setSearch(e.target.value); setVisible(60); }}/>
      <span>{format(copy.selected, { count: chosen.length })}</span></div>
    {sources.length === 0 ? <EmptyState title={copy.empty} description={copy.emptyHint} /> :
      <div className="vua-n3__materials">{filtered.slice(0, visible).map(source => <label key={source.sourceId} className="vua-n3__material" data-selected={chosen.includes(source.sourceId) || undefined}>
        <input type="checkbox" checked={chosen.includes(source.sourceId)} disabled={disabled || (!chosen.includes(source.sourceId) && chosen.length >= 64)}
          onChange={e => onChange(e.target.checked ? [...chosen, source.sourceId] : chosen.filter(id => id !== source.sourceId))}/>
        <span><strong title={source.itemName}>{source.itemName}</strong><small title={source.name}>{source.name}</small></span></label>)}</div>}
    {filtered.length > visible && <Button variant="subtle" onClick={() => setVisible(v => v + 60)}>{copy.more}</Button>}
  </>;
}
