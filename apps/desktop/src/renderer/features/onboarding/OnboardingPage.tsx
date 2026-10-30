import type { EnvGoalId, GoalId } from "../../app/onboarding-model.ts";
import type { PageId } from "../../app/nav-model.ts";
import { appMeta } from "../../app/app-meta.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { currentLocale } from "../../i18n/index.ts";
import { firstUseCopy } from "../../i18n/first-use-copy.ts";
import "./onboarding.css";

export interface OnboardingResult {
  status: "completed" | "skipped";
  goals: GoalId[];
  environments: EnvGoalId[];
  page?: PageId;
  tour?: boolean;
}

export function OnboardingPage({ onComplete }: {
  onComplete: (result: OnboardingResult) => void;
}) {
  const copy = firstUseCopy[currentLocale].notice;
  return <div className="vua-onboarding" data-first-use-notice>
    <div className="vua-onboarding__panel">
      <header className="vua-onboarding__header">
        <h1 className="vua-display">{copy.title}</h1>
      </header>
      <section className="vua-onboarding__part" data-first-use-part="purpose">
        <h2>{copy.purposeTitle}</h2><p>{copy.purpose}</p>
      </section>
      <section className="vua-onboarding__part" data-first-use-part="changelog">
        <h2>{copy.changesTitle}</h2><p>{copy.changes}</p>
        <a href={appMeta.repoUrl + "/releases"} target="_blank" rel="noopener noreferrer">{copy.changelog}</a>
      </section>
      <section className="vua-onboarding__part" data-first-use-part="source">
        <h2>{copy.sourceTitle}</h2><p>{copy.source}</p>
        <a href={appMeta.repoUrl} target="_blank" rel="noopener noreferrer">{copy.github}</a>
      </section>
      <section className="vua-onboarding__part" data-first-use-part="licenses">
        <h2>{copy.licensesTitle}</h2><p>{copy.licenses}</p>
        <a href={appMeta.repoUrl + "/blob/main/THIRD_PARTY_NOTICES.md"} target="_blank" rel="noopener noreferrer">{copy.licensesLink}</a>
      </section>
      <footer className="vua-onboarding__footer">
        <Button variant="subtle" data-nav-id="first-use-beginner" onClick={() => onComplete({
          status: "completed", goals: [], environments: [], page: "help-wizard",
        })}>{copy.beginner}</Button>
        <Button variant="primary" data-nav-id="first-use-enter" onClick={() => onComplete({
          status: "completed", goals: [], environments: [], page: "home", tour: true,
        })}>{copy.enter}</Button>
      </footer>
    </div>
  </div>;
}
