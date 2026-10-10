import { DESKTOP_BROWSER_SITES_V1, DESKTOP_BROWSER_TAB_LIMIT, type DesktopBrowserApiV1, type DesktopBrowserEventV1, type DesktopBrowserSiteIdV1, type RemoteContentViewStateV1 } from "@vua/contracts";

export type BrowserPurpose = "assets" | "knowledge";
export interface BrowserTab {
  readonly id: string;
  readonly view: RemoteContentViewStateV1 | null;
  readonly site: DesktopBrowserSiteIdV1 | null;
  readonly title: string;
  readonly loading: boolean;
  readonly failure: number | "unavailable" | null;
  readonly failedPage: boolean;
}
export interface BrowserSessionState {
  readonly tabs: readonly BrowserTab[];
  readonly activeId: string | null;
  readonly initialized: boolean;
  readonly limitReached: boolean;
}

function siteForUrl(value: string): DesktopBrowserSiteIdV1 | null {
  try {
    const host = new URL(value).hostname;
    return DESKTOP_BROWSER_SITES_V1.find(entry => {
      const expected = new URL(entry.url).hostname;
      return host === expected || host.endsWith(`.${expected}`);
    })?.id ?? null;
  } catch { return null; }
}

/** Presentation state lives for this renderer session. Native pages/history/scroll
 * remain in Main. No URLs, titles or browser history are written to storage. */
