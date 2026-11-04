export { default } from "./AmfPages.tsx";
export const uiAbi = "amf-ui/0.1";
declare const __VUA_AMF_VERSION__: string;
declare const __VUA_AMF_BUILD__: string;
export const componentVersion = __VUA_AMF_VERSION__;
export const componentBuild = __VUA_AMF_BUILD__;
export const hotReloadAbi = "amf-hot/0.1";
import { readComposeDraft } from "../../app/compose-draft-store.ts";
export function canUnload(): boolean { return !readComposeDraft().dirty; }
