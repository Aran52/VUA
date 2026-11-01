import { nativeImage } from "electron";
import { appIconPng } from "./app-icon-data.js";
/** Embedded for both sandboxed packaged builds and development, without resource-path fallbacks. */
export const vuaAppIcon = () => nativeImage.createFromBuffer(Buffer.from(appIconPng, "base64"));
