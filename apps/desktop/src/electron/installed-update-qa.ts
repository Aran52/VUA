/** Explicit isolated installed-app diagnostic. Uses real preload/native update APIs, never a fixture Gateway. */
import { app, type BrowserWindow } from "electron";
import fs from "node:fs";
import path from "node:path";
import { installedContext } from "./installed-context.js";

interface Config { schemaVersion: "0.1"; runId: string; failHostBuild?: string | null }
const digest = (value: unknown): value is string => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
function atomic(file: string, value: unknown): void {
  const temporary = `${file}.${process.pid}.pending`;
  fs.writeFileSync(temporary, JSON.stringify(value, null, 2), { mode: 0o600 }); fs.renameSync(temporary, file);
}
export class InstalledUpdateQa {
  private readonly commands: string;
  private readonly replies: string;
  private blockedUi: string | undefined;
  constructor(private readonly directory: string, private readonly config: Config) {
    this.commands = path.join(directory, "component-update-commands"); this.replies = path.join(directory, "component-update-replies");
    for (const folder of [this.commands, this.replies]) {
      if (fs.existsSync(folder) && fs.lstatSync(folder).isSymbolicLink()) throw Error("QA control directory is a link");
      fs.mkdirSync(folder, { recursive: true });
    }
  }
  static from(directory: string): InstalledUpdateQa | undefined {
    const file = path.join(directory, "component-update-qa.json"); if (!fs.existsSync(file)) return;
    if (!installedContext()) throw Error("Installed update QA requires a verified component installation");
    if (fs.lstatSync(file).isSymbolicLink() || fs.statSync(file).size > 4096) throw Error("Invalid installed-update diagnostic configuration");
    const value = JSON.parse(fs.readFileSync(file, "utf8")) as Config;
    if (value.schemaVersion !== "0.1" || !/^[a-f0-9]{32}$/.test(value.runId)
        || (value.failHostBuild != null && !/^[a-f0-9]{40}$/.test(value.failHostBuild))) throw Error("Invalid installed-update diagnostic configuration");
    return new InstalledUpdateQa(directory, value);
  }
  allowsLaunchHealth(): boolean { return this.config.failHostBuild !== installedContext()?.host.host.build; }
  observeAmfReadiness(state: string, acceptingCalls: boolean, shellMounted: boolean): void {
    this.trace("amf-readiness", { state, acceptingCalls, shellMounted });
  }
  private trace(kind: string, facts: Record<string, unknown>): void {
    try {
      fs.appendFileSync(path.join(this.directory, "component-update-trace.jsonl"),
        `${JSON.stringify({ at: new Date().toISOString(), runId: this.config.runId, pid: process.pid, kind, ...facts })}\n`, { mode: 0o600 });
    } catch { /* Diagnostics must not change low-space recovery behavior. */ }
  }
  blocksAmfUi(url: string): boolean {
    if (!this.blockedUi) return false;
    const value = new URL(url);
    const blocked = value.protocol === "vua-module:" && value.hostname === "amf" && value.pathname.split("/")[2] === this.blockedUi;
    if (blocked) this.trace("amf-ui-request-blocked", { manifestId: this.blockedUi, pathname: value.pathname });
    return blocked;
  }
  async start(window: BrowserWindow): Promise<void> {
    const seeded = await window.webContents.executeJavaScript(`localStorage.getItem('vua.component-update-qa') === ${JSON.stringify(this.config.runId)}`) as boolean;
    if (!seeded) {
      await window.webContents.executeJavaScript(`localStorage.setItem('vua-goals',JSON.stringify({version:1,onboarding:'completed',goals:['env','production'],environments:['play','create']}));localStorage.setItem('vua-tour-progress',JSON.stringify({v:2,status:'skipped',step:0}));localStorage.setItem('vua-last-page','settings-modules');localStorage.setItem('vua-locale','en');localStorage.setItem('vua.component-update-qa',${JSON.stringify(this.config.runId)});`);
      await this.reload(window);
    }
    for (let attempt = 0; attempt < 100; attempt++) {
      if (await window.webContents.executeJavaScript("Boolean(document.querySelector('#root')?.children.length)")) break;
      if (attempt === 99) throw Error("Installed QA renderer did not mount");
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    await window.webContents.session.flushStorageData();
    const installed = installedContext();
    this.trace("main-ready", { hostBuild: installed?.host.host.build, allowsLaunchHealth: this.allowsLaunchHealth() });
    atomic(path.join(this.directory, "component-update-ready.json"), { schemaVersion: "0.1", runId: this.config.runId, status: "ready", pid: process.pid,
      host: { version: installed?.host.host.version, build: installed?.host.host.build }, amf: { version: installed?.amf.amf.version, build: installed?.amf.amf.build },
      allowsLaunchHealth: this.allowsLaunchHealth(), scope: "Real installed Main/preload/native providers and update APIs; no UI or clean-Windows acceptance claim" });
    const scan = () => {
      for (const name of fs.readdirSync(this.commands).filter(name => /^qa-[a-f0-9]{32}\.json$/.test(name)).sort()) {
        const file = path.join(this.commands, name);
        if (fs.lstatSync(file).isSymbolicLink() || fs.statSync(file).size > 16384) continue;
        // Claim before dispatch and retain the claim across restart. Never replay an interrupted action implicitly.
        try { fs.closeSync(fs.openSync(`${file}.claimed`, "wx")); } catch { continue; }
        void Promise.resolve().then(() => this.handle(window, JSON.parse(fs.readFileSync(file, "utf8")) as Record<string, unknown>)).then(
          result => atomic(path.join(this.replies, name), { ok: true, pid: process.pid, result }),
          error => atomic(path.join(this.replies, name), { ok: false, pid: process.pid, error: error instanceof Error ? error.message : "diagnostic_failed" }),
        );
      }
    };
    const timer = setInterval(scan, 100); timer.unref(); app.once("will-quit", () => clearInterval(timer)); scan();
  }
  private async reload(window: BrowserWindow): Promise<void> {
    await new Promise<void>((resolve, reject) => {
      window.webContents.once("did-finish-load", () => resolve());
      window.webContents.once("did-fail-load", () => reject(Error("Installed update diagnostic reload failed")));
      window.webContents.reload();
    });
  }
  private async handle(window: BrowserWindow, request: Record<string, unknown>): Promise<unknown> {
    const component = request.component;
    if (["check", "stage", "cancel", "apply", "restore"].includes(String(request.method))) {
      if (component !== "host" && component !== "amf") throw Error("Invalid component");
      if (request.method === "stage" && !digest(request.manifestId)) throw Error("Invalid manifest");
      const call = request.method === "stage" ? `window.vua.componentUpdates.stage(${JSON.stringify(component)},${JSON.stringify(request.manifestId)})`
        : `window.vua.componentUpdates.${request.method}(${JSON.stringify(component)})`;
      return window.webContents.executeJavaScript(call);
    }
    switch (request.method) {
      case "snapshot": return window.webContents.executeJavaScript("Promise.all([window.vua.componentUpdates.snapshot('host'),window.vua.componentUpdates.snapshot('amf'),window.vua.amfModule.snapshot(),window.vua.amfUiRuntime.snapshot()]).then(([host,amf,module,runtime])=>({host,amf,module,runtime,uiVersion:document.documentElement.dataset.amfUiVersion,text:document.body.innerText.slice(0,4000)}))");
      case "set-enabled":
        if (typeof request.enabled !== "boolean") throw Error("Invalid module preference");
        return window.webContents.executeJavaScript(`window.vua.amfModule.setEnabled(${JSON.stringify(request.enabled)})`);
      case "navigate": {
        if (!["settings-about", "settings-modules", "warehouse", "factory", "home"].includes(String(request.page))) throw Error("Invalid diagnostic page");
        await window.webContents.executeJavaScript(`localStorage.setItem('vua-last-page',${JSON.stringify(request.page)})`);
        await this.reload(window); return { page: request.page };
      }
      case "fail-ui":
        if (request.manifestId != null && !digest(request.manifestId)) throw Error("Invalid fault identity");
        this.blockedUi = request.manifestId == null ? undefined : request.manifestId as string; return { armed: !!this.blockedUi };
      case "fail-host-health":
        if (request.build != null && (typeof request.build !== "string" || !/^[a-f0-9]{40}$/.test(request.build))) throw Error("Invalid fault identity");
        this.config.failHostBuild = request.build == null ? null : request.build as string;
        atomic(path.join(this.directory, "component-update-qa.json"), this.config); return { armed: !!this.config.failHostBuild };
      case "capture": {
        const file = path.join(this.directory, `component-update-${process.pid}-${Date.now()}.png`);
        fs.writeFileSync(file, (await window.webContents.capturePage()).toPNG()); return { file };
      }
      case "show": window.show(); window.focus(); return { visible: window.isVisible() };
      case "quit": app.quit(); return { quitting: true };
      default: throw Error("Unknown installed-update diagnostic action");
    }
  }
}
