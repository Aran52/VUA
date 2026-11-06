import { describe, expect, it } from "vitest";
import { initialConnection, PICO_OTHER_CONNECTIONS, PLANNED_CONNECTIONS, PLANNED_ROUTES } from "./planned-routes.ts";
import { plannedRoutesCopy } from "../../i18n/planned-routes-copy.ts";

describe("planned routes", () => {
  it("recommends a route's first connection until another one is on the computer", () => {
    const quest = PLANNED_ROUTES.find(route => route.id === "quest")!.connections;
    expect(initialConnection(quest, [])).toBe("quest-link");
    expect(initialConnection(quest, ["virtual_desktop"])).toBe("virtual-desktop");
    expect(initialConnection(quest, ["oculus_runtime", "alvr"])).toBe("quest-link");
  });

  it("does not treat SteamVR alone as a chosen connection, since most players have it", () => {
    const vive = PLANNED_ROUTES.find(route => route.id === "vive")!.connections;
    expect(initialConnection(vive, ["steamvr"])).toBe("vive-hub");
  });

  it("names every headset except the one for models it does not list", () => {
    expect(PLANNED_ROUTES.filter(route => route.title === "").map(route => route.id)).toEqual(["other-headset"]);
  });

  it("keeps the less common headsets after the ones most players have", () => {
    const order = PLANNED_ROUTES.map(route => route.other === true);
    expect(order.lastIndexOf(false)).toBeLessThan(order.indexOf(true));
    expect(PLANNED_ROUTES.filter(route => route.other).map(route => route.id)).toEqual(expect.arrayContaining(["bigscreen", "pimax", "varjo"]));
  });

  it("uses only known connections and official https pages", () => {
    const used = new Set([...PLANNED_ROUTES.flatMap(route => route.connections), ...PICO_OTHER_CONNECTIONS]);
    for (const id of used) expect(PLANNED_CONNECTIONS[id].url.startsWith("https://")).toBe(true);
    expect([...used].sort()).toEqual(Object.keys(PLANNED_CONNECTIONS).sort());
  });

  it("describes every route and connection in every language", () => {
    for (const copy of Object.values(plannedRoutesCopy)) {
      expect(copy.hints.wmr && copy.hints["other-headset"]).toBeTruthy();
      expect(copy.otherModel && copy.otherHeadsets && copy.guide).toBeTruthy();
      for (const id of Object.keys(PLANNED_CONNECTIONS) as (keyof typeof PLANNED_CONNECTIONS)[]) {
        const text = copy.connections[id];
        expect(text.tag && text.summary && text.pc && text.headset).toBeTruthy();
      }
    }
  });
});
