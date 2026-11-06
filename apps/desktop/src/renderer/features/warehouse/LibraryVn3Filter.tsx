/** The VN3 filter: which terms state to show, and which uses the terms must allow. Counts follow the other
 * filters, so a player sees what a choice would leave before making it. */
import { useEffect, useLayoutEffect, useRef } from "react";
import { Icon } from "@vua/design-system";
import { LIBRARY_VN3_STATES, type LibraryVn3CountsV03, type LibraryVn3Item, type LibraryVn3State } from "@vua/contracts";
import { Button } from "../../components/primitives/Button.tsx";
import { POPOVER_ENTER, POPOVER_EXIT, usePresenceMotion } from "../../components/primitives/motion.ts";
import { format, strings } from "../../i18n/index.ts";
import { libraryViewCopy } from "../../i18n/library-view-copy.ts";
import { LibraryTermsRead } from "./LibraryTermsRead.tsx";
import { NO_VN3_CHOICE, toggleVn3Item, VN3_FILTER_GROUPS, vn3ChoiceActive, type LibraryVn3Choice } from "./library-view-model.ts";

const copy = libraryViewCopy.vn3;
const names = strings.useTerms.names;

const itemName = (item: LibraryVn3Item) => item === "V" ? copy.noCredit : names[item];
const stateCount = (counts: LibraryVn3CountsV03, state: LibraryVn3State) =>
  state === "readable" ? counts.readable : state === "no_vn3" ? counts.noVn3 : state === "unread" ? counts.unread : counts.unreadable;
const allCount = (counts: LibraryVn3CountsV03) => counts.readable + counts.noVn3 + counts.unread + counts.unreadable;

export function LibraryVn3Filter({ value, counts, disabled, open, onOpenChange, onChange, onUpdated }: {
  value: LibraryVn3Choice; counts: LibraryVn3CountsV03 | undefined; disabled: boolean; open: boolean;
  onOpenChange: (open: boolean) => void; onChange: (next: LibraryVn3Choice) => void; onUpdated: () => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const shown = usePresenceMotion(open, panel, POPOVER_ENTER, POPOVER_EXIT);
  const close = useRef(onOpenChange); close.current = onOpenChange;
  // The panel ends above whatever a theme draws under the page.
  useLayoutEffect(() => {
    if (!shown) return;
    const fit = () => {
      const element = panel.current, bottom = element?.closest(".vua-shell__main")?.getBoundingClientRect().bottom ?? window.innerHeight;
      element?.style.setProperty("--vua-vn3-panel-height", `${Math.max(240, Math.min(600, bottom - element.getBoundingClientRect().top - 12))}px`);
    };
    fit();
    window.addEventListener("resize", fit);
    return () => window.removeEventListener("resize", fit);
  }, [shown]);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => { if (event.target instanceof Node && !container.current?.contains(event.target)) close.current(false); };
    // Esc closes only this panel; the page's selection keeps until the next Esc.
    const key = (event: KeyboardEvent) => { if (event.key === "Escape") { event.stopPropagation(); close.current(false); } };
    window.addEventListener("pointerdown", outside, true);
    window.addEventListener("keydown", key, { capture: true });
    return () => { window.removeEventListener("pointerdown", outside, true); window.removeEventListener("keydown", key, { capture: true }); };
  }, [open]);
  const chosen = value.items.length + (value.state === null ? 0 : 1);
  return <div ref={container} className="vua-vn3-filter" data-active={vn3ChoiceActive(value) || undefined}>
    <button type="button" className="vua-warehouse__filter vua-vn3-filter__button" data-nav-id="warehouse-vn3" disabled={disabled} aria-haspopup="dialog" aria-expanded={open} onClick={() => onOpenChange(!open)}>
      <span>{chosen > 0 ? format(copy.active, { count: chosen }) : copy.button}</span><Icon name="chevron-down" size={16} />
    </button>
    {shown ? <div ref={panel} className="vua-vn3-filter__panel" role="dialog" aria-label={copy.button}>
      <div className="vua-vn3-filter__states" role="radiogroup" aria-label={strings.useTerms.title}>
        {[null, ...LIBRARY_VN3_STATES].map(state => {
          const count = counts === undefined ? null : state === null ? allCount(counts) : stateCount(counts, state);
          return <label key={state ?? "all"} data-checked={value.state === state || undefined}>
            <input type="radio" name="library-vn3-state" checked={value.state === state} data-nav-id={`warehouse-vn3-state-${state ?? "all"}`} onChange={() => onChange({ ...value, state, ...(state !== null && state !== "readable" ? { items: [] } : {}) })} />
            <span>{copy.states[state ?? "all"]}</span>{count === null ? null : <small>{count}</small>}
          </label>;
        })}
      </div>
      <div className="vua-vn3-filter__allow">
      {(counts?.unread ?? 0) > 0 ? <div className="vua-vn3-filter__read"><LibraryTermsRead onUpdated={onUpdated} label={format(copy.read, { count: counts!.unread })} primary={counts!.readable === 0} /></div> : null}
      <fieldset className="vua-vn3-filter__items" disabled={value.state !== null && value.state !== "readable"}>
        <legend>{copy.allow}</legend>
        <div className="vua-vn3-filter__groups">
          {VN3_FILTER_GROUPS.map(group => <section key={group.id}>
            <h3>{copy.groups[group.id]}</h3>
            {group.items.map(item => <label key={item} data-checked={value.items.includes(item) || undefined}>
              <input type="checkbox" checked={value.items.includes(item)} data-nav-id={`warehouse-vn3-item-${item}`} onChange={() => onChange(toggleVn3Item(value, item))} />
              <span>{itemName(item)}</span>{counts === undefined ? null : <small data-zero={counts.items[item] === 0 || undefined}>{counts.items[item]}</small>}
            </label>)}
          </section>)}
        </div>
      </fieldset>
      </div>
      <div className="vua-vn3-filter__foot">
        <label className="vua-vn3-filter__contact">
          <input type="checkbox" checked={value.includeContact} data-nav-id="warehouse-vn3-contact" onChange={event => onChange({ ...value, includeContact: event.target.checked })} />
          <span>{copy.includeContact}</span>
        </label>
        <Button variant="subtle" disabled={!vn3ChoiceActive(value) && !value.includeContact} data-nav-id="warehouse-vn3-reset" onClick={() => onChange(NO_VN3_CHOICE)}>{copy.reset}</Button>
      </div>
    </div> : null}
  </div>;
}
