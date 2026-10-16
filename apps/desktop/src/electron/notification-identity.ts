import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import type { DesktopProfile } from "./runtime-profile.js";
import { trayIcons } from "./tray-icons.js";

/** Windows groups and labels notifications by app identity. Development keeps its own identity so
 * its notification settings stay apart from the release's. */
export function notificationIdentity(kind: DesktopProfile["kind"]): { appId: string; displayName: string } {
  return kind === "release"
    ? { appId: "org.vua-project.desktop", displayName: "VUA" }
    : { appId: "org.vua-project.desktop.dev", displayName: "VUA (dev)" };
}

/** `reg.exe add` arguments that register the identity for the current user: name and icon. */
export function registrationArguments(appId: string, displayName: string, iconPath: string): string[][] {
  const key = `HKCU\\Software\\Classes\\AppUserModelId\\${appId}`;
  return [["DisplayName", displayName], ["IconUri", iconPath]]
    .map(([name, value]) => ["add", key, "/v", name!, "/t", "REG_SZ", "/d", value!, "/f"]);
}

/** Windows shows a desktop app's notifications once its identity is registered. VUA registers itself
 * for the current user at each start, with the icon written next to its data. */
export async function registerNotificationIdentity(identity: { appId: string; displayName: string }, userData: string): Promise<void> {
  const iconPath = path.join(userData, "notification-icon.png");
  const largest = trayIcons.dark.reduce((best, icon) => icon.scaleFactor > best.scaleFactor ? icon : best);
  await fs.writeFile(iconPath, Buffer.from(largest.png, "base64"));
  const reg = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "reg.exe");
  for (const args of registrationArguments(identity.appId, identity.displayName, iconPath)) {
    await new Promise<void>((resolve, reject) => {
      execFile(reg, args, { windowsHide: true }, error => error ? reject(error) : resolve());
    });
  }
}
