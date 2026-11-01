import { lazy, useEffect, useLayoutEffect, useRef, useState, type ComponentProps } from "react";
import type { AmfUiGenerationV01, AmfUiRuntimeSnapshotV01 } from "@vua/contracts";
import { useAmfModule, useGateway } from "../../gateway/index.ts";
import { format, strings, TERMS } from "../../i18n/index.ts";
import { AmfBoundary } from "./AmfBoundary.tsx";
import { canUnloadAmfUi, disposeAmfUi, loadAmfPages, splitAmfUi } from "./amf-ui-loader.ts";

const pages = new Map<string, ReturnType<typeof lazy<typeof import("../amf/AmfPages.tsx")["default"]>>>();
export function useAmfUiRuntime() {
  const port = useGateway().amfUiRuntime;
  const [view, setView] = useState<AmfUiRuntimeSnapshotV01>({ schemaVersion: "0.1", available: false, phase: "idle" });
  const [loaded, setLoaded] = useState(false);
  useEffect(() => {
    let mounted = true;
    const unsubscribe = port?.subscribe(value => { if (mounted) { setView(value); setLoaded(true); } });
    void port?.snapshot().then(value => { if (mounted) { setView(value); setLoaded(true); } }).catch(() => { if (mounted) setLoaded(true); });
    if (!port) setLoaded(true);
    return () => { mounted = false; unsubscribe?.(); };
  }, [port]);
  return { port, view, loaded };
}
/** Always mounted in the shell, including when no AMF page is visible. */
export function AmfUiRuntimeBridge() {
  const { port, view } = useAmfUiRuntime();
  const amf = useAmfModule();
  const previous = useRef<AmfUiGenerationV01 | undefined>(undefined);
  const candidate = useRef<AmfUiGenerationV01 | undefined>(undefined);
  useEffect(() => {
    if (amf.state !== "ready" || splitAmfUi()) return;
    // Load the optional UI while the shell is idle, before the first station
    // visit. This mounts no page and starts no library/project reads.
    void loadAmfPages().catch(() => { /* The page boundary reports a failed load on entry. */ });
  }, [amf.state]);
  useLayoutEffect(() => {
    if (view.candidate) candidate.current = view.candidate;
    if (view.phase !== "idle") return;
    if (previous.current && previous.current.instanceId !== view.active?.instanceId) {
      disposeAmfUi(previous.current); pages.delete(previous.current.instanceId);
    }
    if (candidate.current && candidate.current.instanceId !== view.active?.instanceId) disposeAmfUi(candidate.current);
    previous.current = view.active; candidate.current = undefined;
  }, [view]);
  useEffect(() => {
    if (!port || !view.transactionId || !view.candidate || (view.phase !== "quiescing" && view.phase !== "loading")) return;
    let mounted = true;
    const { transactionId, phase, candidate } = view;
    void (async () => {
      if (phase === "quiescing") {
        const admission = document.querySelector('[data-amf-surface], [aria-modal="true"]') ? "busy" : canUnloadAmfUi(view.active);
        if (mounted) await port.respond(transactionId, phase, admission === "ready", admission === "ready" ? undefined : admission);
      } else {
        try { await loadAmfPages(candidate, true); if (mounted) await port.respond(transactionId, phase, true); }
        catch { if (mounted) await port.respond(transactionId, phase, false, "failed"); }
      }
    })().catch(() => {});
    return () => { mounted = false; };
  }, [port, view]);
  return null;
}
export function AmfSurface(props: ComponentProps<typeof import("../amf/AmfPages.tsx")["default"]>) {
  const { port, view, loaded } = useAmfUiRuntime();
  const generation = view.active;
  if (splitAmfUi() && port && (!loaded || (view.phase !== "idle" && view.phase !== "quiescing"))) return <p role="status">{format(strings.amfModule.loading, { amf: TERMS.amf })}</p>;
  const key = splitAmfUi() ? generation?.instanceId ?? "legacy" : "builtin";
  let Pages = pages.get(key);
  if (!Pages) { Pages = lazy(() => loadAmfPages(generation)); pages.set(key, Pages); }
  return <div style={{ display: "contents" }} data-amf-surface={key}><AmfBoundary key={key} manage={() => props.navigate("settings-modules")}><Pages {...props} /></AmfBoundary></div>;
}
