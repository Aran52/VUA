import { describe, expect, it } from "vitest";
import {
  EXTERNAL_TOOL_IDS,
  INSTALLER_TOOL_IDS,
  type DeploymentProgress,
  type InstallerToolSnapshot,
  type PlayScenarioSopV02,
} from "@vua/contracts";
import { installerToolCopy } from "../../i18n/installer-tool-copy.ts";
import { toolConnectionCopy } from "../../i18n/tool-connection-copy.ts";
import { resolveSetupSteps } from "../home/PlaySetupFlow.tsx";
import { PLANNED_TOOLS, isInstallerTool, isPlannedTool } from "./planned-tools.ts";
import {
  deriveInstallerCardState,
  describeInstallerError,
  describeInstallerStage,
  installerProgressPercent,
} from "./use-installer-tool.ts";

const copy = installerToolCopy.en;
const tc = toolConnectionCopy.en;

const baseSnapshot: InstallerToolSnapshot = {
  schemaVersion: "vua.installer-tool/v0.1",
  toolId: "slimevr",
  capturedAt: "2026-11-12T00:00:00Z",
  presence: "missing",
  location: null,
  version: null,
  running: false,
  canStop: false,
  activity: "idle",
  issue: null,
};

describe("ToolsHub v1.1 partition", () => {
  it("keeps only XSOverlay and LIV as planned placeholders while connecting Steam and installer tools", () => {
    expect(Object.keys(PLANNED_TOOLS)).toEqual(["xsoverlay", "liv"]);
    expect(isPlannedTool("xsoverlay")).toBe(true);
    expect(isPlannedTool("liv")).toBe(true);
    for (const id of INSTALLER_TOOL_IDS) {
      expect(isPlannedTool(id)).toBe(false);
      expect(isInstallerTool(id)).toBe(true);
    }
    for (const id of ["desktopplus", "fpsvr"] as const) {
      expect(isPlannedTool(id)).toBe(false);
      expect(isInstallerTool(id)).toBe(false);
      expect(EXTERNAL_TOOL_IDS).toContain(id);
    }
  });
});

describe("InstallerToolCard state derivation", () => {
  it("drives install, running, close and download percentage states", () => {
    expect(
      deriveInstallerCardState(
        { snapshot: null, pending: false, failed: false, available: true, installing: false, cancelling: false, step: null },
        copy,
        tc,
      ),
    ).toEqual({ action: "busy", status: tc.short.checking, percent: null, busy: true });

    expect(
      deriveInstallerCardState(
        { snapshot: baseSnapshot, pending: false, failed: false, available: true, installing: false, cancelling: false, step: null },
        copy,
        tc,
      ),
    ).toEqual({ action: "prepare", status: tc.short.install, actionIcon: "download", percent: null, busy: false });

    const dlStep: DeploymentProgress = {
      component: "slimevr",
      action: "install_official_tool",
      phase: "downloading",
      completedBytes: 25_000_000,
      totalBytes: 50_000_000,
    };
    expect(installerProgressPercent(dlStep)).toBe(50);
    expect(describeInstallerStage(dlStep, copy)).toBe(copy.stages.downloading);
    expect(
      deriveInstallerCardState(
        { snapshot: baseSnapshot, pending: false, failed: false, available: true, installing: true, cancelling: false, step: dlStep },
        copy,
        tc,
      ),
    ).toEqual({ action: "busy", status: `${copy.installing} 50%`, percent: 50, busy: true });

    expect(
      deriveInstallerCardState(
        {
          snapshot: { ...baseSnapshot, presence: "installed", version: "5.0.0", running: true, canStop: true },
          pending: false,
          failed: false,
          available: true,
          installing: false,
          cancelling: false,
          step: null,
        },
        copy,
        tc,
      ),
    ).toEqual({ action: "stop", status: tc.short.closeApp, percent: null, busy: false });
  });

  it("maps UAC cancellation and download errors to clear player copy", () => {
    expect(describeInstallerError("vua.deployment.uac_cancelled", copy)).toBe(copy.errors.uacCancelled);
    expect(describeInstallerError("vua.deployment.source_failed", copy)).toBe(copy.errors.downloadFailed);
    expect(describeInstallerError("vua.deployment.busy_lock", copy)).toContain("busy_lock");
  });
});

describe("PlaySetupFlow v0.2 SOP fallback", () => {
  it("falls back synchronously to local setupSteps and switches to v0.2 SOP steps when available", () => {
    const setup = { platform: "vrchat" as const, device: "quest" as const, connection: "standalone" as const };
    expect(resolveSetupSteps(setup, null)).toEqual([
      { id: "store-install", actor: "player" },
      { id: "store-launch", actor: "player" },
    ]);
    const sop = {
      steps: [
        { id: "store-install", actor: "player", title: "Install", explanation: "x", expectedResult: "y", automatic: false, verification: "v", onFailure: "f" },
        { id: "store-launch", actor: "player", title: "Launch", explanation: "x", expectedResult: "y", automatic: false, verification: "v", onFailure: "f" },
      ],
    } as unknown as PlayScenarioSopV02;
    expect(resolveSetupSteps(setup, sop)).toEqual([
      { id: "store-install", actor: "player" },
      { id: "store-launch", actor: "player" },
    ]);
  });
});
