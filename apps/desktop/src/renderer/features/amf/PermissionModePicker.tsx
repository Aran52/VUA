/** The workshop's permission mode: a button naming the mode in effect, and below it a card of the three modes. */
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { Icon, type IconName } from "@vua/design-system";
import { useDialogMotion } from "../../components/primitives/dialog-motion.ts";
import { strings } from "../../i18n/index.ts";
import { AUTOMATION_LEVELS, type AutomationLevel } from "./factory-model.ts";

const factory = strings.factory;
const ICONS: Record<AutomationLevel, IconName> = { ask: "hand", smart: "square-check", full: "shield" };

export function PermissionModePicker({ value, onChoose }: { value: AutomationLevel; onChoose: (level: AutomationLevel) => void }) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const card = useRef<HTMLDivElement>(null);
  const noOverlay = useRef<HTMLElement>(null);
  const shown = useDialogMotion(open, noOverlay, card, -6);
  const menuId = useId();

  useEffect(() => {
    if (!open) return;
    card.current?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus();
    const away = (event: PointerEvent) => { if (!root.current?.contains(event.target as Node)) setOpen(false); };
    document.addEventListener("pointerdown", away);
    return () => document.removeEventListener("pointerdown", away);
  }, [open]);

  const close = () => { setOpen(false); button.current?.focus(); };
  const choose = (level: AutomationLevel) => { close(); if (level !== value) onChoose(level); };
  const keys = (event: KeyboardEvent<HTMLDivElement>) => {
    const items = [...(card.current?.querySelectorAll<HTMLElement>('[role="menuitemradio"]') ?? [])];
    const at = items.indexOf(document.activeElement as HTMLElement);
    if (event.key === "Escape") { event.preventDefault(); close(); }
    else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      items[(at + (event.key === "ArrowDown" ? 1 : items.length - 1)) % items.length]?.focus();
    } else if (event.key === "Tab") setOpen(false);
  };

  return <div className="vua-mode-picker" ref={root}>
    <button ref={button} type="button" className="vua-mode-picker__button" aria-haspopup="menu" aria-expanded={open} aria-controls={menuId}
      aria-label={`${factory.automation}: ${factory.levels[value].name}`} onClick={() => setOpen(!open)} data-nav-id="workshop-mode">
      <Icon name={ICONS[value]} size={20} /><span className="vua-mode-picker__value"><span>{factory.automation}</span><strong>{factory.levels[value].name}</strong></span><Icon name="chevron-down" size={16} />
    </button>
    {shown && <div ref={card} id={menuId} role="menu" aria-label={factory.automation} className="vua-mode-picker__card"
      data-closing={open ? undefined : true} onKeyDown={keys}>
      {AUTOMATION_LEVELS.map(level => <button key={level} type="button" role="menuitemradio" aria-checked={level === value}
        className="vua-mode-picker__item" onClick={() => choose(level)} data-nav-id={`workshop-mode-${level}`}>
        <Icon name={ICONS[level]} size={20} />
        <span className="vua-mode-picker__text"><strong>{factory.levels[level].name}</strong><small>{factory.levels[level].hint}</small></span>
        <span className="vua-mode-picker__mark">{level === value && <Icon name="check" size={16} />}</span>
      </button>)}
    </div>}
  </div>;
}
