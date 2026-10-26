/** The player's confirmation for one upload: the Avatar's name on VRChat and the content rights statement. */
import { useId, useState, type ReactNode } from "react";
import { Alert } from "../../../components/primitives/Alert.tsx";
import { isAmfAvatarName, type AiUploadConfirmationV01 } from "@vua/contracts";
import { ContentDialog } from "../../../components/primitives/ContentDialog.tsx";
import { Button } from "../../../components/primitives/Button.tsx";
import { format, strings } from "../../../i18n/index.ts";
import { UploadEligibilityNote } from "../../help/UploadEligibilityNote.tsx";

const copy = strings.releaseRoom;

export function ReleaseUploadDialog({ proposedName, avatarId, picture, requestedByAi, error, starting, sdkMustFix = 0, onConfirm, onClose }: {
  proposedName: string;
  /** The VRChat Avatar this upload updates, when an earlier upload created it. */
  avatarId: string | null;
  picture: ReactNode;
  requestedByAi: boolean;
  error: ReactNode;
  starting: boolean;
  /** Items the SDK listed to fix before uploading, at the workshop's last check. */
  sdkMustFix?: number;
  onConfirm: (confirmation: AiUploadConfirmationV01) => void;
  onClose: () => void;
}) {
  const [name, setName] = useState(proposedName);
  const [rights, setRights] = useState(false);
  const nameId = useId();
  const trimmed = name.trim();
  const ready = rights && isAmfAvatarName(trimmed) && !starting;
  return <ContentDialog open title={copy.dialog.title} closeLabel={strings.common.dialogClose} onClose={onClose}>
    <form className="vua-release-dialog" onSubmit={event => { event.preventDefault(); if (ready) onConfirm({ avatarName: trimmed, rightsConfirmed: true }); }}>
      <div className="vua-release-dialog__picture">{picture}</div>
      <div className="vua-release-dialog__fields">
        {requestedByAi && <Alert tone="info">{copy.dialog.requestedByAi}</Alert>}
        {sdkMustFix > 0 && <Alert tone="warning">{format(strings.sdkReview.uploadWarning, { count: String(sdkMustFix) })}</Alert>}
        {avatarId === null ? <label htmlFor={nameId} className="vua-release-dialog__name">
          <span>{copy.dialog.name}</span>
          <input id={nameId} value={name} maxLength={128} onChange={event => setName(event.target.value)} aria-describedby={`${nameId}-hint`} />
          <small id={`${nameId}-hint`} className="vua-text-secondary">{copy.dialog.nameHint}</small>
        </label> : <p>{format(copy.dialog.update, { id: avatarId })}</p>}
        {avatarId === null && <p className="vua-text-secondary">{copy.upload.privateNote}</p>}
        <label className="vua-release-dialog__rights">
          <input type="checkbox" checked={rights} onChange={event => setRights(event.target.checked)} data-nav-id="release-rights" />
          <span>{copy.dialog.rights}</span>
        </label>
        <UploadEligibilityNote />
        <div className="vua-release-dialog__unity">
          <strong>{copy.dialog.inUnity}</strong>
          <ol>{[copy.dialog.inUnity1, copy.dialog.inUnity2, copy.dialog.inUnity3].map(step => <li key={step}>{step}</li>)}</ol>
        </div>
        {error}
        <div className="vua-release-dialog__actions">
          <Button variant="subtle" onClick={onClose}>{copy.dialog.cancel}</Button>
          <button type="submit" className="vua-release-upload__button vua-release-upload__button--compact" disabled={!ready} data-nav-id="release-confirm">
            {starting ? copy.dialog.starting : copy.dialog.confirm}</button>
        </div>
      </div>
    </form>
  </ContentDialog>;
}