export class SiteBrowserSession {
  readonly initial: DesktopBrowserSiteIdV1;
  readonly #api: DesktopBrowserApiV1 | undefined;
  readonly #listeners = new Set<() => void>();
  readonly #early = new Map<string, DesktopBrowserEventV1[]>();
  #state: BrowserSessionState = { tabs: [], activeId: null, initialized: false, limitReached: false };
  constructor(api: DesktopBrowserApiV1 | undefined, purpose: BrowserPurpose) {
    this.#api = api;
    this.initial = purpose === "assets" ? "booth" : "vrchat-wiki";
    api?.events.subscribe(event => this.#event(event));
  }
  getSnapshot = (): BrowserSessionState => this.#state;
  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener); return () => this.#listeners.delete(listener);
  };
  #write(state: BrowserSessionState): void {
    this.#state = state; for (const listener of this.#listeners) listener();
  }
  #patch(id: string, patch: Partial<BrowserTab>): void {
    if (!this.#state.tabs.some(tab => tab.id === id)) return;
    this.#write({ ...this.#state, tabs: this.#state.tabs.map(tab => tab.id === id ? { ...tab, ...patch } : tab) });
  }
  ensureInitial = (): void => {
    if (!this.#state.initialized) this.open(this.initial);
  };
  open = (site: DesktopBrowserSiteIdV1 = this.initial): void => {
    if (this.#state.tabs.length >= DESKTOP_BROWSER_TAB_LIMIT) {
      this.#write({ ...this.#state, limitReached: true }); return;
    }
    const id = crypto.randomUUID();
    this.#write({ tabs: [...this.#state.tabs, { id, view: null, site, title: "", loading: true, failure: null, failedPage: false }],
      activeId: id, initialized: true, limitReached: false });
    this.#openView(id, site);
  };
  #openView(id: string, site: DesktopBrowserSiteIdV1): void {
    if (!this.#api) { this.#patch(id, { failure: "unavailable", loading: false }); return; }
    void this.#api.open(site).then(view => {
      if (!this.#state.tabs.some(tab => tab.id === id)) { void this.#api!.close(view.viewId).catch(() => {}); return; }
      this.#patch(id, { view: { ...view, url: view.url || DESKTOP_BROWSER_SITES_V1.find(entry => entry.id === site)!.url } });
      const events = this.#early.get(view.viewId) ?? [];
      this.#early.delete(view.viewId);
      if (!this.#state.tabs.some(tab => tab.loading && !tab.view)) this.#early.clear();
      for (const event of events) this.#event(event);
    }).catch(() => this.#patch(id, { failure: "unavailable", loading: false }));
  }
  activate = (id: string): void => {
    if (this.#state.tabs.some(tab => tab.id === id)) this.#write({ ...this.#state, activeId: id, limitReached: false });
  };
  close = (id: string): void => {
    const index = this.#state.tabs.findIndex(tab => tab.id === id);
    if (index < 0) return;
    const removed = this.#state.tabs[index]!;
    const tabs = this.#state.tabs.filter(tab => tab.id !== id);
    const activeId = this.#state.activeId === id ? tabs[Math.min(index, tabs.length - 1)]?.id ?? null : this.#state.activeId;
    this.#write({ ...this.#state, tabs, activeId, limitReached: false });
    if (removed.view) void this.#api?.close(removed.view.viewId).catch(() => {});
  };
  navigate = (id: string, site: DesktopBrowserSiteIdV1): void => {
    const tab = this.#state.tabs.find(tab => tab.id === id);
    if (!tab?.view || !this.#api) return;
    this.#patch(id, { loading: true, failure: null, failedPage: false });
    void this.#api.navigate(tab.view.viewId, site).catch(() => this.#patch(id, { failure: "unavailable", loading: false }));
  };
  action = (id: string, command: "goBack" | "goForward" | "reload"): void => {
    const tab = this.#state.tabs.find(tab => tab.id === id);
    if (!tab || (!tab.view && tab.loading)) return;
    this.#patch(id, { loading: true, failure: null, failedPage: false });
    if (!tab.view) { this.#openView(id, tab.site ?? this.initial); return; }
    void this.#api?.[command](tab.view.viewId).catch(() => this.#patch(id, { failure: "unavailable", loading: false }));
  };
  #event(event: DesktopBrowserEventV1): void {
    if (event.kind === "tab-opened") {
      const parent = this.#state.tabs.find(tab => tab.view?.viewId === event.parentViewId);
      if (!parent) return;
      if (this.#state.tabs.length >= DESKTOP_BROWSER_TAB_LIMIT) {
        void this.#api?.close(event.viewId).catch(() => {});
        this.#write({ ...this.#state, limitReached: true }); return;
      }
      const id = crypto.randomUUID();
      this.#write({ ...this.#state, tabs: [...this.#state.tabs, { id, site: siteForUrl(event.url), title: "", loading: true, failure: null, failedPage: false,
        view: { viewId: event.viewId, url: event.url, canGoBack: false, canGoForward: false, visible: false } }], activeId: id, limitReached: false });
      return;
    }
    const tab = this.#state.tabs.find(tab => tab.view?.viewId === event.viewId);
    if (!tab) {
      // Native loading/title events can arrive before the open IPC receipt.
      if (this.#state.tabs.some(tab => tab.loading && !tab.view) && this.#early.size < 16) {
        const events = this.#early.get(event.viewId) ?? [];
        this.#early.set(event.viewId, [...events.slice(-15), event]);
      }
      return;
    }
    if (event.kind === "navigated") this.#patch(tab.id, { view: { ...tab.view!, url: event.url, canGoBack: event.canGoBack, canGoForward: event.canGoForward }, site: siteForUrl(event.url), title: "" });
    else if (event.kind === "loading") this.#patch(tab.id, { loading: event.loading, ...(event.loading ? { failure: null, failedPage: false } : {}) });
    else if (event.kind === "load-failed") this.#patch(tab.id, { failure: event.code, failedPage: true, loading: false });
    else if (event.kind === "title-updated") this.#patch(tab.id, { title: event.title });
    else if (event.kind === "view-closed") {
      const index = this.#state.tabs.indexOf(tab);
      const tabs = this.#state.tabs.filter(entry => entry.id !== tab.id);
      this.#write({ ...this.#state, tabs, activeId: this.#state.activeId === tab.id ? tabs[Math.min(index, tabs.length - 1)]?.id ?? null : this.#state.activeId, limitReached: false });
    } else if (event.kind === "tab-limit") this.#write({ ...this.#state, limitReached: true });
  }
}

const sessions = new WeakMap<DesktopBrowserApiV1, Partial<Record<BrowserPurpose, SiteBrowserSession>>>();
export function siteBrowserSession(api: DesktopBrowserApiV1 | undefined, purpose: BrowserPurpose): SiteBrowserSession {
  if (!api) return new SiteBrowserSession(undefined, purpose);
  let owned = sessions.get(api);
  if (!owned) { owned = {}; sessions.set(api, owned); }
  return owned[purpose] ??= new SiteBrowserSession(api, purpose);
}
