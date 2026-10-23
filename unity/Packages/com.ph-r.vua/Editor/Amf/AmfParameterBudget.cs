using System;
using System.Collections.Generic;
using System.Linq;
using UnityEditor;
using UnityEngine;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        // Source evidence for VRChat's 256 synced expression-parameter bits. Modular Avatar merges
        // parameters into the Avatar's asset only at build time, so every list here is an estimate
        // of that merge, not the built cost. See docs/protocols/unity-bridge-v5.md.
        [Serializable] internal sealed class ParameterEntry { public string name; public int bits; public bool unique; }
        [Serializable] internal sealed class ParameterComponent { public string path; public List<ParameterEntry> parameters = new List<ParameterEntry>(); }
        [Serializable] internal sealed class ParameterBudget
        {
            // True when the candidate's own Expression Parameters are fully known: an outfit has
            // none by definition, an Avatar needs a readable asset. A missing Avatar asset means
            // the build substitutes the SDK default, whose cost this inspection cannot read.
            public bool expressionParametersObserved;
            public List<ParameterEntry> expressionParameters = new List<ParameterEntry>();
            public List<ParameterComponent> maComponents = new List<ParameterComponent>();
            // Prefix configs match parameters that exist only at build time; they carry no entry.
            public int unresolvedPrefixConfigs;
            public bool truncated;
        }
        private const int MaxParameterEntries = 256;
        private const int MaxParameterComponents = 64;
        private static bool ParameterText(string value, int limit) =>
            !string.IsNullOrEmpty(value) && value.Length <= limit && !value.Any(char.IsControl);
        // An empty path is the prefab's own root object, which commonly carries MA Parameters.
        private static bool ParameterPath(string value) =>
            value.Length <= 1024 && !value.Any(char.IsControl);

        internal static ParameterBudget ObserveParameterBudget(GameObject prefab, bool isAvatar)
        {
            var budget = new ParameterBudget { expressionParametersObserved = !isAvatar };
            if (isAvatar) ReadExpressionParameters(prefab, budget);
            CollectMaParameters(prefab, budget);
            return budget;
        }
        private static void ReadExpressionParameters(GameObject prefab, ParameterBudget budget)
        {
            var descriptorType = DescriptorType();
            var descriptor = descriptorType == null ? null : prefab.GetComponent(descriptorType);
            var asset = descriptor == null ? null : new SerializedObject(descriptor)
                .FindProperty("expressionParameters")?.objectReferenceValue;
            if (asset == null) return; // Unobserved: the build substitutes a default asset.
            var entries = new SerializedObject(asset).FindProperty("parameters");
            if (entries == null || !entries.isArray) return;
            budget.expressionParametersObserved = true;
            for (var i = 0; i < entries.arraySize; i++)
            {
                var item = entries.GetArrayElementAtIndex(i);
                var name = item.FindPropertyRelative("name")?.stringValue;
                var synced = item.FindPropertyRelative("networkSynced");
                var type = item.FindPropertyRelative("valueType");
                if (!ParameterText(name, 256) || synced == null || type == null || type.enumValueIndex < 0 || type.enumValueIndex >= type.enumNames.Length)
                { budget.expressionParametersObserved = false; break; }
                var kind = type.enumNames[type.enumValueIndex];
                if (kind != "Bool" && kind != "Int" && kind != "Float")
                { budget.expressionParametersObserved = false; break; }
                if (!synced.boolValue) continue; // Local parameters cost no synced bits.
                if (budget.expressionParameters.Count >= MaxParameterEntries)
                { budget.expressionParametersObserved = false; budget.truncated = true; break; }
                budget.expressionParameters.Add(new ParameterEntry { name = name, bits = kind == "Bool" ? 1 : 8, unique = false });
            }
            if (!budget.expressionParametersObserved) budget.expressionParameters.Clear();
        }
        private static void CollectMaParameters(GameObject prefab, ParameterBudget budget)
        {
            var total = 0;
            foreach (var component in prefab.GetComponentsInChildren<nadena.dev.modular_avatar.core.ModularAvatarParameters>(true))
            {
                if (budget.maComponents.Count >= MaxParameterComponents) { budget.truncated = true; return; }
                var path = AnimationUtility.CalculateTransformPath(component.transform, prefab.transform);
                var usable = ParameterPath(path);
                if (!usable) budget.truncated = true;
                var record = new ParameterComponent { path = path };
                foreach (var config in component.parameters ?? new List<nadena.dev.modular_avatar.core.ParameterConfig>())
                {
                    if (!ParameterText(config.nameOrPrefix, 256)) { budget.truncated = true; continue; }
                    if (config.isPrefix) { budget.unresolvedPrefixConfigs++; continue; }
                    if (config.syncType == nadena.dev.modular_avatar.core.ParameterSyncType.NotSynced) continue;
                    if (config.localOnly) continue; // The merged parameter keeps networkSynced off.
                    if (total >= MaxParameterEntries) { budget.truncated = true; return; }
                    // remapTo renames the parameter at build time, so it owns the merged name.
                    var name = string.IsNullOrEmpty(config.remapTo) ? config.nameOrPrefix : config.remapTo;
                    if (!ParameterText(name, 256)) { budget.truncated = true; continue; }
                    record.parameters.Add(new ParameterEntry {
                        name = name,
                        bits = config.syncType == nadena.dev.modular_avatar.core.ParameterSyncType.Bool ? 1 : 8,
                        unique = config.internalParameter, // MA gives internal parameters unique names.
                    });
                    total++;
                }
                if (usable) budget.maComponents.Add(record);
            }
        }
    }
}
