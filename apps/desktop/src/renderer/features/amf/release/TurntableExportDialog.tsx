import "../amf-production.css";
import { imageExportProblem } from "../recipe-share-model.ts";
import { useEffect, useState } from "react";
import type { AmfSharedRecipe } from "@vua/contracts";
import { useGateway, type AmfRecipeSummary } from "../../../gateway/index.ts";
import { ContentDialog } from "../../../components/primitives/ContentDialog.tsx";
import { Button } from "../../../components/primitives/Button.tsx";
import { strings, recipeShareCopy as copy, recipeShareError } from "../../../i18n/index.ts";
import { AmfRecipeShareDialog } from "../AmfRecipeShareDialog.tsx";
import { recipeSharing } from "../recipe-sharing.ts";

export function TurntableExportDialog({ runId, onClose }: { runId: string; onClose(): void }) {
  const gateway = useGateway(), sharing = recipeSharing();
  const [format, setFormat] = useState<"png" | "gif">("png");
  const [frame, setFrame] = useState(0), [photo, setPhoto] = useState<string | null>(null);
  const [include, setInclude] = useState(false), [confirmed, setConfirmed] = useState(false);
  const [recipes, setRecipes] = useState<readonly AmfRecipeSummary[]>([]), [recipeId, setRecipeId] = useState("");
  const [document, setDocument] = useState<AmfSharedRecipe | null>(null), [review, setReview] = useState(false);
  const [url, setUrl] = useState<string | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState<string | null>(null), [notice, setNotice] = useState<string | null>(null);
  useEffect(() => { let alive = true; void gateway.amfProduction?.listRecipes().then(value => { if (alive) setRecipes(value); }).catch(e => { if (alive) setError(recipeShareError(e)); }); return () => { alive = false; }; }, [gateway.amfProduction]);
  useEffect(() => { let alive = true; void gateway.amfRelease?.turntableFrame(runId, frame).then(value => { if (alive) setPhoto(value); }).catch(() => { if (alive) setPhoto(null); }); return () => { alive = false; }; }, [gateway.amfRelease, runId, frame]);
  if (review) return <AmfRecipeShareDialog recipeId={recipeId} onClose={() => setReview(false)} onExport={value => { setDocument(value.document); setReview(false); setConfirmed(false); }}/>
  return <ContentDialog open title={copy.export} closeLabel={strings.common.dialogClose} onClose={() => { if (!busy) onClose(); }}>
    <div className="vua-recipe-transfer">
      {photo && <img className="vua-share-photo" src={photo} alt={`${copy.frame} ${frame + 1}`}/>}
      <label className="vua-n3__field">{copy.format}<select disabled={busy} value={format} onChange={e => setFormat(e.target.value as "png" | "gif")}><option value="png">{copy.png}</option><option value="gif">{copy.gif}</option></select></label>
      {format === "png" && <label className="vua-n3__field">{copy.angle} · {frame * 6}°<input type="range" min={0} max={59} value={frame} disabled={busy} onChange={e => setFrame(Number(e.target.value))}/></label>}
      <label><input type="checkbox" disabled={busy} checked={include} onChange={e => { setInclude(e.target.checked); setConfirmed(false); }}/>{copy.include}</label>
      {include && <>
        <label className="vua-n3__field">{copy.recipe}<select value={recipeId} disabled={busy} onChange={e => { setRecipeId(e.target.value); setDocument(null); setConfirmed(false); }}><option value="">{copy.unknown}</option>{recipes.map(recipe => <option key={recipe.recipeId} value={recipe.recipeId}>{recipe.title}</option>)}</select></label>
        <Button disabled={busy || !recipeId} onClick={() => setReview(true)}>{document ? copy.reviewed : copy.review}</Button>
        {document && <p>{document.recipe.title} · {document.recipe.materials.length}</p>}
        <p className="vua-caption">{copy.retention}</p>
        <label><input type="checkbox" checked={confirmed} disabled={busy || !document} onChange={e => setConfirmed(e.target.checked)}/>{copy.public}</label>
      </>}
      <p className="vua-caption vua-text-secondary">{copy.hint}</p>
      {url && <label className="vua-n3__field">{copy.copy}<input readOnly value={url} onFocus={e => e.target.select()}/></label>}
      {error && <p role="alert">{error}</p>}{notice && <p role="status">{notice}</p>}
      <Button variant="primary" disabled={!sharing || busy || imageExportProblem({include,recipeId,reviewedRecipeId:document ? recipeId : null,confirmed}) !== null} onClick={() => {
        if (!sharing || busy) return; setBusy(true); setError(null); setNotice(null);
        void sharing.exportImage({runId, format, frameIndex:frame, ...(include && document ? {document, recipeId, publicConfirmed:true} : {})})
          .then(result => { if (result.saved) { setNotice(copy.saved); setUrl(result.url); setConfirmed(false); } }).catch(e => setError(recipeShareError(e))).finally(() => { setBusy(false); setConfirmed(false); });
      }}>{busy ? copy.loading : copy.save}</Button>
    </div>
  </ContentDialog>;
}
