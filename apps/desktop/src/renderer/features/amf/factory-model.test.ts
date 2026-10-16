import { describe, expect, it } from "vitest";
import { currentStation, formatDuration, lineStations, lineStatus, phaseStatus, stopsAt } from "./factory-model.ts";

const steps = (...pairs: [string, string][]) => pairs.map(([key, state]) => ({ key, state }));

describe("AMF production line stations", () => {
  it("follows a fresh preparation through intake", () => {
    const line = { phase: "preparing" as const, steps: steps(["project", "succeeded"], ["materials", "succeeded"], ["inspect", "running"]) };
    expect(lineStations(line)).toEqual({ prepare: "done", inspect: "running", plan: "todo", assemble: "todo", check: "todo" });
    expect(currentStation(lineStations(line))).toBe("inspect");
    expect(lineStatus(line)).toBe("running");
  });

  it("treats a reinspection that runs under a failed phase as running", () => {
    const line = { phase: "failed" as const, steps: steps(["project", "succeeded"], ["materials", "succeeded"], ["inspect", "failed"], ["reinspect", "running"]) };
    expect(lineStations(line).inspect).toBe("running");
    expect(lineStatus(line)).toBe("running");
  });

  it("waits for the player at the assembly plan once intake succeeded", () => {
    const line = { phase: "ready" as const, steps: steps(["project", "succeeded"], ["materials", "succeeded"], ["inspect", "failed"], ["reinspect", "succeeded"]) };
    const states = lineStations(line);
    expect(states).toEqual({ prepare: "done", inspect: "done", plan: "waiting", assemble: "todo", check: "todo" });
    expect(currentStation(states)).toBe("plan");
    expect(lineStatus(line)).toBe("waiting");
  });

  it("finishes every station after a successful assembly", () => {
    const line = { phase: "succeeded" as const, steps: steps(["project", "succeeded"], ["materials", "succeeded"], ["inspect", "succeeded"], ["snapshot", "succeeded"], ["assembly", "succeeded"]) };
    expect(Object.values(lineStations(line)).every(state => state === "done")).toBe(true);
    expect(currentStation(lineStations(line))).toBe("check");
  });

  it("marks the failing station and the line as needing attention", () => {
    const line = { phase: "failed" as const, steps: steps(["project", "succeeded"], ["materials", "succeeded"], ["inspect", "failed"]) };
    expect(lineStations(line).inspect).toBe("failed");
    expect(lineStatus(line)).toBe("attention");
    expect(phaseStatus("cancelled")).toBe("stopped");
  });

  it("stops at more stations as the automation level lowers", () => {
    expect([...stopsAt("auto")]).toEqual(["plan"]);
    expect([...stopsAt("semi")]).toEqual(["plan", "check"]);
    expect(stopsAt("manual").size).toBe(5);
  });

  it("formats durations as minutes and seconds", () => {
    expect(formatDuration(161_000)).toBe("2:41");
    expect(formatDuration(3_725_000)).toBe("1:02:05");
  });
});
