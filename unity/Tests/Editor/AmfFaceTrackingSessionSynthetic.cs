// Isolated project only. This harness simulates the player's completion after a domain reload.
using System;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using nadena.dev.modular_avatar.core;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        public static void PrepareFaceTrackingSessionSynthetic()
        {
            EditorSceneManager.OpenScene(ScenePath);
            var avatar = UnityEngine.SceneManagement.SceneManager.GetActiveScene().GetRootGameObjects().Single(IsAvatar);
            var previousOutfit = avatar.transform.Find("Synthetic Outfit");
            if (previousOutfit != null) UnityEngine.Object.DestroyImmediate(previousOutfit.gameObject);
            Directory.CreateDirectory("Assets/Synthetic");
            AssetDatabase.Refresh();
            var prefab = PrefabUtility.SaveAsPrefabAsset(avatar, "Assets/Synthetic/Avatar.prefab");
            UnityEngine.Object.DestroyImmediate(avatar);
            var instance = (GameObject)PrefabUtility.InstantiatePrefab(prefab);
            var outfitSource = new GameObject("Synthetic Outfit");
            outfitSource.AddComponent<ModularAvatarMergeArmature>();
            var outfitPrefab = PrefabUtility.SaveAsPrefabAsset(outfitSource, "Assets/Synthetic/Outfit.prefab");
            UnityEngine.Object.DestroyImmediate(outfitSource);
            var outfit = (GameObject)PrefabUtility.InstantiatePrefab(outfitPrefab);
            outfit.name = "Synthetic Outfit";
            outfit.transform.SetParent(instance.transform, false);
            var merge = outfit.GetComponent<ModularAvatarMergeArmature>();
            merge.mergeTarget = new AvatarObjectReference(instance);
            PrefabUtility.RecordPrefabInstancePropertyModifications(merge);
            EditorSceneManager.SaveScene(instance.scene, ScenePath);
            var avatarId = GlobalObjectId.GetGlobalObjectIdSlow(instance).ToString();
            var owners = new Owners { runId = "ft-synthetic" };
            owners.objects.Add(new Owner { key = "avatar", globalObjectId = avatarId,
                prefabGuid = AssetDatabase.AssetPathToGUID("Assets/Synthetic/Avatar.prefab") });
            owners.objects.Add(new Owner { key = "outfit-" + ShortHash("synthetic-outfit"),
                globalObjectId = GlobalObjectId.GetGlobalObjectIdSlow(outfit).ToString(),
                prefabGuid = AssetDatabase.AssetPathToGUID("Assets/Synthetic/Outfit.prefab") });
            // This fixture prepares a new line; discard the previous fixture's imported TextAsset
            // through AssetDatabase before replacing it, so repeated local harness runs are safe.
            if (File.Exists(OwnershipPath)) AssetDatabase.DeleteAsset(OwnershipPath);
            AtomicWrite(OwnershipPath, JsonUtility.ToJson(owners));
            var request = new Request { schemaVersion = "0.1", runId = owners.runId, sources = new[] {
                new Source { sourceId = "synthetic", role = "avatar", assetPaths = new[] {
                    "Assets/Synthetic/Avatar.prefab", "Assets/Editor/FtSyntheticMenu.cs" } }
                , new Source { sourceId = "synthetic-outfit", role = "outfit", assetPaths = new[] { "Assets/Synthetic/Outfit.prefab" } }
            } };
            var menu = DiscoverFtMenus(request).Single(m => m.menuPath == "Tools/VUA Synthetic/Face tracking");
            request.faceTrackingPatch = new FtPatch { menuId = menu.id, avatarGlobalObjectId = avatarId, confirmTrustedCode = true };
            Directory.CreateDirectory(".vua/bridge");
            File.WriteAllText(".vua/amf-run.txt", owners.runId);
            PlayerSettings.companyName = WorkCompany; PlayerSettings.productName = WorkProduct;
            AssetDatabase.SaveAssets();
            var bytes = Encoding.UTF8.GetBytes(JsonUtility.ToJson(request));
            File.WriteAllBytes(".vua/bridge/ft-synthetic.amf-plan.json", bytes);
            string hash;
            using (var sha = SHA256.Create()) hash = string.Concat(sha.ComputeHash(bytes).Select(b => b.ToString("x2")));
            var command = new BridgeCommand { schemaVersion = 5, commandId = "ft-synthetic",
                projectId = owners.runId, operation = "amf_ft_patch", dryRun = false,
                expectedProjectFingerprint = ProjectFingerprint.Compute(),
                payload = new BridgePayload { planRef = ".vua/bridge/ft-synthetic.amf-plan.json", planHash = hash } };
            AtomicWrite(".vua/bridge/ft-synthetic.request.json", JsonUtility.ToJson(command));
            File.WriteAllText("ft-interactive-mode.txt", "complete_after_reload");
            Debug.Log("VUA_FT_SYNTHETIC_SESSION_PREPARED");
        }
        [InitializeOnLoadMethod]
        private static void CompleteSyntheticSessionAfterReload()
        {
            if (Application.isBatchMode || !File.Exists("ft-interactive-mode.txt")) return;
            // A fresh process has no SessionState; only the post-dispatch domain reload reaches this.
            if (string.IsNullOrEmpty(SessionState.GetString(FtSessionKey, ""))) return;
            EditorApplication.delayCall += () => {
                try
                {
                    var invocations = File.ReadAllLines("ft-menu-invoked.txt");
                    if (invocations.Length != 1) throw new Exception("Creator menu was dispatched more than once.");
                    var session = JsonUtility.FromJson<FtSession>(SessionState.GetString(FtSessionKey, ""));
                    File.WriteAllText("ft-domain-reload-checked.txt", "one dispatch; completion state survived reload");
                    if (File.ReadAllText("ft-interactive-mode.txt").Trim() == "cancel_after_reload")
                    {
                        typeof(FtSessionWindow).GetMethod("Cancel", BindingFlags.Static | BindingFlags.NonPublic)
                            .Invoke(null, null);
                        return;
                    }
                    var window = EditorWindow.GetWindow<FtSessionWindow>();
                    typeof(FtSessionWindow).GetMethod("Complete", BindingFlags.Instance | BindingFlags.NonPublic)
                        .Invoke(window, new object[] { session });
                    // Complete normally requests exit. A failed completion leaves its diagnostic on the window.
                    if (!string.IsNullOrEmpty(SessionState.GetString(FtSessionKey, "")))
                        throw new Exception("Synthetic completion did not clear the session: " +
                            typeof(FtSessionWindow).GetField("failure", BindingFlags.Instance | BindingFlags.NonPublic).GetValue(window));
                }
                catch (Exception error)
                {
                    Debug.LogError("VUA_FT_SYNTHETIC_SESSION_FAILED: " + error);
                    EditorApplication.Exit(1);
                }
            };
        }
    }
}
