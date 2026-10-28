import { useId, useRef } from "react";
import { currentLocale, format } from "../../i18n/index.ts";
import { encyclopediaCopy } from "../../i18n/encyclopedia-copy.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { GuideEntryButton } from "../guide/GuideEntryButton.tsx";
import { ENCYCLOPEDIA_TOPIC_IDS, guidePage, type GuideTopicId } from "../guide/guide-content.ts";
import { GUIDE_ARTICLES, GUIDE_TASKS, searchGuideArticles, type GuideArticleKind } from "../guide/guide-catalog.ts";
import { openEncyclopedia } from "./encyclopedia-navigation.ts";

export function EncyclopediaHome({
  query,
  onQuery,
  topic,
  onTopic,
  kind,
  onKind,
  hasReading,
  onContinue,
  onClear,
}: {
  query: string;
  onQuery: (value: string) => void;
  topic: GuideTopicId | "all";
  onTopic: (value: GuideTopicId | "all") => void;
  kind: GuideArticleKind | "all";
  onKind: (value: GuideArticleKind | "all") => void;
  hasReading: boolean;
  onContinue: () => void;
  onClear: () => void;
}) {
  const copy = encyclopediaCopy[currentLocale];
  const inputRef = useRef<HTMLInputElement>(null);
  const searchId = useId();
  const tasksId = useId();
  const catalogId = useId();
  const resultsId = useId();
  const filtered = searchGuideArticles({ query, topic, kind });
  const active = query.trim() !== "" || topic !== "all" || kind !== "all";
  const clear = () => { onClear(); inputRef.current?.focus(); };
  return <div className="vua-encyclopedia-home">
    <h2>{copy.homeTitle}</h2>
    <div className="vua-encyclopedia-search" role="search">
      <div className="vua-encyclopedia-search__query">
        <label htmlFor={searchId}>{copy.searchLabel}</label>
        <input id={searchId} ref={inputRef} type="search" value={query} placeholder={copy.searchPlaceholder}
          autoComplete="off" aria-describedby={resultsId}
          onChange={event => onQuery(event.target.value)}
          onKeyDown={event => { if (event.key === "Escape" && query) { event.preventDefault(); onQuery(""); } }} />
      </div>
      <label>{copy.topicFilter}<select value={topic} onChange={event => onTopic(event.target.value as GuideTopicId | "all")}>
        <option value="all">{copy.allTopics}</option>
        {ENCYCLOPEDIA_TOPIC_IDS.map(id => <option key={id} value={id}>{guidePage(id).title}</option>)}
      </select></label>
      <label>{copy.kindFilter}<select value={kind} onChange={event => onKind(event.target.value as GuideArticleKind | "all")}>
        <option value="all">{copy.allKinds}</option>
        {Object.entries(copy.kinds).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
      </select></label>
      {active ? <Button variant="subtle" onClick={clear}>{copy.clearSearch}</Button> : null}
    </div>
    <p id={resultsId} className="vua-caption" role="status" aria-live="polite">{format(copy.resultCount, { count: filtered.length })}</p>
    {active ? filtered.length > 0 ? <ol className="vua-encyclopedia-results">
      {filtered.map(article => <li key={article.id}>
        <Button variant="subtle" className="vua-encyclopedia-result" data-guide-result={article.id} onClick={() => openEncyclopedia(article.target)}>
          <span className="vua-encyclopedia-result__kind">{copy.kinds[article.kind]} · {guidePage(article.target.topic).title}</span>
          <strong>{article.title}</strong>
          <span>{article.summary}</span>
        </Button>
      </li>)}
    </ol> : <div className="vua-encyclopedia-empty" role="status"><strong>{copy.noResultsTitle}</strong><p>{copy.noResultsBody}</p></div> : <>
      {hasReading ? <Button variant="subtle" data-guide-continue onClick={onContinue}>{copy.continueReading}</Button> : null}
      <section aria-labelledby={tasksId}>
        <h2 id={tasksId}>{copy.taskTitle}</h2>
        <div className="vua-encyclopedia-tasks">
          {GUIDE_TASKS.map(task => {
            const content = copy.tasks[task.id];
            return <Button variant="subtle" key={task.id} data-guide-task={task.id} onClick={() => task.target ? openEncyclopedia(task.target) : inputRef.current?.focus()}>
              <strong>{content.title}</strong><span>{content.body}</span>
            </Button>;
          })}
        </div>
      </section>
      <section aria-labelledby={catalogId}>
        <h2 id={catalogId}>{copy.catalogTitle}</h2>
        <ul className="vua-encyclopedia-tree">
          {ENCYCLOPEDIA_TOPIC_IDS.map((id, index) => {
            const articles = GUIDE_ARTICLES.filter(article => article.target.topic === id);
            return <li key={id}><details open={index === 0 ? true : undefined}>
              <summary><span>{guidePage(id).title}</span><span className="vua-caption">{articles.length}</span></summary>
              <ul>{articles.map(article => <li key={article.id}><GuideEntryButton target={article.target} label={article.title} /></li>)}</ul>
            </details></li>;
          })}
        </ul>
      </section>
    </>}
  </div>;
}
