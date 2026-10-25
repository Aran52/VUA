// 素材导入弹窗 DOM 回归夹具(W25 走查缺陷③根因修复批,操作者第 178 批派单):
// 合成 Gateway,零生产/远程服务。钉死三条用户关闭路径(Esc/背板/×)与两条
// 提交终态(受理自动关闭/失败驻留醒目可关),另钉「受理窗口内手动关闭后
// 重开不被陈旧定时器误关」。真机浏览器 DOM 为准,不宣称端到端。
import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { ContentDialog } from "../../src/renderer/components/primitives/ContentDialog.tsx";
import { ImportPage } from "../../src/renderer/features/import/ImportPage.tsx";
import { GatewayProvider } from "../../src/renderer/gateway/GatewayProvider.tsx";
import { emptyGateway } from "../../src/renderer/gateway/empty-gateway.ts";
import { strings } from "../../src/renderer/i18n/index.ts";
import { IMPORT_ACCEPTED_AUTO_CLOSE_MS } from "../../src/renderer/features/import/import-model.ts";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";

const root = createRoot(document.getElementById("root")!);
// 壳面桩:文件系统窄面返回合成目录树(零真实文件操作);原生文件夹拾取
// 返回合成路径(零真实对话框);capabilities 自报 remoteBrowser 可用 =
// 云端段入口可达(下载面板无宿主降级场景需要);gateway 宿主刻意缺席——
// 下载读面不可达的诚实降级由此钉死。
(window as any).vua = {
  dialog: { pickWarehouseFolders: async () => ["C:/synthetic/material-pack"] },
  fs: {
    listDirectory: async (target: string | null) => {
      if (target === null || target === "C:/Users/Synthetic") {
        return {
          ok: true as const,
          value: {
            path: "C:/Users/Synthetic",
            parent: "C:/Users",
            entries: [
              { name: "material-pack", path: "C:/Users/Synthetic/material-pack", hidden: false },
              { name: "more-assets", path: "C:/Users/Synthetic/more-assets", hidden: false },
            ],
          },
        };
      }
      return { ok: false as const, error: "not_found" as const };
    },
    createDirectory: async (parentPath: string, name: string) => ({
      ok: true as const,
      value: { path: `${parentPath}/${name}` },
    }),
  },
  capabilities: { remoteBrowser: true },
};
const wait = (ms = 40) => new Promise<void>((resolve) => setTimeout(resolve, ms));
const results: string[] = [];
function check(ok: unknown, name: string) {
  if (!ok) throw new Error(name);
  results.push(name);
}
function button(text: string, within: ParentNode = document): HTMLButtonElement {
  const found = [...within.querySelectorAll<HTMLButtonElement>("button")].find(
    (node) => node.textContent?.trim() === text,
  );
  if (!found) {
    const body = (document.body.textContent ?? "").replace(/\s+/g, " ").slice(0, 1200);
    throw new Error("Button missing: " + text + " | BODY: " + body);
  }
  return found;
}
async function key(value: string) {
  document.activeElement?.dispatchEvent(
    new KeyboardEvent("keydown", { key: value, bubbles: true, cancelable: true }),
  );
  await wait();
}

/* ---- 云端段合成桩(2026-09-25 自动收口场景):remoteContent 事件流 +
 *      gateway downloads.listCompleted 信封。默认缺席(无宿主降级场景
 *      需要);仅自动收口场景临时挂上,用完即卸。 */
