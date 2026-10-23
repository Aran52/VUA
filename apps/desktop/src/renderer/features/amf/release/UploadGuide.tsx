/** The VRChat SDK panel's build-and-upload in four steps, for a first self-upload
 * or for troubleshooting a managed one: one sentence per step plus the slot for
 * its screenshot. The slots stay empty placeholders — no fake SDK UI is drawn.
 *
 * Screenshot list for the author (VRChat SDK 3.10.5 panel, one image per step):
 * 1. Builder tab: the Avatar row selected, the Avatar's name visible.
 * 2. Builder tab: the Build & Publish button; a new Avatar's Name field and its
 *    thumbnail slot filled in.
 * 3. Builder tab, the validation area: one red error and one yellow warning.
 * 4. The content-rights confirmation and the upload progress bar.
 * Drop the four images in as <img> (or MediaSlot) inside each .vua-upload-guide__shot.
 */
import { strings } from "../../../i18n/index.ts";

const copy = strings.releaseRoom.uploadGuide;

/** Jump links from the check items to the matching step. */
export function scrollToUploadGuide(step: 1 | 2 | 3 | 4): void {
  document.getElementById(`release-upload-guide-step-${step}`)?.scrollIntoView({ block: "center" });
}

export function UploadGuide() {
  const steps = [copy.step1, copy.step2, copy.step3, copy.step4];
  return <section className="vua-album__block vua-upload-guide" id="release-upload-guide" aria-label={copy.title}>
    <h3>{copy.title}</h3>
    <p className="vua-caption vua-text-secondary">{copy.intro}</p>
    <ol className="vua-upload-guide__steps">{steps.map((step, index) => <li key={index} id={`release-upload-guide-step-${index + 1}`}>
      <strong>{step.title}</strong>
      <p>{step.text}</p>
      <div className="vua-upload-guide__shot" role="img" aria-label={step.shot}>
        <span aria-hidden="true">{copy.shotPlaceholder}</span>
      </div>
    </li>)}</ol>
  </section>;
}
