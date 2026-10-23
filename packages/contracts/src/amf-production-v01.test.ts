import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { isAmfProductionParams, isAmfEditorCloseResult, isAmfAssemblySelection, readAmfProductionProgress, isAmfStep, isAmfUsualDurations, isAmfRunProgress, AMF_RUN_PHASES } from "./amf-production-v01.js";
import { DESKTOP_GATEWAY_METHOD_KINDS, isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
import { isApplicationRequestV01 } from "./application-contract.js";

const selection = { avatarPrefab:"Assets/Avatar.prefab",outfits:[{sourceId:"outfit",prefabPath:"Assets/Outfit.prefab",mode:"merge_armature",bone:"",toggleName:"Coat",active:true}] };
describe("AMF material-first Candidate boundary", () => {
  it("registers saving and closing as an explicitly confirmed command", () => {
    const params={schemaVersion:"0.1",runId:"n3-test",confirmed:true};
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"close",method:"amf.production.closeEditor",params})).toBe(true);
    expect(isApplicationRequestV01({contractVersion:"0.1",requestId:"close",correlationId:"close",commandId:"cmd-close",kind:"command",method:"amf.production.closeEditor",params})).toBe(true);
    for (const bad of [{...params,confirmed:false},{...params,confirmed:"true"},{schemaVersion:"0.1",runId:"n3-test"},{...params,force:true}]) {
      expect(isAmfProductionParams("amf.production.closeEditor",bad)).toBe(false);
    }
    expect(isApplicationRequestV01({contractVersion:"0.1",requestId:"close",correlationId:"close",kind:"query",method:"amf.production.closeEditor",params})).toBe(false);
  });
  it("registers capacity as a command with exactly three admitted values", () => {
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.setCapacity"]).toBe("command");
    for (const capacity of [1, 2, 3]) {
      const params = {schemaVersion:"0.1",capacity};
      expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"capacity",method:"amf.production.setCapacity",params})).toBe(true);
    }
    for (const capacity of [0, 4, 1.5, "2", true, null]) {
      expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"capacity",method:"amf.production.setCapacity",params:{schemaVersion:"0.1",capacity}})).toBe(false);
    }
  });
  it("registers the lighting review as a query and its digest-bound application as a command", () => {
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.planLighting"]).toBe("query");
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.applyLighting"]).toBe("command");
    const digest = "b".repeat(64);
    const materials = ["Assets/Outfit/Coat.mat"];
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"lighting",method:"amf.production.planLighting",params:{schemaVersion:"0.1",runId:"n3-test"}})).toBe(true);
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"lighting",method:"amf.production.applyLighting",params:{schemaVersion:"0.1",runId:"n3-test",planDigest:digest,materials}})).toBe(true);
    for (const params of [
      {schemaVersion:"0.1",runId:"n3-test",planDigest:digest,materials:[]},
      {schemaVersion:"0.1",runId:"n3-test",planDigest:digest,materials:["Assets/Outfit/Coat.mat","Assets/Outfit/Coat.mat"]},
      {schemaVersion:"0.1",runId:"n3-test",planDigest:digest,materials:["Assets/Outfit/Coat.png"]},
      {schemaVersion:"0.1",runId:"n3-test",planDigest:digest,materials:["../Outside.mat"]},
      {schemaVersion:"0.1",runId:"n3-test",planDigest:"bad",materials},
    ]) {
      expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"lighting",method:"amf.production.applyLighting",params})).toBe(false);
    }
  });
  it("registers saved fits as one query and three synchronous commands", () => {
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.listFits"]).toBe("query");
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.saveFit"]).toBe("command");
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.renameFit"]).toBe("command");
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.deleteFit"]).toBe("command");
    const digest = "b".repeat(64);
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"fit",method:"amf.production.listFits",params:{schemaVersion:"0.1"}})).toBe(true);
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"fit",method:"amf.production.saveFit",params:{schemaVersion:"0.1",runId:"n3-test",sourceId:"outfit",name:"Coat fit",changesDigest:digest}})).toBe(true);
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"fit",method:"amf.production.renameFit",params:{schemaVersion:"0.1",fitId:"fit-1",name:"Coat fit"}})).toBe(true);
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"fit",method:"amf.production.deleteFit",params:{schemaVersion:"0.1",fitId:"fit-1"}})).toBe(true);
    for (const params of [
      {schemaVersion:"0.1",runId:"n3-test",sourceId:"outfit",name:"Coat fit",changesDigest:digest,extra:true},
      {schemaVersion:"0.1",runId:"../other",sourceId:"outfit",name:"Coat fit",changesDigest:digest},
      {schemaVersion:"0.1",runId:"n3-test",sourceId:"",name:"Coat fit",changesDigest:digest},
      {schemaVersion:"0.1",runId:"n3-test",sourceId:"outfit",name:"",changesDigest:digest},
      {schemaVersion:"0.1",runId:"n3-test",sourceId:"outfit",name:"   ",changesDigest:digest},
      {schemaVersion:"0.1",runId:"n3-test",sourceId:"outfit",name:"Coat\nfit",changesDigest:digest},
      {schemaVersion:"0.1",runId:"n3-test",sourceId:"outfit",name:"Coat fit",changesDigest:"bad"},
    ]) {
      expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"fit",method:"amf.production.saveFit",params})).toBe(false);
    }
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"fit",method:"amf.production.renameFit",params:{schemaVersion:"0.1",fitId:"rev-1",name:"Coat fit"}})).toBe(false);
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"fit",method:"amf.production.renameFit",params:{schemaVersion:"0.1",fitId:"fit-1",name:" "}})).toBe(false);
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"fit",method:"amf.production.deleteFit",params:{schemaVersion:"0.1",fitId:"fit-"}})).toBe(false);
  });
  it("consumes the shared positive and negative native request vectors", () => {
    const root = new URL("../../../schemas/amf-production-flow/v0.1/examples/", import.meta.url);
    for (const name of readdirSync(root)) {
      const value = JSON.parse(readFileSync(new URL(name,root),"utf8"));
      const envelope = {contractVersion:"0.1",requestId:"req-vector",correlationId:"corr-vector",...value,
        ...(value.kind === "command" ? {commandId:"cmd-vector"} : {})};
      expect(isApplicationRequestV01(envelope),name).toBe(name.includes(".valid."));
    }
  });
  it("registers the same closed face in both desktop and native requests", () => {
    const params = {schemaVersion:"0.1",runId:"n3-test",selection,planDigest:"a".repeat(64)};
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"req",method:"amf.production.assemble",params})).toBe(true);
    expect(isApplicationRequestV01({contractVersion:"0.1",requestId:"req",correlationId:"corr",kind:"command",commandId:"cmd",method:"amf.production.assemble",params})).toBe(true);
  });
  it("admits automatic source selection without inventing file paths", () => {
    const params = {schemaVersion:"0.1",sources:[{sourceId:"copy-1",role:"auto"}],editorPath:"C:/Unity/Editor/Unity.exe"};
    expect(isAmfProductionParams("amf.production.prepare",params)).toBe(true);
    expect(isAmfProductionParams("amf.production.prepare",{...params,projectRoot:"C:/Player/Project"})).toBe(false);
    expect(isAmfProductionParams("amf.production.prepare",{...params,sources:[{sourceId:"copy-1",role:"auto",path:"C:/Private"}]})).toBe(false);
  });
  it("refuses duplicate source identities and unknown schema versions", () => {
    expect(isAmfProductionParams("amf.production.prepare",{schemaVersion:"0.1",sources:[{sourceId:"copy-1",role:"auto"},{sourceId:"copy-1",role:"outfit"}],editorPath:"Unity.exe"})).toBe(false);
    expect(isAmfProductionParams("amf.production.list",{schemaVersion:"0.2"})).toBe(false);
  });
  it("requires a digest on execution and disallows implicit deletion fields", () => {
    expect(isAmfProductionParams("amf.production.assemble",{schemaVersion:"0.1",runId:"n3-test",selection,planDigest:""})).toBe(false);
    expect(isAmfAssemblySelection({...selection,deleteUnmentioned:true})).toBe(false);
  });
  it("reads progress events and run status from the shared vectors without log text", () => {
    const folder = (name: string) => new URL(`../../../schemas/amf-production-flow/v0.1/${name}/`, import.meta.url);
    for (const name of readdirSync(folder("progress-examples"))) {
      const value: unknown = JSON.parse(readFileSync(new URL(name, folder("progress-examples")), "utf8"));
      expect(readAmfProductionProgress(value) !== null, name).toBe(name.includes(".valid."));
    }
    const status = (v: Record<string, unknown>) => (AMF_RUN_PHASES as readonly unknown[]).includes(v.phase)
      && Array.isArray(v.steps) && v.steps.every(isAmfStep) && isAmfUsualDurations(v.usualDurations) && (v.progress === null || isAmfRunProgress(v.progress))
      && (v.editorClose === undefined || isAmfEditorCloseResult(v.editorClose));
    for (const name of readdirSync(folder("status-examples"))) {
      const value = JSON.parse(readFileSync(new URL(name, folder("status-examples")), "utf8")) as Record<string, unknown>;
      expect(status(value), name).toBe(name.includes(".valid."));
    }
    expect(readAmfProductionProgress({ completed: 1, messageKey: "tasks.download.transfer" })).toBeNull();
  });
  it("does not authorize mutating commands as queries or traversal identities", () => {
    expect(isApplicationRequestV01({contractVersion:"0.1",requestId:"req",correlationId:"corr",kind:"query",method:"amf.production.open",params:{schemaVersion:"0.1",runId:"n3-test"}})).toBe(false);
    expect(isAmfProductionParams("amf.production.get",{schemaVersion:"0.1",runId:"../other"})).toBe(false);
  });

  it("accepts every step the AMF engine records, including release steps", () => {
    for (const key of ["project", "materials", "inspect", "reinspect", "snapshot", "assembly", "protect_current", "restore", "turntable", "upload"])
      expect(isAmfStep({ key, name: key, state: "succeeded", detail: "", startedAt: "2026-10-17T15:00:00.000Z", endedAt: "2026-10-17T15:01:00.000Z" }), key).toBe(true);
  });
});

describe("switch options in a selection", () => {
  const outfit = { sourceId: "coat", prefabPath: "Assets/Coat.prefab", mode: "merge_armature", bone: "", toggleName: "Coat", active: true };
  const selection = (extra: Record<string, unknown>) => ({ avatarPrefab: "Assets/Avatar.prefab", outfits: [{ ...outfit, ...extra }] });
  it("accepts the options when present and keeps older selections valid", () => {
    expect(isAmfAssemblySelection(selection({}))).toBe(true);
    expect(isAmfAssemblySelection(selection({ saved: false, group: "main" }))).toBe(true);
    // Unity reports an independent switch with an empty group.
    expect(isAmfAssemblySelection(selection({ saved: true, group: "" }))).toBe(true);
  });
  it("rejects malformed options and unknown keys", () => {
    for (const extra of [{ saved: "no" }, { group: "主服装" }, { group: "g".repeat(33) }, { group: "VUA/Group/main" }, { exclusive: true }]) {
      expect(isAmfAssemblySelection(selection(extra))).toBe(false);
    }
  });
});
