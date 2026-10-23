/**
 * The face-tracking add-on station: the line's shape-key coverage, the creator patch menus its
 * materials carry, and the lock note a patched line shows beside the operations it disables.
 */
import { useState } from "react";
import type { FaceTrackingMenuV01, FaceTrackingPlanV01 } from "@vua/contracts";
import { AmfProductionError, type AmfProductionPort, type AmfProductionRun } from "../../gateway/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { ConfirmDialog } from "../../components/primitives/ConfirmDialog.tsx";
import { format, formatDateMinute, strings } from "../../i18n/index.ts";
import { ftCapability, ftFailure } from "./face-tracking-model.ts";

const copy = strings.factory.panels.faceTracking;
const profileCount = (mesh: { readonly profiles: readonly { readonly id: string; readonly matched: readonly string[] }[] }, id: string) =>
  mesh.profiles.find(profile => profile.id === id)?.matched.length ?? 0;

/** Beside an operation a creator-patched line locks: why, and the two ways out. */
export function FtLockNote({ onRestore, onNewLine, disabled }: {
  onRestore: (() => void) | null; onNewLine: () => void; disabled: boolean;
}) {
  return <div className="vua-ft-lock" role="note">
    <p>{copy.locked}</p>
    <div className="vua-n3__actions">
      {onRestore !== null && <Button variant="subtle" disabled={disabled} onClick={onRestore} data-nav-id="workshop-ft-restore">{copy.lockedRestore}</Button>}
      <Button variant="subtle" disabled={disabled} onClick={onNewLine} data-nav-id="workshop-ft-newline">{copy.lockedNewLine}</Button>
    </div>
  </div>;
}

export function FaceTrackingCard({ port, run, disabled, onStarted, onRestore, onNewLine, onUnityHolds }: {
  port: AmfProductionPort; run: AmfProductionRun; disabled: boolean;
  onStarted: (taskId: string) => void; onRestore: (() => void) | null; onNewLine: () => void;
  /** Unity holds the project: the page offers saving and closing it. */
  onUnityHolds: (error: AmfProductionError) => void;
}) {
  const facts = run.result?.facts?.faceTracking;
  const locked = facts?.manualReplayRequired === true;
  const running = run.steps.some(step => step.key === "face_tracking" && step.state === "running");
  const [menu, setMenu] = useState<FaceTrackingMenuV01 | null>(null);
  const [plan, setPlan] = useState<FaceTrackingPlanV01 | null>(null);
  const [reviewing, setReviewing] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sourceName = (sourceId: string) => run.sources.find(source => source.sourceId === sourceId)?.itemName ?? sourceId;
  // The dialog opens only with the reviewed plan in hand; a refused review stays on the card.
  const review = async (choice: FaceTrackingMenuV01) => {
    setReviewing(choice.id); setError(null);
    try { setPlan(await port.faceTrackingPlan(run.runId, choice.id)); setMenu(choice); }
    catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setReviewing(null); }
  };
  const close = () => { setMenu(null); setPlan(null); setError(null); };
  const start = async () => {
    if (menu === null || plan === null || starting) return;
    setStarting(true); setError(null);
    try {
      const accepted = await port.faceTrackingPatch(run.runId, menu.id, plan.planDigest, true, copy.task);
      close(); onStarted(accepted.taskId);
    } catch (reason) {
      if (reason instanceof AmfProductionError && reason.suggestedAction === "amf.production.closeEditor") { close(); onUnityHolds(reason); return; }
      setError(reason instanceof Error ? reason.message : String(reason));
    }
    finally { setStarting(false); }
  };
  const hint = error === null ? null : ftFailure(error);
  return <Card className="vua-ft">
    <h2>{copy.title}</h2>
    {running && <p role="status" className="vua-n3__notice">{copy.runningHint}</p>}
    {facts === undefined ? <p className="vua-text-secondary">{copy.notChecked}</p> : <>
      <p><strong>{copy.capability[ftCapability(facts)]}</strong></p>
      {facts.truncated && <p className="vua-caption vua-text-secondary">{copy.partial}</p>}
      {locked && <FtLockNote onRestore={onRestore} onNewLine={onNewLine} disabled={disabled} />}
      {facts.patchMenus.length === 0
        ? (!locked && <p className="vua-text-secondary">{copy.noMenus}</p>)
        : <>
          <h3>{copy.menusTitle}</h3>
          <ul className="vua-ft__menus">{facts.patchMenus.map(item => <li key={item.id}>
            <button type="button" className="vua-ft__menu" disabled={disabled || reviewing !== null || starting}
              onClick={() => { void review(item); }} data-nav-id={`workshop-ft-menu-${item.id.slice(0, 8)}`}>
              <strong>{item.menuPath}</strong>
              <small>{sourceName(item.sourceId)}{reviewing === item.id ? ` · ${copy.reviewing}` : ""}</small>
            </button>
          </li>)}</ul>
        </>}
      {error !== null && <div role="alert">
        <p>{error}</p>
        {hint !== null && <p className="vua-caption vua-text-secondary">{copy.hint[hint]}</p>}
      </div>}
      <details className="vua-n3__advanced"><summary>{strings.factory.details}</summary>
        <ul className="vua-ft__meshes">{facts.meshes.map(mesh => <li key={mesh.renderer}>{format(copy.meshLine, {
          renderer: mesh.renderer || "—",
          arkit: profileCount(mesh, "arkit52-v1"), unified: profileCount(mesh, "unified-core12-v1"), sranipal: profileCount(mesh, "sranipal-core12-v1"),
        })}</li>)}</ul>
        <p className="vua-caption vua-text-secondary">{format(copy.parameters, { count: facts.parameters.length })}</p>
        {facts.components.length > 0 && <p className="vua-caption vua-text-secondary">{format(copy.componentsLine, { names: facts.components.join(", ") })}</p>}
        <p className="vua-caption vua-text-secondary">{format(copy.observedAt, { time: formatDateMinute(facts.observedAt) })}</p>
        <p className="vua-caption vua-text-secondary">{copy.inGame}</p>
      </details>
    </>}
    <ConfirmDialog open={menu !== null} title={copy.confirmTitle} cancelLabel={copy.cancel} confirmLabel={copy.confirm}
      onCancel={close} onConfirm={() => { void start(); }}>
      <ul className="vua-ft__confirm">
        {menu !== null && <li>{format(copy.confirmTrust, { menu: menu.menuPath, script: menu.scriptPath })}</li>}
        <li>{copy.confirmRestore}</li>
        <li>{copy.confirmLock}</li>
      </ul>
      {plan !== null && <p className="vua-caption vua-text-secondary">{plan.warning}</p>}
    </ConfirmDialog>
  </Card>;
}
