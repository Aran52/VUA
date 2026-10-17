/** The album: one spread per finished Avatar. A page turns over the spine like a photo album; with
 * reduced motion, or on a narrow window where the pages stack, the next spread fades in. */
import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type PointerEvent, type ReactNode, type RefObject } from "react";
import { Icon } from "@vua/design-system";
import { useSceneMode } from "../../../components/three/useSceneMode.ts";
import { format, strings } from "../../../i18n/index.ts";
import { dragProgress, easeInOut, turnCompletes, turnTo, type Turn, type TurnDirection } from "./release-model.ts";

const copy = strings.releaseRoom;
const TURN_MS = 720;
const NARROW_PX = 860;

export interface AlbumSpread {
  readonly key: string;
  readonly tab: string;
  readonly left: (live: boolean) => ReactNode;
  readonly right: (live: boolean) => ReactNode;
}

function Page({ side, children, inert }: { side: "left" | "right"; children: ReactNode; inert?: boolean }) {
  return <section className="vua-album__page" data-side={side} inert={inert || undefined} aria-hidden={inert || undefined}>{children}</section>;
}

/** The open album at one spread, or with one leaf turning from `turn.from` to `turn.to`. */
export function AlbumBook({ spreads, index, turn, progress, bookRef, onCorner, fadeKey, onStep }: {
  spreads: readonly AlbumSpread[]; index: number; turn: Turn | null; progress: number;
  bookRef?: RefObject<HTMLDivElement | null>; onCorner?: (direction: TurnDirection, event: PointerEvent<HTMLButtonElement>) => void; fadeKey?: number;
  /** Keyboard activation of a page corner. */
  onStep?: (steps: number) => void;
}) {
  const current = spreads[index];
  if (!current) return null;
  const style = { "--turn": progress } as CSSProperties;
  if (turn === null) {
    return <div className="vua-album__book" ref={bookRef} style={style} key={fadeKey} data-fade={fadeKey ? "" : undefined}>
      <Page side="left">{current.left(true)}</Page>
      <Page side="right">{current.right(true)}</Page>
      {onCorner && index > 0 && <button type="button" className="vua-album__dogear" data-side="left" aria-label={copy.previous} onPointerDown={event => onCorner("prev", event)} onClick={() => onStep?.(-1)} />}
      {onCorner && index < spreads.length - 1 && <button type="button" className="vua-album__dogear" data-side="right" aria-label={copy.next} onPointerDown={event => onCorner("next", event)} onClick={() => onStep?.(1)} />}
    </div>;
  }
  const target = spreads[turn.to] ?? current;
  const next = turn.direction === "next";
  return <div className="vua-album__book" ref={bookRef} style={style} data-turning={turn.direction}>
    <Page side="left" inert>{(next ? current : target).left(false)}</Page>
    <Page side="right" inert>{(next ? target : current).right(false)}</Page>
    <div className="vua-album__leaf" data-direction={turn.direction} aria-hidden="true">
      <div className="vua-album__face" data-face="front" inert>
        <Page side={next ? "right" : "left"}>{next ? current.right(false) : current.left(false)}</Page>
      </div>
      <div className="vua-album__face" data-face="back" inert>
        <Page side={next ? "left" : "right"}>{next ? target.left(false) : target.right(false)}</Page>
      </div>
    </div>
  </div>;
}

