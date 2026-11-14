import assert from "node:assert/strict";
import { test } from "vitest";
import {
  isLocalDiagnosticsSnapshotV01,
  type LocalDiagnosticsSnapshotV01,
  type SymptomDiagnosisV01,
} from "@vua/contracts";
import { buildRedactedDiagnosticSummary } from "./LocalDiagnosticsPanel.tsx";

test("buildRedactedDiagnosticSummary strips process names, PIDs, and blocked driver identifiers", () => {
  const snapshot: LocalDiagnosticsSnapshotV01 = {
    schemaVersion: "vua.local-diagnostics/v0.1",
    capturedAt: "2026-11-14T00:00:00Z",
    clientRunning: false,
    clientCache: {
      presence: "found",
      totalBytes: 10 * 1024 * 1024 * 1024,
      truncated: false,
      driveFreeBytes: 64 * 1024 * 1024 * 1024,
      driveTotalBytes: 512 * 1024 * 1024 * 1024,
    },
    clientConfig: {
      presence: "found",
      cacheDirectoryCustom: true,
      cacheSizeGb: 30,
      cacheExpiryDelayDays: 45,
      cameraResWidth: null,
      cameraResHeight: null,
      screenshotResWidth: null,
      screenshotResHeight: null,
      fpvSteadycamFov: null,
    },
    osc: {
      avatarCachePresence: "found",
      avatarConfigCount: 3,
      avatarCacheBytes: 12288,
      newestModifiedAt: "2026-11-14T00:00:00Z",
      udp9000: {
        state: "bound",
        ownerPid: 4242,
        ownerProcess: "SecretLocalProcess.exe",
      },
      udp9001: {
        state: "unbound",
        ownerPid: null,
        ownerProcess: null,
      },
      oscQuery: {
        state: "found",
        httpPort: 54321,
        advertisedOscPort: 9000,
        ownerProcess: "SecretLocalProcess.exe",
      },
    },
    steamvrSettings: {
      presence: "found",
      renderTargetMultiplier: 1.25,
      motionSmoothing: false,
      vrchatResolutionScale: 1.0,
      safeModeEnabled: true,
      blockedDrivers: ["01spacecalibrator", "custom_private_driver"],
    },
    recentClientLog: {
      classification: "clean_or_inconclusive",
      newestModifiedAt: "2026-11-14T00:00:00Z",
    },
    audioEndpoints: {
      state: "observed",
      defaultCaptureRole: "vr_virtual",
      communicationsMismatch: true,
      virtualMicHijack: "oculus_virtual",
    },
    streamingSettings: {
      questLink: {
        presence: "found",
        encodeResolutionWidth: 3664,
        encodeBitrateMbps: 350,
        dynamicBitrate: false,
        linkSharpening: 1,
      },
      picoConnect: {
        presence: "missing",
        preset: null,
        bitrateMbps: null,
      },
    },
  };
  assert.equal(isLocalDiagnosticsSnapshotV01(snapshot), true);

  const diagnosis: SymptomDiagnosisV01 = {
    schemaVersion: "vua.local-diagnostics/v0.1",
    symptom: "audio_silent_or_wrong_mic",
    capturedAt: "2026-11-14T00:00:00Z",
    snapshot,
    findings: [
      { code: "audio_virtual_mic_hijack_detected", severity: "attention", layer: "hardware_os" },
    ],
    sopSteps: [],
    suggestedHelper: "none",
  };

  const raw = buildRedactedDiagnosticSummary(snapshot, diagnosis);
  assert.equal(raw.includes("SecretLocalProcess.exe"), false);
  assert.equal(raw.includes("4242"), false);
  assert.equal(raw.includes("custom_private_driver"), false);

  const parsed = JSON.parse(raw) as Record<string, unknown>;
  assert.equal(parsed.schemaVersion, "vua.local-diagnostics/v0.1");
  assert.equal((parsed.steamvr as { blockedDriverCount: number }).blockedDriverCount, 2);
  assert.equal((parsed.osc as { udp9000: string }).udp9000, "bound");
  assert.deepEqual(parsed.diagnosis, {
    symptom: "audio_silent_or_wrong_mic",
    findings: ["audio_virtual_mic_hijack_detected"],
  });
});
