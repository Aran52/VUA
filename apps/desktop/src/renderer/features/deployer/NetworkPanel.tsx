/** Network latency and website tests. Preferences belong to the renderer; observations come
 * only from the typed Gateway. Individual cards never clear unrelated results. */
import { useEffect, useRef, useState, type FormEvent } from "react";
import { createPortal } from "react-dom";
import { Icon } from "@vua/design-system";
import { WEBSITE_TEST_LIMIT, type WebsiteObservation } from "@vua/contracts";
import { useGateway } from "../../gateway/index.ts";
import { format, strings } from "../../i18n/index.ts";
import { storageKeys } from "../../app/storage-keys.ts";
import { Card } from "../../components/primitives/Card.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { ContentDialog } from "../../components/primitives/ContentDialog.tsx";
import { DEFAULT_TEST_WEBSITES, normalizeWebsiteUrl, parseTestWebsites, type TestWebsite } from "./website-model.ts";
import { brandGlyphPath } from "./website-glyphs.ts";
import { REGIONAL_REFERENCES } from "./regional-reference.ts";
import { RegionalTierList, closestRegion, regionalMeasuredViaProxy } from "./RegionalTiers.tsx";
import "./website-tests.css";

const copy = strings.websiteTests;
type CardState = { readonly busy: true } | { readonly busy: false; readonly result?: WebsiteObservation };
const regionSites: readonly TestWebsite[] = REGIONAL_REFERENCES.map(region => ({ name: copy.regionCities[region.id], url: region.url }));

/** 品牌站用 Simple Icons 官方字形(Simple Icons CC0),其余回落中性 globe;
 *  全部内联,不发远程 favicon 请求(测试前不暴露浏览行踪)。 */
function WebsiteGlyph({ url }: { url: string }) {
  const host = new URL(url).hostname;
  const brand = brandGlyphPath(host);
  return brand ? (
    <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d={brand} /></svg>
  ) : (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
      <circle cx="12" cy="12" r="8.5" />
      <ellipse cx="12" cy="12" rx="3.75" ry="8.5" />
      <path d="M3.5 12h17" />
    </svg>
  );
}

function useNetworkTests() {
  const port = useGateway().environment.network;
  const [ready, setReady] = useState<boolean | null>(null);
  const [sites, setSites] = useState<readonly TestWebsite[]>(() => {
    try { return parseTestWebsites(localStorage.getItem(storageKeys.testWebsites)); } catch { return DEFAULT_TEST_WEBSITES; }
  });
  const [states, setStates] = useState<Record<string, CardState>>({});
  const [referenceStates, setReferenceStates] = useState<Record<string, CardState>>({});
  const pending = useRef(new Set<string>());
  const referencePending = useRef(new Set<string>());
  const generation = useRef(0);
  const [editor, setEditor] = useState<{ original: string | null; name: string; url: string } | null>(null);
  const [formError, setFormError] = useState(false);
  const [storageFailed, setStorageFailed] = useState(false);

  useEffect(() => {
    const ticket = ++generation.current;
    pending.current.clear(); referencePending.current.clear(); setStates({}); setReferenceStates({}); setReady(null);
    if (port) void port.websiteCapability().then(c => { if (generation.current === ticket) setReady(c.state === "ready"); }, () => { if (generation.current === ticket) setReady(false); });
    else setReady(false);
    return () => { generation.current++; };
  }, [port]);

  const saveSites = (next: readonly TestWebsite[]) => {
    setSites(next);
    try { localStorage.setItem(storageKeys.testWebsites, JSON.stringify(next)); setStorageFailed(false); }
    catch { setStorageFailed(true); }
  };
  const test = async (selected: readonly TestWebsite[], regional = false) => {
    if (!port || !ready) return;
    const requests = regional ? referencePending.current : pending.current;
    const update = regional ? setReferenceStates : setStates;
    const urls = selected.map(s => s.url).filter(url => !requests.has(url));
    if (!urls.length) return;
    const ticket = generation.current;
    urls.forEach(url => requests.add(url));
    update(previous => ({ ...previous, ...Object.fromEntries(urls.map(url => [url, { busy: true }])) }));
    try {
      const results = await port.testWebsites(urls);
      if (ticket === generation.current) update(previous => ({ ...previous,
        ...Object.fromEntries(results.map(result => [result.url, { busy: false, result }])) }));
    } catch {
      if (ticket === generation.current) update(previous => ({ ...previous,
        ...Object.fromEntries(urls.map(url => [url, { busy: false }])) }));
    } finally { if (ticket === generation.current) urls.forEach(url => requests.delete(url)); }
  };
  const saveEditor = (event: FormEvent) => {
    event.preventDefault();
    if (!editor) return;
    const url = normalizeWebsiteUrl(editor.url);
    const name = editor.name.trim();
    if (!url || !name || name.length > 40 || sites.some(s => s.url !== editor.original && s.url === url)) { setFormError(true); return; }
    const next = { name, url };
    saveSites(editor.original ? sites.map(s => s.url === editor.original ? next : s) : [...sites, next]);
    setStates(previous => { const next = { ...previous }; if (editor.original) delete next[editor.original]; return next; });
    setEditor(null);
  };
  return { port, ready, sites, states, referenceStates, editor, setEditor, formError, setFormError, storageFailed, saveSites, test, saveEditor };
}
type NetworkTests = ReturnType<typeof useNetworkTests>;

