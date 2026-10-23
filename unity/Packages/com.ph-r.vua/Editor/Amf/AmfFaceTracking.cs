using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using UnityEditor;
using UnityEditor.Animations;
using UnityEngine;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        private const string FtMarkerPath = "ProjectSettings/VUA.FaceTracking.json";
        [Serializable] internal sealed class FtProfile
        {
            public string id; public int referenceCount; public List<string> matched = new List<string>();
        }
        [Serializable] internal sealed class FtMesh
        {
            public string renderer; public int blendShapeCount; public bool missingMesh;
            public List<FtProfile> profiles = new List<FtProfile>();
        }
        [Serializable] internal sealed class FtMenu
        {
            public string id; public string sourceId; public string scriptPath; public string menuPath;
        }
        [Serializable] internal sealed class FtFacts
        {
            public string schemaVersion = "0.1";
            public string evidence = "unity_source";
            public string observedAt; public string state = "no_avatar";
            public string runtime = "unknown"; public string builtParameterBudget = "unknown";
            public bool truncated; public bool manualReplayRequired;
            public bool expressionParametersObserved;
            public int sourceSyncedParameterBits;
            public List<FtMesh> meshes = new List<FtMesh>();
            public List<string> parameters = new List<string>();
            public List<string> components = new List<string>();
            public List<FtMenu> patchMenus = new List<FtMenu>();
        }
        // Detection vocabularies, not required shapes or quality scores. Keep names case-sensitive.
        // References and dated provenance: docs/research/face-tracking-2026-10.md.
        internal static readonly string[] Arkit52 = (
            "browDownLeft browDownRight browInnerUp browOuterUpLeft browOuterUpRight " +
            "cheekPuff cheekSquintLeft cheekSquintRight eyeBlinkLeft eyeBlinkRight " +
            "eyeLookDownLeft eyeLookDownRight eyeLookInLeft eyeLookInRight eyeLookOutLeft eyeLookOutRight " +
            "eyeLookUpLeft eyeLookUpRight eyeSquintLeft eyeSquintRight eyeWideLeft eyeWideRight " +
            "jawForward jawLeft jawOpen jawRight mouthClose mouthDimpleLeft mouthDimpleRight " +
            "mouthFrownLeft mouthFrownRight mouthFunnel mouthLeft mouthLowerDownLeft mouthLowerDownRight " +
            "mouthPressLeft mouthPressRight mouthPucker mouthRight mouthRollLower mouthRollUpper " +
            "mouthShrugLower mouthShrugUpper mouthSmileLeft mouthSmileRight mouthStretchLeft mouthStretchRight " +
            "mouthUpperUpLeft mouthUpperUpRight noseSneerLeft noseSneerRight tongueOut").Split(' ');
        internal static readonly string[] UnifiedCore = (
            "EyeClosedLeft EyeClosedRight EyeSquintLeft EyeSquintRight EyeWideLeft EyeWideRight " +
            "JawOpen JawLeft JawRight JawForward MouthClosed TongueOut").Split(' ');
        internal static readonly string[] SranipalCore = (
            "Eye_Left_Blink Eye_Right_Blink Eye_Left_Wide Eye_Right_Wide " +
            "Jaw_Open Jaw_Left Jaw_Right Jaw_Forward Mouth_Ape_Shape Mouth_Pout Tongue_LongStep1 Tongue_LongStep2").Split(' ');

        internal static List<FtProfile> MatchFtProfiles(IEnumerable<string> names)
        {
            var present = new HashSet<string>(names, StringComparer.Ordinal);
            return new[] { ("arkit52-v1", Arkit52), ("unified-core12-v1", UnifiedCore), ("sranipal-core12-v1", SranipalCore) }
                .Select(set => new FtProfile { id = set.Item1, referenceCount = set.Item2.Length,
                    matched = set.Item2.Where(present.Contains).ToList() }).ToList();
        }
        internal static bool FtParameterName(string name) => !string.IsNullOrEmpty(name) &&
            (name.Contains("/v2/") || name.StartsWith("FT/", StringComparison.Ordinal) ||
             name.StartsWith("VRCFT/", StringComparison.Ordinal) || name == "FacialExpressionsDisabled" ||
             name == "EyeTrackingActive" || name == "LipTrackingActive" ||
             Arkit52.Contains(name) || UnifiedCore.Contains(name) || SranipalCore.Contains(name));
        private static bool FtText(string value, int limit) =>
            value != null && value.Length <= limit && !value.Any(char.IsControl);

        private static FtFacts ObserveFaceTracking(Request request, string avatarId)
        {
            var facts = new FtFacts { observedAt = DateTime.UtcNow.ToString("o"),
                manualReplayRequired = File.Exists(FtMarkerPath) };
            var menus = DiscoverFtMenus(request);
            facts.patchMenus = menus.Take(64).ToList();
            facts.truncated = menus.Count > 64;
            var avatar = string.IsNullOrEmpty(avatarId) ? null : FindOwned(avatarId);
            if (avatar == null) return facts;
            facts.state = "observed";
            var renderers = avatar.GetComponentsInChildren<SkinnedMeshRenderer>(true);
            facts.truncated |= renderers.Length > 128;
            foreach (var renderer in renderers.Take(128))
            {
                var mesh = renderer.sharedMesh;
                var names = mesh == null ? Array.Empty<string>() :
                    Enumerable.Range(0, mesh.blendShapeCount).Select(mesh.GetBlendShapeName).ToArray();
                var path = AnimationUtility.CalculateTransformPath(renderer.transform, avatar.transform);
                if (!FtText(path, 1024) || names.Length > 65536) { facts.truncated = true; continue; }
                facts.meshes.Add(new FtMesh { renderer = path,
                    missingMesh = mesh == null, blendShapeCount = names.Length, profiles = MatchFtProfiles(names) });
                if (mesh == null) facts.state = "partial";
            }
            var components = avatar.GetComponentsInChildren<Component>(true);
            if (components.Any(c => c == null)) facts.state = "partial";
            var types = components.Where(c => c != null).Select(c => c.GetType().FullName)
                .Where(n => n != null && (n.StartsWith("nadena.dev.modular_avatar.", StringComparison.Ordinal) ||
                    n.StartsWith("VF.", StringComparison.Ordinal) || n.IndexOf("VRCFury", StringComparison.OrdinalIgnoreCase) >= 0 ||
                    n.StartsWith("Pawlygon.", StringComparison.Ordinal))).Distinct().OrderBy(n => n, StringComparer.Ordinal).ToArray();
            facts.components = types.Where(n => FtText(n, 256)).Take(64).ToList();
            facts.truncated |= types.Length != facts.components.Count;
            var parameters = new HashSet<string>(StringComparer.Ordinal);
            foreach (var animator in avatar.GetComponentsInChildren<Animator>(true))
                FtControllerParameters(animator.runtimeAnimatorController, parameters);
            foreach (var merge in avatar.GetComponentsInChildren<nadena.dev.modular_avatar.core.ModularAvatarMergeAnimator>(true))
                FtControllerParameters(merge.animator, parameters);
            var descriptorType = DescriptorType();
            var descriptor = descriptorType == null ? null : avatar.GetComponent(descriptorType);
            if (descriptor != null)
            {
                var serialized = new SerializedObject(descriptor);
                foreach (var field in new[] { "baseAnimationLayers", "specialAnimationLayers" })
                {
                    var layers = serialized.FindProperty(field);
                    if (layers == null || !layers.isArray) { facts.state = "partial"; continue; }
                    for (var i = 0; i < layers.arraySize; i++)
                    {
                        var controller = layers.GetArrayElementAtIndex(i).FindPropertyRelative("animatorController");
                        if (controller != null) FtControllerParameters(controller.objectReferenceValue as RuntimeAnimatorController, parameters);
                    }
                }
                var asset = serialized.FindProperty("expressionParameters")?.objectReferenceValue;
                var entries = asset == null ? null : new SerializedObject(asset).FindProperty("parameters");
                if (entries != null && entries.isArray)
                {
                    facts.expressionParametersObserved = true;
                    for (var i = 0; i < entries.arraySize; i++)
                    {
                        var item = entries.GetArrayElementAtIndex(i);
                        var name = item.FindPropertyRelative("name")?.stringValue;
                        if (string.IsNullOrEmpty(name)) continue;
                        parameters.Add(name);
                        var synced = item.FindPropertyRelative("networkSynced");
                        var type = item.FindPropertyRelative("valueType");
                        if (synced == null || type == null || type.enumValueIndex < 0 || type.enumValueIndex >= type.enumNames.Length)
                        { facts.expressionParametersObserved = false; facts.state = "partial"; continue; }
                        var kind = type.enumNames[type.enumValueIndex];
                        if (kind != "Bool" && kind != "Int" && kind != "Float")
                        { facts.expressionParametersObserved = false; facts.state = "partial"; continue; }
                        if (synced.boolValue) facts.sourceSyncedParameterBits += kind == "Bool" ? 1 : 8;
                    }
                    if (facts.sourceSyncedParameterBits > 65536)
                    { facts.expressionParametersObserved = false; facts.state = "partial"; }
                    if (!facts.expressionParametersObserved) facts.sourceSyncedParameterBits = 0;
                }
            }
            var recognized = parameters.Where(FtParameterName).OrderBy(n => n, StringComparer.Ordinal).ToArray();
            facts.parameters = recognized.Where(n => FtText(n, 256)).Take(1024).ToList();
            facts.truncated |= recognized.Length != facts.parameters.Count;
            if (facts.truncated) facts.state = "partial";
            return facts;
        }
        private static void FtControllerParameters(RuntimeAnimatorController controller, HashSet<string> names)
        {
            // Overrides do not add parameters. Never instantiate a controller or execute a behavior.
            if (controller is AnimatorOverrideController overrides) controller = overrides.runtimeAnimatorController;
            if (controller is AnimatorController concrete)
                foreach (var parameter in concrete.parameters) if (!string.IsNullOrEmpty(parameter.name)) names.Add(parameter.name);
        }
        private static List<FtMenu> DiscoverFtMenus(Request request)
        {
            // Count ALL registered non-validator methods before selecting source scripts. A colliding
            // built-in/package menu must not redirect a trusted source's menu identity.
            var all = TypeCache.GetMethodsWithAttribute<MenuItem>()
                .SelectMany(m => m.GetCustomAttributes(typeof(MenuItem), false).Cast<MenuItem>()
                    .Where(a => !a.validate).Select(a => (method: m, path: a.menuItem)))
                .Where(m => !string.IsNullOrEmpty(m.path)).ToArray();
            var unique = all.GroupBy(m => m.path, StringComparer.Ordinal).Where(g => g.Count() == 1)
                .Select(g => g.Single()).ToArray();
            var candidates = new List<FtMenu>();
            foreach (var source in request.sources)
            foreach (var path in source.assetPaths.Distinct(StringComparer.Ordinal).Where(p => p.EndsWith(".cs", StringComparison.OrdinalIgnoreCase)))
            {
                if (string.IsNullOrEmpty(source.sourceId) || !FtText(source.sourceId, 100) ||
                    !SafeAssetPath(path) || !FtText(path, 1024)) continue;
                var type = AssetDatabase.LoadAssetAtPath<MonoScript>(path)?.GetClass();
                if (type == null) continue;
                foreach (var menu in unique.Where(m => m.method.DeclaringType == type && m.method.IsStatic))
                {
                    // Creator Tools/ or custom top-level menus; native menu roots are not authority.
                    var root = menu.path.Split('/')[0];
                    if (!menu.path.Contains("/") || string.IsNullOrEmpty(root) ||
                        new[] { "File", "Edit", "Assets", "GameObject", "Component", "Window", "Help", "CONTEXT", "VRChat SDK" }.Contains(root) ||
                        menu.path.Split('/').Any(string.IsNullOrEmpty) || menu.path.Length > 400 ||
                        menu.path.Any(char.IsControl) || menu.path.IndexOfAny(new[] { '%', '#', '&' }) >= 0 ||
                        menu.path.Contains(" _")) continue;
                    var arguments = menu.method.GetParameters();
                    if (arguments.Length > 1 || (arguments.Length == 1 && arguments[0].ParameterType != typeof(MenuCommand))) continue;
                    using (var sha = SHA256.Create())
                    {
                        var identity = source.sourceId + "\n" + path + "\n" + menu.path;
                        candidates.Add(new FtMenu { id = string.Concat(sha.ComputeHash(Encoding.UTF8.GetBytes(identity)).Select(b => b.ToString("x2"))),
                            sourceId = source.sourceId, scriptPath = path, menuPath = menu.path });
                    }
                }
            }
            // Shared script paths imported by two materials have ambiguous authority.
            return candidates.GroupBy(m => m.menuPath, StringComparer.Ordinal).Where(g => g.Count() == 1)
                .Select(g => g.Single()).OrderBy(m => m.id, StringComparer.Ordinal).ToList();
        }
    }
}
