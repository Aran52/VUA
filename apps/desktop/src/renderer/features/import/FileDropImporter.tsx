import { useEffect, useRef, useState } from "react";
import { Icon } from "@vua/design-system";
import { Button } from "../../components/primitives/Button.tsx";
import { ContentDialog } from "../../components/primitives/ContentDialog.tsx";
import { useAmfModule, useGateway } from "../../gateway/index.ts";
import { strings, format, TERMS } from "../../i18n/index.ts";
import "./file-drop.css";

/** The host catches native drops; AMF owns copying and the normal task channel. */
export function FileDropImporter({ onOpenLibrary }: { onOpenLibrary: () => void }) {
  const gateway = useGateway();
  const amf = useAmfModule();
  const copy = strings.fileDrop;
  const [dragging, setDragging] = useState(false);
  const [pending, setPending] = useState<readonly File[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [working, setWorking] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [openLibrary, setOpenLibrary] = useState(false);
  const busy = useRef(false);
  const commandId = useRef<string | null>(null);
  const beginRef = useRef<(files: readonly File[]) => void>(() => {});

  const submit = async (files: readonly File[], enable: boolean) => {
    if (busy.current) return;
    busy.current = true; setWorking(true); setError(null);
    try {
      if (files.length > 200) { setPending(null); setError(copy.limit); setDialogOpen(true); return; }
      const face = window.vua?.library;
      if (typeof face?.resolveDroppedFiles !== "function") { setPending(null); setError(copy.unavailable); setDialogOpen(true); return; }
      const snapshot = enable ? await gateway.amfModule.setEnabled(true) : null;
      const current = snapshot?.snapshot ?? await gateway.amfModule.snapshot();
      if (snapshot?.outcome === "busy") { setError(copy.busy); setDialogOpen(true); return; }
      if (current.state !== "ready" || snapshot?.outcome === "failed") { setPending(files); setDialogOpen(true); if (enable) setError(copy.failed); return; }
      const paths = await face.resolveDroppedFiles(files);
      commandId.current ??= `whcmd-${crypto.randomUUID()}`;
      const outcome = await gateway.warehouseCommands.importPaths(paths, { commandId: commandId.current });
      if (!outcome.ok || !("accepted" in outcome)) { setPending(files); setError(copy.failed); setDialogOpen(true); return; }
      commandId.current = null; setPending(null); setDialogOpen(false); setOpenLibrary(true);
    } catch { setPending(files); setError(copy.failed); setDialogOpen(true); }
    finally { busy.current = false; setWorking(false); }
  };
  beginRef.current = files => {
    if (busy.current) return;
    commandId.current = null;
    setPending(files);
    void submit(files, false);
  };

  // Wait for the module subscription before navigating; the host's route guard
  // must not see the saved disabled snapshot immediately after enabling AMF.
  useEffect(() => {
    if (openLibrary && amf.state === "ready") { setOpenLibrary(false); onOpenLibrary(); }
  }, [openLibrary, amf.state, onOpenLibrary]);

  useEffect(() => {
    let depth = 0;
    const nativeFiles = (event: DragEvent) => event.dataTransfer?.types.includes("Files") === true;
    const enter = (event: DragEvent) => {
      if (!nativeFiles(event)) return;
      event.preventDefault(); depth++; setDragging(true);
    };
    const over = (event: DragEvent) => {
      if (!nativeFiles(event)) return;
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = busy.current ? "none" : "copy";
    };
    const leave = (event: DragEvent) => {
      if (!nativeFiles(event)) return;
      depth = Math.max(0, depth - 1); if (depth === 0) setDragging(false);
    };
    const drop = (event: DragEvent) => {
      if (!nativeFiles(event)) return;
      event.preventDefault(); event.stopPropagation(); depth = 0; setDragging(false);
      // Read before DataTransfer's protected store closes; no File contents leave VUA.
      const files = Array.from(event.dataTransfer?.files ?? []);
      if (files.length > 0) beginRef.current(files);
    };
    const reset = () => { depth = 0; setDragging(false); };
    window.addEventListener("dragenter", enter, true); window.addEventListener("dragover", over, true);
    window.addEventListener("dragleave", leave, true); window.addEventListener("drop", drop, true);
    window.addEventListener("dragend", reset); window.addEventListener("blur", reset);
    return () => {
      window.removeEventListener("dragenter", enter, true); window.removeEventListener("dragover", over, true);
      window.removeEventListener("dragleave", leave, true); window.removeEventListener("drop", drop, true);
      window.removeEventListener("dragend", reset); window.removeEventListener("blur", reset);
    };
  }, []);
  const close = () => { if (!busy.current) { commandId.current = null; setPending(null); setError(null); setDialogOpen(false); } };
  return <>
    {dragging ? <div className="vua-file-drop" aria-hidden="true"><div className="vua-file-drop__label vua-title"><Icon name="download" size={24} />{copy.drop}</div></div> : null}
    <ContentDialog open={dialogOpen} title={copy.title} closeLabel={copy.cancel} onClose={close}>
      <div className="vua-page__stack">
        {amf.state !== "ready" ? <p>{format(copy.needsAmf, { amf: TERMS.amf })}</p> : null}
        {error !== null ? <p role="status">{error}</p> : null}
        <div className="vua-page__actions">
          {pending !== null ? <Button variant="primary" data-drop-action="import" disabled={working || amf.state === "starting" || amf.state === "stopping"} onClick={() => void submit(pending, amf.state !== "ready")}>{working ? copy.importing : amf.state !== "ready" ? format(copy.enable, { amf: TERMS.amf }) : error !== null ? copy.retry : copy.continue}</Button> : null}
          <Button variant="subtle" disabled={working} onClick={close}>{copy.cancel}</Button>
        </div>
      </div>
    </ContentDialog>
  </>;
}
