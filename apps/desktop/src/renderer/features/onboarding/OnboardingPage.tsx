import { useState } from "react";
import type { EnvGoalId, GoalId } from "../../app/onboarding-model.ts";
import type { PageId } from "../../app/nav-model.ts";
import { storageKeys } from "../../app/storage-keys.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { currentLocale, localeRegistry, strings, type LocaleId } from "../../i18n/index.ts";
import "./onboarding.css";

export interface OnboardingResult {
  status: "completed" | "skipped";
  goals: GoalId[];
  environments: EnvGoalId[];
  page?: PageId;
}

export function OnboardingPage({ onComplete }: {
  onComplete: (result: OnboardingResult) => void;
}) {
  const [locale, setLocale] = useState<LocaleId>(currentLocale);
  const copy = strings.onboarding;
  return <div className="vua-onboarding" data-first-run-language>
    <div className="vua-onboarding__panel">
      <header className="vua-onboarding__header">
        <h1 className="vua-display">{copy.firstRunTitle}</h1>
        <p>{copy.firstRunBody}</p>
      </header>
      <label className="vua-onboarding__language">
        <span>{strings.settings.language.aria}</span>
        <select className="vua-settings-select" aria-label={strings.settings.language.aria} value={locale} onChange={event => setLocale(event.target.value as LocaleId)}>
          {localeRegistry.filter(entry => entry.available).map(entry => <option key={entry.id} value={entry.id}>{entry.endonym}</option>)}
        </select>
      </label>
      <footer className="vua-onboarding__footer">
        <Button variant="subtle" onClick={() => onComplete({ status: "skipped", goals: [], environments: [], page: "home" })}>{copy.skip}</Button>
        <Button variant="primary" data-nav-id="first-run-language-continue" onClick={() => {
          try { localStorage.setItem(storageKeys.locale, locale); } catch { /* The current table remains available. */ }
          onComplete({ status: "completed", goals: [], environments: [], page: "home" });
          window.location.reload();
        }}>{copy.continue}</Button>
      </footer>
    </div>
  </div>;
}
