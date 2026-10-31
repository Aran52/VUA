import { describe, expect, it } from "vitest";
import type { PlaySession, ScenarioEnvironmentCheck, ScenarioSop, TaskSnapshotV01 } from "@vua/contracts";
import { checklistSignature, playChecklist, playFlowPhase, progressPercent, readConfirmed, writeConfirmed, type PlayFlowFacts } from "./play-flow-model.ts";

const check = (softwareReady: boolean) => ({ softwareReady, observations: [] }) as unknown as ScenarioEnvironmentCheck;
const session = (state: PlaySession["state"], canStop = false) => ({ state, canStop, issue: null, software: [] }) as unknown as PlaySession;
const task = (state: string, extra: Partial<TaskSnapshotV01> = {}) => ({ state, recoveryDisposition: "none", ...extra }) as unknown as TaskSnapshotV01;
const facts = (over: Partial<PlayFlowFacts> = {}): PlayFlowFacts => ({ unavailable: false, session: null, launchPending: false, checking: false, check: null,
  checkFailed: false, confirmed: false, preparing: false, prepareFailed: false, task: null, taskActive: false, ...over });

describe("the play button names one next step", () => {
  it("walks check, checklist, run and launch in order", () => {
    expect(playFlowPhase(facts())).toBe("check");
    expect(playFlowPhase(facts({ checking: true }))).toBe("checking");
    expect(playFlowPhase(facts({ check: check(false) }))).toBe("confirm");
    expect(playFlowPhase(facts({ check: check(false), confirmed: true }))).toBe("execute");
    expect(playFlowPhase(facts({ check: check(false), confirmed: true, preparing: true }))).toBe("running");
    expect(playFlowPhase(facts({ check: check(false), confirmed: true, task: task("running"), taskActive: true }))).toBe("running");
    expect(playFlowPhase(facts({ check: check(true), confirmed: true }))).toBe("launch");
    expect(playFlowPhase(facts({ check: check(true), confirmed: true, launchPending: true }))).toBe("starting");
    expect(playFlowPhase(facts({ check: check(true), confirmed: true, session: session("running", true) }))).toBe("stop");
  });
  it("offers a retry after a failure and the official page after a handoff", () => {
    expect(playFlowPhase(facts({ checkFailed: true }))).toBe("retry");
    expect(playFlowPhase(facts({ check: check(false), confirmed: true, prepareFailed: true }))).toBe("retry");
    expect(playFlowPhase(facts({ check: check(false), confirmed: true, task: task("failed") }))).toBe("retry");
    expect(playFlowPhase(facts({ check: check(false), confirmed: true, task: task("succeeded", { result: { outcome: "manual_required" } }) }))).toBe("handoff");
  });
  it("is unavailable without the play connection and stops before anything else while running", () => {
    expect(playFlowPhase(facts({ unavailable: true, session: session("running", true) }))).toBe("unavailable");
    expect(playFlowPhase(facts({ session: session("stopping", true), task: task("failed") }))).toBe("starting");
  });
});

describe("the play checklist", () => {
  const sop = { steps: [
    { id: "check", automatic: true }, { id: "install", automatic: true }, { id: "steam-account", automatic: false },
    { id: "launch", automatic: true }, { id: "recovery", automatic: false },
  ] } as unknown as ScenarioSop;
  it("reflects observed facts for automatic steps and the confirmation for the player's own", () => {
    expect(playChecklist(sop, { check: check(false), confirmed: false, session: null }).map(i => [i.id, i.done]))
      .toEqual([["check", true], ["install", false], ["steam-account", false], ["launch", false]]);
    expect(playChecklist(sop, { check: check(true), confirmed: true, session: session("running", true) }).every(i => i.done)).toBe(true);
  });
  it("asks again when the steps or the connection change", () => {
    const usb = checklistSignature(sop, "usb");
    const raw = writeConfirmed(null, "pico_pcvr", usb);
    expect(readConfirmed(raw, "pico_pcvr", usb)).toBe(true);
    expect(readConfirmed(raw, "pico_pcvr", checklistSignature(sop, "wifi"))).toBe(false);
    expect(readConfirmed(raw, "desktop_play", usb)).toBe(false);
    expect(readConfirmed("not json", "pico_pcvr", usb)).toBe(false);
    expect(JSON.parse(writeConfirmed(raw, "desktop_play", "x"))).toEqual({ pico_pcvr: usb, desktop_play: "x" });
  });
  it("shows whole percents only when the installer reports sizes", () => {
    expect(progressPercent(null)).toBeNull();
    expect(progressPercent({ component: "steam", action: "install_steam", phase: "downloading" } as never)).toBeNull();
    expect(progressPercent({ component: "steam", action: "install_steam", phase: "downloading", completedBytes: 333, totalBytes: 1000 } as never)).toBe(33);
  });
});
