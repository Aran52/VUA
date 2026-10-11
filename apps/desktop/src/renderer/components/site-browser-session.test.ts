import { describe, it, expect, vi } from "vitest";
import { DESKTOP_BROWSER_TAB_LIMIT, type DesktopBrowserApiV1, type DesktopBrowserEventV1, type RemoteContentViewStateV1 } from "@vua/contracts";
import { SiteBrowserSession, siteBrowserSession } from "./site-browser-session.ts";

function harness() {
  let listener: (event: DesktopBrowserEventV1) => void = () => {};
  let sequence = 0;
  const pending: ((view: RemoteContentViewStateV1) => void)[] = [];
  const api: DesktopBrowserApiV1 = {
    open: vi.fn(() => new Promise<RemoteContentViewStateV1>(resolve => pending.push(resolve))),
    close: vi.fn(async () => {}),
    navigate: vi.fn(), goBack: vi.fn(), goForward: vi.fn(), reload: vi.fn(), setViewport: vi.fn(),
    events: { subscribe: callback => { listener = callback; return () => {}; } },
  };
  return { api, emit: (event: DesktopBrowserEventV1) => listener(event),
    settle: async (url = "https://booth.pm/") => {
      const view = { viewId: `view-${++sequence}`, url, visible: false, canGoBack: false, canGoForward: false };
      pending.shift()!(view); await Promise.resolve(); return view;
    } };
}
describe("retained browser session", () => {
  it("keeps the same session across route mounts and does not reopen or navigate its page", async () => {
    const h = harness(), first = siteBrowserSession(h.api, "assets");
    first.ensureInitial(); first.ensureInitial(); const view = await h.settle();
    h.emit({ kind: "navigated", viewId: view.viewId, url: "https://booth.pm/items/12", canGoBack: true, canGoForward: false });
    h.emit({ kind: "loading", viewId: view.viewId, loading: false });
    const remounted = siteBrowserSession(h.api, "assets"); remounted.ensureInitial();
    expect(remounted).toBe(first); expect(h.api.open).toHaveBeenCalledTimes(1);
    expect(remounted.getSnapshot().tabs[0]!.view?.url).toBe("https://booth.pm/items/12");
    expect(h.api.navigate).not.toHaveBeenCalled(); expect(h.api.close).not.toHaveBeenCalled();
    expect(siteBrowserSession(h.api, "knowledge")).not.toBe(first);
  });
  it("does not leak a native view when a tab is closed before its open receipt", async () => {
    const h = harness(), session = new SiteBrowserSession(h.api, "assets");
    session.open(); session.close(session.getSnapshot().activeId!);
    const view = await h.settle();
    expect(session.getSnapshot().tabs).toHaveLength(0); expect(h.api.close).toHaveBeenCalledWith(view.viewId);
  });
  it("updates inactive tabs, retains independent history, and closes only the selected tab", async () => {
    const h = harness(), session = new SiteBrowserSession(h.api, "assets");
    session.open(); const firstId = session.getSnapshot().activeId!, first = await h.settle();
    session.open("vrcfinder"); const secondId = session.getSnapshot().activeId!, second = await h.settle("https://vrcfinder.net/");
    h.emit({ kind: "navigated", viewId: first.viewId, url: "https://booth.pm/items/33", canGoBack: true, canGoForward: false });
    h.emit({ kind: "title-updated", viewId: first.viewId, title: "First material" });
    session.activate(firstId);
    expect(session.getSnapshot().tabs[0]).toMatchObject({ title: "First material", view: { canGoBack: true } });
    session.close(firstId);
    expect(session.getSnapshot().activeId).toBe(secondId);
    expect(session.getSnapshot().tabs[0]!.view?.viewId).toBe(second.viewId);
    expect(h.api.close).toHaveBeenCalledTimes(1);
  });
  it("buffers loading completion before the open receipt and keeps failures out of other tabs", async () => {
    const h = harness(), session = new SiteBrowserSession(h.api, "assets");
    session.open(); h.emit({ kind: "loading", viewId: "view-1", loading: false }); await h.settle();
    expect(session.getSnapshot().tabs[0]!.loading).toBe(false);
    session.open(); const other = await h.settle();
    h.emit({ kind: "load-failed", viewId: other.viewId, code: -105 });
    expect(session.getSnapshot().tabs[0]!.failure).toBeNull();
    expect(session.getSnapshot().tabs[1]).toMatchObject({ failure: -105, failedPage: true });
  });
  it("routes native child tabs to their owner and enforces a bounded open count", async () => {
    const h = harness(), session = new SiteBrowserSession(h.api, "assets");
    session.open(); const first = await h.settle();
    h.emit({ kind: "tab-opened", viewId: "child", parentViewId: first.viewId, url: "https://booth.pm/items/55" });
    expect(session.getSnapshot().tabs).toHaveLength(2);
    expect(session.getSnapshot().tabs[1]!.view?.url).toBe("https://booth.pm/items/55");
    h.emit({ kind: "tab-opened", viewId: "not-ours", parentViewId: "import-view", url: "https://booth.pm/" });
    expect(session.getSnapshot().tabs).toHaveLength(2);
    while (session.getSnapshot().tabs.length < DESKTOP_BROWSER_TAB_LIMIT) session.open();
    const calls = vi.mocked(h.api.open).mock.calls.length;
    session.open(); expect(session.getSnapshot().tabs).toHaveLength(DESKTOP_BROWSER_TAB_LIMIT);
    expect(h.api.open).toHaveBeenCalledTimes(calls); expect(session.getSnapshot().limitReached).toBe(true);
    h.emit({ kind: "tab-opened", viewId: "racing-child", parentViewId: first.viewId, url: "https://booth.pm/items/88" });
    expect(session.getSnapshot().tabs).toHaveLength(DESKTOP_BROWSER_TAB_LIMIT);
    expect(h.api.close).toHaveBeenCalledWith("racing-child");
  });
  it("removes disposed native views without automatically replacing them after module shutdown", async () => {
    const h = harness(), session = new SiteBrowserSession(h.api, "assets");
    session.open(); const view = await h.settle();
    h.emit({ kind: "view-closed", viewId: view.viewId }); session.ensureInitial();
    expect(session.getSnapshot().tabs).toHaveLength(0); expect(h.api.open).toHaveBeenCalledTimes(1);
  });
});
