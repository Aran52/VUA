import { useCallback, useEffect, useState } from "react";
import { strings, format, termLabel } from "../../i18n/index.ts";
import { formatDateMinute } from "../../i18n/date-time.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { openExternalUrl } from "../../app/open-external.ts";
import { useGateway } from "../../gateway/index.ts";
import { ACCOUNT_GUIDE_URLS, type UnityLicenseSnapshotV01 } from "@vua/contracts";
const copy = strings.unityLicense;
const status = copy.status;
const text = (value: string) => format(value, { amf: termLabel("amf") });
/** A check that has not reported after this long is treated as stopped. */
const CHECK_WAIT_MS = 7 * 60_000;

/** The line VUA shows for the license: what it observed, and the one next step. */
function statusLine(snapshot: UnityLicenseSnapshotV01): { readonly state: string; readonly title: string; readonly detail: string } {
  const last = snapshot.lastCheck;
  const when = last ? { date: formatDateMinute(last.checkedAt), version: last.editorVersion } : { date: "", version: "" };
  if (last?.state === "active" && snapshot.nextAction === "none") return { state: "active", title: status.active, detail: format(status.activeDetail, when) };
  if (last?.state === "not_activated" && snapshot.nextAction === "sign_in_and_activate") return { state: "attention", title: status.notActivated, detail: format(status.notActivatedDetail, when) };
  const lastUnknown = last?.state === "unknown" ? `${format(status.lastUnknown, when)} ` : "";
  switch (snapshot.state) {
    case "needs_hub_client": return { state: "attention", title: status.needsHub, detail: lastUnknown + status.needsHubDetail };
    case "not_found": return { state: "attention", title: status.notFound, detail: lastUnknown + status.notFoundDetail };
    case "likely_active": return { state: "hint", title: status.likelyActive, detail: lastUnknown + status.checkDetail };
    default: return { state: "hint", title: status.unknown, detail: lastUnknown + status.checkDetail };
  }
}

/** The license as VUA sees it on this computer, with a check that opens Unity once. */
function UnityLicenseStatus() {
  const gateway = useGateway();
  const port = gateway.environment.unityLicense;
  const amf = gateway.amfProduction;
  const [snapshot, setSnapshot] = useState<UnityLicenseSnapshotV01 | null>(null);
  const [editor, setEditor] = useState<string | null>(null);
  const [checking, setChecking] = useState<{ readonly since: string | null; readonly at: number } | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const read = useCallback(async () => {
    if (!port) return null;
    try { const next = await port.snapshot(); setSnapshot(next); return next; } catch { return null; }
  }, [port]);
  useEffect(() => { void read(); }, [read]);
  useEffect(() => {
    if (!amf) return;
    void amf.list().then(list => setEditor(list.editors[0]?.path ?? null), () => setEditor(null));
  }, [amf]);
  // While a check runs, read again until its result arrives.
  useEffect(() => {
    if (checking === null) return;
    const timer = setInterval(() => {
      void read().then(next => {
        if (next?.lastCheck && next.lastCheck.checkedAt !== checking.since) setChecking(null);
        else if (Date.now() - checking.at > CHECK_WAIT_MS) { setChecking(null); setFailed(status.checkStopped); }
      });
    }, 2000);
    return () => clearInterval(timer);
  }, [checking, read]);
  if (!port || snapshot === null) return null;
  const line = statusLine(snapshot);
  const start = async () => {
    if (!port || !editor || checking) return;
    setFailed(null);
    try {
      await port.check(editor, status.taskTitle);
      setChecking({ since: snapshot.lastCheck?.checkedAt ?? null, at: Date.now() });
    } catch (error) { setFailed(format(status.checkFailed, { reason: error instanceof Error ? error.message : "" })); }
  };
  return <div className="vua-unity-license-status" data-state={line.state} role="status">
    <p className="vua-unity-license-status__title">{line.title}</p>
    <p>{line.detail}</p>
    {editor && amf
      ? <Button variant={line.state === "active" ? "subtle" : "primary"} disabled={checking !== null} onClick={() => { void start(); }}>{checking ? status.checking : status.checkNow}</Button>
      : <p className="vua-caption vua-text-secondary">{text(status.needsEditor)}</p>}
    {failed ? <p role="alert">{failed}</p> : null}
  </div>;
}

export function UnityLicenseGuide() {
  return <div className="vua-unity-license-guide">
    <UnityLicenseStatus />
    <p>{copy.intro}</p>
    <ol>{[copy.register, copy.hub, copy.activate, copy.locate, copy.resume].map((paragraph, i) => <li key={i}><p>{text(paragraph)}</p></li>)}</ol>
    <p>{text(copy.expected)}</p>
    <div className="vua-ai-tools__controls">
      <Button onClick={() => void openExternalUrl(ACCOUNT_GUIDE_URLS.unity)}>{copy.account}</Button>
      <Button onClick={() => void openExternalUrl("https://unity.com/download")}>{copy.download}</Button>
      <Button variant="subtle" onClick={() => void openExternalUrl("https://unity.com/pages/license-compliance")}>{copy.policy}</Button>
      <Button variant="subtle" onClick={() => void openExternalUrl("https://docs.unity3d.com/2022.3/Documentation/Manual/LicenseActivationMethods.html")}>{copy.official}</Button>
    </div>
  </div>;
}
