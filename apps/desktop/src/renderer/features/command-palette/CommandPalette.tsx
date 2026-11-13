import { useEffect, useRef, useState } from "react";
import { strings } from "../../i18n/index.ts";
import { POPOVER_ENTER, useEntranceMotion } from "../../components/primitives/motion.ts";
import {
  filterCommands,
  moveCursor,
  type CommandItem,
} from "./command-palette-model.ts";
import "./command-palette.css";

const copy = strings.commandPalette;

/**
 * 命令面板(C-EFFICIENCY,ui-ux §6.1 Ctrl+P):
 * - 页面跳转 + 全局动作统一命令列表;过滤/光标语义见 command-palette-model;
 * - 键盘:↑↓ 循环移动光标,Enter 执行,Esc 关闭;鼠标 hover 同步光标;
 * - 可访问性:dialog + combobox/listbox 模式(APG);背景点击关闭;
 * - 执行即关闭;动作失败不在面板内呈现(本切片命令均为恒可执行的导航/主题)。
 */
export function CommandPalette({
  commands,
  onClose,
}: {
  commands: readonly CommandItem[];
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const backdropRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const results = filterCommands(commands, query);
  const active = Math.min(cursor, Math.max(0, results.length - 1));

  useEffect(() => {
    inputRef.current?.focus();
  }, []);
  useEntranceMotion(true, backdropRef, [{ opacity: 0 }, { opacity: 1 }]);
  useEntranceMotion(true, panelRef, POPOVER_ENTER);

  const execute = (command: CommandItem) => {
    command.run();
    onClose();
  };

  return (
    <div ref={backdropRef} className="vua-palette__backdrop" onClick={onClose}>
      <div
        ref={panelRef}
        className="vua-palette"
        role="dialog"
        aria-modal="true"
        aria-label={copy.aria}
        onClick={(event) => event.stopPropagation()}
      >
        <input
          ref={inputRef}
          className="vua-palette__input"
          role="combobox"
          aria-expanded="true"
          aria-controls="vua-palette-list"
          aria-activedescendant={
            results.length > 0 ? `vua-palette-option-${results[active]?.id}` : undefined
          }
          value={query}
          placeholder={copy.placeholder}
          onChange={(event) => {
            setQuery(event.target.value);
            setCursor(0);
          }}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown") {
              event.preventDefault();
              setCursor(moveCursor(active, 1, results.length));
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setCursor(moveCursor(active, -1, results.length));
            } else if (event.key === "Enter") {
              event.preventDefault();
              const command = results[active];
              if (command) execute(command);
            } else if (event.key === "Escape") {
              event.preventDefault();
              onClose();
            }
          }}
        />
        <ul className="vua-palette__list" id="vua-palette-list" role="listbox">
          {results.map((command, index) => (
            <li
              key={command.id}
              id={`vua-palette-option-${command.id}`}
              role="option"
              aria-selected={index === active}
              data-active={index === active || undefined}
              className="vua-palette__option"
              onMouseEnter={() => setCursor(index)}
              onClick={() => execute(command)}
            >
              <span className="vua-caption vua-text-secondary">
                {command.group === "pages" ? copy.groupPages : copy.groupActions}
              </span>
              {command.label}
            </li>
          ))}
          {results.length === 0 ? (
            <li className="vua-palette__empty vua-text-secondary">{copy.empty}</li>
          ) : null}
        </ul>
      </div>
    </div>
  );
}
