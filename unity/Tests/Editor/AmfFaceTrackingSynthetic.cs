// Copy into the Bridge's Editor/Amf directory ONLY in an isolated validation project.
// No downloaded model, hardware, network or creator patch is used by these checks.
using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        public static void RunFaceTrackingSyntheticTests()
        {
            void Require(bool value, string detail) { if (!value) throw new Exception(detail); }
            Directory.CreateDirectory("Assets/VUA");
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var avatar = new GameObject("Synthetic Avatar");
            var descriptor = avatar.AddComponent(DescriptorType());
            var parameterType = AppDomain.CurrentDomain.GetAssemblies()
                .Select(a => a.GetType("VRC.SDK3.Avatars.ScriptableObjects.VRCExpressionParameters", false)).First(t => t != null);
            var parameterAsset = ScriptableObject.CreateInstance(parameterType);
            var serializedParameters = new SerializedObject(parameterAsset);
            var entries = serializedParameters.FindProperty("parameters");
            entries.arraySize = 4;
            var parameterNames = new[] { "FT/Enabled", "Custom/v2/JawOpen", "UserChoice", "Custom/v2/EyeClosedLeft" };
            var kinds = new[] { "Bool", "Float", "Int", "Float" };
            for (var i = 0; i < 4; i++)
            {
                var entry = entries.GetArrayElementAtIndex(i);
                entry.FindPropertyRelative("name").stringValue = parameterNames[i];
                var kind = entry.FindPropertyRelative("valueType");
                kind.enumValueIndex = Array.IndexOf(kind.enumNames, kinds[i]);
                entry.FindPropertyRelative("networkSynced").boolValue = i != 3;
            }
            serializedParameters.ApplyModifiedPropertiesWithoutUndo();
            AssetDatabase.CreateAsset(parameterAsset, "Assets/VUA/SyntheticParameters.asset");
            var serializedDescriptor = new SerializedObject(descriptor);
            serializedDescriptor.FindProperty("expressionParameters").objectReferenceValue = parameterAsset;
            serializedDescriptor.ApplyModifiedPropertiesWithoutUndo();
            const string controllerPath = "Assets/VUA/SyntheticFtController.controller";
            if (File.Exists(controllerPath)) AssetDatabase.DeleteAsset(controllerPath);
            var controller = UnityEditor.Animations.AnimatorController.CreateAnimatorControllerAtPath(controllerPath);
            controller.AddParameter("Custom/v2/MouthSmileLeft", AnimatorControllerParameterType.Float);
            avatar.AddComponent<nadena.dev.modular_avatar.core.ModularAvatarMergeAnimator>().animator = controller;
            var body = new GameObject("Body");
            body.transform.parent = avatar.transform;
            var renderer = body.AddComponent<SkinnedMeshRenderer>();
            var mesh = new Mesh { vertices = new[] { Vector3.zero, Vector3.up, Vector3.right }, triangles = new[] { 0, 1, 2 } };
            foreach (var name in Arkit52.Concat(new[] { "EyeClosedLeft", "Eye_Right_Blink", "eyeblinkleft" }))
                mesh.AddBlendShapeFrame(name, 100, new Vector3[3], new Vector3[3], new Vector3[3]);
            AssetDatabase.CreateAsset(mesh, "Assets/VUA/SyntheticMesh.asset");
            renderer.sharedMesh = mesh;
            // Different renderers must never be unioned into apparent full coverage.
            var other = new GameObject("Other"); other.transform.parent = avatar.transform;
            var small = new Mesh { vertices = mesh.vertices, triangles = mesh.triangles };
            small.AddBlendShapeFrame("JawOpen", 100, new Vector3[3], new Vector3[3], new Vector3[3]);
            AssetDatabase.CreateAsset(small, "Assets/VUA/SyntheticOther.asset");
            other.AddComponent<SkinnedMeshRenderer>().sharedMesh = small;
            EditorSceneManager.SaveScene(avatar.scene, ScenePath);
            var avatarId = GlobalObjectId.GetGlobalObjectIdSlow(avatar).ToString();
            var request = new Request { schemaVersion = "0.1", runId = "ft-synthetic", sources = new[] {
                new Source { sourceId = "synthetic", assetPaths = new[] { "Assets/Editor/FtSyntheticMenu.cs" } }
            } };
            var validatorBefore = File.Exists("ft-validator-count.txt") ? File.ReadAllText("ft-validator-count.txt") : "";
            var facts = ObserveFaceTracking(request, avatarId);
            Require(facts.patchMenus.Count == 2 && facts.patchMenus.Any(m => m.menuPath == "Tools/VUA Synthetic/Face tracking") &&
                facts.patchMenus.Any(m => m.menuPath == "FT Synthetic/Apply"), "Imported Tools and custom-root menu discovery failed.");
            Require(facts.expressionParametersObserved && facts.sourceSyncedParameterBits == 17, "Source budget must count Bool/Float/Int and exclude the unsynced parameter.");
            Require(facts.parameters.Count == 4 && facts.parameters.Contains("Custom/v2/MouthSmileLeft") &&
                !facts.parameters.Contains("UserChoice"), "FT hints must include MA's attached controller without claiming its built budget.");
            var ambiguous = new Request { sources = new[] { request.sources[0], new Source { sourceId = "other", assetPaths = request.sources[0].assetPaths } } };
            Require(DiscoverFtMenus(ambiguous).Count == 0, "Shared source ownership must not authorize a menu.");
            Require(facts.meshes.Count == 2, "Expected two separately observed meshes.");
            Require(facts.meshes[0].profiles[0].matched.Count == 52, "ARKit coverage must match 52 exact names.");
            Require(facts.meshes[1].profiles[0].matched.Count == 0, "Do not union renderers or fold case.");
            Require(facts.meshes[1].profiles[1].matched.SequenceEqual(new[] { "JawOpen" }), "UE core detection failed.");
            Require(facts.runtime == "unknown" && facts.builtParameterBudget == "unknown", "Source facts cannot prove runtime or built cost.");
            body.name = new string('x', 1025);
            var bounded = ObserveFaceTracking(request, avatarId);
            Require(bounded.truncated && bounded.state == "partial" && bounded.meshes.Count == 1,
                "An oversized evidence path must remain partial instead of breaking an existing inspection.");
            body.name = "Body";
            Require(MatchFtProfiles(new[] { "eyeblinkleft" })[0].matched.Count == 0, "Case-sensitive shape names are required.");
            Require(FtParameterName("Custom/v2/JawOpen") && !FtParameterName("RandomMouth"), "Parameter hints must honor the v2 namespace.");
            Require(!File.Exists("ft-menu-invoked.txt") && validatorBefore == (File.Exists("ft-validator-count.txt") ? File.ReadAllText("ft-validator-count.txt") : ""), "Inspection invoked patch code or validators.");
            // A versioned patch command must reject batch mode before any menu dispatch.
            var result = new Result();
            var rejected = false;
            try { Process(new BridgeCommand { schemaVersion = 5, commandId = "synthetic", projectId = "ft-synthetic", operation = "amf_ft_patch" }, result); }
            catch (InvalidOperationException error) { rejected = error.Message.Contains("interactive"); }
            Require(rejected && !File.Exists("ft-menu-invoked.txt"), "Batch execution must reject the creator menu.");
            File.WriteAllText("ft-synthetic-checks.json", "{\"status\":\"passed\",\"shapeNames\":55,\"meshes\":2,\"sourceSyncedBits\":17,\"batchPatch\":\"refused\",\"runtime\":\"not_tested\"}");
            Debug.Log("VUA_FT_SYNTHETIC_CHECKS_PASSED");
        }
    }
}
