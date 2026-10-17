/** The right album page: what the Avatar is, its checks, and its upload to VRChat. */
import { useState, type ReactNode } from "react";
import { Icon } from "@vua/design-system";
import { AMF_UPLOAD_STAGES, type AmfProductionRun, type AmfReleaseState, type AmfUpload } from "../../../gateway/index.ts";
import { format, formatDateTime, strings } from "../../../i18n/index.ts";
import { avatarTitle, materialsLine, outfitLines, releaseChecks, runningUpload, uploadedAvatarId, type CheckLine } from "./release-model.ts";

const copy = strings.releaseRoom;
const time = (iso: string | null | undefined) => (iso && !Number.isNaN(Date.parse(iso)) ? formatDateTime(iso) : "");

export interface SheetActions {
  readonly busy: boolean;
  readonly onUpload: () => void;
  readonly onStop: () => void;
  readonly onCheckAgain: () => void;
}

function checkText(check: CheckLine): string {
  const c = copy.checks;
  switch (check.key) {
    case "descriptor": return check.state === "ok" ? c.descriptor : c.descriptorMissing;
    case "bindings": return check.state === "ok" ? c.bindings : c.bindingsOpen;
    case "menus": return check.state === "ok" ? c.menus : c.menusOpen;
    case "references": return check.state === "ok" ? c.references : format(c.referencesMissing, { count: check.count ?? 0 });
    case "scripts": return check.state === "ok" ? c.scripts : c.scriptsMissing;
    case "shaders": return check.state === "ok" ? c.shaders : format(c.shadersMissing, { count: check.count ?? 0 });
    case "signIn": return check.signIn === "yes" ? c.signedIn : check.signIn === "no" ? c.signedOut : c.signInUnknown;
  }
}

function UploadProgress({ upload, onStop }: { upload: AmfUpload; onStop?: () => void }) {
  const at = AMF_UPLOAD_STAGES.indexOf(upload.stage);
  return <div className="vua-release-upload__progress" role="status">
    <ol className="vua-release-upload__stages">
      {AMF_UPLOAD_STAGES.filter(stage => stage !== "finishing").map((stage, index) => <li key={stage}
        data-state={index < at ? "done" : index === at ? "current" : "todo"}>{copy.stages[stage]}</li>)}
    </ol>
    <p className="vua-release-upload__hint">{copy.stageHints[upload.stage]}</p>
    {upload.stage === "uploading" && upload.progress !== null && <span className="vua-release-upload__bar" role="progressbar"
      aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(upload.progress * 100)}><span style={{ width: `${Math.round(upload.progress * 100)}%` }} /></span>}
    {upload.status && <small className="vua-text-secondary">{upload.status}</small>}
    {onStop && <button type="button" className="vua-release-upload__stop" onClick={onStop}><Icon name="close" size={16} />{copy.upload.stop}</button>}
  </div>;
}

function CopyId({ id }: { id: string }) {
  const [copied, setCopied] = useState(false);
  return <button type="button" className="vua-release-upload__copy" onClick={() => {
    void navigator.clipboard?.writeText(id).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1600); }, () => {});
  }}>{copied ? copy.upload.copied : copy.upload.copyId}</button>;
}

function UploadResult({ upload, onCheckAgain }: { upload: AmfUpload; onCheckAgain?: () => void }) {
  if (upload.state === "succeeded" && upload.blueprintId) {
    return <div className="vua-release-upload__result" data-tone="success" role="status">
      <strong><Icon name="check" size={16} />{copy.upload.uploaded}</strong>
      <span className="vua-text-secondary">{time(upload.endedAt)}</span>
      <span className="vua-release-upload__id"><span>{copy.upload.avatarId}</span><code>{upload.blueprintId}</code><CopyId id={upload.blueprintId} /></span>
      {upload.detail && <small className="vua-text-secondary">{upload.detail}</small>}
    </div>;
  }
  const reason = upload.reason ?? "unknown";
  return <div className="vua-release-upload__result" data-tone={upload.state === "cancelled" ? "neutral" : "attention"} role="status">
    <strong><Icon name={upload.state === "cancelled" ? "pause" : "warning"} size={16} />{upload.state === "cancelled" ? copy.upload.cancelled : copy.upload.failed}</strong>
    <p>{copy.reasons[reason]}</p>
    {upload.errors.length > 0 && <ul className="vua-release-upload__errors">{upload.errors.map((error, index) => <li key={index}>{error}</li>)}</ul>}
    {upload.detail && <details><summary>{copy.upload.details}</summary><p>{upload.detail}</p></details>}
    {onCheckAgain && (reason === "avatar_missing" || reason === "thumbnail_missing") && <button type="button" className="vua-release-upload__link" onClick={onCheckAgain}>{copy.upload.checkAgain}</button>}
  </div>;
}

