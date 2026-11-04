import { useLayoutEffect } from "react";
import { fixedThemeForFamily, isThemeFamily, loadThemeFamily, loadThemePreference, resolveTheme } from "./theme-preference.ts";
import { storageKeys } from "./storage-keys.ts";

/** Auxiliary windows share the main window's saved appearance and system changes. */
export function useSurfaceAppearance() {
  useLayoutEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const query = new URLSearchParams(window.location.search);
    const read = (key: string) => { try { return localStorage.getItem(key); } catch { return null; } };
    const sync = () => {
      const root = document.documentElement;
      const override = query.get("theme");
      const familyOverride = query.get("themeFamily");
      root.dataset.theme = override === "light" || override === "dark"
        ? override : resolveTheme(loadThemePreference(read), media.matches);
      const family = isThemeFamily(familyOverride) ? familyOverride : loadThemeFamily(read);
      root.dataset.themeFamily = family;
      root.dataset.theme = fixedThemeForFamily(family) ?? root.dataset.theme;
      if (query.get("hc") === "on" || read(storageKeys.hc) === "on") root.dataset.hc = "on";
      else delete root.dataset.hc;
      if (query.get("effects") === "off" || read(storageKeys.effects) === "off") root.dataset.effects = "off";
      else delete root.dataset.effects;
    };
    const onStorage = (event: StorageEvent) => {
      if (event.key === null || [storageKeys.theme, storageKeys.themeFamily, storageKeys.hc, storageKeys.effects].some(key => key === event.key)) sync();
    };
    sync();
    window.addEventListener("storage", onStorage);
    media.addEventListener("change", sync);
    return () => {
      window.removeEventListener("storage", onStorage);
      media.removeEventListener("change", sync);
    };
  }, []);
}
