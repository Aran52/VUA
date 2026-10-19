/** vrchat-session v0.1 的渲染层端口(实验性 VRChat 登录)。返回值只含登录态与
 * 展示事实;机密从不过 Gateway 返回。 */
import type { DesktopGatewayRequestV1 } from "@vua/contracts";
import type { GatewayClient } from "./gateway-client.ts";

export const VRCHAT_TWO_FACTOR_METHODS = ["emailotp", "totp", "otp"] as const;
export type VrchatTwoFactorMethod = typeof VRCHAT_TWO_FACTOR_METHODS[number];

export interface VrchatAccountView {
  readonly displayName: string;
  /** `data:` URL or null; the renderer makes no remote request. */
  readonly thumbnail: string | null;
}

export interface VrchatStatus {
  readonly enabled: boolean;
  readonly state: "signed_in" | "needs_code" | "signed_out";
  readonly account: VrchatAccountView | null;
  readonly rememberPassword: boolean;
  readonly pendingMethods: readonly VrchatTwoFactorMethod[];
  readonly canCheckUploads: boolean;
}

export type VrchatSignInOutcome = {
  readonly outcome:
    | "signed_in" | "two_factor_required" | "invalid_credentials" | "invalid_code"
    | "not_enabled" | "no_sign_in_in_progress" | "resend_too_soon" | "sent"
    | "network" | "rate_limited" | "server";
  readonly methods?: readonly VrchatTwoFactorMethod[];
};

export interface VrchatAvatarCheck {
  readonly blueprintId: string;
  readonly state: "found" | "not_found" | "signed_out" | "error";
  readonly checkedAt: string | null;
  readonly avatar: { readonly name: string; readonly releaseStatus: string; readonly platforms: readonly string[]; readonly updatedAt: string } | null;
}

/** One of the account's own Avatars as VRChat lists it. */
export interface VrchatAvatarEntry {
  readonly id: string;
  readonly name: string;
  readonly releaseStatus: string;
  readonly platforms: readonly string[];
  readonly updatedAt: string;
  readonly hasImage: boolean;
}

export interface VrchatAvatarList {
  readonly state: "listed" | "signed_out" | "error";
  readonly listedAt: string | null;
  /** False when the account holds more Avatars than VUA reads. */
  readonly complete: boolean;
  /** Newest update first. */
  readonly avatars: readonly VrchatAvatarEntry[];
}

export interface VrchatPort {
  status(): Promise<VrchatStatus>;
  /** Turning the feature off signs the player out and removes every stored value. */
  setExperimental(enabled: boolean): Promise<VrchatStatus>;
  signIn(username: string, password: string, rememberPassword: boolean): Promise<VrchatSignInOutcome>;
  /** Signs in again with the remembered password, when there is one. */
  relogin(): Promise<VrchatSignInOutcome>;
  verifyCode(method: VrchatTwoFactorMethod, code: string): Promise<VrchatSignInOutcome>;
  resendEmailCode(): Promise<VrchatSignInOutcome>;
  signOut(): Promise<VrchatStatus>;
  forget(): Promise<VrchatStatus>;
  /** The uploaded Avatar as VRChat lists it, beside the upload record. */
  avatarCheck(blueprintId: string, refresh: boolean): Promise<VrchatAvatarCheck>;
  /** The account's own Avatars; the kept list unless `refresh`. */
  avatars(refresh: boolean): Promise<VrchatAvatarList>;
  /** An Avatar's thumbnail as a `data:` URL, read by Main. */
  avatarImage(avatarId: string): Promise<string | null>;
  /** Opens the Avatar's page on VRChat's site in the player's browser. */
  openAvatarPage(avatarId: string): Promise<boolean>;
}

type VrchatStatusWire = VrchatStatus & { readonly schemaVersion: "0.1" };
type VrchatAvatarCheckWire = VrchatAvatarCheck & { readonly schemaVersion: "0.1" };

const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const text = (v: unknown): v is string => typeof v === "string";
const isMethod = (v: unknown): v is VrchatTwoFactorMethod => (VRCHAT_TWO_FACTOR_METHODS as readonly string[]).includes(String(v));

function isThumbnail(v: unknown): v is string | null {
  return v === null || (text(v) && v.startsWith("data:image/") && v.length <= 256 * 1024);
}

function isAccount(v: unknown): v is VrchatAccountView {
  return record(v) && text(v.displayName) && v.displayName.length > 0 && v.displayName.length <= 64 && isThumbnail(v.thumbnail);
}

/** 响应守卫:恰好这些键,防机密或未知字段悄悄回流。 */
export function isVrchatStatusWire(v: unknown): v is VrchatStatusWire {
  return record(v) && v.schemaVersion === "0.1" && typeof v.enabled === "boolean"
    && ["signed_in", "needs_code", "signed_out"].includes(String(v.state))
    && (v.account === null || isAccount(v.account))
    && typeof v.rememberPassword === "boolean"
    && Array.isArray(v.pendingMethods) && v.pendingMethods.length <= 3 && v.pendingMethods.every(isMethod)
    && typeof v.canCheckUploads === "boolean"
    && Object.keys(v).length === 8;
}

