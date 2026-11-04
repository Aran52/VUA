import { useEffect, useId, useRef, useState } from "react";
import type { DeploymentIntent, ScenarioConnection, ScenarioEnvironmentCheck, ScenarioSop } from "@vua/contracts";
import { useGateway } from "../../gateway/index.ts";
import { strings, format, termLabel } from "../../i18n/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { UnityLicenseStatus } from "../settings/UnityLicenseGuide.tsx";

const copy = strings.scenarioEnvironment;
export function ScenarioPreparation({ intent, checked, onChecked, autoCheck = false, disabled = false, onInstallUnity, focusLicense = false, onReturnToWorkshop, installUnavailable = false }: {
  intent: DeploymentIntent; checked: ScenarioEnvironmentCheck | null;
  onChecked: (check: ScenarioEnvironmentCheck | null) => void; autoCheck?: boolean; disabled?: boolean;
  onInstallUnity?: (() => void) | undefined; focusLicense?: boolean; onReturnToWorkshop?: (() => void) | undefined;
  installUnavailable?: boolean;
}) {
  const port = useGateway().environment.scenarios;
  const [sop, setSop] = useState<ScenarioSop | null>(null);
  const [sopOpen, setSopOpen] = useState(false);
  const [connection, setConnection] = useState<ScenarioConnection | "">("");
  const [scenario, setScenario] = useState(intent.purposes[0]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const generation = useRef(0), autoStarted = useRef(false);
  const licenseRef = useRef<HTMLElement>(null), licenseId = useId();
  const showLicense = intent.purposes.some(p => p === "pc_avatar" || p === "quest_avatar");
  const focusLicenseStep = () => { licenseRef.current?.scrollIntoView({ block: "center" }); licenseRef.current?.focus({ preventScroll: true }); };
  useEffect(() => { if (focusLicense && showLicense) focusLicenseStep(); }, [focusLicense, showLicense]);
  const identity = JSON.stringify(intent);
  const current = useRef(identity); current.current = identity;
  useEffect(() => {
    generation.current++; setSop(null); setSopOpen(false); setError(false);
    if (!intent.purposes.includes(scenario!)) setScenario(intent.purposes[0]);
  }, [identity]);
  useEffect(() => () => { generation.current++; }, []);
  const check = async () => {
    if (!port) return;
    const version = ++generation.current, key = identity;
    setBusy(true); setError(false); onChecked(null);
    try { const facts = await port.check(intent); if (version === generation.current && current.current === key) onChecked(facts); }
    catch { if (version === generation.current) setError(true); }
    finally { setBusy(false); }
  };
  // The panel keeps the result for its intent; a remount (for example when installation
  // availability arrives) reuses it instead of clearing it and checking again.
  useEffect(() => { if (autoCheck && port && !autoStarted.current && !checked && intent.purposes.length) { autoStarted.current = true; void check(); } }, [autoCheck, port]);
  const readSop = async () => {
    if (!port || !scenario) return;
    if (sopOpen) { setSopOpen(false); return; }
    const version = ++generation.current;
    setError(false); setBusy(true);
    try { const steps = await port.sop({ scenario, ...(scenario === "pico_pcvr" && connection ? { connection } : {}) });
      if (version === generation.current) { setSop(steps); setSopOpen(true); } }
    catch { if (version === generation.current) setError(true); }
    finally { setBusy(false); }
  };
  if (!port && !showLicense) return null;
  return <section className="vua-scenario-preparation" aria-label={copy.title}>
    {port ? <>
    <p>{copy.intro}</p>
    {intent.purposes.length > 1 ? <label>{copy.choose}<select value={scenario} onChange={e => { setScenario(e.target.value as typeof scenario); setSopOpen(false); }}>
      {intent.purposes.map(value => <option key={value} value={value}>{strings.deployment.purposes[value]}</option>)}
    </select></label> : null}
    {scenario === "pico_pcvr" ? <label>{copy.connection}<select value={connection} onChange={e => { setConnection(e.target.value as typeof connection); setSopOpen(false); }}>
      <option value="">{copy.unspecified}</option><option value="usb">{copy.usb}</option><option value="wifi">{copy.wifi}</option>
    </select></label> : null}
    <div className="vua-scenario-preparation__controls">
      <Button variant="subtle" disabled={busy || !scenario} onClick={() => { void readSop(); }}>{copy.sop}</Button>
      <Button disabled={busy || disabled || !intent.purposes.length} onClick={() => { void check(); }}>{busy ? copy.checking : copy.check}</Button>
    </div>
    {sop && sopOpen ? <ol>{sop.steps.map(step => {
      const localized = copy.steps[step.id as keyof typeof copy.steps];
      return <li key={step.id}><h3>{localized?.title ?? step.title}</h3><p>{format(localized?.explanation ?? step.explanation, { amf: termLabel("amf") })}</p><p className="vua-caption">{copy.next}: {localized?.result ?? step.expectedResult}</p>
        {step.id === "unity-license" ? <a href={`#${licenseId}`} onClick={event => { event.preventDefault(); focusLicenseStep(); }}>{copy.licenseStepLink}</a> : null}
      </li>;
    })}</ol> : null}
    {checked ? <div role="status" data-scenario-software-check><p>{copy.done}</p><ul>{checked.observations.map(fact => <li key={fact.component}>
      <strong>{strings.deployment.components[fact.component as keyof typeof strings.deployment.components] ?? fact.component}</strong>{" · "}{strings.deployment.reasons[fact.presence]}
      {fact.version ? ` (${fact.version})` : ""}
    </li>)}</ul><p>{checked.softwareReady ? copy.ready : copy.missing}</p></div> : null}
    {error ? <p role="alert">{copy.failed}</p> : null}
    </> : null}
    {showLicense ? <section id={licenseId} ref={licenseRef} tabIndex={-1} aria-label={strings.unityLicense.title} className="vua-scenario-license-step">
      <h3>{strings.unityLicense.title}</h3>
      <UnityLicenseStatus onInstall={onInstallUnity} disabled={disabled} onReturn={onReturnToWorkshop} installUnavailable={installUnavailable} />
    </section> : null}
  </section>;
}
