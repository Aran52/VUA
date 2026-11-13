import { useEffect, useRef, useState } from "react";
import { storageKeys } from "../../app/storage-keys.ts";
import type { PageId } from "../../app/nav-model.ts";
import { currentLocale, format, strings, termLabel } from "../../i18n/index.ts";
import { firstUseCopy } from "../../i18n/first-use-copy.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { useSwapFade } from "../../components/primitives/motion.ts";
import { guidePage } from "../guide/guide-content.ts";
import { GuideEntryButton } from "../guide/GuideEntryButton.tsx";
import { directoryPageTitle } from "../shell/directory.ts";
import { BEGINNER_CONCEPTS, parseBeginnerBookmark, serializeBeginnerBookmark } from "./beginner-wizard-model.ts";
import "../overlay/overlay.css";
import "./beginner-wizard.css";

/** The VRChat getting-started wizard: one concept per page, its three buttons always in the same place. */
export function VuaGuide({ onBack, navigate }: { onBack: () => void; navigate: (page: PageId) => void }) {
  const copy = firstUseCopy[currentLocale].wizard;
  const [index, setIndex] = useState(() => {
    try { return parseBeginnerBookmark(localStorage.getItem(storageKeys.beginnerWizard)); } catch { return 0; }
  });
  const title = useRef<HTMLHeadingElement>(null);
  const page = useRef<HTMLElement>(null);
  useEffect(() => { title.current?.focus(); }, [index]);
  useSwapFade(index, page);
  const choose = (next: number) => {
    try { localStorage.setItem(storageKeys.beginnerWizard, serializeBeginnerBookmark(next)); } catch { /* Session-only reading remains usable. */ }
    setIndex(next);
  };
  const concept = BEGINNER_CONCEPTS[index]!;
  const section = guidePage(concept.target.topic).sections.find(item => item.id === concept.target.section);
  const last = index + 1 === BEGINNER_CONCEPTS.length;
  return <div className="vua-page vua-knowledge vua-vua-guide" data-focus-scope onKeyDown={event => {
    if (event.target instanceof HTMLElement && event.target.closest("input, textarea, select")) return;
    if (event.key === "ArrowRight" && !last) choose(index + 1);
    if (event.key === "ArrowLeft" && index > 0) choose(index - 1);
  }}>
    <header className="vua-page__hero vua-environment-heading">
      <h1 className="vua-title">{copy.title}</h1>
      <Button variant="subtle" data-back data-nav-id="help-child-back" onClick={onBack}>{strings.journey.backToHelp}</Button>
    </header>
    <div className="vua-beginner-wizard" data-wizard-step={concept.id}>
      <p className="vua-beginner-wizard__progress" role="status">{format(copy.progress, { current: index + 1, total: BEGINNER_CONCEPTS.length })}</p>
      {section ? <article ref={page} className="vua-beginner-wizard__article" data-guide-section={section.id}>
        <h2 className="vua-title" ref={title} tabIndex={-1}>{section.title}</h2>
        {section.paragraphs.map((paragraph, paragraphIndex) => <p key={paragraphIndex}>{format(paragraph, { amf: termLabel("amf") })}</p>)}
        <GuideEntryButton target={concept.target} label={copy.article} />
      </article> : <p role="status">{copy.unavailable}</p>}
      <footer className="vua-beginner-wizard__actions">
        <Button data-nav-id="wizard-previous" disabled={index === 0} onClick={() => choose(index - 1)}>{copy.previous}</Button>
        <Button data-nav-id="wizard-want" onClick={() => navigate(concept.setup)}>{format(copy.want, { page: directoryPageTitle(concept.setup) })}</Button>
        <Button variant="primary" data-nav-id="wizard-next" onClick={() => {
          if (last) { choose(0); navigate("home"); } else choose(index + 1);
        }}>{last ? copy.finish : copy.next}</Button>
      </footer>
    </div>
  </div>;
}