type DownloadsMode = "empty" | "one";
let downloadsMode: DownloadsMode = "empty";
const syntheticDownloadRow = {
  adoptedWarehouseItemIds: [] as string[],
  completedAt: "2026-09-25T00:00:00.000Z",
  downloadId: "dl-synthetic-1",
  receivedBytes: 2048,
  sourceUrl: "https://booth.pm/download/1",
  suggestedFileName: "pack.unitypackage",
};
const remoteListeners = new Set<(event: unknown) => void>();
let syntheticViewSeq = 0;
const syntheticRemoteContent = {
  open: async ({ url }: { url: string }) => {
    syntheticViewSeq += 1;
    const viewId = `view-synthetic-${syntheticViewSeq}`;
    queueMicrotask(() => {
      for (const listener of remoteListeners) listener({ kind: "view-opened", viewId, url });
    });
    return { viewId, url, visible: true, canGoBack: false, canGoForward: false };
  },
  navigate: async (viewId: string, url: string) => ({ viewId, url, visible: true, canGoBack: true, canGoForward: false }),
  goBack: async (viewId: string) => ({ viewId, url: "https://booth.pm/", visible: true, canGoBack: false, canGoForward: true }),
  goForward: async (viewId: string) => ({ viewId, url: "https://booth.pm/", visible: true, canGoBack: true, canGoForward: false }),
  reload: async (viewId: string) => ({ viewId, url: "https://booth.pm/", visible: true, canGoBack: false, canGoForward: false }),
  close: async (viewId: string) => {
    queueMicrotask(() => {
      for (const listener of remoteListeners) listener({ kind: "view-closed", viewId });
    });
  },
  setVisible: async (viewId: string, visible: boolean) => ({ viewId, url: "https://booth.pm/", visible, canGoBack: false, canGoForward: false }),
  signInHint: async () => "stored" as const,
  events: {
    subscribe: (listener: (event: unknown) => void) => {
      remoteListeners.add(listener);
      return () => {
        remoteListeners.delete(listener);
      };
    },
  },
};
const syntheticGatewayHost = {
  invoke: async (request: { method: string }) => {
    if (request.method === "downloads.listCompleted") {
      const downloads = downloadsMode === "one" ? [syntheticDownloadRow] : [];
      return {
        ok: true as const,
        value: {
          schemaVersion: "0.5",
          operation: "downloads.listCompleted",
          result: { downloads },
        },
      };
    }
    return { ok: false as const, error: { kind: "unavailable" as const } };
  },
};
function setCloudStubs(on: boolean) {
  const shell = (window as any).vua;
  if (on) {
    shell.gateway = syntheticGatewayHost;
    shell.remoteContent = syntheticRemoteContent;
  } else {
    delete shell.gateway;
    delete shell.remoteContent;
  }
}
function navCloseButton(): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>(".vua-import__browse-button")].find(
    (node) => node.getAttribute("aria-label") === strings.importPage.navClose,
  );
  if (!found) throw new Error("导航条 × 不在场");
  return found;
}

/* ---- 合成写面:importFolders/importDownloads 由场景切换 ok/fail ---- */
const gateway = emptyGateway();
let importMode: "ok" | "fail" = "ok";
const accepted = { taskId: "synthetic-task-1", correlationId: "synthetic-corr-1" };
const failureOutcome = {
  ok: false as const,
  error: {
    kind: "application" as const,
    code: "vua.warehouse.unavailable",
    messageKey: "errors.warehouse.unavailable",
    recoverable: true,
    retryable: false,
  },
};
const syntheticGateway = {
  ...gateway,
  warehouseCommands: {
    ...gateway.warehouseCommands,
    importFolders: async () => (importMode === "ok" ? { ok: true as const, accepted } : failureOutcome),
    importDownloads: async () => (importMode === "ok" ? { ok: true as const, accepted } : failureOutcome),
  },
} as typeof gateway;

/* ---- 宿主:开态/关闭计数由夹具持有,onRequestClose 与 ContentDialog.onClose
 *      同收口(与仓储页宿主同构) ---- */
let closeCount = 0;
let cloudMode = false;
function Harness() {
  const [open, setOpen] = useState(false);
  (window as any).__setImportOpen = (value: boolean) => setOpen(value);
  return (
    <StrictMode>
      <GatewayProvider gateway={syntheticGateway}>
        <ContentDialog
          open={open}
          title={strings.importPage.title}
          closeLabel={strings.common.dialogClose}
          onClose={() => {
            closeCount++;
            setOpen(false);
          }}
        >
          <ImportPage
            initialUrl={cloudMode ? "https://booth.pm/" : undefined}
            onRequestClose={() => {
              closeCount++;
              setOpen(false);
            }}
          />
        </ContentDialog>
      </GatewayProvider>
    </StrictMode>
  );
}

