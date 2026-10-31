/** One route's play preparation as a single button that always names the next step:
 * check the environment, confirm the checklist, run the installation, retry after a failure, launch.
 * The Gateway owns checks, plans, tasks and sessions; this view only derives the next step from them. */
import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import { isTerminalTaskStateV01, type DeploymentIntent, type DeploymentPlan, type DeploymentProgress, type PicoInstallRegion, type PlayRoute, type PlaySession, type ScenarioConnection, type ScenarioEnvironmentCheck, type ScenarioSop, type TaskSnapshotV01 } from "@vua/contracts";
import { Icon, type IconName } from "@vua/design-system";
import { useGateway } from "../../gateway/index.ts";
import { currentLocale, strings } from "../../i18n/index.ts";
import { playFlowCopy } from "../../i18n/play-flow-copy.ts";
import { storageKeys } from "../../app/storage-keys.ts";
import { useUnityMirrors } from "../../app/unity-download-preference.ts";
import { openExternalUrl } from "../../app/open-external.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { readDeploymentReceipt, type DeploymentReceiptBookmark } from "../deployer/deployment-receipt.ts";
import { readDeploymentManualHandoff } from "../deployer/deployment-handoff.ts";
import type { DeploymentCardState } from "../deployer/DeploymentPanel.tsx";
import { ActionGlyph } from "./EnvironmentCard.tsx";
import { checklistSignature, playChecklist, playFlowPhase, progressPercent, readConfirmed, writeConfirmed, type PlayFlowPhase } from "./play-flow-model.ts";
import "./play-flow.css";

const EDITOR_ROOT = "C:\\Program Files\\Unity\\Hub\\Editor";
const DEFAULT_SOFTWARE: Record<PlayRoute, readonly string[]> = { desktop_play: ["steam", "vrchat"], pico_pcvr: ["steam", "pico_runtime", "steamvr", "vrchat"] };
/** The last check of each route in this app session, so returning to the tab shows it at once. */
const lastChecks = new Map<PlayRoute, ScenarioEnvironmentCheck>();

const read = (key: string) => { try { return localStorage.getItem(key); } catch { return null; } };
const write = (key: string, value: string | null) => { try { if (value === null) localStorage.removeItem(key); else localStorage.setItem(key, value); } catch { /* This visit keeps its choice. */ } };
const readConnection = (): ScenarioConnection => read(storageKeys.picoConnection) === "wifi" ? "wifi" : "usb";
const readRegion = (): PicoInstallRegion | "" => { const value = read(storageKeys.picoRegion); return value === "china_mainland" || value === "other" ? value : ""; };
const finished = (task: TaskSnapshotV01 | null) => task !== null && (isTerminalTaskStateV01(task.state) || task.recoveryDisposition === "inspect_required");

const PHASE_ICONS: Partial<Record<PlayFlowPhase, IconName>> = { check: "refresh", confirm: "square-check", execute: "download", handoff: "arrow-right", retry: "refresh", unavailable: "warning" };

