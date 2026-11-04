import { useEffect, useState } from "react";
import { strings } from "../../i18n/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { SiteBrowser } from "../../components/SiteBrowser.tsx";
import { GuideOverlayView, type GuideRequest } from "../overlay/GuideOverlayView.tsx";
import { loadGuideReading } from "../guide/guide-target.ts";
import { encyclopediaCopy } from "../../i18n/encyclopedia-copy.ts";
import { currentLocale } from "../../i18n/index.ts";
import { EncyclopediaHome } from "./EncyclopediaHome.tsx";
import { openEncyclopedia, readEncyclopediaHash, recordEncyclopediaCatalog } from "./encyclopedia-navigation.ts";
import type { GuideArticleKind } from "../guide/guide-catalog.ts";
import type { GuideTopicId } from "../guide/guide-content.ts";
import "../overlay/overlay.css";
import "./encyclopedia.css";

export function EncyclopediaPage({ onBack, guideRequest, acknowledgeGuide }: {
  onBack: () => void;
  guideRequest: GuideRequest | null;
  acknowledgeGuide: (nonce: number) => void;
}) {
  const copy = strings.journey;
  const uiCopy = encyclopediaCopy[currentLocale];
  const [wikiOpen, setWikiOpen] = useState(false);
  const [catalog, setCatalog] = useState(() => {
    if (guideRequest?.target || readEncyclopediaHash(window.location.hash)) return false;
    return loadGuideReading() === null;
  });
  const [query, setQuery] = useState("");
  const [topic, setTopic] = useState<GuideTopicId | "all">("all");
  const [kind, setKind] = useState<GuideArticleKind | "all">("all");

  useEffect(() => {
    setWikiOpen(false);
    if (guideRequest !== null) setCatalog(guideRequest.target !== null || loadGuideReading() !== null ? false : true);
  }, [guideRequest?.nonce]);

  useEffect(() => {
    const onHistory = () => {
      if (readEncyclopediaHash(window.location.hash)) setCatalog(false);
      else if (window.location.hash === "#help-encyclopedia") setCatalog(true);
    };
    window.addEventListener("hashchange", onHistory);
    return () => window.removeEventListener("hashchange", onHistory);
  }, []);

  const backToCatalog = () => {
    recordEncyclopediaCatalog();
    setCatalog(true);
  };
  const clear = () => { setQuery(""); setTopic("all"); setKind("all"); };

  return <div className="vua-page vua-knowledge" data-focus-scope>
    <header className="vua-page__hero vua-environment-heading">
      <h1 className="vua-title">{copy.guide}</h1>
      {!catalog ? <Button variant="subtle" data-nav-id="help-catalog" onClick={backToCatalog}>{uiCopy.backToCatalog}</Button> : null}
      <Button data-nav-id="knowledge-wiki" onClick={() => setWikiOpen(true)}>{strings.browser.wiki}</Button>
      <Button variant="subtle" data-back data-nav-id="help-child-back" onClick={onBack}>{copy.backToHelp}</Button>
    </header>
    <div className="vua-overlay__body vua-knowledge__body">
      {catalog ? <EncyclopediaHome
        query={query} onQuery={setQuery}
        topic={topic} onTopic={setTopic}
        kind={kind} onKind={setKind}
        hasReading={loadGuideReading() !== null}
        onContinue={() => openEncyclopedia(null)}
        onClear={clear}
      /> : <GuideOverlayView embedded guideRequest={guideRequest} onGuideRequestApplied={acknowledgeGuide} />}
    </div>
    {wikiOpen ? <SiteBrowser purpose="knowledge" onClose={() => setWikiOpen(false)} /> : null}
  </div>;
}
