// Copy into the Bridge's Editor/Amf directory ONLY in an isolated validation project.
// No downloaded model, hardware, network or creator patch is used by these checks.
using System;
using System.Linq;
using UnityEditor;
using UnityEngine;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        public static void RunParameterBudgetSyntheticTests()
        {
            void Require(bool value, string detail) { if (!value) throw new Exception(detail); }
            var avatar = new GameObject("Budget Avatar");
            var descriptor = avatar.AddComponent(DescriptorType());
            var parameterType = AppDomain.CurrentDomain.GetAssemblies()
                .Select(a => a.GetType("VRC.SDK3.Avatars.ScriptableObjects.VRCExpressionParameters", false)).First(t => t != null);
            var parameterAsset = ScriptableObject.CreateInstance(parameterType);
            var serializedParameters = new SerializedObject(parameterAsset);
            var entries = serializedParameters.FindProperty("parameters");
            entries.arraySize = 4;
            var parameterNames = new[] { "VRCSwitch", "OutfitMode", "LocalOnly", "FaceShape" };
            var kinds = new[] { "Bool", "Int", "Float", "Int" };
            for (var i = 0; i < 4; i++)
            {
                var entry = entries.GetArrayElementAtIndex(i);
                entry.FindPropertyRelative("name").stringValue = parameterNames[i];
                var kind = entry.FindPropertyRelative("valueType");
                kind.enumValueIndex = Array.IndexOf(kind.enumNames, kinds[i]);
                entry.FindPropertyRelative("networkSynced").boolValue = i != 2;
            }
            serializedParameters.ApplyModifiedPropertiesWithoutUndo();
            var serializedDescriptor = new SerializedObject(descriptor);
            serializedDescriptor.FindProperty("expressionParameters").objectReferenceValue = parameterAsset;
            serializedDescriptor.ApplyModifiedPropertiesWithoutUndo();
            // An embedded outfit inside the Avatar prefab declares its parameters through MA.
            var embedded = new GameObject("Embedded Outfit");
            embedded.transform.parent = avatar.transform;
            var embeddedParameters = embedded.AddComponent<nadena.dev.modular_avatar.core.ModularAvatarParameters>();
            embeddedParameters.parameters = new System.Collections.Generic.List<nadena.dev.modular_avatar.core.ParameterConfig>
            {
                new nadena.dev.modular_avatar.core.ParameterConfig { nameOrPrefix = "OutfitMode", syncType = nadena.dev.modular_avatar.core.ParameterSyncType.Bool },
                new nadena.dev.modular_avatar.core.ParameterConfig { nameOrPrefix = "HiddenToggle", syncType = nadena.dev.modular_avatar.core.ParameterSyncType.Bool, localOnly = true },
                new nadena.dev.modular_avatar.core.ParameterConfig { nameOrPrefix = "LocalOnly", syncType = nadena.dev.modular_avatar.core.ParameterSyncType.Int },
                new nadena.dev.modular_avatar.core.ParameterConfig { nameOrPrefix = "InnerToggle", syncType = nadena.dev.modular_avatar.core.ParameterSyncType.Bool, internalParameter = true },
                new nadena.dev.modular_avatar.core.ParameterConfig { nameOrPrefix = "OldName", remapTo = "FaceShape", syncType = nadena.dev.modular_avatar.core.ParameterSyncType.Int },
                new nadena.dev.modular_avatar.core.ParameterConfig { nameOrPrefix = "Outfit/", isPrefix = true, syncType = nadena.dev.modular_avatar.core.ParameterSyncType.Bool },
                new nadena.dev.modular_avatar.core.ParameterConfig { nameOrPrefix = "DriverOnly", syncType = nadena.dev.modular_avatar.core.ParameterSyncType.NotSynced },
            };
            var budget = ObserveParameterBudget(avatar, true);
            Require(budget.expressionParametersObserved, "The Avatar's asset must be observed when it is readable.");
            Require(budget.expressionParameters.Count == 3 && budget.expressionParameters[0].name == "VRCSwitch"
                && budget.expressionParameters[0].bits == 1 && budget.expressionParameters[1].bits == 8
                && budget.expressionParameters[2].name == "FaceShape", "Synced asset entries count Bool 1 and Int/Float 8; unsynced stays out.");
            Require(budget.maComponents.Count == 1 && budget.maComponents[0].path == "Embedded Outfit",
                "Embedded MA Parameters report their object path as the source.");
            var embeddedEntries = budget.maComponents[0].parameters;
            Require(embeddedEntries.Count == 4, "localOnly, NotSynced and prefix configs carry no synced entry.");
            Require(embeddedEntries[0].name == "OutfitMode" && embeddedEntries[0].bits == 1 && !embeddedEntries[0].unique,
                "An ordinary synced Bool config counts one bit.");
            Require(embeddedEntries[1].name == "LocalOnly" && embeddedEntries[1].bits == 8,
                "A synced config whose name matches an unsynced asset entry still reports its own bits.");
            Require(embeddedEntries[2].name == "InnerToggle" && embeddedEntries[2].unique,
                "Internal parameters keep their own identity instead of merging by name.");
            Require(embeddedEntries[3].name == "FaceShape" && embeddedEntries[3].bits == 8,
                "remapTo owns the merged name.");
            Require(budget.unresolvedPrefixConfigs == 1 && !budget.truncated, "Prefix configs are reported as unresolved, not counted.");
            var outfit = new GameObject("Budget Outfit");
            outfit.AddComponent<nadena.dev.modular_avatar.core.ModularAvatarParameters>().parameters =
                new System.Collections.Generic.List<nadena.dev.modular_avatar.core.ParameterConfig>
                { new nadena.dev.modular_avatar.core.ParameterConfig { nameOrPrefix = "DressOn", syncType = nadena.dev.modular_avatar.core.ParameterSyncType.Bool } };
            var outfitBudget = ObserveParameterBudget(outfit, false);
            Require(outfitBudget.expressionParametersObserved && outfitBudget.expressionParameters.Count == 0
                && outfitBudget.maComponents.Count == 1 && outfitBudget.maComponents[0].parameters[0].name == "DressOn",
                "An outfit has no asset of its own and reports its MA Parameters by path.");
            var bare = ObserveParameterBudget(new GameObject("Bare"), false);
            Require(bare.expressionParametersObserved && bare.maComponents.Count == 0, "A component-free outfit reports an empty budget.");
            // The wire shape must stay camelCase for the schema and the Rust reader.
            var wire = JsonUtility.ToJson(new Candidate { sourceId = "s", path = "Assets/Budget.prefab", name = "Budget",
                isAvatar = true, parameterBudget = budget });
            Require(wire.Contains("\"parameterBudget\":") && wire.Contains("\"expressionParametersObserved\":true")
                && wire.Contains("\"unresolvedPrefixConfigs\":1") && wire.Contains("\"maComponents\":[{\"path\":\"Embedded Outfit\""),
                "The serialized candidate names the budget fields exactly as the contract requires.");
            // An Avatar without an asset gets the SDK default at build time, which inspection cannot read.
            var unassigned = ObserveParameterBudget(new GameObject("No Asset Avatar"), true);
            Require(!unassigned.expressionParametersObserved && unassigned.expressionParameters.Count == 0,
                "A missing Expression Parameters asset stays unobserved instead of reading as zero.");
            entries.arraySize = 300;
            for (var i = 0; i < 300; i++)
            {
                var entry = entries.GetArrayElementAtIndex(i);
                entry.FindPropertyRelative("name").stringValue = "Overflow" + i;
                var kind = entry.FindPropertyRelative("valueType");
                kind.enumValueIndex = Array.IndexOf(kind.enumNames, "Bool");
                entry.FindPropertyRelative("networkSynced").boolValue = true;
            }
            serializedParameters.ApplyModifiedPropertiesWithoutUndo();
            var overflow = ObserveParameterBudget(avatar, true);
            Require(overflow.truncated && !overflow.expressionParametersObserved && overflow.expressionParameters.Count == 0,
                "A source asset beyond the contract bound must not pose as a measured budget.");
            Debug.Log("VUA_PARAMETER_BUDGET_CHECKS_PASSED");
        }
    }
}