async function openDialog() {
  (window as any).__setImportOpen(true);
  await wait();
}
const dialogOpen = () => document.querySelector('[role="dialog"]') !== null;
async function waitForClosed() {
  const deadline = performance.now() + 2000;
  while (dialogOpen()) {
    if (performance.now() > deadline) throw new Error(`The import dialog did not finish closing: ${JSON.stringify({
      checks: results.slice(-3), closeCount,
      focus: document.activeElement?.outerHTML.slice(0, 400),
      dialogs: [...document.querySelectorAll('[role="dialog"]')].map(node => ({ class: node.className, closing: node.parentElement?.dataset.closing })),
      animations: document.getAnimations().map(animation => ({ state: animation.playState, time: animation.currentTime, timing: animation.effect?.getComputedTiming() })),
    })}`);
    await wait();
  }
}

/* ---- 场景 ---- */

/** 应用内文件夹选择器(2026-09-25 用户裁决)确认当前浏览目录:选择器
 *  面板内点「打开」(空选 = 确认当前目录),合成路径进入待确认清单。 */
async function confirmViaPicker() {
  const panel = document.querySelector(".vua-folder-picker");
  check(panel !== null, "文件夹选择器在场");
  await button(strings.folderPicker.openCta, panel as ParentNode).click();
  await wait();
  check(
    !document.querySelector(".vua-folder-picker"),
    "选择器确认后关闭(回到待确认清单)",
  );
}

/** 受理段行进:选择本地 → 拾取(应用内选择器,合成 fs 面)→ 确认提交。
 *  返回前停在提交回执落定后(等待微任务排空)。 */
async function driveToLocalSubmission() {
  await openDialog();
  check(dialogOpen(), "弹窗打开");
  check(document.querySelector(".vua-folder-picker") !== null, "本地导入直接打开文件夹选择器");
  await confirmViaPicker();
  await button(strings.warehouse.acquire.importConfirmCta).click();
  await wait();
}

async function escClosesPath() {
  await openDialog();
  check(dialogOpen(), "Esc 路径:弹窗打开");
  await key("Escape");
  await waitForClosed();
  check(!dialogOpen(), "Esc 关闭弹窗(路径一)");
  check(closeCount === 1, "Esc 恰一次关闭请求");
}

async function backdropClosesPath() {
  await openDialog();
  const overlay = document.querySelector(".vua-folder-picker__overlay") as HTMLElement;
  check(overlay !== null, "背板路径:遮罩在场");
  overlay.click();
  await wait();
  await waitForClosed();
  check(!dialogOpen(), "背板点击关闭弹窗(路径二)");
}

async function closeButtonClosesPath() {
  await openDialog();
  const close = document.querySelector(".vua-folder-picker__close") as HTMLButtonElement;
  check(close !== null, "× 路径:关闭钮在场");
  close.click();
  await wait();
  await waitForClosed();
  check(!dialogOpen(), "× 关闭弹窗(路径三)");
}

async function acceptAutoCloses() {
  importMode = "ok";
  closeCount = 0;
  await driveToLocalSubmission();
  check(document.body.textContent!.includes(strings.warehouse.acquire.importAccepted), "受理词面呈现(已受理)");
  check(
    document.body.textContent!.includes(strings.importPage.acceptedAutoClose),
    "自动关闭提示词面呈现",
  );
  check(dialogOpen(), "时滞内弹窗仍在(用户看得见受理信息)");
  await wait(IMPORT_ACCEPTED_AUTO_CLOSE_MS + 250);
  await waitForClosed();
  check(!dialogOpen(), "受理后弹窗自动关闭(不滞留模态)");
  check(closeCount === 1, "自动关闭恰一次请求");
}

