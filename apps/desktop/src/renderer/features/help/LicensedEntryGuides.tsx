/** Lazy-loaded signed SOP guides (`guide-content v0.1`) for licensed asset entry and BOOTH setup.
 *  Zero startup calls: `guideContent.read(id)` runs only when the player opens or switches a topic. */
import { useEffect, useState } from "react";
import {
  GUIDE_CONTENT_IDS,
  isAllowedGuideHttpsUrl,
  resolveGuideLocalizedText,
  useGateway,
  type GuideContentIdV01,
  type GuideContentReadResultV01,
} from "../../gateway/index.ts";
import { openExternalUrl } from "../../app/open-external.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { currentLocale, format, strings } from "../../i18n/index.ts";
import "./help.css";

const copy = strings.guideChannel;

export function LicensedEntryGuides({
  initialId = null,
  topics = GUIDE_CONTENT_IDS,
}: {
  initialId?: GuideContentIdV01 | null;
  topics?: readonly GuideContentIdV01[];
}) {
  const gateway = useGateway();
  const port = gateway.guideContent;
  const [selectedId, setSelectedId] = useState<GuideContentIdV01 | null>(initialId);
  const [cache, setCache] = useState<Readonly<Partial<Record<GuideContentIdV01, GuideContentReadResultV01>>>>({});
  const [loading, setLoading] = useState(false);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (selectedId === null) return;
    if (cache[selectedId] !== undefined) {
      setFailed(false);
      return;
    }
    if (!port) {
      setFailed(true);
      return;
    }
    let active = true;
    setLoading(true);
    setFailed(false);
    void port.read(selectedId).then(
      result => {
        if (!active) return;
        setCache(previous => ({ ...previous, [selectedId]: result }));
        setLoading(false);
      },
      () => {
        if (!active) return;
        setFailed(true);
        setLoading(false);
      },
    );
    return () => {
      active = false;
    };
  }, [port, selectedId, cache]);

  const current = selectedId ? cache[selectedId] ?? null : null;
  const sop = current?.document.sop ?? null;
  const text = (localized: Parameters<typeof resolveGuideLocalizedText>[0]) =>
    resolveGuideLocalizedText(localized, currentLocale);

  return (
    <div className="vua-guide-channel" data-nav-id="licensed-entry-guides">
      <div className="vua-guide-channel__topics" role="group" aria-label={copy.sectionTitle}>
        {topics.map(id => (
          <Button
            key={id}
            variant={selectedId === id ? "primary" : "subtle"}
            aria-pressed={selectedId === id}
            data-guide-id={id}
            onClick={() => setSelectedId(prev => (prev === id && initialId === null ? null : id))}
          >
            {copy.topics[id]}
          </Button>
        ))}
      </div>
      {selectedId !== null && (
        <article className="vua-guide-channel__body" aria-live="polite">
          {loading && current === null ? (
            <p className="vua-caption vua-text-secondary">{copy.loading}</p>
          ) : failed || sop === null || current === null ? (
            <p className="vua-caption vua-text-secondary">{copy.unavailable}</p>
          ) : (
            <>
              <header className="vua-guide-channel__head">
                <h3>{text(sop.title)}</h3>
                <small className="vua-caption vua-text-secondary">
                  {format(copy.verifiedLine, {
                    date: sop.lastVerification.date,
                    source: copy.sources[current.source],
                  })}
                </small>
              </header>
              {sop.prerequisites.length > 0 && (
                <section>
                  <strong>{copy.prerequisites}</strong>
                  <ul className="vua-guide-channel__notes">
                    {sop.prerequisites.map((item, idx) => (
                      <li key={idx}>{text(item)}</li>
                    ))}
                  </ul>
                </section>
              )}
              <section>
                <strong>{copy.steps}</strong>
                <ol className="vua-guide-channel__steps">
                  {sop.steps.map(step => {
                    const safeLinks = step.links.filter(link => isAllowedGuideHttpsUrl(link.url));
                    return (
                      <li key={step.id} className="vua-guide-channel__step">
                        <span className="vua-guide-channel__badge">{copy.performers[step.performer]}</span>
                        <p>{text(step.action)}</p>
                        <small className="vua-caption vua-text-secondary">
                          {format(copy.expected, { text: text(step.expectedResult) })}
                        </small>
                        <small className="vua-caption vua-text-secondary">
                          {format(copy.check, { text: text(step.verification) })}
                        </small>
                        <small className="vua-caption vua-text-secondary">
                          {format(copy.ifBlocked, { text: text(step.onFailure) })}
                        </small>
                        {safeLinks.length > 0 && (
                          <div className="vua-guide-channel__links">
                            {safeLinks.map(link => (
                              <Button
                                key={link.url}
                                variant="subtle"
                                onClick={() => openExternalUrl(link.url)}
                              >
                                {text(link.label)}
                              </Button>
                            ))}
                          </div>
                        )}
                      </li>
                    );
                  })}
                </ol>
              </section>
              {sop.regionalNotes.length > 0 && (
                <section>
                  <strong>{copy.regionalTitle}</strong>
                  <ul className="vua-guide-channel__notes">
                    {sop.regionalNotes.map((note, idx) => (
                      <li key={`${note.region}-${idx}`}>{text(note.text)}</li>
                    ))}
                  </ul>
                </section>
              )}
              {sop.sources.some(src => isAllowedGuideHttpsUrl(src.url)) && (
                <section>
                  <strong>{copy.officialSources}</strong>
                  <div className="vua-guide-channel__links">
                    {sop.sources
                      .filter(src => isAllowedGuideHttpsUrl(src.url))
                      .map(src => (
                        <Button
                          key={src.url}
                          variant="subtle"
                          onClick={() => openExternalUrl(src.url)}
                        >
                          {src.label}
                        </Button>
                      ))}
                  </div>
                </section>
              )}
              {initialId === null && (
                <div>
                  <Button variant="subtle" onClick={() => setSelectedId(null)}>
                    {copy.closeGuide}
                  </Button>
                </div>
              )}
            </>
          )}
        </article>
      )}
    </div>
  );
}
