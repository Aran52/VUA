import type { PageId } from "../../app/nav-model.ts";
import { useState } from "react";
import { strings } from "../../i18n/index.ts";
import { RouteTile } from "../../components/RouteTile.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import type { GuideRequest } from "../overlay/GuideOverlayView.tsx";
import "../overlay/overlay.css";
import "./help.css";
import { AiToolsPage } from "./AiToolsPage.tsx";
import { VuaGuide } from "./VuaGuide.tsx";
import { EncyclopediaPage } from "./EncyclopediaPage.tsx";
import { GuideEntryButton } from "../guide/GuideEntryButton.tsx";
import { GUIDE_TARGETS } from "../guide/guide-target.ts";

export function HelpPage({ page, navigate, startTour, guideRequest, acknowledgeGuide }: {
  page: "help" | "help-wizard" | "help-tour" | "help-game-assistant" | "help-encyclopedia" | "help-ai";
  navigate: (page: PageId) => void;
  startTour: () => void;
  guideRequest: GuideRequest | null;
  acknowledgeGuide: (nonce: number) => void;
}) {
  const copy = strings.journey;
  const faq = strings.helpFaq;
  const [openFailed, setOpenFailed] = useState(false);
  const back = <Button variant="subtle" data-back data-nav-id="help-child-back" onClick={() => navigate("help")}>{copy.backToHelp}</Button>;
  if (page === "help-ai") return <div className="vua-page vua-help-child" data-focus-scope>
    <header className="vua-page__hero vua-environment-heading"><h1 className="vua-title">{strings.aiTools.title}</h1>{back}</header>
    <AiToolsPage navigate={navigate} />
  </div>;
  if (page === "help-encyclopedia") return <EncyclopediaPage onBack={() => navigate("help")} guideRequest={guideRequest} acknowledgeGuide={acknowledgeGuide} />;
  if (page === "help-wizard") return <VuaGuide onBack={() => navigate("help")} navigate={navigate} guideRequest={guideRequest} acknowledgeGuide={acknowledgeGuide} />;
  if (page === "help-tour" || page === "help-game-assistant") return <div className="vua-page vua-help-child" data-focus-scope>
    <header className="vua-page__hero vua-environment-heading"><h1 className="vua-title">{page === "help-tour" ? strings.tour.paletteEntry : copy.gameGuide}</h1>{back}</header>
    <section className="vua-environment-detail">
      {page === "help-game-assistant" ? <p>{strings.helpUi.gameHint}</p> : null}
      <div><Button variant="primary" data-nav-id={page === "help-tour" ? "help-start-tour" : "help-open-game"} onClick={() => {
        if (page === "help-tour") startTour();
        else {
          setOpenFailed(false);
          const request = window.vua?.window.showGameGuide();
          if (!request) setOpenFailed(true);
          else void request.catch(() => setOpenFailed(true));
        }
      }}>{page === "help-tour" ? strings.helpUi.startTour : strings.helpUi.openGame}</Button></div>
      {openFailed ? <p role="status">{strings.helpUi.openFailed}</p> : null}
    </section>
  </div>;

  const articles = [
    { id: "shield", title: faq.shield.title, body: faq.shield.body, target: GUIDE_TARGETS.shield },
    { id: "personal-space", title: faq.personalSpace.title, body: faq.personalSpace.body, target: GUIDE_TARGETS.personalSpace },
    { id: "untrusted-urls", title: faq.untrustedUrls.title, body: faq.untrustedUrls.body, target: GUIDE_TARGETS.untrustedUrls },
  ];
  return <div className="vua-page vua-help" data-focus-scope>
    <header className="vua-page__hero"><h1 className="vua-title">{copy.help}</h1></header>
    <section className="vua-help-section" aria-labelledby="help-faq-title">
      <h2 id="help-faq-title">{faq.sectionTitle}</h2>
      <div className="vua-help-faq">
        <article className="vua-help-faq__item" data-faq="trust-rank">
          <h3>{faq.trust.title}</h3>
          <p>{faq.trust.body}</p>
          <div className="vua-help-faq__actions">
            <GuideEntryButton target={GUIDE_TARGETS.trustRank} label={faq.trust.what} />
            <GuideEntryButton target={GUIDE_TARGETS.improveTrust} label={faq.trust.improve} />
          </div>
        </article>
        {articles.map(article => <article className="vua-help-faq__item" data-faq={article.id} key={article.id}>
          <h3>{article.title}</h3>
          <p>{article.body}</p>
          <div className="vua-help-faq__actions"><GuideEntryButton target={article.target} label={faq.readMore} /></div>
        </article>)}
      </div>
    </section>
    <section className="vua-help-section" aria-labelledby="help-vua-title">
      <h2 id="help-vua-title">{faq.vuaTitle}</h2>
      <div className="vua-route-grid">
        <RouteTile title={copy.wizard} icon="arrow-right" onClick={() => navigate("help-wizard")} id="help-wizard" />
        <RouteTile title={strings.tour.paletteEntry} icon="question" onClick={() => navigate("help-tour")} id="help-tour" />
        <RouteTile title={copy.gameGuide} icon="anim" onClick={() => navigate("help-game-assistant")} id="help-game-assistant" />
        <RouteTile title={strings.aiTools.title} icon="computer" onClick={() => navigate("help-ai")} id="help-ai" />
      </div>
    </section>
    <section className="vua-help-section" aria-labelledby="help-encyclopedia-title">
      <h2 id="help-encyclopedia-title">{copy.guide}</h2>
      <div className="vua-route-grid">
        <RouteTile title={copy.guide} icon="folder" onClick={() => navigate("help-encyclopedia")} id="help-encyclopedia" />
      </div>
    </section>
  </div>;
}