function WebsiteTests({ tests }: { tests: NetworkTests }) {
  const { ready, sites, states, setEditor, setFormError, storageFailed, test } = tests;
  const busy = sites.some(site => states[site.url]?.busy);
  return <>
    <header className="vua-network__header">
      <h3>{copy.title}</h3>
      <div className="vua-network__toolbar">
        <button type="button" className="vua-network__icon-button" title={copy.testAll} aria-label={copy.testAll}
          disabled={!ready || busy || !sites.length} onClick={() => { void test(sites); }}><Icon name="refresh" size={20} /></button>
        <button type="button" className="vua-network__icon-button" title={copy.add} aria-label={copy.add}
          disabled={sites.length >= WEBSITE_TEST_LIMIT} onClick={() => { setFormError(false); setEditor({ original: null, name: "", url: "" }); }}><Icon name="add" size={20} /></button>
      </div>
    </header>
    <ul className="vua-network__cards">{sites.map(site => {
      const state = states[site.url];
      const result = state && !state.busy ? state.result : undefined;
      const label = state?.busy ? copy.testing : result ? result.status === "reachable"
        ? format(copy.milliseconds, { ms: result.elapsedMs })
        : result.httpStatus !== null ? format(copy.httpStatus, { code: result.httpStatus }) : copy.statuses[result.status]
        : state ? copy.statuses.probe_error : copy.test;
      return <li className="vua-network__site" key={site.url}>
        <button type="button" className="vua-network__edit" title={copy.edit} aria-label={format(copy.editSite, { name: site.name })}
          disabled={state?.busy} onClick={() => { setFormError(false); setEditor({ original: site.url, ...site }); }}>⋯</button>
        <span className="vua-network__site-icon"><WebsiteGlyph url={site.url} /></span><strong title={site.url}>{site.name}</strong>
        <button type="button" className="vua-network__test" disabled={!ready || state?.busy} data-result={result?.status}
          aria-label={format(copy.testSite, { name: site.name })}
          title={result ? format(copy.resultDetail, { status: copy.statuses[result.status], ms: result.elapsedMs }) : site.url}
          onClick={() => { void test([site]); }}><span role="status">{label}</span></button>
      </li>;
    })}</ul>
    {!sites.length ? <p className="vua-network__note">{copy.empty}</p> : null}
    {ready === false ? <p className="vua-network__note" role="status">{copy.unavailable}</p> : null}
    {storageFailed ? <p className="vua-network__note" role="status">{copy.storageFailed}</p> : null}
  </>;
}