/** The right page. A turning page shows the same content without its controls. */
export function ReleaseSheet({ run, state, number, live, actions, notice }: {
  run: AmfProductionRun; state: AmfReleaseState | null; number: number; live: boolean; actions?: SheetActions; notice?: ReactNode;
}) {
  const outfits = outfitLines(run);
  const checks = releaseChecks(run);
  const running = runningUpload(state);
  const latest = state?.uploads[0] ?? null;
  const earlier = (state?.uploads ?? []).filter(upload => upload !== latest && upload.state !== "running").slice(0, 3);
  const uploaded = uploadedAvatarId(state) !== null;
  return <div className="vua-album__sheet">
    <header className="vua-album__heading">
      <span className="vua-album__number">{format(copy.number, { number: String(number).padStart(2, "0") })}</span>
      <h2>{avatarTitle(run)}</h2>
      <p className="vua-text-secondary">{materialsLine(run)}</p>
      {run.completedAt && <p className="vua-caption vua-text-secondary">{format(copy.finishedAt, { time: time(run.completedAt) })}</p>}
    </header>
    {notice}
    <section className="vua-album__block">
      <h3>{copy.outfits}</h3>
      {outfits.length === 0 ? <p className="vua-text-secondary">{copy.noOutfits}</p> : <ul className="vua-album__outfits">
        {outfits.map((outfit, index) => <li key={index}><Icon name="outfit" size={16} /><span>{outfit.name}</span>
          <small>{outfit.toggle ? `${format(copy.outfitSwitch, { name: outfit.toggle })} · ${outfit.active ? copy.onAtStart : copy.offAtStart}` : copy.alwaysWorn}</small></li>)}
      </ul>}
    </section>
    {checks.length > 0 && <section className="vua-album__block">
      <h3>{copy.checks.title}</h3>
      <ul className="vua-album__checks">{checks.map(check => <li key={check.key} data-state={check.state}>
        <Icon name={check.state === "ok" ? "check" : check.state === "attention" ? "warning" : "question"} size={16} />{checkText(check)}</li>)}</ul>
    </section>}
    <section className="vua-release-upload" aria-label={copy.upload.button}>
      {running ? <UploadProgress upload={running} {...(live && actions ? { onStop: actions.onStop } : {})} /> : <>
        <button type="button" className="vua-release-upload__button" disabled={!live || !actions || actions.busy} tabIndex={live ? 0 : -1}
          onClick={actions?.onUpload} data-nav-id="release-upload"><Icon name="arrow-up" size={20} />{uploaded ? copy.upload.again : copy.upload.button}</button>
        <small className="vua-text-secondary">{actions?.busy ? copy.upload.busy : state?.editorOpen ? copy.upload.unityOpen : copy.upload.privateNote}</small>
        {latest && latest.state !== "running" && <UploadResult upload={latest} {...(live && actions ? { onCheckAgain: actions.onCheckAgain } : {})} />}
      </>}
      {earlier.length > 0 && <details className="vua-release-upload__history"><summary>{copy.upload.history}</summary>
        <ul>{earlier.map(upload => <li key={upload.uploadId}>{format(copy.upload.historyLine, { time: time(upload.endedAt ?? upload.startedAt), result: copy.upload.results[upload.state] })}
          {upload.blueprintId && <code>{upload.blueprintId}</code>}</li>)}</ul></details>}
    </section>
  </div>;
}
