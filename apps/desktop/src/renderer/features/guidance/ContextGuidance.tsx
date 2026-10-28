import { useEffect, useState } from "react";
import type { PageId } from "../../app/nav-model.ts";
import { strings } from "../../i18n/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { openEncyclopedia } from "../help/encyclopedia-navigation.ts";
import {
  contextGuideForPage,
  dismissContextGuide,
  readContextGuideSnoozed,
  readContextGuideState,
  snoozeContextGuide,
  type ContextGuideDefinition,
} from "./context-guidance.ts";
import "./context-guidance.css";

export function ContextGuidance({ page, startTour, suppressed = false, replayToken = 0 }: {
  page: PageId;
  startTour: () => void;
  suppressed?: boolean;
  replayToken?: number;
}) {
  const definition = contextGuideForPage(page);
  const [dismissed, setDismissed] = useState(() => readContextGuideState(localStorage).dismissed);
  const [snoozed, setSnoozed] = useState(() => readContextGuideSnoozed(sessionStorage));
  useEffect(() => {
    setDismissed(readContextGuideState(localStorage).dismissed);
    setSnoozed(readContextGuideSnoozed(sessionStorage));
  }, [replayToken]);

  if (suppressed || definition === null || (dismissed[definition.id] ?? 0) >= definition.revision || snoozed.includes(definition.id)) return null;
  const copy = strings.contextGuide;
  const content = copy[definition.id];
  const actionLabel = content.action || strings.helpFaq.uploadAction;
  const dismiss = () => setDismissed(dismissContextGuide(localStorage, definition).dismissed);
  const later = () => setSnoozed(snoozeContextGuide(sessionStorage, definition.id));
  const action = (definition: ContextGuideDefinition) => {
    dismiss();
    if (definition.action === "tour") startTour();
    else if (definition.action === "guide" && definition.target !== null) openEncyclopedia(definition.target);
  };

  return <aside className="vua-context-guide" aria-label={content.title} data-context-guide={definition.id}
    onKeyDown={event => { if (event.key === "Escape") { event.preventDefault(); later(); } }}>
    <h2>{content.title}</h2>
    <p>{content.body}</p>
    <div className="vua-context-guide__actions">
      {definition.action !== "none" ? <Button variant={definition.action === "tour" ? "primary" : "default"} onClick={() => action(definition)}>{actionLabel}</Button> : null}
      <Button variant="subtle" onClick={later}>{copy.later}</Button>
      <Button variant="subtle" onClick={dismiss}>{copy.dismiss}</Button>
    </div>
  </aside>;
}
