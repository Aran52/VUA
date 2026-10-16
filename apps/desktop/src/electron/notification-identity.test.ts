import { describe, expect, it } from "vitest";
import { notificationIdentity, registrationArguments } from "./notification-identity.js";

describe("Windows notification identity", () => {
  it("keeps development notifications apart from the release", () => {
    expect(notificationIdentity("release")).toEqual({ appId: "org.vua-project.desktop", displayName: "VUA" });
    expect(notificationIdentity("development").appId).toBe("org.vua-project.desktop.dev");
    expect(notificationIdentity("packaged-smoke").appId).toBe("org.vua-project.desktop.dev");
  });
  it("registers name and icon for the current user only", () => {
    const commands = registrationArguments("org.vua-project.desktop", "VUA", "C:\\Users\\p\\AppData\\Roaming\\VUA\\notification-icon.png");
    expect(commands).toEqual([
      ["add", "HKCU\\Software\\Classes\\AppUserModelId\\org.vua-project.desktop", "/v", "DisplayName", "/t", "REG_SZ", "/d", "VUA", "/f"],
      ["add", "HKCU\\Software\\Classes\\AppUserModelId\\org.vua-project.desktop", "/v", "IconUri", "/t", "REG_SZ", "/d", "C:\\Users\\p\\AppData\\Roaming\\VUA\\notification-icon.png", "/f"],
    ]);
  });
});
