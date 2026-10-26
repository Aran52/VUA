/** First-party AMF UI generation handshake; paths and code selection remain Main-owned. */
import type { VuaDesktopApiV6 } from "./update-plans.js";
export interface AmfUiGenerationV01 {
  readonly id: string;
  readonly instanceId: string;
  readonly version: string;
  readonly build: string;
  readonly uiAbi: "amf-ui/0.1";
}
export type AmfUiTransitionV01 = "quiescing" | "loading";
export interface AmfUiRuntimeSnapshotV01 {
  readonly schemaVersion: "0.1";
  readonly available: boolean;
  readonly phase: "idle" | AmfUiTransitionV01 | "restoring";
  readonly active?: AmfUiGenerationV01 | undefined;
  readonly candidate?: AmfUiGenerationV01 | undefined;
  readonly transactionId?: string | undefined;
  readonly error?: string | undefined;
}
export interface DesktopAmfUiRuntimeApiV01 {
  snapshot(): Promise<AmfUiRuntimeSnapshotV01>;
  respond(transactionId: string, phase: AmfUiTransitionV01, accepted: boolean, reason?: "busy" | "restart_required" | "failed"): Promise<void>;
  subscribe(listener: (snapshot: AmfUiRuntimeSnapshotV01) => void): () => void;
}
export interface VuaDesktopApiV7 extends VuaDesktopApiV6 {
  readonly amfUiRuntime?: DesktopAmfUiRuntimeApiV01;
}
