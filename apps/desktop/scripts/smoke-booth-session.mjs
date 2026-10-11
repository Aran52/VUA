// Actual Main/preload/renderer/providers with synthetic BOOTH pages and an isolated
// Chromium profile. No real account, product, download or Explorer window is used.
import { app, BrowserWindow, dialog, session, shell } from "electron";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const output = fs.mkdtempSync(path.join(os.tmpdir(), "vua-booth-session-"));
const home = path.join(output, "home"), profile = path.join(output, "profile");
for (const directory of [home, profile, path.join(home,"AppData","Roaming"), path.join(home,"AppData","Local")]) fs.mkdirSync(directory,{recursive:true});
app.setPath("home",home); app.setPath("appData",path.join(home,"AppData","Roaming")); app.setPath("userData",profile);
process.env.LOCALAPPDATA=path.join(home,"AppData","Local"); process.env.VUA_DEV_USER_DATA=profile;
process.env.VUA_PROVIDER_EXECUTABLE=process.env.VUA_TEST_HOST_EXECUTABLE ?? path.resolve(desktop,"../../target/release/vua-orchestrator-provider.exe");
process.env.VUA_AMF_EXECUTABLE=process.env.VUA_TEST_AMF_EXECUTABLE ?? path.resolve(desktop,"../../target/release/vua-amf-provider.exe");
delete process.env.VUA_RENDERER_URL;
app.getPreferredSystemLanguages=()=>["zh-CN"];
BrowserWindow.prototype.show=function(){}; BrowserWindow.prototype.focus=function(){};
dialog.showErrorBox=()=>console.error("Desktop bootstrap failed (native dialog suppressed in this smoke)");
const folderRequests=[];
shell.openPath=async folder=>{folderRequests.push(folder);return "";};
const checks=[];
const requests=[];
let window, exitCode=1;
app.on("will-quit",()=>{if(exitCode)app.exit(exitCode);});
const deadline=setTimeout(()=>{console.error("BOOTH session smoke timed out");app.exit(1);},180_000);
const assert=(ok,label)=>{if(!ok)throw new Error(label);checks.push(label);};
async function waitFor(check,label,limit=15000){const start=Date.now();while(Date.now()-start<limit){if(await check())return;await delay(60);}throw new Error(`Timeout: ${label}`);}
async function run(){try{
  createRequire(import.meta.url)(path.join(desktop,"dist/packaged-electron/main.js"));
  await app.whenReady();
  const remote=session.fromPartition("persist:vua-remote");
  let libraryRequests=0;
  await remote.protocol.handle("https",async request=>{
    const url=new URL(request.url);
    // Intercepted protocol Requests omit Chromium's Cookie header. The fixture
    // models its server session from this isolated jar; it is not a wire-cookie test.
    const authenticated=(await remote.cookies.get({url:request.url})).some(cookie=>cookie.name==="vua_test_session"&&cookie.value==="synthetic");
    requests.push({path:url.pathname,authenticated});
    if(url.hostname!=="accounts.booth.pm")return new Response("Synthetic unsupported origin",{status:404});
    if(url.pathname==="/session-complete"){
      await remote.cookies.set({url:"https://booth.pm/",domain:".booth.pm",name:"vua_test_session",value:"synthetic",path:"/",secure:true,httpOnly:true});
      return Response.redirect("https://accounts.booth.pm/library?page=1",302);
    }
    if(url.pathname.startsWith("/library")){
      libraryRequests++;
      if(!authenticated)return Response.redirect("https://accounts.booth.pm/users/sign_in",302);
      return new Response('<html><body><header data-user-name="Synthetic account"></header><h1>Synthetic account library</h1><div id="js-library-search-header"></div></body></html>',{headers:{"content-type":"text/html; charset=utf-8"}});
    }
    return new Response('<html><body><form action="/users/auth/pixiv"></form><a id="finish" href="/session-complete">Finish synthetic sign-in</a></body></html>',{headers:{"content-type":"text/html; charset=utf-8"}});
  });
  await waitFor(()=>{window=BrowserWindow.getAllWindows().find(w=>w.webContents.getURL().startsWith("file:")&&!w.webContents.getURL().includes("surface="));return window&&!window.webContents.isLoading();},"renderer");
  const js=source=>window.webContents.executeJavaScript(source,true);
  window.webContents.setBackgroundThrottling(false);
  const click=async id=>{await waitFor(()=>js(`!!document.querySelector('[data-nav-id="${id}"]')`),id);await js(`document.querySelector('[data-nav-id="${id}"]').click()`);};
  const views=()=>window.contentView.children.filter(view=>view.webContents&&view.webContents!==window.webContents);
  await waitFor(()=>js('document.querySelector(".vua-tour")?.dataset.tourStep==="welcome"'),"welcome");
  await js('document.querySelector(".vua-tour__actions button").click()');
  await waitFor(()=>js('!!document.querySelector(".vua-onboarding")&&!document.querySelector(".vua-boot-splash")'),"guide");
  await js('document.querySelector(".vua-journey-top button").click()');
  await click("nav-warehouse");
  await waitFor(()=>js('document.querySelector("[data-nav-id=warehouse-sync]")?.textContent.includes("登录")'),"signed-out library");
  assert(libraryRequests===0,"An empty session does not make anonymous account-library requests");
  await click("shell-settings");await click("nav-settings-accounts");await click("accounts-booth-login");
  await waitFor(()=>views().some(view=>view.webContents.getURL()==="https://accounts.booth.pm/users/sign_in"),"embedded login");
  const login=views().find(view=>view.webContents.getURL()==="https://accounts.booth.pm/users/sign_in");
  await login.webContents.executeJavaScript('document.querySelector("#finish").click()');
  await waitFor(()=>login.webContents.getURL().startsWith("https://accounts.booth.pm/library"),"completed synthetic login");
  assert((await remote.cookies.get({url:"https://accounts.booth.pm/library"})).some(cookie=>cookie.domain===".booth.pm"),"Library-URL matching recognizes parent-domain login storage without guessing cookie names");
  const fetchResponse=await remote.fetch("https://accounts.booth.pm/library?page=1");
  assert(fetchResponse.status===200&&fetchResponse.url==="","Pinned Electron reproduces the empty fetch Response.url defect");
  await fetchResponse.text();
  assert((await js('window.vua.remoteContent.authProbe()')).authOk===true,"Main authenticates an actual session read even though fetch Response.url is empty");
  await js('document.querySelector(".vua-login-browser__bar .vua-import__browse-button--close").click()');
  await waitFor(()=>js('!document.querySelector("[data-nav-id=accounts-booth-login]")&&document.querySelector(".vua-settings-account__head")?.textContent.includes("Synthetic account")'),"account refresh after close");
  assert(true,"Closing embedded login refreshes Settings → Accounts to signed in");
  await click("logo-home");await click("nav-warehouse");
  await waitFor(()=>js('document.querySelector("[data-nav-id=warehouse-sync]")?.textContent.includes("同步 BOOTH")'),"signed-in library");
  assert(true,"Returning to the asset library offers sync instead of another login");
  await click("warehouse-sync");
  await waitFor(()=>js('document.querySelector(".vua-warehouse")?.textContent.includes("账号列表同步完成")'),"completed native provider sync",30000);
  assert(true,"Three synthetic empty account libraries sync through the real AMF provider");
  await click("warehouse-open-folder");
  await waitFor(()=>folderRequests.length===1,"Explorer handoff");
  assert(folderRequests[0]===path.join(profile,"modules","amf","data","warehouse"),"Explorer action resolves the active AMF material root in Main");
  await js('window.vua.library.openFolder("C:/ignored-renderer-path")');
  assert(folderRequests[1]===folderRequests[0],"Renderer cannot choose another directory through the folder action");
  const openFolder=shell.openPath;
  shell.openPath=async()=>{throw new Error("Synthetic private system error");};
  assert((await js('window.vua.library.openFolder()')).opened===false,"Folder handler errors produce a bounded failure result without private error text");
  shell.openPath=openFolder;
  for(const locale of ["en","zh-CN","ja","ko"]){
    await js(`localStorage.setItem('vua-locale','${locale}');location.reload()`);
    await waitFor(()=>js(`document.documentElement.lang==='${locale}'&&!!document.querySelector('[data-nav-id=nav-warehouse]')&&!document.querySelector('.vua-boot-splash')`),locale);
    await click("nav-warehouse");
    for(const theme of ["dark","light"]){
      await click("shell-settings");await click("nav-settings-theme");
      await js(`document.querySelector('[data-nav-id="theme-${theme}"]').click()`);
      await click("shell-back");
      for(const [width,height,zoom] of [[1400,900,1],[960,600,1.5]]){
        window.setSize(width,height);window.webContents.setZoomFactor(zoom);
        await waitFor(async()=>{const viewport=await js('({width:innerWidth,height:innerHeight})'),bounds=window.getContentBounds();return Math.abs(viewport.width*zoom-bounds.width)<2&&Math.abs(viewport.height*zoom-bounds.height)<2;},"resized viewport");
        await delay(350); // Let native painting and theme transitions catch up with CSS layout.
        await js(`document.querySelector('.vua-warehouse').scrollTop=0`);
        await js(`document.querySelector('.vua-warehouse__filters').scrollIntoView({block:'nearest'})`);
        await delay(100);
        const layout=await js(`(()=>{const f=document.querySelector('.vua-warehouse__filters'),controls=[...f.querySelectorAll('input,select')],r=controls.map(c=>c.getBoundingClientRect()),s=r.slice(1);return {onlyTwo:controls.filter(c=>c.tagName==='SELECT').length===2,aligned:Math.abs(s[0].top-s[1].top)<1&&Math.abs(s[0].height-s[1].height)<1,responsive:innerWidth>1000?Math.abs(r[0].top-s[0].top)<1:r[0].bottom<s[0].top,height:r.every(x=>Math.abs(x.height-r[0].height)<1),fits:r.every(x=>x.left>=0&&x.right<=innerWidth+1&&x.bottom<=innerHeight+1),noOverflow:document.documentElement.scrollWidth<=innerWidth+1};})()`);
        if(!Object.values(layout).every(Boolean)){console.error(JSON.stringify({locale,theme,width,height,zoom,layout,geometry:await js(`({width:innerWidth,height:innerHeight,rects:[...document.querySelectorAll('.vua-warehouse__filters input,.vua-warehouse__filters select')].map(x=>x.getBoundingClientRect().toJSON())})`)}));fs.writeFileSync(path.join(output,"layout-failure.png"),(await window.webContents.capturePage()).toPNG());}
        assert(Object.values(layout).every(Boolean),`${locale} ${theme}: aligned search/source/state controls at ${width}×${height}, zoom ${zoom}`);
        if(width===1400){await window.webContents.capturePage({}, {stayHidden:true,stayAwake:true});await delay(80);fs.writeFileSync(path.join(output,`library-${locale}-${theme}.png`),(await window.webContents.capturePage({}, {stayHidden:true,stayAwake:true})).toPNG());}
      }
    }
  }
  await remote.cookies.set({url:"https://booth.pm/",domain:".booth.pm",name:"vua_test_session",value:"expired",path:"/",secure:true,httpOnly:true});
  assert((await js('window.vua.remoteContent.authProbe()')).authOk===false,"Stored but expired session redirects to sign-in and is never reported authenticated");
  await js('window.vua.remoteContent.signOut()');
  assert((await js('window.vua.remoteContent.authProbe()')).authOk===false,"Sign-out clears the same browser session used by account reads");
  await js('window.vua.amfModule.setEnabled(false)');
  assert(await js('window.vua.library.openFolder().then(()=>false,()=>true)'),"Disabled AMF denies the folder action");
  fs.writeFileSync(path.join(output,"checks.json"),JSON.stringify({scope:"Actual Electron/Main/preload/renderer/providers with synthetic BOOTH and stubbed Explorer; no real account/material acceptance",checks},null,2));
  console.log(JSON.stringify({passed:checks.length,output}));exitCode=0;clearTimeout(deadline);app.quit();
}catch(error){console.error(error.stack??String(error));console.error(JSON.stringify({output,checks,requests,views:window?.contentView.children.map(v=>v.webContents?.getURL())}));clearTimeout(deadline);app.quit();}}
void run();
