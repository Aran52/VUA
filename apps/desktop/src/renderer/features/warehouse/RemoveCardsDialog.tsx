import { useRef, useState } from "react";
import type { LibraryCardTargetV02 } from "@vua/contracts";
import { createLibraryMetadataPort } from "../../gateway/index.ts";
import { strings, format } from "../../i18n/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { ContentDialog } from "../../components/primitives/ContentDialog.tsx";
export interface RemovalCard { readonly target: LibraryCardTargetV02; readonly title: string }
const port = createLibraryMetadataPort();
export function RemoveCardsDialog({ cards, onClose, onChanged }: { cards: readonly RemovalCard[]; onClose: () => void; onChanged: () => void }) {
  const copy = strings.warehouse.removeCards;
  const [busy, setBusy] = useState(false); const [failed, setFailed] = useState(false);
  const command = useRef(crypto.randomUUID()); const submitting = useRef(false);
  const close = () => { if (!submitting.current) onClose(); };
  const remove = async () => {
    if (submitting.current) return;
    submitting.current = true; setBusy(true); setFailed(false);
    try { await port.removeCards(cards.map(card => card.target), command.current); onChanged(); onClose(); }
    catch { setFailed(true); }
    finally { submitting.current = false; setBusy(false); }
  };
  return <ContentDialog open title={copy.title} closeLabel={strings.common.dialogClose} onClose={close}>
    <div className="vua-page__stack">
      <p>{format(copy.description, { count: cards.length })}</p>
      <ul>{cards.map(card => <li key={`${card.target.kind}:${card.target.id}`}>{card.title}</li>)}</ul>
      {failed ? <p role="status">{copy.failed}</p> : null}
      <div className="vua-page__actions"><Button disabled={busy} onClick={close}>{strings.common.dialogClose}</Button>
        <Button variant="danger" disabled={busy || cards.length === 0 || cards.length > 200} onClick={() => void remove()}>{busy ? copy.working : format(copy.confirm, { count: cards.length })}</Button>
      </div>
    </div>
  </ContentDialog>;
}
