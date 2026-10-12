import { useLayoutEffect, useRef } from "react";
import { createPortal } from "react-dom";
import "./context-menu.css";

/**
 * 右键上下文菜单(S-XII)。
 *
 * 纪律(用户裁定):
 * - 大部分区域不放右键菜单:App 壳层统一抑制浏览器默认菜单,
 *   仅文本输入框(input/textarea/contenteditable)放行原生编辑菜单;
 * - 自定义菜单只挂在模型生产模块的素材/配方/成品卡片上,
 *   菜单项必须映射真实存在的动作,不放占位死按钮(诚实纪律);
 * - 经 createPortal 挂到 body:图谱世界层等有 transform 的祖先
 *   会让 fixed 定位失效,portal 是唯一稳妥解。
 *   (顶栏折叠菜单不复用本组件:它展开的是原 Tab 控件,见 NavOverflowMenu)
 */
export interface ContextMenuItem {
  readonly id: string;
  readonly label: string;
  readonly shortcut?: string;
  readonly onSelect: () => void;
  /** 置灰项:动作当前不可用(如节点从未拖动时的"归位") */
  readonly disabled?: boolean;
  /** 置灰原因(ui-ux §6:Disabled 附可发现的原因),经 title 属性 hover 可见 */
  readonly disabledReason?: string;
}

/** 菜单打开状态:null 即关闭;x/y 为视口坐标(取自 contextmenu 的 clientX/Y) */
export interface ContextMenuState {
  readonly x: number;
  readonly y: number;
  readonly items: readonly ContextMenuItem[];
}

export function ContextMenu({ menu, onClose }: { menu: ContextMenuState; onClose: () => void }) {
  const ref = useRef<HTMLDivElement | null>(null);

  // 打开后:视口边缘防溢出 + 聚焦首个可用项;Escape/外击/滚动/窗口失焦即关闭
  useLayoutEffect(() => {
    const menuEl = ref.current;
    if (menuEl === null) return;
    const rect = menuEl.getBoundingClientRect();
    menuEl.style.left = `${Math.min(menu.x, Math.max(8, window.innerWidth - rect.width - 8))}px`;
    menuEl.style.top = `${Math.min(menu.y, Math.max(8, window.innerHeight - rect.height - 8))}px`;
    menuEl
      .querySelector<HTMLButtonElement>(".vua-context-menu__item:not(:disabled)")
      ?.focus();

    const onPointerDown = (event: PointerEvent) => {
      if (event.target instanceof Node && menuEl.contains(event.target)) return;
      onClose();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        // 菜单存续期间吞掉 Escape:抽屉/面板等其他 Escape 监听不应连带触发
        event.stopPropagation();
        onClose();
        return;
      }
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      event.preventDefault();
      const items = [
        ...menuEl.querySelectorAll<HTMLButtonElement>(
          ".vua-context-menu__item:not(:disabled)",
        ),
      ];
      if (items.length === 0) return;
      const index = items.findIndex((item) => item === document.activeElement);
      const next =
        event.key === "ArrowDown"
          ? (index + 1) % items.length
          : (index - 1 + items.length) % items.length;
      items[next]?.focus();
    };
    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("scroll", onClose, true);
    window.addEventListener("blur", onClose);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("scroll", onClose, true);
      window.removeEventListener("blur", onClose);
    };
  }, [menu, onClose]);

  return createPortal(
    <div
      ref={ref}
      className="vua-context-menu"
      role="menu"
      style={{ left: menu.x, top: menu.y }}
    >
      {menu.items.map((item) => (
        <button
          key={item.id}
          type="button"
          role="menuitem"
          className="vua-context-menu__item"
          disabled={item.disabled}
          title={item.disabled ? item.disabledReason : undefined}
          onClick={() => {
            onClose();
            item.onSelect();
          }}
        >
          <span>{item.label}</span>
          {item.shortcut ? <span className="vua-context-menu__shortcut" aria-hidden="true">{item.shortcut}</span> : null}
        </button>
      ))}
    </div>,
    document.body,
  );
}