async function failureStaysWithProminentClose() {
  importMode = "fail";
  closeCount = 0;
  await driveToLocalSubmission();
  const alert = document.querySelector('[role="alert"]');
  check(alert !== null && alert.textContent!.includes("vua.warehouse.unavailable"), "失败 alert 呈现且含协议稳定码详情(failureLogText 律)");
  const closeActions = document.querySelector(".vua-import__failure-actions");
  check(closeActions !== null, "失败动作位在场");
  const prominent = closeActions!.querySelector<HTMLButtonElement>(".vua-button--primary");
  check(prominent !== null && prominent.textContent?.trim() === strings.common.dialogClose, "醒目主按钮「关闭」在场");
  await wait(IMPORT_ACCEPTED_AUTO_CLOSE_MS + 250);
  check(dialogOpen(), "失败态弹窗驻留(不自动关走,失败需用户知悉)");
  prominent!.click();
  await wait();
  await waitForClosed();
  check(!dialogOpen(), "主按钮关闭失败态弹窗(× 仅辅助)");
}

async function manualCloseDuringWindowNoStaleTimer() {
  importMode = "ok";
  closeCount = 0;
  await driveToLocalSubmission();
  check(document.body.textContent!.includes(strings.warehouse.acquire.importAccepted), "受理词面呈现");
  await key("Escape");
  await waitForClosed();
  check(!dialogOpen(), "受理窗口内手动 Esc 先关");
  const closesBefore = closeCount;
  await openDialog();
  check(dialogOpen(), "重开弹窗(重挂载)");
  await wait(IMPORT_ACCEPTED_AUTO_CLOSE_MS + 250);
  check(dialogOpen(), "重开弹窗不被陈旧定时器误关(卸载清理兑现)");
  check(closeCount === closesBefore, "重开期间零额外关闭请求");
}

/** 第 179 批反向审查钉死(受理窗口内用户接管):受理后 1.5s 窗口内用户再次
 *  发起导入(反馈被清空=用户接管)——在飞自动关闭即取消,弹窗不得跑在用户
 *  进行中的操作下面静默关闭(否则原生拾取停留期间弹窗自关、拾取结果落在
 *  已卸载组件上被丢弃);用户完成二次提交后重新武装,照常自动收口(同窗
 *  二次受理重新计时语义保持)。 */
async function userTakeoverCancelsAutoClose() {
  importMode = "ok";
  closeCount = 0;
  await driveToLocalSubmission();
  check(document.body.textContent!.includes(strings.warehouse.acquire.importAccepted), "受理词面呈现(第一次受理)");
  await button(strings.importPage.addFolders).click();
  await wait();
  check(dialogOpen(), "受理窗口内用户再次发起导入,选择器打开(用户接管,反馈清空)");
  await wait(IMPORT_ACCEPTED_AUTO_CLOSE_MS + 250);
  check(dialogOpen(), "用户接管后自动关闭取消——弹窗不跑在用户新操作下面(不静默关走)");
  check(closeCount === 0, "用户接管期间零关闭请求(在飞计时已解除)");
  await confirmViaPicker();
  await button(strings.warehouse.acquire.importConfirmCta).click();
  await wait();
  check(document.body.textContent!.includes(strings.warehouse.acquire.importAccepted), "二次受理词面呈现");
  await wait(IMPORT_ACCEPTED_AUTO_CLOSE_MS + 250);
  await waitForClosed();
  check(!dialogOpen(), "二次受理重新武装后照常自动收口(同窗二次受理重新计时语义保持)");
}

/** 选择器内「使用 Windows 选择文件夹」次级路径(2026-09-25 用户裁决):
 *  原生拾取结果经同一 onConfirm 合流进待确认清单,选择器关闭、弹窗不关。 */
async function windowsPickerPathInsideDialog() {
  closeCount = 0;
  await openDialog();
  check(dialogOpen(), "原生次级路径:弹窗打开");
  const panel = document.querySelector(".vua-folder-picker");
  check(panel !== null, "选择器在场");
  await button(strings.folderPicker.windowsPicker, panel as ParentNode).click();
  await wait();
  check(!document.querySelector(".vua-folder-picker"), "原生拾取确认后选择器关闭");
  check(dialogOpen(), "宿主弹窗不关(选取进入待确认清单)");
  check(
    document.body.textContent!.includes("C:/synthetic/material-pack"),
    "原生选取路径进入待确认清单",
  );
  await key("Escape");
  await wait();
  await waitForClosed();
  check(!dialogOpen(), "原生次级路径收尾:弹窗已关");
}

