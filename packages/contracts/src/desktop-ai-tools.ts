import type { VuaDesktopApiV3 } from "./desktop-distribution.js";
export type AiConnectionLocale = "en" | "zh-CN" | "ja" | "ko";
export interface AiActionRequestV01 {
  readonly id: string; readonly tool: string; readonly title: string; readonly explanation: string;
  readonly expectedResult: string; readonly createdAt: string; readonly expiresAt: string;
  readonly state: "pending" | "running" | "accepted" | "rejected" | "expired" | "failed";
  readonly details: Readonly<Record<string,unknown>>; readonly result?: unknown; readonly error?: string;
}
export interface AiToolsSnapshotV01 {
  readonly schemaVersion: "0.1"; readonly enabled: boolean; readonly endpoint: string | null;
  readonly connections: number; readonly requests: readonly AiActionRequestV01[]; readonly error: string | null;
}
/** Text for the system notification that asks the player to come back to VUA. */
export interface AiAttentionNoticeV01 { readonly requestId: string; readonly title: string; readonly body: string }
export interface DesktopAiToolsApiV01 {
  snapshot(): Promise<AiToolsSnapshotV01>;
  setEnabled(enabled:boolean):Promise<AiToolsSnapshotV01>;
  prompt(locale:AiConnectionLocale):Promise<string>;
  copyPrompt(locale:AiConnectionLocale):Promise<void>;
  resolve(id:string,decision:"approve"|"reject"):Promise<AiToolsSnapshotV01>;
  subscribe(listener:(snapshot:AiToolsSnapshotV01)=>void):()=>void;
  /** While VUA is not the foreground window: flash the tray icon and show a Windows notification. */
  attention?(notice:AiAttentionNoticeV01):Promise<void>;
  /** The player clicked a notification; the listener receives the request to show. */
  subscribeReveal?(listener:(requestId:string)=>void):()=>void;
}
export interface VuaDesktopApiV4 extends VuaDesktopApiV3 {readonly aiTools?:DesktopAiToolsApiV01}
