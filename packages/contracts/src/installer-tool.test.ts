import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { INSTALLER_TOOL_IDS, INSTALLER_TOOL_NAMES, isInstallerToolResult } from "./installer-tool.js";
import { isApplicationRequestV01 } from "./application-contract.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";

const vectors = JSON.parse(readFileSync(new URL("../../../schemas/installer-tool/v0.1/vectors.json", import.meta.url), "utf8")) as Record<"requests" | "results", { name: string; valid: boolean; value: Record<string, unknown> }[]>;

describe("installer-tool v0.1 Candidate wire", () => {
  it("pins closed application requests and observed facts to shared vectors", () => {
    for (const v of vectors.requests) expect(isApplicationRequestV01(v.value), v.name).toBe(v.valid);
    for (const v of vectors.results) expect(isInstallerToolResult(v.value), v.name).toBe(v.valid);
  });

  it("allows only the four fixed official-installer tools and launch actions at IPC", () => {
    for (const toolId of INSTALLER_TOOL_IDS) {
      expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "r", method: "tools.observeInstaller", params: { toolId } })).toBe(true);
      expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "r", method: "tools.actInstaller", params: { toolId, action: "start", commandId: "cmd-1" } })).toBe(true);
      expect(INSTALLER_TOOL_NAMES[toolId].length).toBeGreaterThan(0);
    }
    expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "r", method: "tools.actInstaller", params: { toolId: "vrct", action: "install", commandId: "cmd-1" } })).toBe(false);
    expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "r", method: "tools.observeInstaller", params: { toolId: "vrcft" } })).toBe(false);
  });
});
