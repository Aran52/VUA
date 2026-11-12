using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        private const string AvatarOptimizerTypeName = "Anatawa12.AvatarOptimizer.TraceAndOptimize";
        private const string NdmfAvatarProcessorTypeName = "nadena.dev.ndmf.AvatarProcessor";
        private const string NdmfGeneratedAssetsRoot = "Assets/ZZZ_GeneratedAssets";

        [Serializable]
        internal sealed class OptimizerRequest
        {
            public bool enabled;
        }

        [Serializable]
        internal sealed class OptimizerMetrics
        {
            public string platform = "";
            public string overall = "";
            public List<SdkStat> stats = new List<SdkStat>();
            public long triangles;
            public int materialSlots;
            public int meshes;
            public int skinnedMeshes;
            public long textureBytes;
            public bool textureComplete;
            public long meshBytes;
            public bool meshComplete;
            public int physBones;
            public bool physBonesKnown;
        }

        [Serializable]
        internal sealed class OptimizerReport
        {
            // not_run: no optimizer measurement ran in this Bridge call;
            // measured: local NDMF before/after succeeded;
            // failed: measurement failed without reverting the component toggle.
            public string state = "not_run";
            public bool localMeasurementOnly = true;
            public bool enabled;
            public string measuredAt = "";
            public OptimizerMetrics before = new OptimizerMetrics();
            public OptimizerMetrics after = new OptimizerMetrics();
            public string detail = "";
        }

        private static Type FindLoadedType(string fullName)
        {
            foreach (var assembly in AppDomain.CurrentDomain.GetAssemblies())
            {
                var match = assembly.GetType(fullName, false);
                if (match != null) return match;
            }
            return null;
        }

        private static bool HasOptimizerComponent(GameObject avatar)
        {
            if (avatar == null) return false;
            var traceType = FindLoadedType(AvatarOptimizerTypeName);
            return traceType != null && avatar.GetComponent(traceType) != null;
        }

        // Adds or removes Anatawa12.AvatarOptimizer.TraceAndOptimize with default settings on the
        // active VUA-owned Avatar root. Private/internal fields on TraceAndOptimize are never read
        // or written.
        private static void ApplyOptimizer(Request request, Result result)
        {
            if (request.optimizer == null)
                throw new InvalidOperationException("The Avatar Optimizer request is missing.");
            var avatar = ActiveOwnedAvatar(request);
            if (avatar == null)
                throw new InvalidOperationException("The finished Avatar is not in the work scene. Check the work project again.");
            var traceType = FindLoadedType(AvatarOptimizerTypeName);
            var changed = false;

            if (request.optimizer.enabled)
            {
                if (traceType == null)
                    throw new InvalidOperationException("Avatar Optimizer (Anatawa12.AvatarOptimizer.TraceAndOptimize) is not loaded in this project.");
                var existing = avatar.GetComponents(traceType);
                if (existing.Length == 0)
                {
                    avatar.AddComponent(traceType);
                    changed = true;
                }
                else
                {
                    for (var index = 1; index < existing.Length; index++)
                    {
                        UnityEngine.Object.DestroyImmediate(existing[index]);
                        changed = true;
                    }
                }
            }
            else if (traceType != null)
            {
                var existing = avatar.GetComponents(traceType);
                foreach (var component in existing)
                {
                    UnityEngine.Object.DestroyImmediate(component);
                    changed = true;
                }
            }

            if (!changed) return;
            EditorSceneManager.MarkSceneDirty(avatar.scene);
            AssetDatabase.SaveAssets();
            if (!EditorSceneManager.SaveScene(avatar.scene, ScenePath))
                throw new IOException("Could not save the work scene.");
            if (!result.changedPaths.Contains(ScenePath))
                result.changedPaths.Add(ScenePath);
        }

        // Lines carrying face-tracking components, parameters or a creator patch warn that OSC-driven
        // gimmicks should be visually verified in Play Mode / VRChat when TraceAndOptimize is active.
        private static List<string> ObserveOptimizerWarnings(Data data)
        {
            var warnings = new List<string>();
            if (data == null || string.IsNullOrEmpty(data.avatarGlobalObjectId)) return warnings;
            var avatar = FindOwned(data.avatarGlobalObjectId);
            if (!HasOptimizerComponent(avatar) || data.faceTracking == null) return warnings;
            var ft = data.faceTracking;
            var ftActive = ft.manualReplayRequired
                || ft.state == "parameters_found"
                || (ft.parameters != null && ft.parameters.Count > 0)
                || (ft.components != null && ft.components.Any(name => !string.IsNullOrEmpty(name) && !name.StartsWith("nadena.dev.modular_avatar.", StringComparison.Ordinal)));
            if (ftActive) warnings.Add("osc_gimmick_possible");
            return warnings;
        }

        // Local before/after NDMF build measurement using NDMF's public AvatarProcessor.ManualProcessAvatar.
        // Temporary scene clones and Assets/ZZZ_GeneratedAssets outputs are cleaned up without saving
        // the scene; a measurement failure never aborts the TraceAndOptimize toggle.
        private static OptimizerReport MeasureOptimizer(Request request, string avatarGlobalObjectId)
        {
            var enabled = request.optimizer != null && request.optimizer.enabled;
            var report = new OptimizerReport
            {
                state = "failed",
                localMeasurementOnly = true,
                enabled = enabled,
                measuredAt = DateTime.UtcNow.ToString("o"),
            };

            var avatar = string.IsNullOrEmpty(avatarGlobalObjectId) ? null : FindOwned(avatarGlobalObjectId);
            if (avatar == null)
            {
                report.detail = "The finished Avatar is not in the work scene.";
                return report;
            }

            var traceType = FindLoadedType(AvatarOptimizerTypeName);
            var processorType = FindLoadedType(NdmfAvatarProcessorTypeName);
            if (traceType == null || processorType == null)
            {
                report.detail = traceType == null
                    ? "Avatar Optimizer is not loaded in this project."
                    : "NDMF AvatarProcessor is not loaded in this project.";
                return report;
            }

            var manualProcess = processorType.GetMethods(BindingFlags.Public | BindingFlags.Static)
                .FirstOrDefault(method => method.Name == "ManualProcessAvatar"
                    && method.GetParameters().Length >= 1
                    && method.GetParameters().Length <= 2
                    && method.GetParameters()[0].ParameterType == typeof(GameObject));
            var cleanTemporary = processorType.GetMethod("CleanTemporaryAssets", BindingFlags.Public | BindingFlags.Static, null, Type.EmptyTypes, null);
            if (manualProcess == null)
            {
                report.detail = "NDMF AvatarProcessor.ManualProcessAvatar is unavailable in this version.";
                return report;
            }

            var hadGeneratedRoot = false;
            var existingSubfolders = new HashSet<string>(StringComparer.Ordinal);
            var existingAssetPaths = new HashSet<string>(StringComparer.Ordinal);
            var snapshotCaptured = false;

            GameObject inputWithout = null;
            GameObject inputWith = null;
            GameObject processedWithout = null;
            GameObject processedWith = null;
            try
            {
                hadGeneratedRoot = AssetDatabase.IsValidFolder(NdmfGeneratedAssetsRoot);
                if (hadGeneratedRoot)
                {
                    existingSubfolders = new HashSet<string>(
                        AssetDatabase.GetSubFolders(NdmfGeneratedAssetsRoot),
                        StringComparer.Ordinal);
                    existingAssetPaths = new HashSet<string>(
                        AssetDatabase.FindAssets("", new[] { NdmfGeneratedAssetsRoot })
                            .Select(AssetDatabase.GUIDToAssetPath)
                            .Where(path => !string.IsNullOrEmpty(path)),
                        StringComparer.Ordinal);
                }
                snapshotCaptured = true;

                var suffix = Guid.NewGuid().ToString("N").Substring(0, 12);
                inputWithout = UnityEngine.Object.Instantiate(avatar);
                inputWithout.name = "__vua_ndmf_before_" + suffix;
                UnpackAllPrefabs(inputWithout);
                foreach (var component in inputWithout.GetComponents(traceType))
                    UnityEngine.Object.DestroyImmediate(component);

                inputWith = UnityEngine.Object.Instantiate(avatar);
                inputWith.name = "__vua_ndmf_after_" + suffix;
                UnpackAllPrefabs(inputWith);
                if (inputWith.GetComponent(traceType) == null)
                    inputWith.AddComponent(traceType);

                object[] Args(GameObject target) => manualProcess.GetParameters().Length == 2
                    ? new object[] { target, null }
                    : new object[] { target };

                processedWithout = manualProcess.Invoke(null, Args(inputWithout)) as GameObject ?? inputWithout;
                processedWith = manualProcess.Invoke(null, Args(inputWith)) as GameObject ?? inputWith;

                report.before = ReadOptimizerMetrics(processedWithout);
                report.after = ReadOptimizerMetrics(processedWith);
                report.state = "measured";
                report.detail = "";
            }
            catch (Exception error)
            {
                report.state = "failed";
                report.before = new OptimizerMetrics();
                report.after = new OptimizerMetrics();
                report.detail = SdkText(Unwrap(error).Message);
            }
            finally
            {
                try
                {
                    if (processedWithout != null && processedWithout != inputWithout)
                        UnityEngine.Object.DestroyImmediate(processedWithout);
                    if (processedWith != null && processedWith != inputWith)
                        UnityEngine.Object.DestroyImmediate(processedWith);
                    if (inputWithout != null)
                        UnityEngine.Object.DestroyImmediate(inputWithout);
                    if (inputWith != null)
                        UnityEngine.Object.DestroyImmediate(inputWith);
                }
                catch (Exception) { }

                try { cleanTemporary?.Invoke(null, null); }
                catch (Exception) { }

                if (snapshotCaptured)
                {
                    try { CleanupGeneratedAssets(hadGeneratedRoot, existingSubfolders, existingAssetPaths); }
                    catch (Exception) { }
                }
            }

            return report;
        }

        private static OptimizerMetrics ReadOptimizerMetrics(GameObject processed)
        {
            var sdkReview = new SdkReview();
            try { ReadPerformance(processed, sdkReview); }
            catch (Exception) { sdkReview.stats.Clear(); }
            var platform = sdkReview.platform == "android" ? "android" : "pc";
            var footprint = ObserveFootprint(processed, platform);
            return new OptimizerMetrics
            {
                platform = platform,
                overall = sdkReview.overall ?? "",
                stats = sdkReview.stats,
                triangles = footprint.triangles,
                materialSlots = footprint.materialSlots,
                meshes = footprint.meshes,
                skinnedMeshes = footprint.skinnedMeshes,
                textureBytes = footprint.textureBytes,
                textureComplete = footprint.textureComplete,
                meshBytes = footprint.meshBytes,
                meshComplete = footprint.meshComplete,
                physBones = footprint.physBones,
                physBonesKnown = footprint.physBonesKnown,
            };
        }

        private static void UnpackAllPrefabs(GameObject root)
        {
            if (root == null) return;
            for (var guard = 0; guard < 256; guard++)
            {
                var instance = root.GetComponentsInChildren<Transform>(true)
                    .Select(transform => transform.gameObject)
                    .FirstOrDefault(PrefabUtility.IsOutermostPrefabInstanceRoot);
                if (instance == null) break;
                PrefabUtility.UnpackPrefabInstance(instance, PrefabUnpackMode.Completely, InteractionMode.AutomatedAction);
            }
        }

        private static void CleanupGeneratedAssets(
            bool hadGeneratedRoot,
            HashSet<string> existingSubfolders,
            HashSet<string> existingAssetPaths)
        {
            if (!AssetDatabase.IsValidFolder(NdmfGeneratedAssetsRoot)) return;
            if (!hadGeneratedRoot)
            {
                AssetDatabase.DeleteAsset(NdmfGeneratedAssetsRoot);
                return;
            }
            foreach (var subfolder in AssetDatabase.GetSubFolders(NdmfGeneratedAssetsRoot).ToArray())
            {
                if (!existingSubfolders.Contains(subfolder))
                    AssetDatabase.DeleteAsset(subfolder);
            }
            foreach (var guid in AssetDatabase.FindAssets("", new[] { NdmfGeneratedAssetsRoot }))
            {
                var path = AssetDatabase.GUIDToAssetPath(guid);
                if (!string.IsNullOrEmpty(path) && !existingAssetPaths.Contains(path) && path != NdmfGeneratedAssetsRoot)
                    AssetDatabase.DeleteAsset(path);
            }
        }
    }
}
