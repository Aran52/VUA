import { ModalOwnerContext, useModalLayer } from "./modal-layer.tsx";
import { useDialogMotion } from "./dialog-motion.ts";
import type { ReactNode } from "react";
import { Button } from "./Button.tsx";
import "./confirm-dialog.css";

/**
 * 确认对话框(W15 重做形态,示意图 B):模态面板 + 取消/确认双按钮。
 * - danger=true:确认按钮为危险红实心(§8.1 高危确认形态);
 * - Esc 与遮罩点击 = 取消(破坏性操作默认退出,不默认确认);
 * - 共享模态栈管理顶层 Esc、焦点循环、背景隔离与关闭焦点恢复。
 */
export interface ConfirmDialogProps {
  readonly open: boolean;
  readonly title: ReactNode;
  readonly children: ReactNode;
  readonly cancelLabel: string;
  readonly confirmLabel: string;
  readonly danger?: boolean;
  readonly onCancel: () => void;
  readonly onConfirm: () => void;
}

export function ConfirmDialog({
  open,
  title,
  children,
  cancelLabel,
  confirmLabel,
  danger = false,
  onCancel,
  onConfirm,
}: ConfirmDialogProps) {
  const modal = useModalLayer(open, onCancel);
  const shown = useDialogMotion(open, modal.overlayRef, modal.panelRef);

  if (!shown) return null;
  return (
    <ModalOwnerContext value={modal.id}>
      <div
        ref={modal.overlayRef}
        className="vua-confirm-dialog__overlay"
        data-closing={open ? undefined : true}
        onClick={modal.closeTop}
        data-testid="confirm-dialog-overlay"
      >
        <div
          ref={modal.panelRef}
          tabIndex={-1}
          role="dialog"
          aria-modal="true"
          aria-label={typeof title === "string" ? title : undefined}
          className="vua-confirm-dialog"
          onClick={(event) => event.stopPropagation()}
        >
          <div className="vua-confirm-dialog__header">
            <span className="vua-confirm-dialog__title" data-danger={danger || undefined}>
              {title}
            </span>
            <button
              type="button"
              className="vua-confirm-dialog__close"
              aria-label={cancelLabel}
              onClick={modal.closeTop}
            >
              ×
            </button>
          </div>
          <div className="vua-confirm-dialog__body">{children}</div>
          <div className="vua-confirm-dialog__actions">
            <Button variant="default" onClick={modal.closeTop}>
              {cancelLabel}
            </Button>
            <Button variant={danger ? "danger" : "primary"} onClick={onConfirm}>
              {confirmLabel}
            </Button>
          </div>
        </div>
      </div>
    </ModalOwnerContext>
  );
}