/** 第 181 批反向审查钉死(无宿主降级,族②/#36 旁支):capabilities 自报可用
 *  而 gateway 宿主缺席(本夹具桩即此形态)时,云端段下载清单读面不可达 =
 *  诚实 unavailable——不悬挂「加载中」过程态(修复前可选链整条短路,
 *  .then 不执行,loading 假陈述恒挂;同配方库列表 !result?.ok → unavailable
 *  先例)。真机 DOM 为准。 */
async function cloudDownloadsWithoutHostHonestUnavailable() {
  cloudMode = true;
  const unavailableText = strings.warehouse.acquire.commandErrors.vua_warehouse_unavailable;
  await openDialog();
  check(dialogOpen(), "无宿主场景:弹窗打开");
  check(
    document.body.textContent!.includes(unavailableText),
    "无宿主时下载面板诚实 unavailable(读面不可达如实呈现)",
  );
  check(
    !document.body.textContent!.includes(strings.importPage.downloadsLoading),
    "无宿主时不悬挂「加载中」假陈述",
  );
  await key("Escape");
  await wait();
  await waitForClosed();
  check(!dialogOpen(), "无宿主场景收尾:弹窗已关");
  cloudMode = false;
}

/** 2026-09-25 用户裁决:用户经导航条 × 关闭内嵌视图且已完成下载为零 →
 *  宿主弹窗自动收口(恰一次关闭请求);拆卸/代次兜底关闭永不武装(由
 *  模型测试钉死,此处钉用户路径的 DOM 级接线)。 */
async function userClosesViewWithNoDownloadsAutoCloses() {
  cloudMode = true;
  closeCount = 0;
  downloadsMode = "empty";
  setCloudStubs(true);
  try {
    await openDialog();
    check(dialogOpen(), "零下载收口:弹窗打开");
    check(document.querySelector(".vua-import__browse-bar") !== null, "首开自动导航:内嵌视图打开,导航条在场");
    navCloseButton().click();
    await wait();
    await waitForClosed();
    check(!dialogOpen(), "用户关闭视图且零完成下载:宿主弹窗自动收口");
    check(closeCount === 1, "自动收口恰一次关闭请求");
  } finally {
    setCloudStubs(false);
    downloadsMode = "empty";
    cloudMode = false;
  }
}

/** 2026-09-25 用户裁决:有已完成下载 → 用户关闭视图弹窗驻留,云端段呈现
 *  「重新打开内嵌浏览」细钮继续操作。 */
async function userClosesViewWithDownloadsStaysOpen() {
  cloudMode = true;
  closeCount = 0;
  downloadsMode = "one";
  setCloudStubs(true);
  try {
    await openDialog();
    check(dialogOpen(), "有下载驻留:弹窗打开");
    check(document.querySelector(".vua-import__browse-bar") !== null, "有下载驻留:内嵌视图打开");
    navCloseButton().click();
    await wait();
    check(dialogOpen(), "有已完成下载:弹窗驻留(不自动收口)");
    check(closeCount === 0, "驻留期间零关闭请求");
    check(
      document.body.textContent!.includes(strings.importPage.reopenBrowseCta),
      "「重新打开内嵌浏览」细钮呈现",
    );
  } finally {
    setCloudStubs(false);
    downloadsMode = "empty";
    cloudMode = false;
    await key("Escape");
    await wait();
    await waitForClosed();
    check(!dialogOpen(), "有下载驻留场景收尾:弹窗已关");
  }
}

window.importDialog = {
  run: async () => {
    root.render(<Harness />);
    await wait();
    await escClosesPath();
    await backdropClosesPath();
    await closeButtonClosesPath();
    await acceptAutoCloses();
    await failureStaysWithProminentClose();
    await manualCloseDuringWindowNoStaleTimer();
    await userTakeoverCancelsAutoClose();
    await windowsPickerPathInsideDialog();
    await cloudDownloadsWithoutHostHonestUnavailable();
    await userClosesViewWithNoDownloadsAutoCloses();
    await userClosesViewWithDownloadsStaysOpen();
    return results;
  },
};
