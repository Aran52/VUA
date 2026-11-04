/** The account's own Avatars on VRChat, read with the experimental VRChat sign-in (plan slice 26).
 * Pictures arrive one at a time through Main; VUA's uploads carry a mark, and each card opens the
 * Avatar's page on VRChat's site. */
import { useEffect, useState } from "react";
import { Icon } from "@vua/design-system";
import type { VrchatAvatarEntry, VrchatAvatarList, VrchatPort } from "../../../gateway/index.ts";
import type { PageId } from "../../../app/nav-model.ts";
import { Badge } from "../../../components/primitives/Badge.tsx";
import { format, formatDateMinute, strings } from "../../../i18n/index.ts";

const copy = strings.vrchat.shelf;
const release = strings.vrchat.release;

const statusLabel = (status: string) => release.releaseStatus[status as keyof typeof release.releaseStatus] ?? status;
const platformLabel = (platform: string) => release.platform[platform as keyof typeof release.platform] ?? platform;

function AvatarCard({ avatar, image, byVua, onOpen }: { avatar: VrchatAvatarEntry; image: string | null | undefined; byVua: boolean; onOpen: () => void }) {
  return <button type="button" className="vua-vrchat-shelf__card" title={format(copy.open, { name: avatar.name })} onClick={onOpen}>
    <span className="vua-vrchat-shelf__picture">{image ? <img src={image} alt="" data-audit-skip="avatar-cover" /> : <Icon name="avatar" size={24} />}</span>
    <strong>{avatar.name}</strong>
    <span className="vua-vrchat-shelf__facts">{[statusLabel(avatar.releaseStatus), ...avatar.platforms.map(platformLabel)].join(" · ")}</span>
    <small className="vua-text-secondary">{format(copy.updated, { time: formatDateMinute(avatar.updatedAt) })}</small>
    {byVua && <Badge tone="success" className="vua-vrchat-shelf__mark">{copy.byVua}</Badge>}
  </button>;
}

export function VrchatAvatarShelf({ vrchat, uploadedIds, onNavigate }: {
  vrchat: VrchatPort; uploadedIds: ReadonlySet<string>; onNavigate: (page: PageId) => void;
}) {
  const [list, setList] = useState<VrchatAvatarList | null>(null);
  const [reading, setReading] = useState(true);
  const [images, setImages] = useState<Readonly<Record<string, string | null>>>({});
  const [openFailed, setOpenFailed] = useState(false);

  const read = (refresh: boolean) => {
    setReading(true);
    void vrchat.avatars(refresh).then(
      setList,
      () => setList(previous => ({ state: "error", listedAt: previous?.listedAt ?? null, complete: previous?.complete ?? false, avatars: previous?.avatars ?? [] })),
    ).finally(() => setReading(false));
  };
  useEffect(() => { read(false); }, [vrchat]);

  // Pictures arrive one by one; leaving the page stops the queue.
  useEffect(() => {
    if (list === null) return;
    let alive = true;
    const waiting = list.avatars.filter(avatar => avatar.hasImage && images[avatar.id] === undefined);
    void (async () => {
      for (const avatar of waiting) {
        const image = await vrchat.avatarImage(avatar.id).catch(() => null);
        if (!alive) return;
        setImages(previous => ({ ...previous, [avatar.id]: image }));
      }
    })();
    return () => { alive = false; };
  }, [vrchat, list]);

  const open = (avatarId: string) => {
    setOpenFailed(false);
    void vrchat.openAvatarPage(avatarId).then(opened => setOpenFailed(!opened), () => setOpenFailed(true));
  };

  if (list === null) return <p className="vua-text-secondary vua-release-room__loading" role="status">{copy.reading}</p>;
  if (list.state === "signed_out") {
    return <div className="vua-album__cover">
      <Icon name="cloud" size={24} />
      <h2>{copy.signedOutTitle}</h2>
      <p className="vua-text-secondary">{copy.signedOutHint}</p>
      <button type="button" className="vua-album__bake" onClick={() => onNavigate("settings-experimental")} data-nav-id="release-vrchat-sign-in">{copy.toSignIn}</button>
    </div>;
  }
  return <section className="vua-vrchat-shelf" aria-label={copy.vrchatTab}>
    <div className="vua-vrchat-shelf__bar">
      <span className="vua-text-secondary" role="status">{reading ? copy.reading
        : list.listedAt ? format(copy.summary, { count: list.avatars.length, time: formatDateMinute(list.listedAt) }) : ""}</span>
      <button type="button" className="vua-release-upload__link" onClick={() => read(true)} disabled={reading} data-nav-id="release-vrchat-refresh">
        <Icon name="refresh" size={16} />{copy.refresh}</button>
    </div>
    {list.state === "error" && <p className="vua-release-room__error" role="alert">{copy.error}</p>}
    {list.state === "listed" && !list.complete && <p className="vua-caption vua-text-secondary">{copy.partial}</p>}
    {openFailed && <p className="vua-release-room__error" role="alert">{copy.openFailed}</p>}
    {list.state === "listed" && list.avatars.length === 0 && <p className="vua-text-secondary">{copy.empty}</p>}
    {list.avatars.length > 0 && <ul className="vua-vrchat-shelf__grid">
      {list.avatars.map(avatar => <li key={avatar.id}>
        <AvatarCard avatar={avatar} image={images[avatar.id]} byVua={uploadedIds.has(avatar.id)} onOpen={() => open(avatar.id)} />
      </li>)}
    </ul>}
  </section>;
}
