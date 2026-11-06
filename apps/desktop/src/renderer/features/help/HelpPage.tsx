import type { PageId } from "../../app/nav-model.ts";
import { useState } from "react";
import { Icon, type IconName } from "@vua/design-system";
import { currentLocale, strings } from "../../i18n/index.ts";
import { structureCopy } from "../../i18n/structure-copy.ts";
import type { GuideRequest } from "../overlay/GuideOverlayView.tsx";
import "../overlay/overlay.css";
import "./help.css";
import { AiToolsPage } from "./AiToolsPage.tsx";
import { VuaGuide } from "./VuaGuide.tsx";
import { EncyclopediaPage } from "./EncyclopediaPage.tsx";
import { GuideEntryButton } from "../guide/GuideEntryButton.tsx";
import { GUIDE_TARGETS } from "../guide/guide-target.ts";

const hints = structureCopy[currentLocale].help;

/** One thing Help can do, done directly: a row opens its page or starts its tour or window. */
function HelpRow({ id, icon, title, hint, onClick }: { id: string; icon: IconName; title: string; hint: string; onClick: () => void }) {
  return <button type="button" className="vua-help-row" data-nav-id={id} onClick={onClick}>
    <Icon name={icon} size={20} />
    <span className="vua-help-row__text"><strong>{title}</strong><small>{hint}</small></span>
    <Icon name="arrow-right" size={16} />
  </button>;
}

export function HelpPage({ page, navigate, startTour, guideRequest, acknowledgeGuide }: {
  page: "help" | "help-wizard" | "help-encyclopedia" | "help-ai";
  navigate: (page: PageId) => void;
  startTour: () => void;
  guideRequest: GuideRequest | null;
  acknowledgeGuide: (nonce: number) => void;
}) {
  const copy = strings.journey;
  const faq = strings.helpFaq;
  const [openFailed, setOpenFailed] = useState(false);
  if (page === "help-ai") return <div className="vua-page vua-help-child" data-focus-scope>
    <header className="vua-page__hero vua-environment-heading"><h1 className="vua-title">{strings.aiTools.title}</h1></header>
    <AiToolsPage navigate={navigate} />
  </div>;
  if (page === "help-encyclopedia") return <EncyclopediaPage onBack={() => navigate("help")} guideRequest={guideRequest} acknowledgeGuide={acknowledgeGuide} />;
  if (page === "help-wizard") return <VuaGuide onBack={() => navigate("help")} navigate={navigate} />;

  const openGameGuide = () => {
    setOpenFailed(false);
    const request = window.vua?.window.showGameGuide();
    if (!request) setOpenFailed(true);
    else void request.catch(() => setOpenFailed(true));
  };
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
      <div className="vua-help-rows">
        <HelpRow id="help-tour" icon="question" title={strings.tour.paletteEntry} hint={hints.tourHint} onClick={startTour} />
        <HelpRow id="help-wizard" icon="arrow-right" title={copy.wizard} hint={hints.wizardHint} onClick={() => navigate("help-wizard")} />
        <HelpRow id="help-encyclopedia" icon="folder" title={copy.guide} hint={hints.encyclopediaHint} onClick={() => navigate("help-encyclopedia")} />
        <HelpRow id="help-game-assistant" icon="anim" title={copy.gameGuide} hint={hints.gameHint} onClick={openGameGuide} />
      </div>
      {openFailed ? <p role="status">{strings.helpUi.openFailed}</p> : null}
    </section>
  </div>;
}
