using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using nadena.dev.modular_avatar.core;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        private const int RemovalObjectLimit = 8192;
        [Serializable] internal sealed class RemovalRoot { public string kind; public string globalObjectId; }
        [Serializable] internal sealed class RemovalObject
        {
            public string globalObjectId; public string path; public string typeName; public bool manual;
        }
        [Serializable] internal sealed class OutfitRemoval
        {
            public string sourceId; public string avatarGlobalObjectId;
            public List<RemovalRoot> roots = new List<RemovalRoot>();
            public List<RemovalObject> objects = new List<RemovalObject>();
            public string blockedReason = "";
        }
        [Serializable] internal sealed class RemovalApplied
        {
            public string sourceId; public string avatarGlobalObjectId; public List<string> deletedObjectIds;
        }

        // Only ID resolution grants authority. Paths below are labels for the player's review.
        private static UnityEngine.Object ResolveRemovalId(string id)
        {
            return GlobalObjectId.TryParse(id, out var parsed) ? GlobalObjectId.GlobalObjectIdentifierToObjectSlow(parsed) : null;
        }
        private static string RemovalId(UnityEngine.Object obj) => GlobalObjectId.GetGlobalObjectIdSlow(obj).ToString();

        internal static bool SceneAddition(GameObject root, GameObject obj, Component component, bool menu)
        {
            if (menu)
            {
                if (obj != root) return true;
                return component != null && !(component is Transform) && !(component is ModularAvatarMenuInstaller)
                    && !(component is ModularAvatarMenuItem) && !(component is ModularAvatarObjectToggle);
            }
            var current = obj.transform;
            while (current != null && current != root.transform)
            {
                if (PrefabUtility.GetCorrespondingObjectFromSource(current.gameObject) == null
                    || PrefabUtility.IsAddedGameObjectOverride(current.gameObject)) return true;
                current = current.parent;
            }
            // Conservatively warn about added components, including bindings whose provenance
            // an old ownership record cannot distinguish. This warning never expands authority.
            return component != null && PrefabUtility.GetCorrespondingObjectFromSource(component) == null;
        }

        private static OutfitRemoval RemovalInventory(Owners owners, string sourceId, GameObject avatar)
        {
            var inventory = new OutfitRemoval { sourceId = sourceId, avatarGlobalObjectId = RemovalId(avatar) };
            var keys = new[] { ScopedKey(owners, "outfit-" + ShortHash(sourceId)), ScopedKey(owners, "menu-" + ShortHash(sourceId)) };
            var records = keys.Select(key => owners.objects.SingleOrDefault(o => o.key == key)).Where(o => o != null).ToList();
            if (!records.Any(o => o.key == keys[0])) { inventory.blockedReason = "ownership_invalid"; return inventory; }
            var roots = new List<GameObject>();
            foreach (var record in records)
            {
                var obj = ResolveRemovalId(record.globalObjectId) as GameObject;
                if (obj == null || obj == avatar || !obj.transform.IsChildOf(avatar.transform) || obj.scene != avatar.scene
                    || roots.Any(r => r == obj || obj.transform.IsChildOf(r.transform) || r.transform.IsChildOf(obj.transform)))
                { inventory.blockedReason = "target_unverified"; return inventory; }
                roots.Add(obj);
                inventory.roots.Add(new RemovalRoot { kind = record.key == keys[0] ? "outfit" : "menu", globalObjectId = record.globalObjectId });
            }
            // Another garment or another ownership scope may have been moved under this one.
            // Never let recursive destruction consume it.
            foreach (var other in owners.objects.Where(o => !records.Contains(o)))
            {
                var obj = ResolveRemovalId(other.globalObjectId) as GameObject;
                if (obj == null || records.Any(r => r.globalObjectId == other.globalObjectId))
                { inventory.blockedReason = "ownership_invalid"; return inventory; }
                if (roots.Any(r => obj.transform.IsChildOf(r.transform)))
                { inventory.blockedReason = "contains_owned_object"; return inventory; }
            }
            var ids = new HashSet<string>(StringComparer.Ordinal);
            for (var i = 0; i < roots.Count; i++)
            {
                var root = roots[i]; var menu = inventory.roots[i].kind == "menu";
                foreach (var transform in root.GetComponentsInChildren<Transform>(true))
                {
                    var obj = transform.gameObject;
                    var path = TransformPath(avatar.transform, transform);
                    var members = new List<UnityEngine.Object> { obj };
                    members.AddRange(obj.GetComponents<Component>().Where(c => c != null));
                    if (obj.GetComponents<Component>().Any(c => c == null)) { inventory.blockedReason = "target_unverified"; return inventory; }
                    foreach (var member in members)
                    {
                        var id = RemovalId(member); var type = member.GetType().FullName;
                        if (id.Length > 256 || path.Length > 4096 || type.Length > 256 || !ids.Add(id) || ResolveRemovalId(id) != member)
                        { inventory.blockedReason = "identity_invalid"; return inventory; }
                        if (inventory.objects.Count >= RemovalObjectLimit) { inventory.blockedReason = "inventory_limit"; return inventory; }
                        inventory.objects.Add(new RemovalObject { globalObjectId = id, path = path, typeName = type,
                            manual = SceneAddition(root, obj, member as Component, menu) });
                    }
                }
            }
            inventory.objects = inventory.objects.OrderBy(o => o.globalObjectId, StringComparer.Ordinal).ToList();
            return inventory;
        }

        private static List<OutfitRemoval> ObserveOutfitRemoval(Request request, Data data)
        {
            var inventories = new List<OutfitRemoval>();
            if (data.observedSelection == null || !File.Exists(OwnershipPath)) return inventories;
            var owners = JsonUtility.FromJson<Owners>(File.ReadAllText(OwnershipPath));
            var avatar = FindOwned(data.avatarGlobalObjectId);
            if (owners == null || owners.runId != request.runId || owners.objects == null || avatar == null)
                throw new InvalidOperationException("Removal ownership is unavailable.");
            foreach (var outfit in data.observedSelection.outfits) inventories.Add(RemovalInventory(owners, outfit.sourceId, avatar));
            return inventories;
        }

        private static void RemoveOutfit(Request request, Result result)
        {
            var reviewed = request.outfitRemoval;
            if (reviewed == null || string.IsNullOrEmpty(reviewed.sourceId) || string.IsNullOrEmpty(reviewed.avatarGlobalObjectId)
                || !string.IsNullOrEmpty(reviewed.blockedReason)) throw new InvalidOperationException("No reviewed removal inventory.");
            if (File.Exists("ProjectSettings/VUA.FaceTracking.json")) throw new InvalidOperationException("Opaque face-tracking patches cannot be replayed after outfit removal. Use the retained restore point or a separate line.");
            var before = new Result(); before.data.prefabs = result.data.prefabs;
            ObserveScene(request, before);
            if (before.data.observedSelection == null || before.data.observedSelection.outfits.Length < 2)
                throw new InvalidOperationException("Removal requires a verified Avatar with at least one other outfit remaining.");
            var inventories = ObserveOutfitRemoval(request, before.data);
            var current = inventories.SingleOrDefault(i => i.sourceId == reviewed.sourceId);
            if (current == null || !string.IsNullOrEmpty(current.blockedReason) || JsonUtility.ToJson(current) != JsonUtility.ToJson(reviewed))
                throw new InvalidOperationException("The owned removal inventory changed. Reinspect and review it again.");
            var owners = JsonUtility.FromJson<Owners>(File.ReadAllText(OwnershipPath));
            var avatar = FindOwned(current.avatarGlobalObjectId);
            DestroyRemovalRoots(current);
            var rootIds = new HashSet<string>(current.roots.Select(r => r.globalObjectId));
            var removedKeys = new HashSet<string>(owners.objects.Where(o => rootIds.Contains(o.globalObjectId)).Select(o => o.key));
            owners.objects.RemoveAll(o => rootIds.Contains(o.globalObjectId));
            AtomicWrite(OwnershipPath, JsonUtility.ToJson(owners, true));
            PruneRemovedFit(request.runId, current.sourceId, removedKeys);
            EditorSceneManager.MarkSceneDirty(avatar.scene);
            AssetDatabase.SaveAssets();
            if (!EditorSceneManager.SaveScene(avatar.scene, ScenePath)) throw new IOException("Could not save the scene after removal.");
            result.changedPaths.Add(ScenePath); result.changedPaths.Add(OwnershipPath);
            result.data.outfitRemovalApplied.Add(new RemovalApplied { sourceId = current.sourceId,
                avatarGlobalObjectId = current.avatarGlobalObjectId, deletedObjectIds = current.objects.Select(o => o.globalObjectId).ToList() });
        }

        // Called only after RemoveOutfit independently compared the saved ownership inventory.
        private static void DestroyRemovalRoots(OutfitRemoval current)
        {
            var roots = current.roots.Select(r => ResolveRemovalId(r.globalObjectId) as GameObject).ToList();
            if (roots.Any(r => r == null)) throw new InvalidOperationException("A reviewed owned root is missing.");
            foreach (var root in roots) UnityEngine.Object.DestroyImmediate(root);
            foreach (var member in current.objects)
                if (ResolveRemovalId(member.globalObjectId) != null) throw new InvalidOperationException("A reviewed object survived removal.");
        }

        private static void PruneRemovedFit(string runId, string sourceId, HashSet<string> removedKeys)
        {
            var root = Path.GetFullPath(Path.Combine(Application.dataPath, ".."));
            if (TryReadFitBaseline(root, runId, out var baseline))
            {
                // Keep every other baseline, including other owned Avatar scopes.
                baseline.garments.RemoveAll(g => g != null && removedKeys.Contains(g.ownerKey));
                AtomicWrite(Path.Combine(root, ".vua", "amf-fit-baseline.json"), JsonUtility.ToJson(baseline, true));
            }
            var report = Path.Combine(root, ".vua", "bridge", "amf-fit-changes.json");
            if (!File.Exists(report)) return;
            var changes = JsonUtility.FromJson<FitChangesFile>(File.ReadAllText(report));
            if (changes == null || changes.runId != runId || changes.fitChanges == null)
                throw new InvalidOperationException("The saved fit change report is invalid.");
            changes.fitChanges.RemoveAll(g => g != null && g.sourceId == sourceId);
            AtomicWrite(report, JsonUtility.ToJson(changes, true));
        }
    }
}
