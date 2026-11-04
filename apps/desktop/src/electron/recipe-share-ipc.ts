import { app, dialog, ipcMain, net, type BrowserWindow } from "electron";
import { APPLICATION_CONTRACT_VERSION, isShareImageOptions, shareCodeFromLink, type ApplicationRequestV01, type PendingRecipeShare } from "@vua/contracts";
import type { ModuleProvider } from "./module-provider.js";
import { RecipeShareClient } from "./recipe-share-client.js";
import { RecipeShareIO } from "./recipe-share-io.js";
import { installedContext } from "./installed-context.js";

declare const __VUA_SHARE_ORIGIN__: string;
export const SHARE_ORIGIN = typeof __VUA_SHARE_ORIGIN__ === "undefined" ? "https://ph-r.com" : __VUA_SHARE_ORIGIN__;
let pending: PendingRecipeShare | null = null;
let windowForShare: (()=>BrowserWindow|null) | null = null;
function announce(): void { const window=windowForShare?.();if(window&&!window.isDestroyed()){window.show();window.webContents.send("vua:share:pending");} }
export function queueShareLink(value: string): boolean {
  if(!/^vua:/iu.test(value)||!shareCodeFromLink(value,SHARE_ORIGIN))return false;
  pending={input:value};announce();return true;
}

export function registerRecipeSharing(options:{provider:ModuleProvider;root:()=>string;assertReady:()=>void;assertSender:(event:Electron.IpcMainInvokeEvent)=>void;window:()=>BrowserWindow|null}): void {
  windowForShare=options.window;
  const clients=new Map<string,{client:RecipeShareClient;io:RecipeShareIO}>();
  const services=()=>{
    options.assertReady();const root=options.root();
    let service=clients.get(root);if(service)return service;
    const assertReady=()=>{options.assertReady();if(options.root()!==root)throw new Error("share_unavailable");};
    const client=new RecipeShareClient({origin:SHARE_ORIGIN,directory:root,fetch:(input,init)=>net.fetch(input instanceof URL ? input.href : input,init),assertReady});
    const io=new RecipeShareIO({directory:root,client,assertReady,saveDialog:async format=>{
      const settings={defaultPath:`VUA-turntable.${format}`,filters:[{name:format.toUpperCase(),extensions:[format]}]};
      const window=options.window();const result=window?await dialog.showSaveDialog(window,settings):await dialog.showSaveDialog(settings);
      return result.canceled||!result.filePath?null:result.filePath;
    },invoke:async(method,params)=>{
      assertReady();
      const command=method.endsWith("exportShareImage");
      const request={contractVersion:APPLICATION_CONTRACT_VERSION,requestId:crypto.randomUUID(),kind:command?"command":"query",...(command?{commandId:crypto.randomUUID()}:{}),method,params} as ApplicationRequestV01;
      const response=await options.provider.invoke(request);
      if(!response.ok)throw new Error(response.error.code.replace(/^vua\.amf_production\./u,""));
      return response.value;
    }});
    service={client,io};clients.set(root,service);return service;
  };
  const protect=async <T>(action:()=>Promise<T>):Promise<T>=>{try{return await action();}catch(error){const message=error instanceof Error?error.message:"";throw new Error(/^share_[a-z_]+$/u.test(message)?message:"share_unavailable");}};
  const check=(event:Electron.IpcMainInvokeEvent)=>{options.assertSender(event);options.assertReady();};
  ipcMain.handle("vua:share:export",(event,value:unknown)=>{check(event);if(!isShareImageOptions(value))throw new Error("share_invalid");return protect(()=>services().io.exportImage(value));});
  ipcMain.handle("vua:share:read",(event,text:unknown)=>{check(event);if(typeof text!=="string")throw new Error("share_invalid");return protect(()=>services().client.read(text));});
  ipcMain.handle("vua:share:read-image",(event,files:unknown)=>{check(event);if(!Array.isArray(files)||files.length!==1||typeof files[0]!=="string")throw new Error("share_image_invalid");return protect(()=>services().io.readImage(files[0] as string));});
  ipcMain.handle("vua:share:queue-image",async(event,files:unknown)=>{
    options.assertSender(event);
    try {check(event);if(!Array.isArray(files)||files.length!==1||typeof files[0]!=="string")throw new Error("share_image_invalid");pending={received:await services().io.readImage(files[0])};}
    catch(error){const message=error instanceof Error?error.message:"";pending={error:/^share_[a-z_]+$/u.test(message)?message:"share_image_invalid"};}
    announce();
  });
  ipcMain.handle("vua:share:list",(event,recipeId:unknown)=>{check(event);if(recipeId!==undefined&&(typeof recipeId!=="string"||!/^[A-Za-z0-9-]{1,100}$/u.test(recipeId)))throw new Error("share_invalid");return protect(()=>services().client.list(recipeId));});
  ipcMain.handle("vua:share:withdraw",(event,code:unknown)=>{check(event);if(typeof code!=="string")throw new Error("share_invalid");return protect(()=>services().client.withdraw(code));});
  ipcMain.handle("vua:share:has-pending",event=>{options.assertSender(event);return pending!==null;});
  ipcMain.handle("vua:share:take-pending",event=>{check(event);const value=pending;pending=null;return value;});
  // Verified NSIS context only. Legacy ZIP and development never change OS registration.
  if(app.isPackaged&&installedContext()&&!app.commandLine.hasSwitch("vua-smoke-test"))app.setAsDefaultProtocolClient("vua");
}
