import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { isUnityLicenseParams, isUnityLicenseSnapshotV01, isUnityLicenseCheckResultV01, isUnityLicenseAcceptedV01 } from "./unity-license-v01.js";
import { isApplicationRequestV01 } from "./application-contract.js";
import { DESKTOP_GATEWAY_METHOD_KINDS, isDesktopGatewayRequestV1 } from "./desktop-gateway.js";

describe("independent Unity license v0.1", () => {
  it("consumes the same closed request vectors as the native provider", () => {
    const root = new URL("../../../schemas/unity-license/v0.1/requests/", import.meta.url);
    for (const name of readdirSync(root)) {
      const value = JSON.parse(readFileSync(new URL(name, root), "utf8"));
      const envelope = {contractVersion:"0.1",requestId:"license",correlationId:"license",...value,...(value.kind === "command" ? {commandId:"license"} : {})};
      expect(isApplicationRequestV01(envelope), name).toBe(name.endsWith(".valid.json"));
      if (name.endsWith(".valid.json")) expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"license",method:value.method,params:value.params}), name).toBe(true);
    }
    expect(DESKTOP_GATEWAY_METHOD_KINDS["environment.unityLicense"]).toBe("query");
    expect(DESKTOP_GATEWAY_METHOD_KINDS["environment.checkUnityLicense"]).toBe("command");
    expect(isUnityLicenseParams("environment.checkUnityLicense", {schemaVersion:"0.1",editorPath:"C:/Unity.exe\0"})).toBe(false);
  });
  it("consumes result vectors and never admits an active claim from metadata", () => {
    const root = new URL("../../../schemas/unity-license/v0.1/results/", import.meta.url);
    for (const name of readdirSync(root)) {
      const value: unknown = JSON.parse(readFileSync(new URL(name, root), "utf8"));
      expect(isUnityLicenseSnapshotV01(value) || isUnityLicenseCheckResultV01(value) || isUnityLicenseAcceptedV01(value), name).toBe(name.endsWith(".valid.json"));
    }
  });
});
