import { ModalOwnerContext, useModalLayer } from "./modal-layer.tsx";
import { useDialogMotion } from "./dialog-motion.ts";
import type { ReactNode } from "react";
import "./content-dialog.css";

/**
 * 内容型弹窗(2026-09-20 导航重构切片):宽面板容器,把整页级功能
 * (素材导入/搭配草稿)以弹窗形态承载进宿主页。可访问性模式与
 * ConfirmDialog 同构:role="dialog" aria-modal="true"、Esc 关闭、
 * 遮罩点击关闭、标题栏 + 关闭按钮;open=false 时不渲染。
 *
 * 层叠约定:遮罩/面板 z-index 沿用 --vua-z-dialog(与 ConfirmDialog
 * 同层);ImportPage 内嵌浏览的固定导航条经 createPortal 挂
 * document.body,z-index 1000(import-page.css),刻意浮于本弹窗
 * 之上——它是窗口级条带,语义即「盖在所有应用内容之上」,弹窗打开
 * 期间仍须可点可关。
 */
export interface ContentDialogProps {
  readonly open: boolean;
  readonly title: ReactNode;
  /** 关闭按钮的无障碍文案(宿主从 i18n 表传入) */
  readonly closeLabel: string;
  readonly children: ReactNode;
  readonly onClose: () => void;
}

export function ContentDialog({ open, title, closeLabel, children, onClose }: ContentDialogProps) {
  const modal = useModalLayer(open, onClose);
  const shown = useDialogMotion(open, modal.overlayRef, modal.panelRef);

  if (!shown) return null;
  return (
    <ModalOwnerContext value={modal.id}>
      <div ref={modal.overlayRef} className="vua-content-dialog__overlay"
        data-closing={open ? undefined : true} onClick={modal.closeTop}>
        <div
          ref={modal.panelRef}
          tabIndex={-1}
          role="dialog"
          aria-modal="true"
          aria-label={typeof title === "string" ? title : undefined}
          className="vua-content-dialog"
          onClick={(event) => event.stopPropagation()}
        >
          <div className="vua-content-dialog__header">
            <span className="vua-content-dialog__title">{title}</span>
            <button
              type="button"
              className="vua-content-dialog__close"
              aria-label={closeLabel}
              onClick={modal.closeTop}
            >
              ×
            </button>
          </div>
          <div className="vua-content-dialog__body">{children}</div>
        </div>
      </div>
    </ModalOwnerContext>
  );
}
