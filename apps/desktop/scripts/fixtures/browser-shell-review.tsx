// Real shell/browser/notification components with synthetic ports; no account or remote page.
import { createRoot } from "react-dom/client";
import { GatewayProvider } from "../../src/renderer/gateway/GatewayProvider.tsx";
import { emptyGateway } from "../../src/renderer/gateway/empty-gateway.ts";
import { NotificationPopover } from "../../src/renderer/features/task-center/NotificationPopover.tsx";
import { BrowserFrame } from "../../src/renderer/components/BrowserFrame.tsx";
import { ContentDialog } from "../../src/renderer/components/primitives/ContentDialog.tsx";
import { useModalOwner } from "../../src/renderer/components/primitives/modal-layer.tsx";
import { ShapeScope } from "../../src/renderer/components/primitives/ShapeScope.tsx";
import { Popconfirm } from "../../src/renderer/components/primitives/Popconfirm.tsx";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
import "../../src/renderer/app-shell.css";

const root = createRoot(document.getElementById("root")!);
const receipts: ({ x: number; y: number; width: number; height: number } | null)[] = [];
(window as any).vua = { desktopBrowser: { setViewport: async (_view: string, rect: typeof receipts[number]) => { receipts.push(rect); } } };
const empty = emptyGateway();
const gateway = { ...empty, task: { ...empty.task, capability: async () => ({ state: "ready" }),
  snapshot: async () => ({ schemaVersion: 1, tasks: [] }) } } as any;
const delay = (ms = 30) => new Promise(resolve => setTimeout(resolve, ms));
async function until(predicate: () => boolean, label: string) {
  const deadline = performance.now() + 5000;
  while (!predicate()) { if (performance.now() > deadline) throw Error(label + " " + JSON.stringify({
    expanded:document.querySelector("[data-nav-id='shell-notifications']")?.getAttribute("aria-expanded"),
    panel:!!document.querySelector(".vua-notify-panel"), occluder:!!document.querySelector("[data-vua-native-occluder]"),
    receipts:receipts.slice(-4), focus:document.activeElement?.className })); await delay(); }
}
function check(ok: unknown, label: string) { if (!ok) throw Error(label); }
function Browser() {
  const owner = useModalOwner();
  return <BrowserFrame viewId="synthetic-retained-view" modalOwner={owner}><button id="review-toolbar">Browser toolbar</button></BrowserFrame>;
}
function Review({ modal }: { modal: boolean }) {
  return <GatewayProvider gateway={gateway}><div className="vua-shell" data-module="production" data-review-modal={String(modal)}>
    <header className="vua-shell__header"><NotificationPopover navigate={() => {}} page="warehouse" /></header>
    <div className="vua-shell__body"><main className="vua-shell__main"><ShapeScope value="amf">
      {modal ? <ContentDialog open title="Synthetic browser import" closeLabel="close" onClose={() => {}}><Browser /></ContentDialog> : <Browser />}
      <Popconfirm title="Synthetic AMF question" confirmLabel="Yes" cancelLabel="No" onConfirm={() => {}}><button id="amf-question">Question</button></Popconfirm>
    </ShapeScope></main></div></div></GatewayProvider>;
}
(window as any).browserShellReview = { run: async () => {
  const checks: string[] = [];
  for (const modal of [false, true]) {
    const previousReceipts = receipts.length;
    root.render(<Review key={String(modal)} modal={modal} />);
    await until(() => document.querySelector<HTMLElement>(".vua-shell")?.dataset.reviewModal === String(modal)
      && !!document.querySelector("[data-nav-id='shell-notifications']") && receipts.length > previousReceipts && receipts.at(-1) != null,
      "browser viewport did not become visible");
    const frame = document.querySelector<HTMLElement>("[data-vua-browser-frame]")!;
    const toolbar = document.getElementById("review-toolbar")!;
    check(!toolbar.closest("[inert]"), "browser toolbar remains usable before notifications");
    document.querySelector<HTMLButtonElement>("[data-nav-id='shell-notifications']")!.click();
    await until(() => receipts.at(-1) === null && !!document.querySelector(".vua-notify-panel"), "native view must yield to notifications");
    const panel = document.querySelector<HTMLElement>(".vua-notify-panel")!;
    check(!panel.closest("[inert]"), "notifications stay usable above a browser-owned import modal");
    check(Number(getComputedStyle(panel).zIndex) > Number(getComputedStyle(frame).zIndex), "notification DOM paints above browser chrome");
    check(getComputedStyle(panel).getPropertyValue("--vua-shape-role").trim() === "none", "global notifications retain the host shape");
    await delay(220);
    const rect = panel.getBoundingClientRect();
    check(panel.contains(document.elementFromPoint(rect.left + 24, rect.top + 24)), "the visible notification panel receives pointer hits");
    const close = panel.querySelector<HTMLButtonElement>(".vua-notify-panel__close")!;
    close.focus(); check(document.activeElement === close, "notification controls accept focus");
    if (modal) close.dispatchEvent(new KeyboardEvent("keydown", {key:"Escape",bubbles:true,cancelable:true}));
    else close.click();
    await until(() => !document.querySelector(".vua-notify-panel") && receipts.at(-1) != null, "browser viewport must resume after notifications close");
    check(document.querySelector("[data-vua-browser-frame]") === frame, "notification dismissal retains the same browser frame");
    if (modal) check(!!document.querySelector('[aria-label="Synthetic browser import"]'), "Escape closes the notification without closing its browser-owned import");
    checks.push(`${modal ? "import modal" : "page"}: native viewport yields, notification stays visible and usable, browser resumes`);
  }
  root.render(<ShapeScope value="amf"><Popconfirm title="Portal question" confirmLabel="Yes" cancelLabel="No" onConfirm={() => {}}><button id="portal-question">Question</button></Popconfirm></ShapeScope>);
  await until(() => !!document.getElementById("portal-question"), "scope fixture did not mount");
  document.getElementById("portal-question")!.click();
  await until(() => !!document.querySelector(".vua-popconfirm"), "AMF portal did not open");
  const portal = document.querySelector<HTMLElement>(".vua-popconfirm")!;
  check(portal.parentElement === document.body && portal.dataset.vuaShapeScope === "amf", "AMF scope follows React ownership through body portals");
  check(getComputedStyle(portal).getPropertyValue("--vua-shape-role").trim() === "menu"
    && getComputedStyle(portal).getPropertyValue("--vua-shape-treatment").trim() === "quiet"
    && getComputedStyle(portal).borderTopLeftRadius === "0px", "small AMF portal keeps its menu role and quiet rectangular treatment");
  checks.push("AMF portal keeps its scope independently of the host shell");
  root.unmount(); await delay(); return checks;
} };
