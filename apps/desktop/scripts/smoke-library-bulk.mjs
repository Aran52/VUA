// Real Chromium DOM, temporary profile, synthetic Gateway; no account access or real files.
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import path from "node:path";
import os from "node:os";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const corners = process.argv.includes("--corners");
let server, window, output;
setTimeout(() => { console.error("Isolated library UI smoke timed out"); app.exit(1); }, 60000);
const trace = stage => { if (process.env.VUA_TEST_TRACE === "1") console.log(stage); };
app.on("window-all-closed", () => {});
async function main() {
  try {
    output = await mkdtemp(path.join(os.tmpdir(), "vua-library-bulk-"));
    const profile = path.join(output, "isolated-profile"); await mkdir(profile); app.setPath("userData", profile);
    await app.whenReady();
    server = await createServer({ configFile: false, appType: "custom", root, cacheDir: path.join(output, "vite-cache"), plugins: [react()], server: { host: "127.0.0.1", port: 0 }, optimizeDeps: { force: true, include: ["react", "react-dom/client", "@vua/contracts"] } });
    server.middlewares.use(async (req, res, next) => {
      if (!req.url?.startsWith("/__library-bulk")) return next();
      const locale = new URL(req.url, "http://localhost").searchParams.get("locale") ?? "en";
      if (!["en", "zh-CN", "ja", "ko"].includes(locale)) return next();
      res.setHeader("Content-Type", "text/html");
      res.end(await server.transformIndexHtml(req.url, `<html><body><div id="root"></div><script>localStorage.setItem('vua-locale',${JSON.stringify(locale)})</script><script type="module" src="/scripts/fixtures/${corners ? "library-corners" : "library-bulk"}.tsx"></script></body></html>`));
    });
    await server.listen();
    window = new BrowserWindow({ show: false, width: 1200, height: 900, webPreferences: { contextIsolation: true, nodeIntegration: false, sandbox: true, offscreen: corners, backgroundThrottling: false } });
    const errors = []; window.webContents.on("console-message", event => { if (event.level === "error") errors.push(event.message); });
    const checks = [];
    for (const locale of ["en", "zh-CN", "ja", "ko"]) {
      await window.loadURL(`http://127.0.0.1:${server.httpServer.address().port}/__library-bulk?locale=${locale}`);
      trace(`${locale}: loaded`);
      await window.webContents.executeJavaScript(`new Promise((resolve,reject)=>{let n=0;const t=setInterval(()=>{if(window.libraryBulkReview){clearInterval(t);resolve()}else if(++n>200){clearInterval(t);reject(Error('Fixture timeout'))}},50)})`);
      checks.push(...(await window.webContents.executeJavaScript("window.libraryBulkReview.run()")).map(check => `${locale}: ${check}`));
      trace(`${locale}: fixture complete`);
      if (!corners) {
        window.webContents.debugger.attach("1.3");
        await window.webContents.debugger.sendCommand("DOM.enable");
        await window.webContents.debugger.sendCommand("CSS.enable");
      }
      // Capture the resting appearance, independent of hidden-window animation clocks.
      await window.webContents.executeJavaScript("document.documentElement.dataset.effects='off'");
      for (const theme of ["dark", "light"]) {
        await window.webContents.executeJavaScript(`document.documentElement.dataset.theme=${JSON.stringify(theme)}`);
        if (!corners) {
          const { root: documentNode } = await window.webContents.debugger.sendCommand("DOM.getDocument");
          const { nodeIds } = await window.webContents.debugger.sendCommand("DOM.querySelectorAll", { nodeId: documentNode.nodeId, selector: ".vua-taskbar__task-main" });
          for (const pseudo of [["hover"], ["active"], ["focus", "focus-visible"], ["hover", "active", "focus", "focus-visible"]]) {
            for (const nodeId of nodeIds) await window.webContents.debugger.sendCommand("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: pseudo });
            const transparent = await window.webContents.executeJavaScript('[...document.querySelectorAll(".vua-taskbar__task, .vua-taskbar__task-main, .vua-taskbar__task-glyph")].every(row => getComputedStyle(row).backgroundColor === "rgba(0, 0, 0, 0)")');
            if (!transparent) throw Error(`${locale}/${theme}: notification background in ${pseudo.join("+")}`);
            checks.push(`${locale}/${theme}: every download/sync task row stays transparent in ${pseudo.join("+")}`);
          }
          for (const nodeId of nodeIds) await window.webContents.debugger.sendCommand("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: [] });
        }
        await window.webContents.capturePage({}, { stayHidden: true, stayAwake: true }); await new Promise(resolve => setTimeout(resolve, 350));
        const capture = await window.webContents.capturePage({}, { stayHidden: true, stayAwake: true });
        if (corners) {
          const sample = await window.webContents.executeJavaScript(`(() => {
            const marker=document.querySelector('.vua-library-corner[data-corner-state="cloud"]'), box=marker.getBoundingClientRect();
            const hit=document.elementFromPoint(box.left+45,box.top+5);
            return {left:box.left,top:box.top,width:innerWidth,solid:getComputedStyle(marker).backgroundColor,
              tailPasses:hit?.closest('[data-selection-id]')===marker.closest('[data-selection-id]')&&!hit?.closest('.vua-library-marker')};
          })()`);
          const {width}=capture.getSize(), scale=width/sample.width, bitmap=capture.toBitmap();
          const pixel=(x,y)=>{const at=(Math.floor((sample.top+y)*scale)*width+Math.floor((sample.left+x)*scale))*4;return [...bitmap.subarray(at,at+3)];};
          const solid=sample.solid.match(/[\d.]+/g).slice(0,3).map(Number);
          const distance=(a,b)=>Math.sqrt(a.reduce((sum,value,index)=>sum+(value-b[index])**2,0));
          const core=pixel(20,2),tail=pixel(45,5),clear=pixel(70,5);
          if(Math.min(distance(core,solid),distance([...core].reverse(),solid))>5) throw Error(`${locale}/${theme}: the frame painted over the status triangle`);
          if(distance(tail,clear)<10||distance(tail,core)<5||!sample.tailPasses) throw Error(`${locale}/${theme}: the diagonal tail must blend into transparency without blocking the preview`);
          checks.push(`${locale}/${theme}: status triangle paints over its frame; diagonal fade remains visible and passes pointer input`);
        }
        await writeFile(path.join(output, `${corners ? "corners" : "notifications"}-${locale}-${theme}.png`), capture.toPNG());
        if(corners){
          const fullSize=window.getContentSize(); window.setContentSize(640,900);
          await new Promise(resolve=>setTimeout(resolve,350));
          await window.webContents.executeJavaScript(`new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))`);
          const fits=()=>window.webContents.executeJavaScript(`(() => {
            const filters=document.querySelector('.vua-warehouse__filters'), bounds=filters.getBoundingClientRect();
            return filters.scrollWidth<=filters.clientWidth && [...filters.querySelectorAll('input,select')].every(control=>{
              const rect=control.getBoundingClientRect();return rect.width>0 && rect.left>=bounds.left-1 && rect.right<=bounds.right+1;
            });
          })()`);
          if(!await fits())throw Error(`${locale}/${theme}: material filters overflow a narrow page`);
          await window.webContents.executeJavaScript(`(async()=>{
            const use=document.querySelector('.vua-library-use-filter select');
            if(use.value!==use.options[1].value){Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set.call(use,use.options[1].value);use.dispatchEvent(new Event('change',{bubbles:true}));}
            await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
          })()`);
          if(!await fits())throw Error(`${locale}/${theme}: expanded use filters overflow a narrow page`);
          await window.webContents.executeJavaScript(`new Promise((resolve,reject)=>{
            let attempts=0;const timer=setInterval(()=>{
              if(document.querySelectorAll('[data-selection-id]').length===5&&!document.querySelector('.vua-skeleton')){clearInterval(timer);resolve();}
              else if(++attempts>100){clearInterval(timer);reject(Error('narrow material results did not settle'));}
            },40);
          })`);
          checks.push(`${locale}/${theme}: ordinary and expanded material filters stay aligned inside a 640 px page`);
          await new Promise(resolve=>setTimeout(resolve,350));
          await writeFile(path.join(output,`corners-${locale}-${theme}-narrow.png`),(await window.webContents.capturePage({}, {stayHidden:true,stayAwake:true})).toPNG());
          window.setContentSize(...fullSize);
          await new Promise(resolve=>setTimeout(resolve,350));
        }
      }
      if (!corners) window.webContents.debugger.detach();
    }
    if (errors.length > 0) throw Error(errors.join("\n"));
    await writeFile(path.join(output, "checks.json"), JSON.stringify({ scope: "Real Chromium DOM with synthetic Gateway; not account, material or human UI acceptance", checks }, null, 2));
    console.log(JSON.stringify({ passed: checks.length, evidence: output }));
    window.destroy(); await server.close(); app.exit(0);
  } catch (error) {
    console.error(error);
    if(window&&!window.isDestroyed()&&output){await writeFile(path.join(output,'failure.png'),(await window.webContents.capturePage()).toPNG());console.error(JSON.stringify({evidence:output}));}
    window?.destroy(); await server?.close(); app.exit(1);
  }
}
void main();