export function ReleaseAlbum({ spreads, index, onIndex }: { spreads: readonly AlbumSpread[]; index: number; onIndex: (index: number) => void }) {
  const animated = useSceneMode() === "animated";
  const root = useRef<HTMLElement>(null);
  const book = useRef<HTMLDivElement>(null);
  const [narrow, setNarrow] = useState(false);
  const [turn, setTurn] = useState<Turn | null>(null);
  const [fadeKey, setFadeKey] = useState(0);
  const progress = useRef(0);
  const frame = useRef(0);
  const drag = useRef<{ pointer: number; startX: number; width: number } | null>(null);
  const chosen = useRef(onIndex);
  chosen.current = onIndex;

  useEffect(() => {
    const element = root.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => setNarrow((entry?.contentRect.width ?? NARROW_PX) < NARROW_PX));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  useEffect(() => () => cancelAnimationFrame(frame.current), []);

  const paint = (value: number) => { progress.current = value; book.current?.style.setProperty("--turn", String(value)); };
  const settle = useCallback((to: number, done: () => void) => {
    cancelAnimationFrame(frame.current);
    const from = progress.current, start = performance.now(), length = Math.max(120, TURN_MS * Math.abs(to - from));
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / length);
      paint(from + (to - from) * easeInOut(t));
      if (t < 1) frame.current = requestAnimationFrame(step); else done();
    };
    frame.current = requestAnimationFrame(step);
  }, []);
  const finish = useCallback((done: Turn, completed: boolean) => {
    if (completed) chosen.current(done.to);
    setTurn(null); paint(0);
  }, []);

  const go = (target: number) => {
    if (turn !== null) return;
    const next = turnTo(index, target, spreads.length);
    if (!next) return;
    if (!animated || narrow) { onIndex(target); setFadeKey(key => key + 1); return; }
    paint(0); setTurn(next);
  };
  // A turn started by a button or key plays once its leaf is on screen; a dragged turn follows the pointer.
  useLayoutEffect(() => {
    if (turn === null || drag.current !== null) return;
    settle(1, () => finish(turn, true));
  }, [turn, settle, finish]);

  const onCorner = (direction: TurnDirection, event: PointerEvent<HTMLButtonElement>) => {
    const target = turnTo(index, index + (direction === "next" ? 1 : -1), spreads.length);
    if (!target) return;
    // Without a turning leaf the corner's click turns the page.
    if (!animated || narrow) return;
    const width = book.current?.getBoundingClientRect().width ?? 1;
    root.current?.setPointerCapture(event.pointerId);
    drag.current = { pointer: event.pointerId, startX: event.clientX, width };
    paint(0); setTurn(target);
  };
  const onPointerMove = (event: PointerEvent<HTMLElement>) => {
    const current = drag.current;
    if (!current || current.pointer !== event.pointerId || turn === null) return;
    paint(dragProgress(turn.direction, current.startX, event.clientX, current.width));
  };
  const onPointerUp = (event: PointerEvent<HTMLElement>) => {
    const current = drag.current;
    if (!current || current.pointer !== event.pointerId || turn === null) return;
    drag.current = null;
    // A tap on the corner turns the page; a drag completes past a third of the way.
    const complete = Math.abs(event.clientX - current.startX) < 6 || turnCompletes(progress.current);
    const active = turn;
    settle(complete ? 1 : 0, () => finish(active, complete));
  };
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const target = event.target as HTMLElement;
    if (target.closest("input, textarea, select, [contenteditable='true']")) return;
    if (event.key === "ArrowRight" || event.key === "PageDown") { event.preventDefault(); go(index + 1); }
    if (event.key === "ArrowLeft" || event.key === "PageUp") { event.preventDefault(); go(index - 1); }
  };

  return <section ref={root} className="vua-album" data-narrow={narrow || undefined} aria-label={copy.albumLabel} aria-roledescription="carousel"
    onKeyDown={onKeyDown} onPointerMove={onPointerMove} onPointerUp={onPointerUp} onPointerCancel={onPointerUp}>
    <div className="vua-album__binder">
      <AlbumBook spreads={spreads} index={index} turn={turn} progress={progress.current} bookRef={book} onCorner={onCorner} fadeKey={fadeKey} onStep={steps => go(index + steps)} />
      {spreads.length > 1 && <ol className="vua-album__tabs" aria-label={copy.albumLabel}>
        {spreads.map((spread, at) => <li key={spread.key}><button type="button" aria-current={at === index || undefined} title={spread.tab}
          onClick={() => go(at)}><span>{String(at + 1).padStart(2, "0")}</span>{spread.tab}</button></li>)}
      </ol>}
    </div>
    <nav className="vua-album__nav" aria-label={copy.albumLabel}>
      <button type="button" onClick={() => go(index - 1)} disabled={index === 0 || turn !== null} aria-label={copy.previous}><Icon name="arrow-left" size={16} /></button>
      <span aria-live="polite">{format(copy.pageOf, { page: index + 1, total: spreads.length })}</span>
      <button type="button" onClick={() => go(index + 1)} disabled={index >= spreads.length - 1 || turn !== null} aria-label={copy.next}><Icon name="arrow-right" size={16} /></button>
      {spreads.length > 1 && <small className="vua-text-secondary">{copy.turnHint}</small>}
    </nav>
  </section>;
}
