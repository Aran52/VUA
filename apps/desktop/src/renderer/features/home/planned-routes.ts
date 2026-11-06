import type { VrBrand } from "../../components/vr-brand-glyphs.ts";

/** A way to connect a headset that VUA explains and observes, before it can install and start it. */
export interface PlannedConnection {
  readonly id: ConnectionId;
  /** The product's own name, in every language. */
  readonly name: string;
  /** The environment check that observes its PC software; null when the PC side is Steam itself. */
  readonly component: string | null;
  readonly url: string;
}

export type ConnectionId = "quest-link" | "steam-link" | "virtual-desktop" | "alvr" | "steam-frame" | "vive-hub" | "steamvr-direct" | "psvr2-app";
export type PlannedRouteId = "quest" | "steam-frame" | "vive" | "index" | "psvr2";

export const PLANNED_CONNECTIONS: Readonly<Record<ConnectionId, PlannedConnection>> = {
  "quest-link": { id: "quest-link", name: "Quest Link / Air Link", component: "oculus_runtime", url: "https://www.meta.com/quest/setup/" },
  "steam-link": { id: "steam-link", name: "Steam Link", component: null, url: "https://help.steampowered.com/en/faqs/view/0E2C-406B-9135-38A4" },
  "virtual-desktop": { id: "virtual-desktop", name: "Virtual Desktop", component: "virtual_desktop", url: "https://www.vrdesktop.net/" },
  alvr: { id: "alvr", name: "ALVR", component: "alvr", url: "https://github.com/alvr-org/ALVR" },
  "steam-frame": { id: "steam-frame", name: "SteamVR", component: "steamvr", url: "https://store.steampowered.com/hardware/steamframe" },
  "vive-hub": { id: "vive-hub", name: "VIVE Hub", component: "vive_runtime", url: "https://www.vive.com/us/vive-hub/download/" },
  "steamvr-direct": { id: "steamvr-direct", name: "SteamVR", component: "steamvr", url: "https://store.steampowered.com/app/250820/" },
  "psvr2-app": { id: "psvr2-app", name: "PlayStation VR2 App", component: "psvr2", url: "https://www.playstation.com/en-us/support/hardware/pc-prepare-ps-vr2/" },
};

export interface PlannedRoute {
  readonly id: PlannedRouteId;
  readonly title: string;
  readonly brand?: VrBrand;
  readonly connections: readonly ConnectionId[];
}

/** Routes in the order players are likely to need them. Each lists its recommended connection first. */
export const PLANNED_ROUTES: readonly PlannedRoute[] = [
  { id: "quest", title: "Meta Quest", brand: "meta", connections: ["quest-link", "steam-link", "virtual-desktop", "alvr"] },
  { id: "steam-frame", title: "Steam Frame", brand: "valve", connections: ["steam-frame"] },
  { id: "vive", title: "HTC VIVE", brand: "htcvive", connections: ["vive-hub", "steamvr-direct", "virtual-desktop", "alvr"] },
  { id: "index", title: "Valve Index", brand: "valve", connections: ["steamvr-direct"] },
  { id: "psvr2", title: "PS VR2", connections: ["psvr2-app"] },
];

/** PICO's own route has a full flow; these are the other streaming tools that also support it. */
export const PICO_OTHER_CONNECTIONS: readonly ConnectionId[] = ["virtual-desktop", "alvr"];

/** A connection already on this computer is the one to show first. */
export function initialConnection(connections: readonly ConnectionId[], detected: readonly string[]): ConnectionId {
  const found = connections.find(id => {
    const component = PLANNED_CONNECTIONS[id].component;
    return component !== null && component !== "steamvr" && detected.includes(component);
  });
  return found ?? connections[0]!;
}
