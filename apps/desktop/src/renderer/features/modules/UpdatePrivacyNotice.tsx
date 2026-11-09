import { currentLocale } from "../../i18n/index.ts";
import { updatePrivacyCopy } from "../../i18n/update-privacy-copy.ts";

export function UpdatePrivacyNotice() {
  const copy = updatePrivacyCopy[currentLocale];
  return <section data-update-privacy><h2>{copy.title}</h2><p>{copy.text}</p>
    <a href="https://ph-r.com/vua/downloads/#update-privacy" target="_blank" rel="noopener noreferrer">{copy.link}</a>
  </section>;
}
