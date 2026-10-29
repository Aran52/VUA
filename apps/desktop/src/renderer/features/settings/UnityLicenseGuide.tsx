import { useEffect, useRef, useState } from "react";
import { strings, format, termLabel } from "../../i18n/index.ts";
import { formatDateMinute } from "../../i18n/date-time.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { openExternalUrl } from "../../app/open-external.ts";
import { useGateway } from "../../gateway/index.ts";
import { ACCOUNT_GUIDE_URLS, type UnityLicenseSnapshotV01 } from "@vua/contracts";
import { licenseStep } from "./unity-license-step.ts";
import { requestLicenseStep } from "../deployer/license-step-navigation.ts";
const copy = strings.unityLicense;
const status = copy.status;
const text = (value: string) => format(value, { amf: termLabel("amf") });
/** A check that has not reported after this long is treated as stopped. */
const CHECK_WAIT_MS = 7 * 60_000;

/** The license as VUA sees it on this computer, with a check that opens Unity once. */
export function UnityLicenseStatus({ onInstall, onReturn, showGuide = false, disabled = false, installUnavailable = false }: {
  onInstall?: (() => void) | undefined; onReturn?: (() => void) | undefined; showGuide?: boolean; disabled?: boolean;
  installUnavailable?: boolean;
}) {
  const gateway = useGateway();
  const port = gateway.environment.unityLicense;
  const amf = gateway.amfProduction;
  const [snapshot, setSnapshot] = useState<UnityLicenseSnapshotV01 | null>(null);
  const [editor, setEditor] = useState<string | null | undefined>(amf ? undefined : null);
  const [checking, setChecking] = useState<{ readonly since: string | null; readonly at: number } | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [guideOpen, setGuideOpen] = useState(showGuide);
  const [accountOpened, setAccountOpened] = useState(false);
  const submitted = useRef(false);
  useEffect(() => {
    let alive = true;
    const refresh = () => {
      if (port) void port.snapshot().then(next => { if (alive) { setSnapshot(next); setFailed(null); } }, () => { if (alive) { setSnapshot(null); setFailed(status.readFailed); } });
      else { setSnapshot(null); setFailed(status.readFailed); }
      if (amf) void amf.list().then(list => { if (alive) setEditor(list.editors[0]?.path ?? null); }, () => { if (alive) setEditor(null); });
      else setEditor(null);
    };
    refresh(); window.addEventListener("focus", refresh);
    return () => { alive = false; window.removeEventListener("focus", refresh); };
  }, [port, amf]);
  // While a check runs, read again until its result arrives.
  useEffect(() => {
    if (checking === null || !port) return;
    let alive = true;
    const timer = setInterval(() => {
      if (Date.now() - checking.at > CHECK_WAIT_MS) { submitted.current = false; setChecking(null); setFailed(status.checkStopped); return; }
      void port.snapshot().then(next => {
        if (!alive) return;
        setSnapshot(next);
        if (next.lastCheck && next.lastCheck.checkedAt !== checking.since) { submitted.current = false; setChecking(null); }
      }, () => { /* The bounded wait still applies if a read fails. */ });
    }, 2000);
    return () => { alive = false; clearInterval(timer); };
  }, [checking, port]);
  const line = licenseStep(snapshot, editor === undefined ? null : editor !== null, checking !== null, accountOpened);
  const last = snapshot?.lastCheck;
  const when = { date: last ? formatDateMinute(last.checkedAt) : "", version: last?.editorVersion ?? "", amf: termLabel("amf") };
  const start = async () => {
    if (!port || !editor || submitted.current || disabled) return;
    submitted.current = true; setFailed(null); setAccountOpened(false);
    setChecking({ since: snapshot?.lastCheck?.checkedAt ?? null, at: Date.now() });
    try {
      await port.check(editor, status.taskTitle);
    } catch (error) { submitted.current = false; setChecking(null); setFailed(format(status.checkFailed, { reason: error instanceof Error ? error.message : "" })); }
  };
  const openAccount = async () => {
    setGuideOpen(true); setFailed(null);
    try {
      if (await openExternalUrl(ACCOUNT_GUIDE_URLS.unity)) setAccountOpened(true);
      else setFailed(status.accountFailed);
    } catch { setFailed(status.accountFailed); }
  };
  const install = onInstall ?? (() => { requestLicenseStep(); window.location.hash = "env-create"; });
  return <div className="vua-unity-license-status" data-state={line.state} role="status">
    <p className="vua-unity-license-status__title">{status[line.title]}</p>
    <p>{last?.state === "unknown" && line.state === "hint" ? `${format(status.lastUnknown, when)} ` : ""}{format(status[line.detail], when)}</p>
    {line.action === "check" || checking ? <Button variant="primary" disabled={disabled || checking !== null || !port} onClick={() => { void start(); }}>{checking ? status.checking : status.checkNow}</Button> : null}
    {line.action === "account" ? <Button variant="primary" onClick={() => { void openAccount(); }}>{status.openAccount}</Button> : null}
    {line.action === "install" ? <Button variant="primary" disabled={disabled || installUnavailable} onClick={install}>{status.install}</Button> : null}
    {onReturn ? <Button variant={line.state === "active" ? "primary" : "subtle"} onClick={onReturn}>{copy.returnToWorkshop}</Button> : null}
    {failed ? <p role="alert">{failed}</p> : null}
    <details open={guideOpen} onToggle={event => setGuideOpen(event.currentTarget.open)}><summary>{copy.official}</summary><UnityLicenseInstructions /></details>
  </div>;
}

function UnityLicenseInstructions() {
  return <div className="vua-unity-license-guide">
    <p>{copy.intro}</p>
    <ol>{[copy.register, copy.hub, copy.activate, copy.locate, copy.resume].map((paragraph, i) => <li key={i}><p>{text(paragraph)}</p></li>)}</ol>
    <p>{text(copy.expected)}</p>
    <div className="vua-ai-tools__controls">
      <a href="https://unity.com/download" onClick={event => { event.preventDefault(); void openExternalUrl(event.currentTarget.href); }}>{copy.download}</a>
      <a href="https://unity.com/pages/license-compliance" onClick={event => { event.preventDefault(); void openExternalUrl(event.currentTarget.href); }}>{copy.policy}</a>
      <a href="https://docs.unity3d.com/2022.3/Documentation/Manual/LicenseActivationMethods.html" onClick={event => { event.preventDefault(); void openExternalUrl(event.currentTarget.href); }}>{copy.official}</a>
    </div>
  </div>;
}

export function UnityLicenseGuide() { return <UnityLicenseStatus showGuide />; }
