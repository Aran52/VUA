/** How a play setup reads, wherever it is named. */
import { currentLocale } from "../../i18n/index.ts";
import { plannedRoutesCopy } from "../../i18n/planned-routes-copy.ts";
import { playSetupCopy } from "../../i18n/play-setup-copy.ts";
import { connectionsFor, PLATFORM_NAMES, PLAY_CONNECTIONS, playDevice, type ConnectionId, type DeviceId, type PlayDevice, type PlaySetup } from "./play-setup-model.ts";

export const deviceName = (device: PlayDevice): string => device.name || playSetupCopy[currentLocale].devices[device.id as "pc"].name;
export const deviceHint = (id: DeviceId): string | undefined => (playSetupCopy[currentLocale].devices as Partial<Record<DeviceId, { readonly hint: string }>>)[id]?.hint;
export const connectionName = (id: ConnectionId): string => id === "standalone" ? playSetupCopy[currentLocale].noPc : PLAY_CONNECTIONS[id].name;

export interface ConnectionWords { readonly tag: string; readonly summary: string; readonly pc?: string; readonly headset?: string }
export function connectionWords(id: ConnectionId): ConnectionWords {
  return id === "pico-connect" || id === "standalone" ? playSetupCopy[currentLocale].connections[id] : plannedRoutesCopy[currentLocale].connections[id];
}

/** Names the connection only where the device offers a choice of them. */
export function setupTitle(setup: PlaySetup): string {
  const choice = setup.connection !== null && connectionsFor(setup.platform, setup.device).length > 1 ? [connectionName(setup.connection)] : [];
  return [PLATFORM_NAMES[setup.platform], deviceName(playDevice(setup.device)), ...choice].join(" · ");
}