function WebsiteEditor({ tests }: { tests: NetworkTests }) {
  const { editor, setEditor, formError, sites, saveSites, saveEditor } = tests;
  return <ContentDialog open={editor !== null} title={editor?.original ? copy.edit : copy.add} closeLabel={copy.cancel} onClose={() => setEditor(null)}>
    {editor ? <form className="vua-network__form" onSubmit={saveEditor}>
      <label>{copy.name}<input autoFocus required maxLength={40} value={editor.name} onChange={e => setEditor({ ...editor, name: e.target.value })} /></label>
      <label>{copy.url}<input required maxLength={2048} placeholder="https://" value={editor.url} onChange={e => setEditor({ ...editor, url: e.target.value })} /></label>
      {formError ? <p role="alert">{copy.invalid}</p> : null}
      <div className="vua-network__form-actions">
        {editor.original ? <Button variant="subtle" onClick={() => { saveSites(sites.filter(s => s.url !== editor.original)); setEditor(null); }}>{copy.remove}</Button> : null}
        <Button variant="subtle" onClick={() => setEditor(null)}>{copy.cancel}</Button><Button variant="primary" type="submit">{copy.save}</Button>
      </div>
    </form> : null}
  </ContentDialog>;
}

/** Website tests as a page section. */
export function NetworkPanel() {
  const tests = useNetworkTests();
  if (!tests.port) return null;
  return <>
    <Card className="vua-network"><WebsiteTests tests={tests} /></Card>
    <WebsiteEditor tests={tests} />
  </>;
}

/** Latency in the top bar: a player can check it from any page, and opening it runs the regional test
 * once. Website tests sit one level down, for when a specific site will not load. */
export function NetworkMonitor() {
  const tests = useNetworkTests();
  const { ready, referenceStates, test } = tests;
  const [open, setOpen] = useState(false);
  const toggleRef = useRef<HTMLButtonElement | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const tested = useRef(false);
  useEffect(() => {
    if (!open || !ready || tested.current) return;
    tested.current = true;
    void test(regionSites, true);
  }, [open, ready]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (event.target instanceof Node && (panelRef.current?.contains(event.target) || toggleRef.current?.contains(event.target))) return;
      // The website editor layers above the panel and keeps it open.
      if (event.target instanceof Element && event.target.closest("[role=dialog]")) return;
      setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => { if (event.key === "Escape" && !document.querySelector("[role=dialog]")) { event.stopPropagation(); setOpen(false); } };
    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown, true);
    return () => { window.removeEventListener("pointerdown", onPointerDown, true); window.removeEventListener("keydown", onKeyDown, true); };
  }, [open]);

  const busy = REGIONAL_REFERENCES.some(region => referenceStates[region.url]?.busy);
  const viaProxy = regionalMeasuredViaProxy(referenceStates);
  const closest = closestRegion(referenceStates);
  const value = busy ? copy.testing : closest ? format(copy.monitorClosest, { region: copy.regionShort[closest.id], ms: closest.ms }) : copy.monitorLabel;
  return <>
    <button ref={toggleRef} type="button" className="vua-shell__usage vua-shell__network vua-caption" aria-expanded={open}
      aria-label={copy.regionTitle} title={copy.regionTitle} data-nav-id="shell-network" onClick={() => setOpen(current => !current)}>
      <Icon name="signal" size={16} />
      <span className="vua-shell__network-value">{value}</span>
    </button>
    {open ? createPortal(<div ref={panelRef} className="vua-usage-panel vua-network-panel" data-vua-native-occluder role="region" aria-label={copy.regionTitle} tabIndex={-1}>
      <div className="vua-usage-panel__header">
        <span className="vua-usage-panel__title">{copy.regionTitle}</span>
        <button type="button" className="vua-usage-panel__close" aria-label={strings.resourceMonitor.closeAria} onClick={() => setOpen(false)}><Icon name="close" size={16} /></button>
      </div>
      <RegionalTierList states={referenceStates} ready={ready} onTest={region => { void test([{ name: copy.regionCities[region.id], url: region.url }], true); }} />
      {viaProxy ? <p className="vua-network-panel__proxy">{copy.regionViaProxy}</p> : null}
      <Button data-nav-id="network-test" disabled={!ready || busy} onClick={() => { void test(regionSites, true); }}>
        {busy ? copy.testing : ready === false ? copy.regionUnavailable : copy.testRegions}
      </Button>
      <details className="vua-network-panel__sites" data-nav-id="network-websites">
        <summary>{copy.title}</summary>
        <div className="vua-network"><WebsiteTests tests={tests} /></div>
      </details>
    </div>, document.body) : null}
    <WebsiteEditor tests={tests} />
  </>;
}
