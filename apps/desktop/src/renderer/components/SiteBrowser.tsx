import { useEffect, useMemo, useSyncExternalStore } from "react";
import { Icon } from "@vua/design-system";
import { DESKTOP_BROWSER_SITES_V1, DESKTOP_BROWSER_TAB_LIMIT } from "@vua/contracts";
import { format, strings } from "../i18n/index.ts";
import { displayUrl } from "../features/import/import-model.ts";
import { Button } from "./primitives/Button.tsx";
import { BrowserFrame } from "./BrowserFrame.tsx";
import { siteBrowserSession, type BrowserPurpose, type BrowserTab } from "./site-browser-session.ts";
import "../features/import/import-page.css";

export function SiteBrowser({ purpose, onClose }: { purpose: BrowserPurpose; onClose: () => void }) {
  const session = useMemo(() => siteBrowserSession(window.vua?.desktopBrowser, purpose), [purpose]);
  const state = useSyncExternalStore(session.subscribe, session.getSnapshot);
  useEffect(() => { session.ensureInitial(); }, [session]);
  const copy = strings.browser;
  const nav = strings.importPage;
  const tab = state.tabs.find(entry => entry.id === state.activeId);
  const label = (entry: BrowserTab) => entry.title || DESKTOP_BROWSER_SITES_V1.find(site => site.id === entry.site)?.name
    || (entry.view?.url ? displayUrl(entry.view.url) : copy.newTab);
  const closeTab = (id: string) => { session.close(id); if (state.tabs.length === 1) onClose(); };
  const error = <div className="vua-browser-frame__error" role="status"><p>{tab?.failure === "unavailable" ? copy.unavailable : copy.failed}</p>
    <Button disabled={tab?.loading} onClick={() => tab && session.action(tab.id, "reload")}>{copy.retry}</Button>
    <Button variant="subtle" onClick={() => tab && closeTab(tab.id)}>{nav.navClose}</Button></div>;
  if (!tab) return <Button variant="primary" data-browser-reopen onClick={() => session.open()}>{purpose === "assets" ? copy.reopen : copy.wiki}</Button>;
  const status = !tab.view ? tab.failure !== null ? error : <p role="status">{copy.loading}</p> : tab.failedPage ? error : undefined;
  return <BrowserFrame key={tab.id} viewId={tab.view?.viewId ?? null} hideContent={tab.failedPage || !tab.view} status={status}>
    <div className="vua-browser-frame__tabs" role="tablist" aria-label={copy.tabs}>
      {state.tabs.map(entry => <div key={entry.id} className="vua-browser-frame__tab" data-active={entry.id === state.activeId ? "true" : undefined}>
        <button type="button" role="tab" aria-selected={entry.id === state.activeId} data-browser-tab={entry.id} title={label(entry)} onClick={() => session.activate(entry.id)}>
          <span>{label(entry)}</span>{entry.loading ? <Icon name="refresh" size={16} /> : null}
        </button>
        <button type="button" data-browser-close-tab={entry.id} className="vua-browser-frame__tab-close" aria-label={format(copy.closeTab, { title: label(entry) })} title={nav.navClose} onClick={() => closeTab(entry.id)}><Icon name="close" size={16} /></button>
      </div>)}
      <button type="button" className="vua-browser-frame__new-tab" data-browser-new-tab aria-label={copy.newTab} title={copy.newTab}
        disabled={state.tabs.length >= DESKTOP_BROWSER_TAB_LIMIT} onClick={() => session.open()}><Icon name="add" size={16} /></button>
    </div>
    <div className="vua-browser-frame__bar" role="toolbar" aria-label={nav.navBarAria}>
      {(["goBack", "goForward", "reload"] as const).map((command, index) => <button key={command} type="button" className="vua-import__browse-button"
        aria-label={[nav.navBack, nav.navForward, nav.navReload][index]} title={[nav.navBack, nav.navForward, nav.navReload][index]}
        disabled={command === "goBack" ? !tab.view?.canGoBack : command === "goForward" ? !tab.view?.canGoForward : tab.loading} onClick={() => session.action(tab.id, command)}>
        <Icon name={command === "goBack" ? "arrow-left" : command === "goForward" ? "arrow-right" : "refresh"} size={16} />
      </button>)}
      <span className="vua-import__browse-url" title={tab.view?.url}>{tab.view ? displayUrl(tab.view.url) : ""}</span>
      {tab.loading ? <span className="vua-browser-frame__status" role="status">{copy.loading}</span> : null}
      <button type="button" className="vua-import__browse-button" aria-label={nav.navClose} title={nav.navClose} onClick={() => closeTab(tab.id)}><Icon name="close" size={16} /></button>
    </div>
    {purpose === "assets" ? <nav className="vua-browser-frame__shortcuts" aria-label={copy.shortcuts}>
      {DESKTOP_BROWSER_SITES_V1.filter(entry => entry.purpose === "assets").map(entry => <button type="button" key={entry.id} data-browser-site={entry.id}
        aria-current={tab.site === entry.id ? "true" : undefined} title={entry.url} onClick={event => {
          if (event.ctrlKey || event.metaKey) session.open(entry.id); else session.navigate(tab.id, entry.id);
        }} onAuxClick={event => { if (event.button === 1) { event.preventDefault(); session.open(entry.id); } }}>{entry.name}</button>)}
    </nav> : null}
    {state.limitReached ? <p className="vua-browser-frame__status" role="status">{copy.tabLimit}</p> : null}
    {tab.view && tab.failure !== null ? <div className="vua-browser-frame__status" role="status">{copy.failed} <Button variant="subtle" disabled={tab.loading} onClick={() => session.action(tab.id, "reload")}>{copy.retry}</Button></div> : null}
  </BrowserFrame>;
}
