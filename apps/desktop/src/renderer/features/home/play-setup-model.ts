/** What a player plays and how: platform, device and connection. Each combination that exists is listed here,
 * and its preparation steps are composed from these pieces. A later provider contract serves the same shape,
 * so the app and an agent read one source. */
import type { ExternalToolId, PlayRoute } from "@vua/contracts";
import { storageKeys } from "../../app/storage-keys.ts";
import type { VrBrand } from "../../components/vr-brand-glyphs.ts";

export const PLATFORMS = ["vrchat", "chilloutvr", "reborn"] as const;
export type PlatformId = typeof PLATFORMS[number];
export const PLATFORM_NAMES: Readonly<Record<PlatformId, string>> = { vrchat: "VRChat", chilloutvr: "ChilloutVR", reborn: "Reborn" };

export type ConnectionId = "pico-connect" | "standalone" | "quest-link" | "steam-link" | "virtual-desktop" | "alvr" | "steam-frame" | "vive-hub" | "steamvr-direct" | "psvr2-app"
  | "bigscreen-driver" | "pimax-play" | "varjo-base" | "meganex-configurator" | "rift-app" | "wmr-steamvr" | "steamvr-other";
export type DeviceId = "pc" | "pico" | "quest" | "steam-frame" | "vive-standalone" | "vive-wired" | "index" | "psvr2" | "bigscreen" | "pimax" | "varjo" | "meganex" | "rift-s" | "wmr" | "other-headset";

export interface PlayConnection {
  readonly id: ConnectionId;
  /** The product's own name, in every language. An empty name reads from the copy. */
  readonly name: string;
  /** The environment check that observes its PC software; null when the PC side is Steam itself. */
  readonly component: string | null;
  readonly url: string;
}

export const PLAY_CONNECTIONS: Readonly<Record<ConnectionId, PlayConnection>> = {
  "pico-connect": { id: "pico-connect", name: "PICO Connect", component: "pico_runtime", url: "https://www.picoxr.com/" },
  standalone: { id: "standalone", name: "", component: null, url: "https://wiki.vrchat.com/wiki/Getting_Started" },
  "quest-link": { id: "quest-link", name: "Quest Link / Air Link", component: "oculus_runtime", url: "https://www.meta.com/quest/setup/" },
  "steam-link": { id: "steam-link", name: "Steam Link", component: null, url: "https://help.steampowered.com/en/faqs/view/0E2C-406B-9135-38A4" },
  "virtual-desktop": { id: "virtual-desktop", name: "Virtual Desktop", component: "virtual_desktop", url: "https://www.vrdesktop.net/" },
  alvr: { id: "alvr", name: "ALVR", component: "alvr", url: "https://github.com/alvr-org/ALVR" },
  "steam-frame": { id: "steam-frame", name: "SteamVR", component: "steamvr", url: "https://store.steampowered.com/hardware/steamframe" },
  "vive-hub": { id: "vive-hub", name: "VIVE Hub", component: "vive_runtime", url: "https://www.vive.com/us/vive-hub/download/" },
  "steamvr-direct": { id: "steamvr-direct", name: "SteamVR", component: "steamvr", url: "https://store.steampowered.com/app/250820/" },
  "psvr2-app": { id: "psvr2-app", name: "PlayStation VR2 App", component: "psvr2", url: "https://www.playstation.com/en-us/support/hardware/pc-prepare-ps-vr2/" },
  "bigscreen-driver": { id: "bigscreen-driver", name: "Bigscreen Beyond Utility", component: "bigscreen_beyond", url: "https://bigscreenvr.com/mybeyond/" },
  "pimax-play": { id: "pimax-play", name: "Pimax Play", component: "pimax_runtime", url: "https://www.pimax.com/support/downloads" },
  "varjo-base": { id: "varjo-base", name: "Varjo Base", component: "varjo_runtime", url: "https://support.varjo.com/hc/en-us/downloads" },
  "meganex-configurator": { id: "meganex-configurator", name: "MeganeX Configurator", component: null, url: "https://shiftall.notion.site/meganex8k-manual-en" },
  "rift-app": { id: "rift-app", name: "Meta Horizon Link", component: "oculus_runtime", url: "https://www.meta.com/quest/setup/" },
  "wmr-steamvr": { id: "wmr-steamvr", name: "Windows Mixed Reality for SteamVR", component: null, url: "https://store.steampowered.com/app/719950/" },
  "steamvr-other": { id: "steamvr-other", name: "SteamVR", component: "steamvr", url: "https://store.steampowered.com/app/250820/" },
};