const OUTCOMES = ["signed_in", "two_factor_required", "invalid_credentials", "invalid_code", "not_enabled",
  "no_sign_in_in_progress", "resend_too_soon", "sent", "network", "rate_limited", "server"] as const;

export function isVrchatSignInOutcome(v: unknown): v is VrchatSignInOutcome {
  return record(v) && (OUTCOMES as readonly string[]).includes(String(v.outcome))
    && (v.methods === undefined || (Array.isArray(v.methods) && v.methods.length <= 3 && v.methods.every(isMethod)));
}

const BLUEPRINT = /^avtr_[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/u;

export function isVrchatAvatarCheckWire(v: unknown): v is VrchatAvatarCheckWire {
  return record(v) && v.schemaVersion === "0.1" && text(v.blueprintId) && BLUEPRINT.test(v.blueprintId)
    && ["found", "not_found", "signed_out", "error"].includes(String(v.state))
    && (v.checkedAt === null || (text(v.checkedAt) && !Number.isNaN(Date.parse(v.checkedAt))))
    && (v.avatar === null || (record(v.avatar) && text(v.avatar.name) && v.avatar.name.length > 0
      && text(v.avatar.releaseStatus) && text(v.avatar.updatedAt) && Array.isArray(v.avatar.platforms)
      && v.avatar.platforms.every(text) && v.avatar.platforms.length <= 8));
}

const isEntry = (v: unknown): v is VrchatAvatarEntry => record(v) && text(v.id) && BLUEPRINT.test(v.id)
  && text(v.name) && v.name.length > 0 && v.name.length <= 128 && text(v.releaseStatus) && text(v.updatedAt)
  && Array.isArray(v.platforms) && v.platforms.length <= 8 && v.platforms.every(text) && typeof v.hasImage === "boolean"
  && Object.keys(v).length === 6;

export function isVrchatAvatarListWire(v: unknown): v is VrchatAvatarList & { readonly schemaVersion: "0.1" } {
  return record(v) && v.schemaVersion === "0.1" && ["listed", "signed_out", "error"].includes(String(v.state))
    && (v.listedAt === null || (text(v.listedAt) && !Number.isNaN(Date.parse(v.listedAt))))
    && typeof v.complete === "boolean" && Array.isArray(v.avatars) && v.avatars.length <= 1000 && v.avatars.every(isEntry);
}

function toStatus(v: VrchatStatusWire): VrchatStatus {
  const { schemaVersion, ...status } = v;
  return status;
}

function toCheck(v: VrchatAvatarCheckWire): VrchatAvatarCheck {
  const { schemaVersion, ...check } = v;
  return check;
}

export function createVrchatPort(client: GatewayClient): VrchatPort {
  const commandId = () => `vr-${crypto.randomUUID()}`;
  const ask = async (request: DesktopGatewayRequestV1): Promise<unknown> => {
    const response = await client.invoke(request);
    if (!response.ok) throw new Error(response.error.kind === "application" ? response.error.error.code : response.error.kind);
    return response.value;
  };
  const readStatus = async (request: DesktopGatewayRequestV1): Promise<VrchatStatus> => {
    const value = await ask(request);
    if (!isVrchatStatusWire(value)) throw new Error("vrchat_status_invalid");
    return toStatus(value);
  };
  const readOutcome = async (request: DesktopGatewayRequestV1): Promise<VrchatSignInOutcome> => {
    const value = await ask(request);
    if (!isVrchatSignInOutcome(value)) throw new Error("vrchat_outcome_invalid");
    return value;
  };
  return {
    status: () => readStatus({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.status", params: {} }),
    setExperimental: enabled => readStatus({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.setExperimental", params: { enabled, commandId: commandId() } }),
    signIn: (username, password, rememberPassword) => readOutcome({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.signIn", params: { username, password, rememberPassword, commandId: commandId() } }),
    relogin: () => readOutcome({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.relogin", params: { commandId: commandId() } }),
    verifyCode: (method, code) => readOutcome({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.verifyCode", params: { method, code, commandId: commandId() } }),
    resendEmailCode: () => readOutcome({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.resendEmailCode", params: { commandId: commandId() } }),
    signOut: () => readStatus({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.signOut", params: { commandId: commandId() } }),
    forget: () => readStatus({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.forget", params: { commandId: commandId() } }),
    async avatarCheck(blueprintId, refresh) {
      const value = await ask({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.avatarCheck",
        params: { blueprintId, ...(refresh ? { refresh: true } : {}) } });
      if (!isVrchatAvatarCheckWire(value)) throw new Error("vrchat_check_invalid");
      return toCheck(value);
    },
    async avatars(refresh) {
      const value = await ask({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.avatars", params: refresh ? { refresh: true } : {} });
      if (!isVrchatAvatarListWire(value)) throw new Error("vrchat_list_invalid");
      const { schemaVersion, ...list } = value;
      return list;
    },
    async avatarImage(avatarId) {
      const value = await ask({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.avatarImage", params: { avatarId } });
      if (!record(value) || value.avatarId !== avatarId || !isThumbnail(value.image)) throw new Error("vrchat_image_invalid");
      return value.image;
    },
    async openAvatarPage(avatarId) {
      const value = await ask({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "vrchat.openAvatarPage", params: { avatarId, commandId: commandId() } });
      return record(value) && value.opened === true;
    },
  };
}
