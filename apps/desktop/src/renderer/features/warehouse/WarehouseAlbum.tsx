/**
 * Warehouse 相册(G8 增量,用户反馈 #3):多图商品的浏览组件。
 *
 * 两种交互(同一数据源 imageUrls,详情媒体的完整图片数组):
 * - CardAlbumMedia(卡片):悬停 CARD_ALBUM_HOVER_DELAY_MS 后进入相册模式,
 *   光标在媒体区内的横向位置映射图片序号(位置翻页),圆点指示当前位置;
 *   悬停开始即预载全部图片,离开时回到主图。纯指针增强,键盘用户经详情
 *   相册获得等价能力。
 * - DetailAlbum(详情抽屉):左/右点击区翻页、中间点击区放大(灯箱),
 *   三个区域都是真实按钮(键盘可达);灯箱内点击任意处或 Esc 关闭。
 *
 * 加载语义:每张图都经 catalogImageUrl(vuaimg 缓存协议);卡片主图与详情
 * 当前图用 MediaSlot auto 模式(转圈自动重试,无手动按钮,见反馈 #2)。
 */
import { useEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from "react";
import { createPortal } from "react-dom";
import { catalogImageUrl } from "../../app/catalog-image.ts";
import { MediaSlot } from "../../components/primitives/MediaSlot.tsx";
import { strings } from "../../i18n/index.ts";
import { albumIndexFromOffset, CARD_ALBUM_HOVER_DELAY_MS } from "./warehouse-model.ts";

const copy = strings.warehouse;

/* ---- 卡片相册:悬停激活 + 位置翻页 ---- */

export function CardAlbumMedia({
  imageUrls,
  title,
}: {
  imageUrls: readonly string[];
  title: string;
}) {
  // hovering = 悬停中(触发预载);active = 相册模式(延迟后开启)
  const [hovering, setHovering] = useState(false);
  const [active, setActive] = useState(false);
  const [index, setIndex] = useState(0);
  const [loaded, setLoaded] = useState<ReadonlySet<number>>(new Set());
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const areaRef = useRef<HTMLDivElement | null>(null);

  const stopTimer = () => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };
  // 卸载清理定时器
  useEffect(() => stopTimer, []);

  if (imageUrls.length <= 1) {
    const only = imageUrls[0];
    return only !== undefined ? (
      <MediaSlot src={catalogImageUrl(only)} alt={title} aspectRatio="1 / 1" retry="auto" />
    ) : null;
  }

  const primary = imageUrls[0] as string;

  const enter = () => {
    setHovering(true);
    stopTimer();
    timerRef.current = setTimeout(() => setActive(true), CARD_ALBUM_HOVER_DELAY_MS);
  };
  const leave = () => {
    stopTimer();
    setHovering(false);
    setActive(false);
    setIndex(0);
  };
  const move = (event: ReactMouseEvent) => {
    if (!active) return;
    const rect = areaRef.current?.getBoundingClientRect();
    if (rect === undefined) return;
    setIndex(albumIndexFromOffset(event.clientX - rect.left, rect.width, imageUrls.length));
  };

  return (
    <div
      ref={areaRef}
      className="vua-warehouse-album"
      onMouseEnter={enter}
      onMouseLeave={leave}
      onMouseMove={move}
    >
      {/* 主图常驻:相册未激活时是唯一内容;激活后垫在堆叠层下面,
       *  当前图未加载完时经透明间隙露出,避免闪烁 */}
      <MediaSlot src={catalogImageUrl(primary)} alt={title} aspectRatio="1 / 1" retry="auto" />
      {/* 悬停即挂载全部图片开始预载(隐藏不影响请求);激活后按光标位置翻页。
       *  堆叠层对读屏隐藏——装饰性增强,等价能力在详情相册 */}
      {hovering || active ? (
        <div className="vua-warehouse-album__stack" aria-hidden="true">
          {imageUrls.map((url, i) => (
            <img
              key={url}
              className="vua-warehouse-album__img"
              src={catalogImageUrl(url)}
              alt=""
              data-loaded={loaded.has(i) || undefined}
              hidden={!active || i !== index || !loaded.has(i)}
              onLoad={() =>
                setLoaded((prev) => {
                  if (prev.has(i)) return prev;
                  const next = new Set(prev);
                  next.add(i);
                  return next;
                })
              }
            />
          ))}
        </div>
      ) : null}
      {/* 当前图未加载完:转圈等待(复用 MediaSlot 的等待指示样式) */}
      {active && !loaded.has(index) ? (
        <span
          className="vua-media-slot__spinner"
          role="status"
          aria-label={strings.media.loading}
        />
      ) : null}
      {active ? (
        <div
          className="vua-warehouse-album__dots vua-warehouse-album__dots--overlay"
          aria-hidden="true"
        >
          {imageUrls.map((url, i) => (
            <span key={url} data-current={i === index || undefined} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

/* ---- The details page's album: a large picture, thumbnails under it and a lightbox ---- */

export function DetailAlbum({ imageUrls, title, emptyText }: { imageUrls: readonly string[]; title: string; emptyText?: string }) {
  const [index, setIndex] = useState(0);
  const [zoomed, setZoomed] = useState(false);
  const [flipDir, setFlipDir] = useState<1 | -1>(1);
  const total = imageUrls.length;
  const safeIndex = Math.min(index, Math.max(0, total - 1));
  const flip = (delta: number) => {
    setFlipDir(delta > 0 ? 1 : -1);
    setIndex((value) => (value + delta + total) % total);
  };
  const flipRef = useRef(flip); flipRef.current = flip;
  useEffect(() => {
    if (!zoomed) return;
    // The lightbox takes these keys first, so Esc closes only it and arrows turn its picture.
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.stopPropagation(); setZoomed(false); }
      else if (event.key === "ArrowLeft" || event.key === "ArrowRight") { event.stopPropagation(); if (total > 1) flipRef.current(event.key === "ArrowRight" ? 1 : -1); }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [zoomed, total]);
  const current = imageUrls[safeIndex];
  if (current === undefined) return <div className="vua-warehouse-card__no-image vua-warehouse-detail-album__empty"><span className="vua-caption vua-text-secondary">{emptyText ?? copy.card.noImage}</span></div>;
  return (
    <div className="vua-warehouse-detail-album">
      <div className="vua-warehouse-detail-album__stage">
        {/* key restarts MediaSlot and the page-turn animation; effects-off makes it instant. */}
        <div key={current} className="vua-warehouse-detail-album__flip" data-dir={flipDir}>
          <MediaSlot src={catalogImageUrl(current)} alt={title} aspectRatio="1 / 1" retry="auto" />
        </div>
        <div className="vua-warehouse-detail-album__zones">
          {total > 1 ? <button type="button" className="vua-warehouse-detail-album__zone vua-warehouse-detail-album__zone--prev" aria-label={copy.album.prevImage} onClick={() => flip(-1)}>‹</button> : null}
          <button type="button" className="vua-warehouse-detail-album__zone vua-warehouse-detail-album__zone--zoom" aria-label={copy.album.zoomImage} onClick={() => setZoomed(true)} />
          {total > 1 ? <button type="button" className="vua-warehouse-detail-album__zone vua-warehouse-detail-album__zone--next" aria-label={copy.album.nextImage} onClick={() => flip(1)}>›</button> : null}
        </div>
      </div>
      {total > 1 ? <div className="vua-warehouse-detail-album__thumbs">
        {imageUrls.map((url, i) => <button type="button" key={url} aria-label={`${i + 1} / ${total}`} aria-current={i === safeIndex || undefined} onClick={() => { setFlipDir(i >= safeIndex ? 1 : -1); setIndex(i); }}>
          <img src={catalogImageUrl(url)} alt="" loading="lazy" draggable={false} />
        </button>)}
      </div> : null}
      {zoomed ? createPortal(
        <div className="vua-warehouse-lightbox" role="dialog" aria-modal="true" aria-label={title} onClick={() => setZoomed(false)}>
          <img src={catalogImageUrl(current)} alt={title} />
          <button type="button" className="vua-warehouse-lightbox__close" aria-label={copy.album.closeZoom} autoFocus onClick={() => setZoomed(false)}>×</button>
          {total > 1 ? <span className="vua-warehouse-lightbox__count">{safeIndex + 1} / {total}</span> : null}
        </div>, document.body) : null}
    </div>
  );
}
