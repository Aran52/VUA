import { describe, expect, it } from "vitest";
import { amfRunProblem, isAmfProductionRun, createAmfProductionPort } from "./amf-production-port.ts";
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
    // A rejected run names the part that failed, for diagnosis.
    expect(amfRunProblem(run)).toBeNull();
    expect(amfRunProblem({...run,choices:[{sourceId:"coat",role:"any-role"}]})).toBe("choices");
    expect(amfRunProblem({...run,progress:{step:"unknown"}})).toBe("progress");
    expect(amfRunProblem(null)).toBe("record");
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,observedSelection:selection}}})).toBe(true);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,observedSelection:{...selection,outfits:[{...selection.outfits[0],active:"false"}]}}}})).toBe(false);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,hasMissingScripts:"none"}}})).toBe(false);
  });
  it("accepts the SDK's own checks only in their published shape", () => {
    const sdkReview = { state: "checked", platform: "pc", overall: "Poor", detail: "",
      issues: [{ level: "error", text: "This avatar has no pipeline manager component.", canFix: true }],
      stats: [{ category: "BoneCount", rating: "Poor", text: "Bones: 300" }] };
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,sdkReview}}})).toBe(true);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,sdkReview:{...sdkReview,overall:"None",state:"unavailable"}}}})).toBe(true);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,sdkReview:{...sdkReview,state:"done"}}}})).toBe(false);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,sdkReview:{...sdkReview,stats:[{category:"BoneCount",rating:"Bad",text:""}]}}}})).toBe(false);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,sdkReview:{...sdkReview,issues:[{level:"error",text:"x",canFix:"yes"}]}}}})).toBe(false);
  });
  it("accepts previews only inside the work project's preview folder", () => {
    const previews = [{view:"front",path:".vua/previews/n3-test-1/front.png"},{view:"side",path:".vua/previews/n3-test-1/side.png"}];
    const prefab = {sourceId:"coat",path:"Assets/Coat.prefab",name:"Coat",isAvatar:false,hasSkinnedMesh:true,hasMergeArmature:true,hasMissingScripts:false,humanoidBones:[]};
    expect(isAmfProductionRun({...run,prefabs:[{...prefab,preview:".vua/previews/n3-test-0/candidate-000.png"}],result:{facts:{...run.result.facts,previews}}})).toBe(true);
    expect(isAmfProductionRun({...run,prefabs:[prefab]})).toBe(true);
    expect(isAmfProductionRun({...run,prefabs:[{...prefab,preview:"C:/Users/me/secret.png"}]})).toBe(false);
    expect(isAmfProductionRun({...run,prefabs:[{...prefab,preview:".vua/previews/../../Assets/x.png"}]})).toBe(false);
    expect(isAmfProductionRun({...run,result:{facts:{...run.result.facts,previews:[{view:"top",path:previews[0]?.path}]}}})).toBe(false);
  });
  it("accepts a stopped run with step times, usual durations and live progress", () => {
    const steps = [{key:"inspect",name:"Identify actual Avatar and outfit prefabs",state:"stopped",detail:"",startedAt:"2026-10-17T08:00:00.000Z",endedAt:"2026-10-17T08:04:10.000Z"}];
    const stopped = {...run,phase:"stopped",steps,usualDurations:{inspect:412},progress:null};
    expect(isAmfProductionRun(stopped)).toBe(true);
    const progress = {step:"reinspect",name:"Reinspect the retained work project",startedAt:"2026-10-17T08:10:00.000Z",elapsedSeconds:95,usualSeconds:null,
      unity:{stage:"importing",current:240,total:379,lastActivityAt:"2026-10-17T08:11:33.000Z"}};
    expect(isAmfProductionRun({...stopped,phase:"inspecting",progress})).toBe(true);
    expect(isAmfProductionRun({...stopped,phase:"paused"})).toBe(false);
    expect(isAmfProductionRun({...stopped,steps:[{...steps[0],startedAt:undefined}]})).toBe(false);
    expect(isAmfProductionRun({...stopped,steps:[{...steps[0],state:"waiting"}]})).toBe(false);
    expect(isAmfProductionRun({...stopped,usualDurations:{inspect:-1}})).toBe(false);
    expect(isAmfProductionRun({...stopped,progress:{...progress,unity:{...progress.unity,line:"Start importing Assets/Coat.prefab"}}})).toBe(false);
    expect(isAmfProductionRun({...stopped,progress:{...progress,unity:{...progress.unity,stage:"downloading"}}})).toBe(false);
  });
  it("stops through task cancellation and reads only AMF progress events", async () => {
    const requests: unknown[] = [];
    let emit: (event: unknown) => void = () => {};
    const client: GatewayClient = {async invoke(request){requests.push(request);return{ok:true,value:{} as DesktopGatewaySuccessValueV1};},
      subscribe(listener){emit = event => listener(event as Parameters<typeof listener>[0]);return()=>{};}};
    const port = createAmfProductionPort(client);
    await port.stop({taskId:"task-amf"});
    expect(requests[0]).toMatchObject({method:"task.requestCancellation",params:{taskId:"task-amf"}});
    await expect(port.stop({taskId:""})).rejects.toThrow("not_running");
    const seen: unknown[] = [];
    port.onProgress(progress => seen.push(progress));
    const payload = {operation:"amf.production",runId:"n3-test",step:"inspect",name:"Identify actual Avatar and outfit prefabs",startedAt:"2026-10-17T08:00:00.000Z",
      unity:{stage:"importing",current:12,total:379,lastActivityAt:"2026-10-17T08:01:00.000Z"}};
    emit({kind:"task.progressed",taskId:"task-amf",payload});
    emit({kind:"task.progressed",taskId:"task-download",payload:{completed:1,messageKey:"tasks.download.transfer"}});
    emit({kind:"task.progressed",taskId:"task-amf",payload:{...payload,unity:{...payload.unity,log:"[Licensing::Client] ..."}}});
    emit({kind:"task.stateChanged",taskId:"task-amf",payload:{}});
    expect(seen).toEqual([payload]);
  });
  it("reads the waiting work of the queue in its published shape", async () => {
    const list = {schemaVersion:"0.1",sources:[],editors:[],runs:[],busy:true,capacity:2,
      running:[{taskId:"task-check-a",runId:"n3-a",operation:"amf.production.reinspect"},{taskId:"task-check-b",runId:"n3-b",operation:"amf.production.reinspect"}],
      queue:[{taskId:"task-upload",runId:"n3-test",operation:"amf.production.upload"}]};
    let response: unknown = list;
    const client: GatewayClient = {async invoke(){return{ok:true,value:response as DesktopGatewaySuccessValueV1};},subscribe(){return()=>{};}};
    const port = createAmfProductionPort(client);
    expect((await port.list()).queue).toEqual(list.queue);
    expect((await port.list()).running).toEqual(list.running);
    expect((await port.list()).capacity).toBe(2);
    for (const invalid of [{...list,capacity:4},{...list,capacity:1},{...list,busy:false},{...list,running:undefined}]) {
      response = invalid;
      await expect(port.list()).rejects.toThrow("list_invalid");
    }
    response = {...list,queue:[{taskId:"task-upload",runId:"n3-test",operation:"task.list"}]};
    await expect(port.list()).rejects.toThrow("list_invalid");
    const {queue: _missing, ...withoutQueue} = list; response = withoutQueue;
    await expect(port.list()).rejects.toThrow("list_invalid");
  });
  it("reads the BOOTH products each source maps to", async () => {
    const source = {sourceId:"cpy-1",itemId:"whi-1",itemName:"Meiyun.zip",name:"Meiyun.unitypackage"};
    const list = {schemaVersion:"0.1",sources:[{...source,productIds:["booth:4001"]},source],editors:[],runs:[],busy:false,capacity:1,running:[],queue:[]};
    let response: unknown = list;
    const client: GatewayClient = {async invoke(){return{ok:true,value:response as DesktopGatewaySuccessValueV1};},subscribe(){return()=>{};}};
    const port = createAmfProductionPort(client);
    expect((await port.list()).sources.map(item => item.productIds ?? [])).toEqual([["booth:4001"], []]);
    for (const productIds of ["booth:4001", [""], [7]]) {
      response = {...list,sources:[{...source,productIds}]};
      await expect(port.list()).rejects.toThrow("list_invalid");
    }
  });
  it("sets capacity through the command and verifies the saved result", async () => {
    const requests: unknown[] = [];
    let result: unknown = {schemaVersion:"0.1",capacity:2};
    const client: GatewayClient = {async invoke(request){requests.push(request);return{ok:true,value:result as DesktopGatewaySuccessValueV1};},subscribe(){return()=>{};}};
    const port = createAmfProductionPort(client);
    expect(await port.setCapacity(2)).toBe(2);
    expect(requests[0]).toMatchObject({method:"amf.production.setCapacity",params:{schemaVersion:"0.1",capacity:2}});
    for (const invalid of [{schemaVersion:"0.1",capacity:1},{schemaVersion:"0.1",capacity:4},{}]) {
      result = invalid;
      await expect(port.setCapacity(2)).rejects.toThrow("capacity_invalid");
    }
  });
  it("reads the reviewed lighting plan and applies a chosen subset by its digest", async () => {
    const plan = {schemaVersion:"0.1",runId:"n3-test",planDigest:"b".repeat(64),
      reference:{min:"0.1",max:"1"},referenceReason:null,
      entries:[{material:"Assets/Outfit/Coat.mat",sourceId:"outfit",current:{min:"0.05",max:"1"},proposed:{min:"0.1",max:"1"}}],
      skipped:[{material:"Assets/Shared/Body.mat",reason:"shared_with_avatar"}]};
    let response: unknown = plan;
    const requests: unknown[] = [];
    const client: GatewayClient = {async invoke(request){requests.push(request);return{ok:true,value:response as DesktopGatewaySuccessValueV1};},subscribe(){return()=>{};}};
    const port = createAmfProductionPort(client);
    const reviewed = await port.planLighting("n3-test");
    expect(reviewed.entries[0]?.proposed.min).toBe("0.1");
    expect(requests[0]).toMatchObject({method:"amf.production.planLighting",params:{schemaVersion:"0.1",runId:"n3-test"}});
    for (const invalid of [{...plan,reference:null},{...plan,referenceReason:"tie",reference:{min:"0.1",max:"1"}},{...plan,planDigest:"z".repeat(64)},{...plan,entries:[{material:"Assets/Outfit/Coat.mat"}]}]) {
      response = invalid;
      await expect(port.planLighting("n3-test")).rejects.toThrow("lighting_plan_invalid");
    }
    response = {schemaVersion:"0.1",runId:"n3-test",taskId:"task-lighting"};
    await port.applyLighting("n3-test",reviewed,["Assets/Outfit/Coat.mat"]);
    expect(requests.at(-1)).toMatchObject({method:"amf.production.applyLighting",params:{schemaVersion:"0.1",runId:"n3-test",planDigest:plan.planDigest,materials:["Assets/Outfit/Coat.mat"]}});
    await expect(port.applyLighting("n3-other",reviewed,["Assets/Outfit/Coat.mat"])).rejects.toThrow("lighting_plan_invalid");
  });
  it("turns a reported preview into a PNG data URL", async () => {
    const path = ".vua/previews/n3-test-1/front.png";
    let response: unknown = {schemaVersion:"0.1",runId:"n3-test",path,mediaType:"image/png",data:"iVBORw0KGgoAAAANSUhEUg=="};
    const requests: unknown[] = [];
    const client: GatewayClient = {async invoke(request){requests.push(request);return{ok:true,value:response as DesktopGatewaySuccessValueV1};},subscribe(){return()=>{};}};
    const port = createAmfProductionPort(client);
    expect(await port.previewImage("n3-test",path)).toBe("data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==");
    expect(requests[0]).toMatchObject({method:"amf.production.previewImage",params:{schemaVersion:"0.1",runId:"n3-test",path}});
    response = {schemaVersion:"0.1",runId:"n3-test",path,mediaType:"image/png",data:"PHN2Zz48L3N2Zz4="};
    await expect(port.previewImage("n3-test",path)).rejects.toThrow("preview_invalid");
    response = {schemaVersion:"0.1",runId:"n3-test",path:".vua/previews/n3-test-1/back.png",mediaType:"image/png",data:"iVBORw0KGgo="};
    await expect(port.previewImage("n3-test",path)).rejects.toThrow("preview_invalid");
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
  it("accepts a run without revisions and rejects malformed revision items", () => {
    expect(isAmfProductionRun(run)).toBe(true);
    const revision = {id:"rev-1",before:"assembly",takenAt:"2026-10-18T00:00:00Z",editedInUnity:false,
      summary:{avatar:null,outfits:[],sources:["coat"],lightingMaterials:0,sdkFixes:[]}};
    expect(isAmfProductionRun({...run,revisions:[revision]})).toBe(true);
    expect(amfRunProblem({...run,revisions:[revision]})).toBeNull();
    expect(amfRunProblem({...run,revisions:"none"})).toBe("revisions");
    expect(amfRunProblem({...run,revisions:[{...revision,before:"everything"}]})).toBe("revisions");
    expect(amfRunProblem({...run,revisions:[{...revision,editedInUnity:"no"}]})).toBe("revisions");
    expect(amfRunProblem({...run,revisions:[{...revision,summary:{...revision.summary,lightingMaterials:-1}}]})).toBe("revisions");
    expect(amfRunProblem({...run,revisions:[{...revision,summary:{...revision.summary,avatar:7}}]})).toBe("revisions");
  });
  it("reads a revision plan and binds its restore to this run and revision", async () => {
    const plan = {schemaVersion:"0.1",runId:"n3-test",revisionId:"rev-1",planDigest:"c".repeat(64),before:"assembly" as const,takenAt:"2026-10-18T00:00:00Z",
      summary:{avatar:"Avatar",outfits:["Coat"],sources:["avatar","coat"],lightingMaterials:1,sdkFixes:[]},editedInUnity:true,warning:null,
      steps:[{key:"materials",skipped:false},{key:"assembly",skipped:false,rebuild:false},
        {key:"lighting",skipped:false,materials:[{material:"Assets/Coat.mat",min:"0.1",max:"1"}]},{key:"sdk_fix",skipped:true,fixes:[]}]};
    let response: unknown = plan;
    const requests: unknown[] = [];
    const client: GatewayClient = {async invoke(request){requests.push(request);return{ok:true,value:response as DesktopGatewaySuccessValueV1};},subscribe(){return()=>{};}};
    const port = createAmfProductionPort(client);
    expect((await port.revisionPlan("n3-test","rev-1")).editedInUnity).toBe(true);
    expect(requests[0]).toMatchObject({method:"amf.production.revisionPlan",params:{schemaVersion:"0.1",runId:"n3-test",revisionId:"rev-1"}});
    response = {...plan,planDigest:"z".repeat(64)}; await expect(port.revisionPlan("n3-test","rev-1")).rejects.toThrow("revision_plan_invalid");
    response = {...plan,before:"everything"}; await expect(port.revisionPlan("n3-test","rev-1")).rejects.toThrow("revision_plan_invalid");
    response = {...plan,revisionId:"rev-2"}; await expect(port.revisionPlan("n3-test","rev-1")).rejects.toThrow("revision_plan_invalid");
    const {steps: _missing, ...withoutSteps} = plan; response = withoutSteps;
    await expect(port.revisionPlan("n3-test","rev-1")).rejects.toThrow("revision_plan_invalid");
    response = {...plan,steps:[{key:"upload",skipped:false}]}; await expect(port.revisionPlan("n3-test","rev-1")).rejects.toThrow("revision_plan_invalid");
    await expect(port.restoreRevision("n3-other",plan)).rejects.toThrow("revision_plan_invalid");
    response = {schemaVersion:"0.1",operation:"amf.production.restoreRevision",runId:"n3-test",taskId:"task-restore"};
    await port.restoreRevision("n3-test",plan);
    expect(requests.at(-1)).toMatchObject({method:"amf.production.restoreRevision",params:{schemaVersion:"0.1",runId:"n3-test",revisionId:"rev-1",planDigest:plan.planDigest}});
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
