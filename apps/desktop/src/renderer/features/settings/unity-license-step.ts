import type { UnityLicenseSnapshotV01 } from "@vua/contracts";

/** Presentation only: the provider's nextAction owns the meaning of historical checks. */
export function licenseStep(snapshot: UnityLicenseSnapshotV01 | null, editorAvailable: boolean | null,
  checking = false, accountOpened = false) {
  if (checking) return { state: "checking", title: "checking", detail: "checkDetail", action: "none" } as const;
  if (editorAvailable === false) return { state: "attention", title: "noEditor", detail: "needsEditor", action: "install" } as const;
  if (editorAvailable === null) return { state: "hint", title: "reading", detail: "readingDetail", action: "none" } as const;
  if (snapshot === null) return { state: "hint", title: "unknown", detail: "checkDetail", action: "check" } as const;
  if (snapshot.lastCheck?.state === "active" && snapshot.nextAction === "none")
    return { state: "active", title: "active", detail: "activeDetail", action: "none" } as const;
  switch (snapshot.nextAction) {
    case "start_hub": return { state: "attention", title: "needsHub", detail: "needsHubDetail", action: "check" } as const;
    case "sign_in_and_activate": return { state: "attention", title: "notActivated", detail: snapshot.lastCheck?.state === "not_activated" ? "notActivatedDetail" : "notFoundDetail", action: accountOpened ? "check" : "account" } as const;
    default: return { state: "hint", title: snapshot.state === "likely_active" ? "likelyActive" : "unknown", detail: "checkDetail", action: "check" } as const;
  }
}
