import { useEffect, useRef, useState } from "react";
import { storageKeys } from "../../app/storage-keys.ts";
import type { PageId } from "../../app/nav-model.ts";
import { currentLocale, format, strings, termLabel } from "../../i18n/index.ts";
import { firstUseCopy } from "../../i18n/first-use-copy.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { GuideOverlayView, type GuideRequest } from "../overlay/GuideOverlayView.tsx";
import { loadGuideReading, type GuideTarget } from "../guide/guide-target.ts";
import { guidePage } from "../guide/guide-content.ts";
import { GuideEntryButton } from "../guide/GuideEntryButton.tsx";
import { BEGINNER_CONCEPTS, parseBeginnerBookmark, serializeBeginnerBookmark } from "./beginner-wizard-model.ts";
import "../overlay/overlay.css";
import "./beginner-wizard.css";

export function VuaGuide({ onBack, navigate, guideRequest, acknowledgeGuide }: {
  onBack: () => void;
  navigate: (page: PageId) => void;
  guideRequest: GuideRequest | null;
  acknowledgeGuide: (nonce: number) => void;
}) {
  const copy = firstUseCopy[currentLocale].wizard;
  const [manual, setManual] = useState(guideRequest?.target?.topic === "guide-vua");
  const [index, setIndex] = useState(() => {
    try { return parseBeginnerBookmark(localStorage.getItem(storageKeys.beginnerWizard)); } catch { return 0; }
  });
  const title = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (guideRequest?.target?.topic === "guide-vua") setManual(true);
  }, [guideRequest?.nonce]);
  useEffect(() => { title.current?.focus(); }, [index, manual]);
  const choose = (next: number) => {
    try { localStorage.setItem(storageKeys.beginnerWizard, serializeBeginnerBookmark(next)); } catch { /* Session-only reading remains usable. */ }
    setIndex(next);
  };
  const concept = BEGINNER_CONCEPTS[index]!;
  const section = guidePage(concept.target.topic).sections.find(item => item.id === concept.target.section);
  const legacy = loadGuideReading(storageKeys.vuaGuideReading);
  const initialPosition: GuideTarget = legacy?.topic === "guide-vua" ? legacy : { topic: "guide-vua" };
  return <div className="vua-page vua-knowledge vua-vua-guide" data-focus-scope>
    <header className="vua-page__hero vua-environment-heading">
      <h1 className="vua-title">{manual ? copy.manual : copy.title}</h1>
      <Button variant="subtle" data-back data-nav-id="help-child-back" onClick={onBack}>{strings.journey.backToHelp}</Button>
    </header>
    {manual ? <div className="vua-overlay__body vua-knowledge__body">
      <GuideOverlayView
        embedded
        guideRequest={guideRequest}
        onGuideRequestApplied={acknowledgeGuide}
        topicIds={["guide-vua"]}
        initialTopic="guide-vua"
        initialPosition={initialPosition}
        readingKey={storageKeys.vuaGuideReading}
        historyEnabled={false}
      />
    </div> : <div className="vua-beginner-wizard" data-wizard-step={concept.id}>
      <div className="vua-beginner-wizard__progress">
        <p role="status">{format(copy.progress, { current: index + 1, total: BEGINNER_CONCEPTS.length })}</p>
        {index > 0 ? <Button variant="subtle" data-nav-id="wizard-previous" onClick={() => choose(index - 1)}>{copy.previous}</Button> : null}
      </div>
      {section ? <article className="vua-beginner-wizard__article" data-guide-section={section.id}>
        <h2 className="vua-title" ref={title} tabIndex={-1}>{section.title}</h2>
        {section.paragraphs.map((paragraph, paragraphIndex) => <p key={paragraphIndex}>{format(paragraph, { amf: termLabel("amf") })}</p>)}
        <GuideEntryButton target={concept.target} label={copy.article} />
        <footer className="vua-beginner-wizard__actions">
          <Button variant="primary" data-nav-id="wizard-want" onClick={() => navigate(concept.setup)}>{copy.want}</Button>
          <Button variant="subtle" data-nav-id="wizard-next" onClick={() => {
            if (index + 1 < BEGINNER_CONCEPTS.length) choose(index + 1);
            else { choose(0); navigate("home"); }
          }}>{copy.know}</Button>
        </footer>
      </article> : <p role="status">{copy.unavailable}</p>}
    </div>}
  </div>;
}
