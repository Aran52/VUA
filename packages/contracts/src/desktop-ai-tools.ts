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
export interface DesktopAiToolsApiV01 {
  snapshot(): Promise<AiToolsSnapshotV01>;
  setEnabled(enabled:boolean):Promise<AiToolsSnapshotV01>;
  prompt(locale:AiConnectionLocale):Promise<string>;
  copyPrompt(locale:AiConnectionLocale):Promise<void>;
  resolve(id:string,decision:"approve"|"reject"):Promise<AiToolsSnapshotV01>;
  subscribe(listener:(snapshot:AiToolsSnapshotV01)=>void):()=>void;
  /** While VUA is not the foreground window: blink the tray icon and flash the taskbar button. */
  attention?():Promise<void>;
}
export interface VuaDesktopApiV4 extends VuaDesktopApiV3 {readonly aiTools?:DesktopAiToolsApiV01}
