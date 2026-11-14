import { useEffect, useMemo, useReducer, useState, useSyncExternalStore } from "react";
import { Icon } from "@vua/design-system";
import { SiteBrowser } from "../../components/SiteBrowser.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { format, strings } from "../../i18n/index.ts";
import type { PageId } from "../../app/nav-model.ts";
import { catalogBrowser } from "../../app/catalog-browser-instance.ts";
import { siteBrowserSession } from "../../components/site-browser-session.ts";
import { useAmfModule, useGateway, useTaskCenter } from "../../gateway/index.ts";
import { boothSourceId } from "../warehouse/booth-source.ts";
import { FreeAssetsView } from "./FreeAssetsView.tsx";
import {
  browserDownloadBannerReducer,
  INITIAL_BROWSER_DOWNLOAD_BANNER,
} from "./free-assets-model.ts";
import { MaterialsStartDialog } from "./MaterialsStartDialog.tsx";
import "./free-assets.css";

/** The player may choose an observed BOOTH product as the Avatar input, or switch to
 * opt-in Avatar-scoped ¥0 discovery without leaving retained browser tabs. */
export function AssetBrowserPage({ onNavigate }: { onNavigate: (page: PageId) => void }) {
  const [open, setOpen] = useState(true);
  const [mode, setMode] = useState<"browse" | "free-assets">("browse");
  const [refreshToken, setRefreshToken] = useState(0);
  const [downloadBanner, dispatchDownloadBanner] = useReducer(
    browserDownloadBannerReducer,
    INITIAL_BROWSER_DOWNLOAD_BANNER,
  );
  const copy = strings.browser;
  const freeCopy = strings.freeAssets;
  const start = strings.factory.materialsStart;
  const amf = useAmfModule();
  const gateway = useGateway();
  const taskCenter = useTaskCenter();
  const amfReady = amf.state === "ready" && gateway.amfProduction !== undefined;
  const session = useMemo(() => siteBrowserSession(window.vua?.desktopBrowser, "assets"), []);
  const browserState = useSyncExternalStore(session.subscribe, session.getSnapshot);
  useEffect(() => { session.ensureInitial(); }, [session]);

  useEffect(() => {
    const browser = window.vua?.desktopBrowser;
    if (!browser) return undefined;
    return browser.events.subscribe(event => {
      if (event.kind === "download-completed") {
        dispatchDownloadBanner({
          type: "download-completed",
          downloadId: event.downloadId,
          suggestedFileName: event.suggestedFileName,
        });
      }
    });
  }, []);

  useEffect(() => {
    const taskId = downloadBanner.adoptingTaskId;
    if (taskId === null) return;
    const task = taskCenter.tasks.find(entry => entry.id === taskId);
    if (!task) return;
    if (task.status === "completed" || task.status === "completedWithWarnings") {
      dispatchDownloadBanner({ type: "adopt-succeeded" });
      setRefreshToken(value => value + 1);
    } else if (task.status === "failed" || task.status === "cancelled") {
      dispatchDownloadBanner({ type: "adopt-failed" });
    }
  }, [downloadBanner.adoptingTaskId, taskCenter.tasks]);

  const activeUrl = browserState.tabs.find(tab => tab.id === browserState.activeId)?.view?.url ?? null;
  const productId = activeUrl === null ? null : boothSourceId(activeUrl);
  const [offer, setOffer] = useState<{ productId: string; title: string | null; imageUrl: string | null } | null>(null);
  const [startOpen, setStartOpen] = useState(false);
  useEffect(() => {
    let alive = true; setOffer(null);
    if (productId === null || !amfReady) return;
    const face = window.vua?.catalogSync;
    // Observe title/images first; a failed observation falls back to stored facts.
    void (face?.fetchProduct(productId) ?? Promise.resolve({ ok: false }))
      .then(() => catalogBrowser.detail(productId))
      .then(view => {
        if (!alive || view.kind !== "detail") return;
        setOffer({ productId, title: view.product.title, imageUrl: view.product.imageUrl });
      })
      .catch(() => { /* No observed product, no offer. */ });
    return () => { alive = false; };
  }, [productId, amfReady]);

  const switchToFreeAssets = () => {
    setRefreshToken(value => value + 1);
    setMode("free-assets");
  };

  const handleAdoptDownloads = async () => {
    if (downloadBanner.pending.length === 0 || downloadBanner.adoptingTaskId !== null) return;
    const ids = downloadBanner.pending.map(entry => entry.downloadId);
    const outcome = await gateway.warehouseCommands.importDownloads(ids);
    if (outcome.ok && "accepted" in outcome) {
      dispatchDownloadBanner({ type: "adopt-started", taskId: outcome.accepted.taskId });
    } else {
      dispatchDownloadBanner({ type: "adopt-failed" });
    }
  };

  const latestDownloadName =
    downloadBanner.pending.at(-1)?.suggestedFileName ?? freeCopy.downloadFallbackName;

  const adoptBannerNode =
    downloadBanner.pending.length > 0 || downloadBanner.completedNotice ? (
      <div className="vua-download-adopt-bar" role="status" data-download-adopt-bar>
        <span className="vua-download-adopt-bar__text">
          <Icon name={downloadBanner.completedNotice ? "check" : "download"} size={16} />
          <span>
            {downloadBanner.completedNotice
              ? freeCopy.adoptedDone
              : downloadBanner.adoptingTaskId !== null
                ? freeCopy.adoptingStatus
                : downloadBanner.pending.length === 1
                  ? format(freeCopy.downloadReadyOne, { name: latestDownloadName })
                  : format(freeCopy.downloadReadyMany, {
                      count: downloadBanner.pending.length,
                      name: latestDownloadName,
                    })}
          </span>
        </span>
        <span className="vua-download-adopt-bar__actions">
          {downloadBanner.pending.length > 0 ? (
            <Button
              variant="primary"
              data-download-adopt-action
              disabled={downloadBanner.adoptingTaskId !== null}
              onClick={() => void handleAdoptDownloads()}
            >
              {downloadBanner.adoptingTaskId !== null ? freeCopy.adoptingStatus : freeCopy.adoptAction}
            </Button>
          ) : null}
          <Button
            variant="subtle"
            aria-label={freeCopy.dismissDownload}
            title={freeCopy.dismissDownload}
            onClick={() => dispatchDownloadBanner({ type: "dismiss" })}
          >
            <Icon name="close" size={16} />
          </Button>
        </span>
      </div>
    ) : null;

  return (
    <div className="vua-page" data-focus-scope>
      <div hidden={mode !== "browse"}>
        <header className="vua-page__hero"><h1 className="vua-title">{strings.browser.assets}</h1></header>
        {offer !== null && !startOpen ? (
          <div className="vua-start__browse">
            <span title={offer.title ?? offer.productId}>{offer.title ?? offer.productId}</span>
            <Button variant="primary" onClick={() => setStartOpen(true)}>{start.startCta}</Button>
          </div>
        ) : null}
        {open ? (
          <SiteBrowser
            purpose="assets"
            onClose={() => setOpen(false)}
            shortcutsExtra={
              <button
                type="button"
                className="vua-browser-frame__mode-switch"
                data-free-assets-open
                onClick={switchToFreeAssets}
              >
                <Icon name="search" size={16} />
                <span>{freeCopy.title}</span>
              </button>
            }
            banner={adoptBannerNode}
          />
        ) : (
          <div className="vua-free-assets__toolbar-start">
            <Button variant="primary" onClick={() => setOpen(true)}>{copy.reopen}</Button>
            <Button variant="subtle" data-free-assets-open onClick={switchToFreeAssets}>
              {freeCopy.title}
            </Button>
          </div>
        )}
      </div>
      {mode === "free-assets" ? (
        <FreeAssetsView
          port={gateway.freeAssets}
          refreshToken={refreshToken}
          banner={adoptBannerNode}
          onBrowseSites={() => {
            setOpen(true);
            setMode("browse");
          }}
          onOpenBoothItem={targetProductId => {
            setOpen(true);
            setMode("browse");
            session.openBoothItem(targetProductId);
          }}
        />
      ) : null}
      {startOpen && offer !== null ? (
        <MaterialsStartDialog avatar={offer} onNavigate={onNavigate} onClose={() => setStartOpen(false)} />
      ) : null}
    </div>
  );
}
