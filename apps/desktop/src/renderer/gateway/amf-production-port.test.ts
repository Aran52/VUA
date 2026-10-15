import { describe, expect, it } from "vitest";
import { isAmfProductionRun, createAmfProductionPort } from "./amf-production-port.ts";
import type { GatewayClient } from "./gateway-client.ts";
import type { DesktopGatewaySuccessValueV1 } from "@vua/contracts";

const selection = {avatarPrefab:"Assets/Avatar.prefab",outfits:[{sourceId:"coat",prefabPath:"Assets/Coat.prefab",mode:"merge_armature" as const,bone:"",toggleName:"Coat",active:true}]};
const run = {schemaVersion:"0.1",runId:"n3-test",phase:"succeeded",taskId:"task-test",editor:{path:"C:/Unity/Editor/Unity.exe",version:"2022.3.22f1c1"},projectRoot:"C:/Managed/test",
  choices:[{sourceId:"coat",role:"auto"}],sources:[],prefabs:[],steps:[],selection,error:null,recovery:null,snapshotId:"n3-snapshot",
  result:{facts:{sdkDescriptorRecognized:true,missingReferences:0,avatarName:"Avatar",instances:[{sourceId:"coat",bindingVerified:true,menuVerified:true,menuModified:true}]}}};

describe("AMF production response boundary", () => {
  it("refuses malformed results and preserves full Editor identity", () => {
    expect(isAmfProductionRun(run)).toBe(true);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,instances:"passed"}}})).toBe(false);
    expect(isAmfProductionRun({...run,editor:{...run.editor,version:"2022.3.22"}})).toBe(false);
    expect(isAmfProductionRun({...run,choices:[{sourceId:"coat",role:"any-role"}]})).toBe(false);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,observedSelection:selection}}})).toBe(true);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,observedSelection:{...selection,outfits:[{...selection.outfits[0],active:"false"}]}}}})).toBe(false);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,hasMissingScripts:"none"}}})).toBe(false);
  });
  it("does not accept a plan that omits preservation or fabricates SDK completion", async () => {
    const plan = {schemaVersion:"0.1",runId:"n3-test",planDigest:"a".repeat(64),selection,inputs:[],editor:run.editor,projectRoot:run.projectRoot,preserveOriginals:true,officialSdkChecks:"not_run"};
    let response: unknown = plan;
    const client: GatewayClient = {async invoke(){return{ok:true,value:response as DesktopGatewaySuccessValueV1};},subscribe(){return()=>{};}};
    const port = createAmfProductionPort(client);
    expect((await port.plan("n3-test",selection)).selection).toEqual(selection);
    response = {...plan,officialSdkChecks:"passed"}; await expect(port.plan("n3-test",selection)).rejects.toThrow("plan_invalid");
    response = {...plan,preserveOriginals:false}; await expect(port.plan("n3-test",selection)).rejects.toThrow("plan_invalid");
  });
  it("requires a restore plan to preserve current work and belong to this run", async () => {
    const plan = {schemaVersion:"0.1",runId:"n3-test",planDigest:"a".repeat(64),snapshotId:"n3-snapshot",projectRoot:run.projectRoot,currentChanged:true,preserveCurrent:true};
    let response: unknown = plan;
    const client: GatewayClient = {async invoke(){return{ok:true,value:response as DesktopGatewaySuccessValueV1};},subscribe(){return()=>{};}};
    const port = createAmfProductionPort(client);
    expect((await port.rollbackPlan("n3-test")).currentChanged).toBe(true);
    response = {...plan,preserveCurrent:false}; await expect(port.rollbackPlan("n3-test")).rejects.toThrow("restore_plan_invalid");
    response = {...plan,runId:"n3-other"}; await expect(port.rollbackPlan("n3-test")).rejects.toThrow("restore_plan_invalid");
    response = {...plan,snapshotId:""}; await expect(port.rollbackPlan("n3-test")).rejects.toThrow("restore_plan_invalid");
    await expect(port.rollback("n3-other",{...plan,preserveCurrent:true})).rejects.toThrow("restore_plan_invalid");
  });
  it("binds comparison and execution to the chosen observed Avatar", async () => {
    const plan = {runId:"n3-test",recipeId:"recipe-test",title:"Recipe",avatarName:"Same name",avatarGlobalObjectId:"other-avatar",planDigest:"d".repeat(64),retainedOutfits:0,overlayBlocker:null,
      conflicts:[],recipeSelection:selection,currentSelection:selection,targets:[{globalObjectId:"active-avatar",name:"Same name",verified:true},{globalObjectId:"other-avatar",name:"Same name",verified:true}]};
    let response: unknown = {schemaVersion:"0.1",plan};
    const requests: unknown[] = [];
    const client: GatewayClient = {async invoke(request){requests.push(request);return{ok:true,value:response as DesktopGatewaySuccessValueV1};},subscribe(){return()=>{};}};
    const port = createAmfProductionPort(client);
    const reviewed = await port.recipeApplyPlan("n3-test","recipe-test","other-avatar");
    expect(requests[0]).toMatchObject({params:{avatarGlobalObjectId:"other-avatar"}});
    response = {schemaVersion:"0.1",runId:"n3-test",taskId:"task-apply"};
    await port.applyRecipe(reviewed,"avatar_wins");
    expect(requests[1]).toMatchObject({params:{avatarGlobalObjectId:"other-avatar",choice:"avatar_wins",planDigest:plan.planDigest}});
    for (const changed of [{...plan,avatarGlobalObjectId:"active-avatar"},{...plan,targets:[plan.targets[1],plan.targets[1]]},{...plan,targets:plan.targets.map(t=>({...t,verified:false}))}]) {
      response = {schemaVersion:"0.1",plan:changed};
      await expect(port.recipeApplyPlan("n3-test","recipe-test","other-avatar")).rejects.toThrow("apply_plan_invalid");
    }
  });
});
