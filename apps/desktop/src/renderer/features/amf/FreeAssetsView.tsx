import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from "react";
import { Icon } from "@vua/design-system";
import {
  FREE_ASSET_CATEGORY_IDS_V01,
  FREE_ASSET_MAX_ENABLED_KEYWORDS_V01,
  FREE_ASSET_MAX_STORED_KEYWORDS_V01,
  type FreeAssetCategoryIdV01,
  type FreeAssetsDesktopStatusV01,
  type FreeAssetsStateV01,
} from "@vua/contracts";
import { catalogImageUrl } from "../../app/catalog-image.ts";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { EmptyState } from "../../components/primitives/EmptyState.tsx";
import { IDLE_FREE_ASSETS_STATUS, type FreeAssetsPort } from "../../gateway/index.ts";
import { boothCategoryLabel, format, formatDateMinute, strings } from "../../i18n/index.ts";
import {
  addWatchedKeyword,
  formatCooldownRemaining,
  projectFreeAssetsView,
  removeWatchedKeyword,
  toggleEnabledCategory,
  toggleWatchedKeyword,
} from "./free-assets-model.ts";
import "./free-assets.css";

export function FreeAssetsView({
  port,
  refreshToken,
  banner,
  onBrowseSites,
  onOpenBoothItem,
}: {
  port: FreeAssetsPort | undefined;
  refreshToken: number;
  banner?: ReactNode;
  onBrowseSites: () => void;
  onOpenBoothItem: (productId: string) => void;
}) {
  const copy = strings.freeAssets;
  const [state, setState] = useState<FreeAssetsStateV01 | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [status, setStatus] = useState<FreeAssetsDesktopStatusV01>(IDLE_FREE_ASSETS_STATUS);
  const [keywordInput, setKeywordInput] = useState("");
  const [localNotice, setLocalNotice] = useState<string | null>(null);
  const [nowMs, setNowMs] = useState(() => Date.now());
  const prevStatusRef = useRef<FreeAssetsDesktopStatusV01>(IDLE_FREE_ASSETS_STATUS);

  const reloadState = useCallback(async () => {
    if (!port) {
      setState(null);
      setLoaded(true);
      return;
    }
    const next = await port.state();
    setState(next);
    setLoaded(true);
  }, [port]);

  useEffect(() => {
    void reloadState();
  }, [reloadState, refreshToken]);

  useEffect(() => {
    if (!port) return undefined;
    let mounted = true;
    void port.status().then(initial => {
      if (!mounted) return;
      prevStatusRef.current = initial;
      setStatus(initial);
    });
    const unsubscribe = port.subscribeStatus(next => {
      if (!mounted) return;
      const previous = prevStatusRef.current;
      prevStatusRef.current = next;
      setStatus(next);
      if (next.completedKeywords !== previous.completedKeywords || (previous.running && !next.running)) {
        void port.state().then(updated => {
          if (mounted && updated !== null) setState(updated);
        });
      }
    });
    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [port]);

  useEffect(() => {
    if (status.cooldownUntilMs === null || status.cooldownUntilMs <= Date.now()) return undefined;
    setNowMs(Date.now());
    const timer = window.setInterval(() => {
      const current = Date.now();
      setNowMs(current);
      if (status.cooldownUntilMs !== null && current >= status.cooldownUntilMs) {
        window.clearInterval(timer);
      }
    }, 1000);
    return () => window.clearInterval(timer);
  }, [status.cooldownUntilMs]);

  const projection = useMemo(
    () => (state ? projectFreeAssetsView(state) : null),
    [state],
  );

  const cooldownText = formatCooldownRemaining(status.cooldownUntilMs, nowMs);

  const applyPreferenceUpdate = async (patch: Parameters<FreeAssetsPort["updatePreferences"]>[0]) => {
    if (!port) return;
    const updated = await port.updatePreferences(patch);
    if (updated !== null) setState(updated);
  };

  const handleAddKeywordValue = async (rawValue: string) => {
    if (!state) return;
    const outcome = addWatchedKeyword(state.watchedKeywords, rawValue);
    if (outcome.kind === "invalid" || outcome.kind === "noop") return;
    if (outcome.kind === "limit_reached") {
      setLocalNotice(format(copy.keywordLimitReached, { max: FREE_ASSET_MAX_STORED_KEYWORDS_V01 }));
      return;
    }
    setLocalNotice(
      outcome.addedPaused
        ? format(copy.enabledLimitHint, { max: FREE_ASSET_MAX_ENABLED_KEYWORDS_V01 })
        : null,
    );
    await applyPreferenceUpdate({ watchedKeywords: outcome.watchedKeywords });
  };

  const handleSubmitKeyword = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const submitted = keywordInput;
    if (submitted.trim() === "") return;
    setKeywordInput("");
    void handleAddKeywordValue(submitted);
  };

  const handleToggleKeyword = async (keyword: string) => {
    if (!state) return;
    const outcome = toggleWatchedKeyword(state.watchedKeywords, keyword);
    if (outcome.kind === "enabled_limit") {
      setLocalNotice(format(copy.enabledLimitHint, { max: FREE_ASSET_MAX_ENABLED_KEYWORDS_V01 }));
      return;
    }
    if (outcome.kind === "updated") {
      setLocalNotice(null);
      await applyPreferenceUpdate({ watchedKeywords: outcome.watchedKeywords });
    }
  };

  const handleRemoveKeyword = async (keyword: string) => {
    if (!state) return;
    setLocalNotice(null);
    await applyPreferenceUpdate({
      watchedKeywords: removeWatchedKeyword(state.watchedKeywords, keyword),
    });
  };

  const handleToggleCategory = async (categoryId: FreeAssetCategoryIdV01) => {
    if (!state) return;
    await applyPreferenceUpdate({
      enabledCategories: toggleEnabledCategory(state.enabledCategories, categoryId),
    });
  };

  const handleQuery = async () => {
    if (!port) return;
    setLocalNotice(null);
    const next = await port.query();
    prevStatusRef.current = next;
    setStatus(next);
  };

  return (
    <div className="vua-free-assets" data-free-assets-view>
      <header className="vua-page__hero">
        <h1 className="vua-title">{copy.title}</h1>
        <p className="vua-caption vua-text-secondary">{copy.subtitle}</p>
      </header>

      {banner}

      <div className="vua-free-assets__toolbar">
        <div className="vua-free-assets__toolbar-start">
          <Button variant="subtle" data-free-assets-back onClick={onBrowseSites}>
            <Icon name="arrow-left" size={16} />
            <span>{copy.browseSites}</span>
          </Button>
          {state !== null ? (
            <span className="vua-free-assets__meta">
              {state.lastQueriedAt
                ? format(copy.lastQueried, { time: formatDateMinute(state.lastQueriedAt) })
                : copy.neverQueried}
              {projection ? ` · ${format(copy.resultSummary, { visible: projection.visibleItems.length })}` : ""}
            </span>
          ) : null}
        </div>
        <div className="vua-free-assets__toolbar-end">
          {status.running ? (
            <>
              <span className="vua-free-assets__progress" role="status">
                {status.currentKeyword
                  ? format(copy.runningProgress, {
                      done: Math.min(status.completedKeywords + 1, Math.max(1, status.totalKeywords)),
                      total: Math.max(1, status.totalKeywords),
                      keyword: status.currentKeyword,
                    })
                  : format(copy.runningPreparing, {
                      done: status.completedKeywords,
                      total: Math.max(1, status.totalKeywords),
                    })}
              </span>
              <Button variant="subtle" data-free-assets-cancel onClick={() => void port?.cancel()}>
                {copy.cancelAction}
              </Button>
            </>
          ) : cooldownText !== null ? (
            <Button variant="primary" disabled data-free-assets-query>
              <Icon name="clock" size={16} />
              <span>{format(copy.cooldownAction, { time: cooldownText })}</span>
            </Button>
          ) : (
            <Button
              variant="primary"
              data-free-assets-query
              disabled={!port || !projection || projection.enabledKeywordCount === 0}
              onClick={() => void handleQuery()}
            >
              <Icon name="refresh" size={16} />
              <span>{copy.queryAction}</span>
            </Button>
          )}
        </div>
      </div>

      {status.lastError !== null ? (
        <div className="vua-free-assets__notice" data-tone="error" role="status">
          <span>{copy.errors[status.lastError]}</span>
          {status.lastError === "sign_in_required" ? (
            <Button variant="subtle" onClick={onBrowseSites}>{copy.browseSites}</Button>
          ) : null}
        </div>
      ) : null}

      {localNotice !== null ? (
        <div className="vua-free-assets__notice" role="status">
          <span>{localNotice}</span>
        </div>
      ) : null}

      {!loaded ? null : state === null || projection === null ? (
        <EmptyState
          title={copy.unavailableTitle}
          description={copy.unavailableBody}
          action={<Button variant="subtle" onClick={() => void reloadState()}>{strings.browser.retry}</Button>}
        />
      ) : (
        <>
          <Card className="vua-free-assets__panel">
            <div className="vua-free-assets__panel-head">
              <h2 className="vua-free-assets__panel-title">{copy.keywordsLabel}</h2>
              <span className="vua-free-assets__meta">
                {format(copy.keywordsHint, {
                  enabled: projection.enabledKeywordCount,
                  max: FREE_ASSET_MAX_ENABLED_KEYWORDS_V01,
                })}
              </span>
            </div>
            {state.watchedKeywords.length > 0 ? (
              <div className="vua-free-assets__chips">
                {state.watchedKeywords.map(entry => (
                  <span
                    key={entry.keyword}
                    className="vua-free-assets__keyword-chip"
                    data-enabled={entry.enabled ? "true" : "false"}
                  >
                    <button
                      type="button"
                      className="vua-free-assets__keyword-toggle"
                      aria-pressed={entry.enabled}
                      onClick={() => void handleToggleKeyword(entry.keyword)}
                    >
                      {entry.enabled ? <Icon name="check" size={16} /> : null}
                      <span>{entry.keyword}</span>
                    </button>
                    <button
                      type="button"
                      className="vua-free-assets__keyword-remove"
                      aria-label={format(copy.removeKeyword, { keyword: entry.keyword })}
                      title={format(copy.removeKeyword, { keyword: entry.keyword })}
                      onClick={() => void handleRemoveKeyword(entry.keyword)}
                    >
                      <Icon name="close" size={16} />
                    </button>
                  </span>
                ))}
              </div>
            ) : null}
            {projection.availableSuggestions.length > 0 ? (
              <div className="vua-free-assets__chips">
                <span className="vua-free-assets__meta">{copy.suggestedLabel}</span>
                {projection.availableSuggestions.map(keyword => (
                  <button
                    key={keyword}
                    type="button"
                    className="vua-free-assets__pill"
                    onClick={() => void handleAddKeywordValue(keyword)}
                  >
                    <Icon name="add" size={16} />
                    <span>{keyword}</span>
                  </button>
                ))}
              </div>
            ) : null}
            <form className="vua-free-assets__form" onSubmit={handleSubmitKeyword}>
              <input
                type="text"
                className="vua-free-assets__input"
                maxLength={80}
                value={keywordInput}
                placeholder={copy.keywordPlaceholder}
                aria-label={copy.keywordPlaceholder}
                onChange={event => setKeywordInput(event.target.value)}
              />
              <Button type="submit" variant="subtle" disabled={keywordInput.trim() === ""}>
                <Icon name="add" size={16} />
                <span>{copy.addKeyword}</span>
              </Button>
            </form>
          </Card>

          <Card className="vua-free-assets__panel">
            <div className="vua-free-assets__filters-row">
              <div className="vua-free-assets__chips" role="group" aria-label={copy.categoriesLabel}>
                {FREE_ASSET_CATEGORY_IDS_V01.map(categoryId => {
                  const active = state.enabledCategories.includes(categoryId);
                  return (
                    <button
                      key={categoryId}
                      type="button"
                      className="vua-free-assets__pill"
                      aria-pressed={active}
                      onClick={() => void handleToggleCategory(categoryId)}
                    >
                      <span>{copy.categories[categoryId]}</span>
                    </button>
                  );
                })}
              </div>
              <div className="vua-free-assets__chips">
                <button
                  type="button"
                  className="vua-free-assets__pill"
                  aria-pressed={state.hideOwned}
                  onClick={() => void applyPreferenceUpdate({ hideOwned: !state.hideOwned })}
                >
                  <span>{copy.hideOwned}</span>
                </button>
                <button
                  type="button"
                  className="vua-free-assets__pill"
                  aria-pressed={state.showSamples}
                  onClick={() => void applyPreferenceUpdate({ showSamples: !state.showSamples })}
                >
                  <span>{format(copy.showSamples, { count: projection.sampleCount })}</span>
                </button>
                {projection.dismissedCount > 0 ? (
                  <Button
                    variant="subtle"
                    onClick={() => void applyPreferenceUpdate({ clearDismissed: true })}
                  >
                    {format(copy.restoreDismissed, { count: projection.dismissedCount })}
                  </Button>
                ) : null}
              </div>
            </div>
          </Card>

          {projection.visibleItems.length > 0 ? (
            <div className="vua-free-assets__grid">
              {projection.visibleItems.map(item => {
                const categoryName = item.categoryId
                  ? copy.categories[item.categoryId]
                  : item.categoryLabel
                    ? boothCategoryLabel(item.categoryLabel)
                    : null;
                return (
                  <article
                    key={item.productId}
                    className="vua-free-assets__card"
                    data-product-id={item.productId}
                    data-owned={item.owned ? "true" : undefined}
                  >
                    <button
                      type="button"
                      className="vua-free-assets__card-dismiss"
                      aria-label={format(copy.dismissItem, { title: item.title })}
                      title={format(copy.dismissItem, { title: item.title })}
                      onClick={event => {
                        event.stopPropagation();
                        void applyPreferenceUpdate({ dismissProductId: item.productId });
                      }}
                    >
                      <Icon name="close" size={16} />
                    </button>
                    <button
                      type="button"
                      className="vua-free-assets__card-main"
                      onClick={() => onOpenBoothItem(item.productId)}
                    >
                      <div className="vua-free-assets__thumb-wrap">
                        {item.thumbnailUrl ? (
                          <img
                            className="vua-free-assets__thumb"
                            src={catalogImageUrl(item.thumbnailUrl)}
                            alt=""
                            loading="lazy"
                          />
                        ) : (
                          <div className="vua-free-assets__thumb-placeholder">
                            <Icon name="outfit" size={24} />
                          </div>
                        )}
                      </div>
                      <div className="vua-free-assets__card-body">
                        <div className="vua-free-assets__badges">
                          <Badge tone="brand">{copy.freeBadge}</Badge>
                          {item.owned ? <Badge tone="success">{copy.ownedBadge}</Badge> : null}
                          {item.isSample ? <Badge tone="neutral">{copy.sampleBadge}</Badge> : null}
                          {item.hasDateCue ? <Badge tone="warning">{copy.dateCueBadge}</Badge> : null}
                        </div>
                        <h3 className="vua-free-assets__card-title" title={item.title}>
                          {item.title}
                        </h3>
                        {item.shopName || categoryName ? (
                          <p className="vua-free-assets__card-sub">
                            {[item.shopName, categoryName].filter(Boolean).join(" · ")}
                          </p>
                        ) : null}
                        <div className="vua-free-assets__chips">
                          {item.matchedKeywords.map(keyword => (
                            <Badge key={keyword} tone="neutral">{keyword}</Badge>
                          ))}
                        </div>
                        <div className="vua-free-assets__card-foot">
                          <span>{copy.openOnBooth}</span>
                          <Icon name="arrow-right" size={16} />
                        </div>
                      </div>
                    </button>
                  </article>
                );
              })}
            </div>
          ) : state.watchedKeywords.length === 0 ? (
            <EmptyState
              title={copy.emptyNoKeywordsTitle}
              description={copy.emptyNoKeywordsBody}
            />
          ) : state.lastQueriedAt === null ? (
            <EmptyState
              title={copy.emptyNotQueriedTitle}
              description={copy.emptyNotQueriedBody}
            />
          ) : (
            <EmptyState
              title={copy.emptyFilteredTitle}
              description={copy.emptyFilteredBody}
            />
          )}
        </>
      )}
    </div>
  );
}
