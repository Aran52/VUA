import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { plannedRoutesCopy } from "../../i18n/planned-routes-copy.ts";
import { playSetupCopy } from "../../i18n/play-setup-copy.ts";
import { connectionsFor, detectedSetup, devicesFor, normalizeSetup, PC_SETUP, PLAY_CONNECTIONS, PLAY_DEVICES, readPlaySetup, recommendedConnection, rememberPlaySetup, setupEngine, setupForRoute, setupSteps } from "./play-setup-model.ts";

describe("play setups", () => {
  const stored = new Map<string, string>();
  beforeEach(() => { stored.clear(); vi.stubGlobal("localStorage", { getItem: (key: string) => stored.get(key) ?? null, setItem: (key: string, value: string) => stored.set(key, value) }); });
  afterEach(() => vi.unstubAllGlobals());

  it("offers streaming tools and a headset of its own only on standalone headsets", () => {
    for (const device of PLAY_DEVICES.filter(item => item.connections.includes("steam-link"))) expect(["pico", "quest", "vive-standalone"]).toContain(device.id);
    for (const id of ["index", "bigscreen", "pimax", "varjo", "meganex", "rift-s", "psvr2", "wmr"] as const) expect(connectionsFor("vrchat", id)).toHaveLength(1);
  });

  it("lists Steam Link for PICO and VIVE standalone as well as Quest", () => {
    for (const id of ["pico", "quest", "vive-standalone"] as const) expect(connectionsFor("vrchat", id)).toContain("steam-link");
  });

  it("offers no PC mode for Reborn and no headset version beyond VRChat", () => {
    expect(devicesFor("reborn").map(device => device.id)).not.toContain("pc");
    expect(connectionsFor("chilloutvr", "quest")).not.toContain("standalone");
    expect(connectionsFor("vrchat", "quest")).toContain("standalone");
  });

  it("turns any half-changed choice into a combination that exists", () => {
    expect(normalizeSetup({ platform: "reborn", device: "pc", connection: null })).toEqual({ platform: "reborn", device: "quest", connection: "quest-link" });
    expect(normalizeSetup({ platform: "chilloutvr", device: "quest", connection: "standalone" }).connection).toBe("quest-link");
    expect(normalizeSetup({ platform: "vrchat", device: "index", connection: "steam-link" }).connection).toBe("steamvr-direct");
    expect(normalizeSetup(PC_SETUP)).toEqual(PC_SETUP);
  });

  it("runs the two delivered routes through their own flow and the rest through Steam", () => {
    expect(setupEngine(PC_SETUP)).toEqual({ kind: "route", route: "desktop_play" });
    expect(setupEngine(setupForRoute("pico_pcvr"))).toEqual({ kind: "route", route: "pico_pcvr" });
    expect(setupEngine({ platform: "vrchat", device: "pico", connection: "steam-link" })).toEqual({ kind: "tool", toolId: "vrchat" });
    expect(setupEngine({ platform: "chilloutvr", device: "pc", connection: null })).toEqual({ kind: "tool", toolId: "chilloutvr" });
    expect(setupEngine({ platform: "vrchat", device: "quest", connection: "standalone" })).toEqual({ kind: "headset" });
  });

  it("suggests a headset only when one headset's software is on the computer", () => {
    expect(detectedSetup(["pico_runtime", "steamvr"])).toEqual({ platform: "vrchat", device: "pico", connection: "pico-connect" });
    expect(detectedSetup(["pico_runtime", "oculus_runtime"])).toBeNull();
    expect(detectedSetup(["steamvr", "virtual_desktop"])).toBeNull();
  });

  it("recommends a connection already on the computer, and never SteamVR alone", () => {
    const quest = connectionsFor("vrchat", "quest");
    expect(recommendedConnection(quest, [])).toBe("quest-link");
    expect(recommendedConnection(quest, ["virtual_desktop"])).toBe("virtual-desktop");
    expect(recommendedConnection(connectionsFor("vrchat", "vive-standalone"), ["steamvr"])).toBe("vive-hub");
  });

  it("composes steps: the headset's own version needs no computer", () => {
    expect(setupSteps({ platform: "vrchat", device: "quest", connection: "standalone" }).map(step => step.id)).toEqual(["store-install", "store-launch"]);
    expect(setupSteps({ platform: "vrchat", device: "quest", connection: "steam-link" }).map(step => step.id)).toEqual(["steam", "steam-sign-in", "platform", "connection-pc", "connection-headset", "launch"]);
    expect(setupSteps({ platform: "chilloutvr", device: "pc", connection: null }).map(step => step.id)).toEqual(["steam", "steam-sign-in", "platform", "launch"]);
  });

  it("names who carries out each step", () => {
    const steps = setupSteps({ platform: "vrchat", device: "quest", connection: "quest-link" });
    expect(steps.find(step => step.id === "connection-pc")?.actor).toBe("open");
    expect(steps.find(step => step.id === "connection-headset")?.actor).toBe("player");
    expect(setupSteps({ platform: "vrchat", device: "quest", connection: "steam-link" }).find(step => step.id === "connection-pc")?.actor).toBe("player");
  });

  it("remembers a choice and drops a damaged one", () => {
    expect(readPlaySetup()).toBeNull();
    rememberPlaySetup({ platform: "vrchat", device: "pico", connection: "alvr" });
    expect(readPlaySetup()).toEqual({ platform: "vrchat", device: "pico", connection: "alvr" });
    stored.set("vua-play-setup-v1", "{\"platform\":\"elsewhere\"}");
    expect(readPlaySetup()).toBeNull();
    stored.set("vua-play-setup-v1", "not json");
    expect(readPlaySetup()).toBeNull();
  });

  it("keeps the less common headsets after the ones most players have", () => {
    const order = PLAY_DEVICES.map(device => device.other === true);
    expect(order.lastIndexOf(false)).toBeLessThan(order.indexOf(true));
  });

  it("uses only official https pages", () => {
    for (const connection of Object.values(PLAY_CONNECTIONS)) expect(connection.url.startsWith("https://")).toBe(true);
  });

  it("describes every device and connection in every language", () => {
    for (const [locale, copy] of Object.entries(playSetupCopy)) {
      expect(copy.steps.steam && copy.actors.player && copy.standaloneNote, locale).toBeTruthy();
      for (const device of PLAY_DEVICES) if (device.name === "") expect(copy.devices[device.id as keyof typeof copy.devices]?.name, `${locale} ${device.id}`).toBeTruthy();
      for (const id of Object.keys(PLAY_CONNECTIONS) as (keyof typeof PLAY_CONNECTIONS)[]) {
        const text = id === "pico-connect" || id === "standalone" ? copy.connections[id] : plannedRoutesCopy[locale as keyof typeof plannedRoutesCopy].connections[id];
        expect(text.tag && text.summary, `${locale} ${id}`).toBeTruthy();
      }
    }
  });
});
