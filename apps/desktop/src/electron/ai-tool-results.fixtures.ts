/** Synthetic shapes from the MCP round's counts; all names, paths and URLs are invented. */
const bones = Array.from({ length: 50 }, (_, i) => `HumanoidBone${String(i).padStart(2, "0")}`);
const prefabs = Array.from({ length: 9 }, (_, i) => ({ sourceId: `material-${i}`, path: `Assets/SyntheticMaterial${i}/Avatar${i}.prefab`, name: `Avatar${i}`, isAvatar: i < 3,
  hasSkinnedMesh: true, hasMergeArmature: i >= 3, hasMissingScripts: false, humanoidBones: bones,
  preview: `.vua/previews/n3-report-1/candidate-${String(i).padStart(3, "0")}.png` }));
export const reportRun = {
  schemaVersion: "0.1", runId: "n3-report", taskId: "report-task-079", revision: 7, phase: "ready", recovery: null,
  createdAt: "2026-10-20T00:00:00.000Z", completedAt: "2026-10-20T00:01:00.000Z",
  projectRoot: "C:/Synthetic/WorkProject", projectFingerprint: "a".repeat(64), unityFingerprint: "b".repeat(64),
  selection: { avatarPrefab: prefabs[0]!.path, outfits: [{ sourceId: "material-3", prefabPath: prefabs[3]!.path, mode: "merge_armature", bone: "", toggleName: "Outfit", active: true }] },
  sources: Array.from({ length: 9 }, (_, i) => ({ sourceId: `material-${i}`, role: "auto", itemName: `SyntheticMaterial${i}.unitypackage` })),
  steps: ["project", "materials", "inspect"].map(key => ({ key, name: key, state: "succeeded", detail: "", startedAt: "2026-10-20T00:00:00.000Z", endedAt: "2026-10-20T00:01:00.000Z" })),
  prefabs, result: { officialSdkChecks: "not_run", visualReview: "pending", diagnostics: [], facts: {
    prefabs, avatarName: "SyntheticAvatar", missingReferences: 0, hasMissingScripts: false,
    previews: ["front", "side", "back"].map(view => ({ view, path: `.vua/previews/n3-report-1/${view}.png` })),
    instances: Array.from({ length: 9 }, (_, i) => ({ sourceId: `material-${i}`, prefabPath: prefabs[i]!.path, globalObjectId: `GlobalObjectId-${i}`, bindingVerified: true, menuVerified: true })),
    materialUse: Array.from({ length: 100 }, (_, i) => ({ owner: `SyntheticAvatar/Armature/Hips/Spine/Chest/Outfit${i}`, renderer: `SyntheticAvatar/Armature/Hips/Spine/Chest/Outfit${i}/SyntheticRenderer`, material: `Assets/SyntheticMaterial${i}/Materials/Clothing/SyntheticOutfitMaterial${i}.mat` })),
    sdkReview: { state: "checked", platform: "StandaloneWindows64", overall: "Good", issues: [], stats: [{ category: "Triangles", rating: "Good", text: "Synthetic SDK rating." }] },
  } },
};
export const reportTasks = { contractVersion: "0.1", revision: 80, tasks: Array.from({ length: 80 }, (_, i) => ({
  contractVersion: "0.1", taskId: `report-task-${String(i).padStart(3, "0")}`, revision: 6, correlationId: `correlation-${i}`,
  state: "succeeded", cancellationRequested: false, recoveryDisposition: "none", updatedAt: new Date(Date.UTC(2026, 9, 20, 0, i)).toISOString(),
  result: { schemaVersion: "0.1", operation: "amf.production", runId: "n3-report", detail: `Synthetic completion receipt ${i}. ` + "Observed and retained. ".repeat(5) },
})) };
export const reportLibrary = { schemaVersion: "0.1", total: 12, offset: 0, limit: 25, items: Array.from({ length: 12 }, (_, i) => ({
  kind: "product", sources: ["bought"], operation: null,
  storage: { state: "present", storedCopies: 1, presentCopies: 1, missingCopies: 0, changedCopies: 0, unreadableCopies: 0, supersededGeneratedCopies: 0, currentGeneratedCopies: 0, productionQualification: "not_evaluated" },
  product: { productId: `booth:${100 + i}`, title: `Synthetic product ${i}`, shopName: "Synthetic shop", libraryType: "bought", importedArtifacts: 1, variantName: null, price: null,
    imageUrl: `https://images.invalid/product-${i}/first.png`,
    imageUrls: Array.from({ length: 28 }, (_, j) => `https://images.invalid/product-${i}/synthetic-public-image-${j}-${"a".repeat(56)}.png`),
    availabilityRaw: null, availabilityStatus: "unknown", entityCount: 0, entityTypes: [],
  },
})) };
export const reportStatus = { schemaVersion: 1, productVersion: "0.6.7", runtime: "electron", platform: "win32", capabilities: {
  gateway: true, tasks: true, operations: Array.from({ length: 35 }, (_, i) => ({ operationId: `synthetic.operation.${i}`, availability: "available" })), remoteBrowser: true,
}, vrchat: { experimentalEnabled: false, signedIn: false } };
/** A long-worked line: 50 version records, newest first, with nanosecond IDs and Japanese names. */
export const reportRunWithRevisions = { ...reportRun, revisions: Array.from({ length: 50 }, (_, i) => ({
  id: `rev-${1792461600000000000 - i * 1000000000}`, before: i % 2 === 0 ? "assembly" : "materials", takenAt: new Date(Date.UTC(2026, 9, 20, 1, 50 - i)).toISOString(),
  summary: { avatar: "合成アバター・メイユン風", outfits: ["合成Tシャツ＆ショートパンツ", "合成ヘアクリップ"],
    sources: ["合成アバター_v2.1.0", "合成Tシャツ＆ショートパンツ for 合成アバター v1.00", "合成ヘアクリップ"], lightingMaterials: 12, sdkFixes: [] },
  editedInUnity: false,
})) };
