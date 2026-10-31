import { useEffect, useState } from "react";
import type { PublishedRecipeLink } from "@vua/contracts";
import { Card } from "../../components/primitives/Card.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { recipeShareCopy as copy, recipeShareError } from "../../i18n/index.ts";
import { recipeSharing } from "./recipe-sharing.ts";

export function PublishedRecipeLinks() {
  const sharing = recipeSharing();
  const [links, setLinks] = useState<readonly PublishedRecipeLink[]>([]), [nonce, setNonce] = useState(0);
  const [busy, setBusy] = useState(false), [error, setError] = useState<string | null>(null), [notice, setNotice] = useState<string | null>(null);
  useEffect(() => { let alive = true; void sharing?.list().then(value => { if (alive) setLinks(value); }).catch(e => { if (alive) setError(recipeShareError(e)); }); return () => { alive = false; }; }, [sharing, nonce]);
  if (!sharing || (!links.length && !error)) return null;
  return <Card><h2>{copy.links}</h2><p className="vua-caption">{copy.retention}</p>{error && <p role="alert">{error}</p>}{notice && <p role="status">{notice}</p>}<ul className="vua-share-links">{links.map(link => <li key={link.code}>
    <code>{link.url}</code><div className="vua-n3__actions">
      <Button disabled={busy} onClick={() => { void navigator.clipboard.writeText(link.url).then(() => setNotice(copy.copied)).catch(e => setError(recipeShareError(e))); }}>{copy.copy}</Button>
      <Button disabled={busy} onClick={() => { setBusy(true); setError(null); void sharing.withdraw(link.code).then(() => { setNotice(copy.withdrawn); setNonce(v => v + 1); }).catch(e => setError(recipeShareError(e))).finally(() => setBusy(false)); }}>{copy.withdraw}</Button>
    </div></li>)}</ul></Card>;
}
