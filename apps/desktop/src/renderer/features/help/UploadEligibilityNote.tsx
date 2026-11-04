import { strings } from "../../i18n/index.ts";
import { GuideEntryButton } from "../guide/GuideEntryButton.tsx";
import { GUIDE_TARGETS } from "../guide/guide-target.ts";

export function UploadEligibilityNote() {
  const copy = strings.helpFaq;
  return <section className="vua-upload-eligibility" aria-label={copy.eligibilityTitle}>
    <h2>{copy.eligibilityTitle}</h2>
    <p>{copy.eligibility}</p>
    <div className="vua-upload-eligibility__actions">
      <GuideEntryButton target={GUIDE_TARGETS.trustRank} label={copy.trust.what} />
      <GuideEntryButton target={GUIDE_TARGETS.improveTrust} label={copy.trust.improve} />
    </div>
  </section>;
}