export interface PlayDevice {
  readonly id: DeviceId;
  /** The product's own name, in every language. An empty name reads from the copy. */
  readonly name: string;
  readonly brand?: VrBrand;
  readonly kind?: "screen" | "headset";
  /** The first connection is the recommended one. */
  readonly connections: readonly ConnectionId[];
  /** Shown under "Other headsets", after the devices most players have. */
  readonly other?: true;
  /** Where the headset's own VRChat comes from. */
  readonly store?: "pico" | "meta" | "viveport";
}

/** In the order players are likely to need them. Wired headsets have one way to connect, so that question is skipped. */
export const PLAY_DEVICES: readonly PlayDevice[] = [
  { id: "pc", name: "", kind: "screen", connections: [] },
  { id: "pico", name: "PICO", brand: "pico-mark", connections: ["pico-connect", "steam-link", "virtual-desktop", "alvr", "standalone"], store: "pico" },
  { id: "quest", name: "Meta Quest", brand: "meta", connections: ["quest-link", "steam-link", "virtual-desktop", "alvr", "standalone"], store: "meta" },
  { id: "steam-frame", name: "Steam Frame", brand: "valve", connections: ["steam-frame"] },
  { id: "vive-standalone", name: "", brand: "htcvive", connections: ["vive-hub", "steam-link", "virtual-desktop", "alvr", "standalone"], store: "viveport" },
  { id: "vive-wired", name: "", brand: "htcvive", connections: ["steamvr-direct"] },
  { id: "index", name: "Valve Index", brand: "valve", connections: ["steamvr-direct"] },
  { id: "psvr2", name: "PS VR2", kind: "headset", connections: ["psvr2-app"] },
  { id: "bigscreen", name: "Bigscreen Beyond", kind: "headset", connections: ["bigscreen-driver"], other: true },
  { id: "pimax", name: "Pimax", kind: "headset", connections: ["pimax-play"], other: true },
  { id: "varjo", name: "Varjo", kind: "headset", connections: ["varjo-base"], other: true },
  { id: "meganex", name: "MeganeX", kind: "headset", connections: ["meganex-configurator"], other: true },
  { id: "rift-s", name: "Oculus Rift S", brand: "meta", connections: ["rift-app"], other: true },
  { id: "wmr", name: "Windows MR", kind: "headset", connections: ["wmr-steamvr"], other: true },
  { id: "other-headset", name: "", kind: "headset", connections: ["steamvr-other"], other: true },
];

export interface PlaySetup { readonly platform: PlatformId; readonly device: DeviceId; readonly connection: ConnectionId | null }
export const PC_SETUP: PlaySetup = { platform: "vrchat", device: "pc", connection: null };

const deviceOf = (id: DeviceId): PlayDevice => PLAY_DEVICES.find(device => device.id === id)!;
export const playDevice = deviceOf;

/** Reborn's store page names no desktop mode, so PC mode is left to the platforms that state it. */
export const devicesFor = (platform: PlatformId): readonly PlayDevice[] => PLAY_DEVICES.filter(device => !(platform === "reborn" && device.id === "pc"));
/** Only VRChat runs on the headset itself from here; the other platforms are prepared through Steam. */
export const connectionsFor = (platform: PlatformId, device: DeviceId): readonly ConnectionId[] => deviceOf(device).connections.filter(id => id !== "standalone" || platform === "vrchat");

/** Any stored or half-changed choice becomes a combination that exists. */
export function normalizeSetup(setup: { readonly platform: PlatformId; readonly device: DeviceId; readonly connection: ConnectionId | null }): PlaySetup {
  const device = devicesFor(setup.platform).some(item => item.id === setup.device) ? setup.device : "quest";
  const connections = connectionsFor(setup.platform, device);
  return { platform: setup.platform, device, connection: setup.connection !== null && connections.includes(setup.connection) ? setup.connection : connections[0] ?? null };
}

