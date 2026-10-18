import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { SiteBrowser } from "../../components/SiteBrowser.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { strings } from "../../i18n/index.ts";
import type { PageId } from "../../app/nav-model.ts";
import { catalogBrowser } from "../../app/catalog-browser-instance.ts";
import { siteBrowserSession } from "../../components/site-browser-session.ts";
import { useAmfModule, useGateway } from "../../gateway/index.ts";
import { boothSourceId } from "../warehouse/booth-source.ts";
import { MaterialsStartDialog } from "./MaterialsStartDialog.tsx";

/** The materials room's browse tab. Watching a BOOTH Avatar item page offers the
 * same "Start production" the library detail gives; anything VUA cannot confirm
 * as an Avatar stays out of the way. */
export function AssetBrowserPage({ onNavigate }: { onNavigate: (page: PageId) => void }) {
  const [open, setOpen] = useState(true);
  const copy = strings.browser;
  const start = strings.factory.materialsStart;
  const amf = useAmfModule();
  const gateway = useGateway();
  const amfReady = amf.state === "ready" && gateway.amfProduction !== undefined;
  const session = useMemo(() => siteBrowserSession(window.vua?.desktopBrowser, "assets"), []);
  const browserState = useSyncExternalStore(session.subscribe, session.getSnapshot);
  useEffect(() => { session.ensureInitial(); }, [session]);
  const activeUrl = browserState.tabs.find(tab => tab.id === browserState.activeId)?.view?.url ?? null;
  const productId = activeUrl === null ? null : boothSourceId(activeUrl);
  const [offer, setOffer] = useState<{ productId: string; title: string | null; imageUrl: string | null } | null>(null);
  const [startOpen, setStartOpen] = useState(false);
  useEffect(() => {
    let alive = true; setOffer(null);
    if (productId === null || !amfReady) return;
    const face = window.vua?.catalogSync;
    // Observe the page first so a product the library has not enriched yet still
    // reports its entities; a failed observation falls back to what is stored.
    void (face?.fetchProduct(productId) ?? Promise.resolve({ ok: false }))
      .then(() => catalogBrowser.detail(productId))
      .then(view => {
        if (!alive || view.kind !== "detail") return;
        if (!view.product.entities.some(entity => entity.entityType === "avatar")) return;
        setOffer({ productId, title: view.product.title, imageUrl: view.product.imageUrl });
      })
      .catch(() => { /* No confirmed Avatar facts, no offer. */ });
    return () => { alive = false; };
  }, [productId, amfReady]);
  return <div className="vua-page" data-focus-scope>
    <header className="vua-page__hero"><h1 className="vua-title">{strings.browser.assets}</h1></header>
    {offer !== null && !startOpen ? (
      <div className="vua-start__browse">
        <span title={offer.title ?? offer.productId}>{offer.title ?? offer.productId}</span>
        <Button variant="primary" onClick={() => setStartOpen(true)}>{start.startCta}</Button>
      </div>
    ) : null}
    {open ? <SiteBrowser purpose="assets" onClose={() => setOpen(false)} />
      : <Button variant="primary" onClick={() => setOpen(true)}>{copy.reopen}</Button>}
    {startOpen && offer !== null ? (
      <MaterialsStartDialog avatar={offer} onNavigate={onNavigate} onClose={() => setStartOpen(false)} />
    ) : null}
  </div>;
}
