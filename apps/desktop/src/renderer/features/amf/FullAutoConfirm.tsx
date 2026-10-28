import { ConfirmDialog } from "../../components/primitives/ConfirmDialog.tsx";
import { strings } from "../../i18n/index.ts";
import { rememberFullAutoNotice } from "./factory-model.ts";

/** Why the full-auto warning is open: a switch to full auto, or the first start while already in it. */
export type FullAutoReason = "switch" | "start";

/** The full-auto warning, shared by the materials start dialog and the workshop. Confirming records that the player has read it. */
export function FullAutoConfirm({ reason, onCancel, onConfirm }: {
  reason: FullAutoReason | null;
  onCancel: () => void;
  onConfirm: (reason: FullAutoReason) => void;
}) {
  const copy = strings.factory.fullAccess;
  const starting = reason === "start";
  return <ConfirmDialog open={reason !== null} danger title={starting ? copy.startTitle : copy.title}
    cancelLabel={starting ? copy.startCancel : copy.cancel} confirmLabel={starting ? copy.startConfirm : copy.confirm}
    onCancel={onCancel} onConfirm={() => { if (reason === null) return; rememberFullAutoNotice(); onConfirm(reason); }}>
    <p className="vua-confirm-dialog__text">{copy.body}</p>
    {starting && <p className="vua-confirm-dialog__text">{copy.startNote}</p>}
  </ConfirmDialog>;
}
