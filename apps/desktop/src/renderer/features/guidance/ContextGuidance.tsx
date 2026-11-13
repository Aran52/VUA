import { useEffect, useRef, useState } from "react";
import type { PageId } from "../../app/nav-model.ts";
import { strings } from "../../i18n/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { canAnimate, fastMs, motionOff, slowMs, useEntranceMotion } from "../../components/primitives/motion.ts";
import { openEncyclopedia } from "../help/encyclopedia-navigation.ts";
import {
  contextGuideForPage,
  dismissContextGuide,
  readContextGuideState,
  tourAnswered,
  type ContextGuideDefinition,
} from "./context-guidance.ts";
import "./context-guidance.css";

const GUIDE_ENTER: Keyframe[] = [{ opacity: 0, transform: "translateY(8px)" }, { opacity: 1, transform: "none" }];

export function ContextGuidance({ page, startTour, suppressed = false, replayToken = 0 }: {
  page: PageId;
  startTour: () => void;
  suppressed?: boolean;
  replayToken?: number;
}) {
  const definition = contextGuideForPage(page);
  const [dismissed, setDismissed] = useState(() => readContextGuideState(localStorage).dismissed);
  useEffect(() => { setDismissed(readContextGuideState(localStorage).dismissed); }, [replayToken]);
  const card = useRef<HTMLElement>(null);
  // Offering the tour right after the player took or skipped it would only ask again.
  const visible = !suppressed && definition !== null && (dismissed[definition.id] ?? 0) < definition.revision
    && !(definition.action === "tour" && tourAnswered(localStorage));
  useEntranceMotion(visible ? definition.id : null, card, GUIDE_ENTER, slowMs);

  if (!visible) return null;
  const copy = strings.contextGuide;
  const content = copy[definition.id];
  const actionLabel = content.action || strings.helpFaq.uploadAction;
  const dismiss = () => {
    const next = dismissContextGuide(localStorage, definition).dismissed;
    if (motionOff() || !canAnimate(card.current)) { setDismissed(next); return; }
    const done = () => setDismissed(next);
    card.current.animate([{ opacity: 1 }, { opacity: 0 }], { duration: fastMs(), easing: "ease-in", fill: "forwards" }).finished.then(done, done);
  };
  const action = (definition: ContextGuideDefinition) => {
    dismiss();
    if (definition.action === "tour") startTour();
    else if (definition.action === "guide" && definition.target !== null) openEncyclopedia(definition.target);
  };

  return <aside ref={card} className="vua-context-guide" aria-label={content.title} data-context-guide={definition.id}
    onKeyDown={event => { if (event.key === "Escape") { event.preventDefault(); dismiss(); } }}>
    <h2>{content.title}</h2>
    <p>{content.body}</p>
    <div className="vua-context-guide__actions">
      {definition.action !== "none" ? <Button variant={definition.action === "tour" ? "primary" : "default"} onClick={() => action(definition)}>{actionLabel}</Button> : null}
      <Button variant="subtle" onClick={dismiss}>{copy.dismiss}</Button>
    </div>
  </aside>;
}
