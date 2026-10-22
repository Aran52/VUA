// Synthetic preview of the six notification tiers for visual review.
import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { Alert } from "../../src/renderer/components/primitives/Alert.tsx";
import { Button } from "../../src/renderer/components/primitives/Button.tsx";
import { Card } from "../../src/renderer/components/primitives/Card.tsx";
import { ConfirmDialog } from "../../src/renderer/components/primitives/ConfirmDialog.tsx";
import { Drawer } from "../../src/renderer/components/primitives/Drawer.tsx";
import { Popconfirm } from "../../src/renderer/components/primitives/Popconfirm.tsx";
import { ToastHost } from "../../src/renderer/components/primitives/ToastHost.tsx";
import { toast } from "../../src/renderer/components/primitives/toast-store.ts";
import { Tooltip } from "../../src/renderer/components/primitives/Tooltip.tsx";
import { strings, format, TERMS } from "../../src/renderer/i18n/index.ts";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
import "../../src/renderer/app-shell.css";

const view = location.hash.slice(1).replace(/^feedback-/, "") || "tiers";

function Preview() {
  const popRoot = useRef<HTMLDivElement>(null);
  const tipRoot = useRef<HTMLDivElement>(null);
  const [dialog, setDialog] = useState(view === "dialog");
  useEffect(() => {
    toast(strings.aiTools.copied, "success");
    toast(strings.warehouse.catalogSync.startedHint);
    if (view === "tiers") {
      popRoot.current?.querySelector("button")?.click();
      const anchor = tipRoot.current?.querySelector(".vua-tooltip-anchor");
      anchor?.dispatchEvent(new PointerEvent("pointerover", { bubbles: true }));
      anchor?.dispatchEvent(new PointerEvent("pointerenter", { bubbles: false }));
    }
  }, []);
  const module = strings.amfModule;
  return <div className="vua-page" style={{ maxWidth: 760 }}>
    <h1 className="vua-title">Notification tiers</h1>
    <Alert tone="info" action={<Button variant="subtle">{strings.factory.queued.remove}</Button>}>{strings.factory.queued.hint}</Alert>
    <Alert tone="warning" title={strings.unityLicense.retry} action={<Button>{strings.unityLicense.open}</Button>}>{strings.unityLicense.retryHint}</Alert>
    <Alert tone="error" title={strings.amfProduction.error} action={<Button variant="subtle">{strings.amfProduction.retry}</Button>} onDismiss={() => {}} dismissLabel={strings.common.dialogClose}>vua.amf_production.busy</Alert>
    <Alert tone="success">{strings.controlCenter.packages.repos.added}</Alert>
    <Card><div className="vua-page__actions">
      <div ref={tipRoot}><Tooltip content={strings.factory.stopTip}><Button variant="subtle">{strings.factory.stop}</Button></Tooltip></div>
      <div ref={popRoot} style={{ marginLeft: 220 }}>
        <Popconfirm title={format(module.disableConfirm, { amf: TERMS.amf })} description={format(module.retained, { amf: TERMS.amf })}
          confirmLabel={format(module.disable, { amf: TERMS.amf })} cancelLabel={strings.common.cancel} onConfirm={() => {}}>
          <Button variant="subtle">{format(module.disable, { amf: TERMS.amf })}</Button>
        </Popconfirm>
      </div>
    </div></Card>
    <Drawer open={view === "tiers"} title="Modular Avatar" closeLabel={strings.common.dialogClose} onClose={() => {}}>
      <p className="vua-text-secondary">Non-destructive avatar assembly.</p>
    </Drawer>
    <ConfirmDialog open={dialog} title={strings.factory.panels.materials.confirmTitle} cancelLabel={strings.common.cancel} confirmLabel={strings.factory.panels.materials.confirm}
      onCancel={() => setDialog(false)} onConfirm={() => setDialog(false)}><p className="vua-confirm-dialog__text">{strings.factory.panels.materials.confirmBody}</p></ConfirmDialog>
    <ToastHost />
  </div>;
}

createRoot(document.getElementById("root")!).render(<Preview />);