/** How a combination runs: through a route with its own session, through Steam, or on the headset alone. */
export type PlayEngine = { readonly kind: "route"; readonly route: PlayRoute } | { readonly kind: "tool"; readonly toolId: ExternalToolId } | { readonly kind: "headset" };
export function setupEngine(setup: PlaySetup): PlayEngine {
  if (setup.connection === "standalone") return { kind: "headset" };
  if (setup.platform === "vrchat" && setup.device === "pc") return { kind: "route", route: "desktop_play" };
  if (setup.platform === "vrchat" && setup.connection === "pico-connect") return { kind: "route", route: "pico_pcvr" };
  return { kind: "tool", toolId: setup.platform };
}

export function setupForRoute(route: PlayRoute): PlaySetup {
  return route === "desktop_play" ? PC_SETUP : { platform: "vrchat", device: "pico", connection: "pico-connect" };
}

const DETECTED_DEVICES: readonly (readonly [string, DeviceId])[] = [
  ["pico_runtime", "pico"], ["oculus_runtime", "quest"], ["vive_runtime", "vive-standalone"], ["psvr2", "psvr2"],
  ["pimax_runtime", "pimax"], ["varjo_runtime", "varjo"], ["bigscreen_beyond", "bigscreen"],
];
/** A single headset's software on this computer suggests that headset; several say nothing about which is in use. */
export function detectedSetup(detected: readonly string[]): PlaySetup | null {
  const devices = [...new Set(DETECTED_DEVICES.filter(([component]) => detected.includes(component)).map(([, device]) => device))];
  if (devices.length !== 1) return null;
  return normalizeSetup({ platform: "vrchat", device: devices[0]!, connection: null });
}

/** A device's connection whose software is already on this computer comes first among its choices. */
export function recommendedConnection(connections: readonly ConnectionId[], detected: readonly string[]): ConnectionId | null {
  const found = connections.find(id => { const component = PLAY_CONNECTIONS[id].component; return component !== null && component !== "steamvr" && detected.includes(component); });
  return found ?? connections[0] ?? null;
}

export function readPlaySetup(): PlaySetup | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(storageKeys.playSetup) ?? "null");
    if (typeof value !== "object" || value === null) return null;
    const { platform, device, connection } = value as Record<string, unknown>;
    if (!PLATFORMS.includes(platform as PlatformId) || !PLAY_DEVICES.some(item => item.id === device)) return null;
    return normalizeSetup({ platform: platform as PlatformId, device: device as DeviceId, connection: typeof connection === "string" ? connection as ConnectionId : null });
  } catch { return null; }
}
export function rememberPlaySetup(setup: PlaySetup): void {
  try { localStorage.setItem(storageKeys.playSetup, JSON.stringify(setup)); } catch { /* The choice applies to this visit. */ }
}

/** Who carries out a step: VUA, VUA opening the right place, or the player. */
export type StepActor = "vua" | "open" | "player";
export type SetupStepId = "steam" | "steam-sign-in" | "platform" | "connection-pc" | "connection-headset" | "launch" | "store-install" | "store-launch";
export interface SetupStep { readonly id: SetupStepId; readonly actor: StepActor }

/** The steps of a combination that VUA guides. Routes with their own flow read their steps from the provider. */
export function setupSteps(setup: PlaySetup): readonly SetupStep[] {
  if (setup.connection === "standalone") return [{ id: "store-install", actor: "player" }, { id: "store-launch", actor: "player" }];
  const common: SetupStep[] = [{ id: "steam", actor: "open" }, { id: "steam-sign-in", actor: "player" }, { id: "platform", actor: "vua" }];
  const connection = setup.connection === null ? null : PLAY_CONNECTIONS[setup.connection];
  const pair: SetupStep[] = connection === null ? [] : [{ id: "connection-pc", actor: connection.component === null ? "player" : "open" }, { id: "connection-headset", actor: "player" }];
  return [...common, ...pair, { id: "launch", actor: "vua" }];
}