export function PlayRouteFlow({ route, session, unavailable, launchPending, refreshToken, onLaunch, onAccounts, onStateChange }: {
  route: PlayRoute; session: PlaySession | null; unavailable: boolean; launchPending: boolean;
  /** Changes when the page asks every route to check its environment again. */
  refreshToken: number;
  onLaunch: (stop: boolean) => void; onAccounts: () => void; onStateChange: (state: DeploymentCardState) => void;
}) {
  const gateway = useGateway();
  const scenarios = gateway.environment.scenarios, deployment = gateway.environment.deployment;
  const copy = playFlowCopy[currentLocale];
  const useMirrors = useUnityMirrors();
  const receiptKey = `${storageKeys.deploymentReceiptPlay}.${route}`;
  const pico = route === "pico_pcvr";
  const [connection, setConnection] = useState<ScenarioConnection>(readConnection);
  const [region, setRegion] = useState<PicoInstallRegion | "">(readRegion);
  const [sop, setSop] = useState<ScenarioSop | null>(null);
  const [sopSettled, setSopSettled] = useState(false);
  const [check, setCheck] = useState<ScenarioEnvironmentCheck | null>(() => lastChecks.get(route) ?? null);
  const [checking, setChecking] = useState(false);
  const [checkFailed, setCheckFailed] = useState(false);
  const [confirmedRaw, setConfirmedRaw] = useState(() => read(storageKeys.playChecklist));
  const signature = checklistSignature(sop, pico ? connection : "");
  const confirmed = readConfirmed(confirmedRaw, route, signature);
  const [preparing, setPreparing] = useState(false);
  const [prepareError, setPrepareError] = useState<string | null>(null);
  const [plan, setPlan] = useState<DeploymentPlan | null>(null);
  // A receipt from an earlier visit (or the shared one older versions wrote) is followed only while its
  // task still runs; a finished one is history.
  const [taskId, setTaskId] = useState<string | null>(() => {
    const receipt = readDeploymentReceipt(read(receiptKey) ?? read(storageKeys.deploymentReceiptPlay));
    return receipt !== null && receipt.intent.purposes.includes(route) ? receipt.taskId : null;
  });
  const [task, setTask] = useState<TaskSnapshotV01 | null>(null);
  const [step, setStep] = useState<DeploymentProgress | null>(null);
  const startedHere = useRef(false), command = useRef<string | null>(null), handedOff = useRef(false);
  const taskRef = useRef(task); taskRef.current = task;

  const intent = useCallback((): DeploymentIntent => ({ purposes: [route], editorRoot: EDITOR_ROOT, useMirrors, ...(pico && region !== "" ? { picoRegion: region } : {}) }), [route, useMirrors, pico, region]);
  const taskActive = taskId !== null && !finished(task);

  const runCheck = useCallback(async () => {
    if (!scenarios) return;
    setChecking(true); setCheckFailed(false);
    try {
      const result = await scenarios.check(intent());
      lastChecks.set(route, result); setCheck(result); setPrepareError(null);
      // A fresh check supersedes a finished task's failure or handoff: the button follows the new facts.
      if (finished(taskRef.current)) { setTask(null); setTaskId(null); }
    } catch { setCheckFailed(true); }
    finally { setChecking(false); }
  }, [scenarios, intent, route]);

  // The first visit in an app session checks at once; a refresh from the page checks again.
  const firstCheck = useRef(true);
  useEffect(() => {
    if (firstCheck.current) { firstCheck.current = false; if (lastChecks.has(route)) return; }
    void runCheck();
  }, [refreshToken]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!scenarios) { setSopSettled(true); return; }
    let alive = true;
    setSopSettled(false);
    void scenarios.sop({ scenario: route, ...(pico ? { connection } : {}) })
      .then(value => { if (alive) setSop(value); }, () => { if (alive) setSop(null); })
      .finally(() => { if (alive) setSopSettled(true); });
    return () => { alive = false; };
  }, [scenarios, route, pico, connection]);

  useEffect(() => {
    if (!deployment || taskId === null) return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const unsubscribe = deployment.subscribe(taskId, setStep);
    const refresh = async () => {
      try {
        const snapshot = await deployment.status(taskId);
        if (!alive) return;
        const terminal = finished(snapshot);
        if (terminal && !startedHere.current) { setTaskId(null); setTask(null); return; }
        setTask(snapshot);
        if (!terminal) timer = setTimeout(() => { void refresh(); }, 1000);
        else {
          write(receiptKey, null);
          void gateway.environment.runCheck("play").catch(() => {});
          if (snapshot.state !== "failed" && snapshot.result?.outcome !== "manual_required") void runCheck();
        }
      } catch { if (alive) timer = setTimeout(() => { void refresh(); }, 3000); }
    };
    void refresh();
    return () => { alive = false; unsubscribe(); if (timer !== undefined) clearTimeout(timer); };
  }, [deployment, taskId]); // eslint-disable-line react-hooks/exhaustive-deps

  // After an official page the player comes back to VUA, and the check runs again then.
  useEffect(() => {
    const focus = () => { if (handedOff.current) { handedOff.current = false; void runCheck(); } };
    window.addEventListener("focus", focus);
    return () => window.removeEventListener("focus", focus);
  }, [runCheck]);

  const phase = playFlowPhase({ unavailable: unavailable || !scenarios, session, launchPending, checking: checking || !sopSettled, check, checkFailed, confirmed,
    preparing, prepareFailed: prepareError !== null, task, taskActive });
  useEffect(() => { onStateChange({ busy: preparing || taskActive, error: phase === "retry", ready: check?.softwareReady === true }); }, [preparing, taskActive, phase, check?.softwareReady, onStateChange]);

  const needsRegion = pico && check !== null && !check.softwareReady;
  const regionMissing = needsRegion && region === "";
  const execute = async (reuse: boolean) => {
    if (!deployment || regionMissing) return;
    setPreparing(true); setPrepareError(null);
    try {
      let current = reuse ? plan : null;
      if (current === null) { current = await deployment.plan(intent()); setPlan(current); command.current = null; }
      if (current.steps.some(s => s.action === "inspect")) { setPrepareError(strings.deployment.planFailed); setPlan(null); return; }
      // An uncertain reply keeps the command ID: retrying the same plan returns the same task.
      command.current ??= crypto.randomUUID();
      const id = await deployment.execute(current, command.current);
      const bookmark: DeploymentReceiptBookmark = { v: 1, taskId: id, intent: current.intent, startedAt: Date.now() };
      write(receiptKey, JSON.stringify(bookmark));
      startedHere.current = true; command.current = null; setPlan(null);
      setTask(null); setStep(null); setTaskId(id);
    } catch { setPrepareError(strings.deployment.executeFailed); }
    finally { setPreparing(false); }
  };
  const handoff = readDeploymentManualHandoff(task?.result);
  const act = () => {
    if (phase === "check") void runCheck();
    else if (phase === "confirm") { const raw = writeConfirmed(confirmedRaw, route, signature); write(storageKeys.playChecklist, raw); setConfirmedRaw(raw); }
    else if (phase === "execute") void execute(false);
    else if (phase === "retry") { if (checkFailed) void runCheck(); else void execute(task === null && plan !== null); }
    else if (phase === "handoff") {
      if (handoff) void openExternalUrl(handoff.officialUrl);
      handedOff.current = true; lastChecks.delete(route); setTask(null); setTaskId(null); setCheck(null);
    }
    else if (phase === "launch") onLaunch(false);
    else if (phase === "stop") onLaunch(true);
  };

  const scenario = strings.scenarioEnvironment;
  const componentName = (component: string) => strings.deployment.components[component as keyof typeof strings.deployment.components] ?? component;
  const percent = progressPercent(step);
  const items = playChecklist(sop, { check, confirmed, session });
  const observed = new Map((check?.observations ?? []).map(o => [o.component, o.presence === "verified"] as const));
  const software = (session?.software.map(s => s.component) ?? DEFAULT_SOFTWARE[route]).map(component => {
    const fact = session?.software.find(s => s.component === component);
    return { component, ready: observed.get(component) ?? fact?.presence === "verified", running: !unavailable && !!fact?.running, owned: !!fact?.owned };
  });
  const failure = task?.error?.code.replace(/^vua\.deployment\./, "");
  const notice = phase === "retry"
    ? checkFailed ? scenario.failed
      : prepareError ?? (failure !== undefined && Object.hasOwn(strings.deployment.installerErrors, failure) ? strings.deployment.installerErrors[failure as keyof typeof strings.deployment.installerErrors] : strings.deployment.failedHint)
    : phase === "running" && step ? `${componentName(step.component)} · ${strings.deployment.phases[step.phase]}`
    : phase === "handoff" && handoff ? componentName(handoff.component)
    : phase === "execute" && regionMissing ? copy.regionNeeded
    : session?.issue ? strings.environmentCards.issues[session.issue]
    : null;
  const label = phase === "running" && percent !== null ? `${copy.phases.running} ${percent}%` : copy.phases[phase];
  const spinning = phase === "checking" || phase === "running" || phase === "starting";
  const progress = phase === "running" && percent !== null ? { "--vua-flow-progress": `${percent}%` } as CSSProperties : undefined;

  return <div className="vua-play-flow" data-phase={phase} data-route={route}>
    <ul className="vua-play-flow__software" aria-label={strings.environmentCards.included}>
      {software.map(s => <li key={s.component} data-ready={s.ready} data-running={s.running || undefined}
        title={s.running ? s.owned ? strings.environmentCards.owned : strings.environmentCards.borrowed : undefined}>
        <span className="vua-play-flow__state" aria-hidden="true">{s.ready ? <Icon name="check" size={16} /> : null}</span>
        {componentName(s.component)}
        <span className="vua-play-flow__sr">{s.ready ? strings.environmentCards.installed : strings.environmentCards.missing}</span>
      </li>)}
    </ul>
    {pico ? <div className="vua-play-flow__options">
      <fieldset className="vua-segmented" disabled={spinning}><legend>{copy.connection}</legend>
        {(["usb", "wifi"] as const).map(value => <label key={value} data-checked={connection === value}>
          <input type="radio" name={`vua-${route}-connection`} checked={connection === value} onChange={() => { setConnection(value); write(storageKeys.picoConnection, value); }} />{scenario[value]}
        </label>)}
      </fieldset>
      {needsRegion ? <fieldset className="vua-segmented" disabled={spinning} data-required={regionMissing || undefined}><legend>{copy.region}</legend>
        {(["china_mainland", "other"] as const).map(value => <label key={value} data-checked={region === value}>
          <input type="radio" name={`vua-${route}-region`} checked={region === value} onChange={() => { setRegion(value); write(storageKeys.picoRegion, value); setPlan(null); }} />{strings.deployment.downloadRegions[value]}
        </label>)}
      </fieldset> : null}
    </div> : null}
    {items.length > 0 ? <ol className="vua-play-flow__checklist" aria-label={copy.checklist}>
      {items.map((item, index) => {
        const text = scenario.steps[item.id as keyof typeof scenario.steps];
        return <li key={item.id} data-done={item.done} data-automatic={item.automatic} title={text?.explanation}>
          <span className="vua-play-flow__mark" aria-hidden="true">{item.done ? <Icon name="check" size={16} /> : index + 1}</span>
          <span>{text?.title ?? item.id}</span>
        </li>;
      })}
    </ol> : null}
    {notice ? <p className="vua-play-flow__notice" role="status" data-tone={phase === "retry" ? "warning" : undefined}>{notice}</p> : null}
    <div className="vua-play-flow__actions">
      <button type="button" className="vua-flow-button" data-phase={phase} data-nav-id={`play-flow-${route}`} style={progress}
        disabled={phase === "unavailable" || spinning || (phase === "execute" && regionMissing)} aria-busy={spinning} onClick={act}>
        <span className="vua-flow-button__icon" aria-hidden="true">
          {spinning ? <span className="vua-environment-card__spin"><Icon name="refresh" size={20} /></span>
            : phase === "launch" ? <ActionGlyph action="start" /> : phase === "stop" ? <ActionGlyph action="stop" />
            : <Icon name={PHASE_ICONS[phase] ?? "arrow-right"} size={20} />}
        </span>
        <span className="vua-flow-button__label" key={phase}>{label}</span>
      </button>
      {phase === "running" && task !== null && taskActive && !task.cancellationRequested && deployment
        ? <Button variant="subtle" onClick={() => { void deployment.cancel(task.taskId, task.revision).catch(() => {}); }}>{copy.cancel}</Button> : null}
      <Button variant="subtle" className="vua-play-flow__accounts" data-nav-id={`play-accounts-${route}`} onClick={onAccounts}>{copy.accounts}</Button>
    </div>
  </div>;
}
