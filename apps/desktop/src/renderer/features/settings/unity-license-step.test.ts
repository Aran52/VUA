import { describe, expect, it } from "vitest";
import type { UnityLicenseSnapshotV01 } from "@vua/contracts";
import { licenseStep } from "./unity-license-step.ts";

const checkedAt = "2026-10-28T12:00:00Z";
const snapshot: UnityLicenseSnapshotV01 = {
  schemaVersion: "0.1", files: [
    { kind: "classic_entitlement", present: true, modifiedAt: checkedAt },
    { kind: "msix_entitlement", present: false, modifiedAt: null },
    { kind: "legacy_ulf", present: false, modifiedAt: null },
  ], hubType: "classic", hubRunning: true, licensingClientRunning: true, hubCliPresent: false,
  state: "likely_active", nextAction: "none", lastCheck: { state: "active", editorVersion: "2022.3.22f1", checkedAt },
};
describe("the shared Unity license next step", () => {
  it("offers no check for a still-current successful check", () => {
    expect(licenseStep(snapshot, true)).toMatchObject({ state: "active", detail: "activeDetail", action: "none" });
  });
  it("uses the explicit check to start Hub's licensing client", () => {
    expect(licenseStep({ ...snapshot, state: "needs_hub_client", nextAction: "start_hub" }, true))
      .toMatchObject({ title: "needsHub", action: "check" });
  });
  it("opens account guidance before offering a player-requested check on return", () => {
    const unactivated = { ...snapshot, state: "not_found", nextAction: "sign_in_and_activate", lastCheck: null } as const;
    expect(licenseStep(unactivated, true)).toMatchObject({ title: "notActivated", action: "account" });
    expect(licenseStep(unactivated, true, false, true).action).toBe("check");
    expect(licenseStep({ ...snapshot, nextAction: "sign_in_and_activate", lastCheck: { ...snapshot.lastCheck!, state: "not_activated" } }, true))
      .toMatchObject({ detail: "notActivatedDetail", action: "account" });
  });
  it.each(["likely_active", "unknown"] as const)("offers a check for %s observations", state => {
    expect(licenseStep({ ...snapshot, state, nextAction: "run_check", lastCheck: null }, true).action).toBe("check");
  });
  it("requires installation when AMF has no available Editor, even with historical success", () => {
    expect(licenseStep(snapshot, false)).toMatchObject({ title: "noEditor", action: "install" });
  });
  it("offers no new action while a check is pending", () => {
    expect(licenseStep(snapshot, true, true)).toMatchObject({ state: "checking", action: "none" });
  });
  it("does not reuse a successful check when the provider reports a changed license file", () => {
    const changed = { ...snapshot, files: snapshot.files.map(file => file.present ? { ...file, modifiedAt: "2026-10-29T00:00:00Z" } : file), nextAction: "run_check" } as const;
    expect(licenseStep(changed, true)).toMatchObject({ state: "hint", action: "check" });
  });
  it("waits for Editor discovery and keeps missing observations unknown", () => {
    expect(licenseStep(null, null)).toMatchObject({ title: "reading", action: "none" });
    expect(licenseStep(null, true)).toMatchObject({ title: "unknown", action: "check" });
    expect(licenseStep(null, false)).toMatchObject({ action: "install" });
  });
});
