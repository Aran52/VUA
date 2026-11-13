import { BrowserWindow, type WebPreferences } from "electron";
import { vuaAppIcon } from "./app-icon.js";

/** A presentation-only window; no Gateway, account or installation work runs here. */
export function createStartupWindow(options: {
  rendererUrl?: string;
  rendererFile: string;
  preferences: WebPreferences;
  onShown?: () => void;
}): BrowserWindow {
  const window = new BrowserWindow({
    width: 480, height: 320, useContentSize: true, frame: false, thickFrame: false, show: false,
    resizable: false, maximizable: false, minimizable: false,
    backgroundColor: "#ffffff", title: "VUA", webPreferences: options.preferences,
    icon: vuaAppIcon(),
  });
  window.center();
  window.setMenu(null);
  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", event => event.preventDefault());
  let shown = false;
  const showOnce = () => {
    if (shown || window.isDestroyed()) return;
    shown = true;
    window.show();
    options.onShown?.();
  };
  window.once("ready-to-show", showOnce);
  window.webContents.once("did-finish-load", showOnce);
  const load = options.rendererUrl
    ? (() => {
        const url = new URL("splash.html", `${options.rendererUrl.replace(/\/$/, "")}/`);
        url.searchParams.set("surface", "splash");
        return window.loadURL(url.toString());
      })()
    : window.loadFile(options.rendererFile, { query: { surface: "splash" } });
  void load.catch(() => { /* Teardown or missing splash falls back to the main window. */ });
  return window;
}
