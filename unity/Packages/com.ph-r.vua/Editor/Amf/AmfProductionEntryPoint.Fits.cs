using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Vua.Editor.Bridge
{
    // Saved fits: a per-garment local-Transform baseline recorded after each assembly, the moved
    // objects reported on every observation and on a scene save in the open Editor, and saved
    // fits applied right after an outfit instance is bound. Object names containing "/" or an
    // ambiguous "#" suffix do not survive a path round-trip; that is a known limitation.
    internal static partial class AmfProductionEntryPoint
    {
        private const float FitMoveThreshold = 0.0001f;
        private const float FitRotateThreshold = 0.01f;
        [Serializable] internal sealed class FitVec3
        {
            public float x; public float y; public float z;
            public static FitVec3 From(Vector3 value) => new FitVec3 { x = value.x, y = value.y, z = value.z };
        }
        [Serializable] internal sealed class FitQuat
        {
            public float x; public float y; public float z; public float w;
            public static FitQuat From(Quaternion value) => new FitQuat { x = value.x, y = value.y, z = value.z, w = value.w };
        }
        [Serializable] internal sealed class FitObject { public string path; public FitVec3 localPosition; public FitQuat localRotation; public FitVec3 localScale; }
        [Serializable] internal sealed class FitObjects { public FitObject[] objects; }
        [Serializable] internal sealed class FitChangeEntry { public string sourceId; public List<FitObject> moved = new List<FitObject>(); public List<string> missing = new List<string>(); }
        [Serializable] internal sealed class FitSkipEntry { public string sourceId; public List<string> paths = new List<string>(); }
        [Serializable] internal sealed class BaselineGarment { public string sourceId; public string ownerKey; public List<FitObject> objects = new List<FitObject>(); }
        [Serializable] internal sealed class FitBaseline { public string schemaVersion; public string runId; public List<BaselineGarment> garments = new List<BaselineGarment>(); }
        [Serializable] internal sealed class FitChangesFile { public string schemaVersion = "0.1"; public string runId; public string writtenAt; public List<FitChangeEntry> fitChanges = new List<FitChangeEntry>(); }

        // The path of t below root: "" for the root itself, then /-joined child names. A name
        // shared by several siblings is emitted as name#i (1-based, sibling order); a unique
        // name stays bare.
        internal static string TransformPath(Transform root, Transform t)
        {
            if (t == root) return "";
            var segments = new List<string>();
            for (var current = t; current != null && current != root; current = current.parent)
            {
                var parent = current.parent;
                if (parent == null) break;
                var shared = parent.Cast<Transform>().Where(child => child.name == current.name).ToList();
                segments.Add(shared.Count > 1 ? current.name + "#" + (shared.IndexOf(current) + 1).ToString(CultureInfo.InvariantCulture) : current.name);
            }
            segments.Reverse();
            return string.Join("/", segments);
        }

        // The reverse of TransformPath. A name#n segment names the n-th sibling of that name
        // (the suffix counts as an index only when it is all digits, split at the last "#").
        // When that does not resolve, a child literally named like the whole segment matches,
        // so a bare-emitted name that ends in "#<digits>" still round-trips.
        internal static Transform FindByPath(Transform root, string path)
        {
            if (string.IsNullOrEmpty(path)) return root;
            var current = root;
            foreach (var segment in path.Split('/'))
            {
                var name = segment; var index = 1; var indexed = false;
                var hash = segment.LastIndexOf('#');
                if (hash >= 0 && hash + 1 < segment.Length
                    && int.TryParse(segment.Substring(hash + 1), NumberStyles.None, CultureInfo.InvariantCulture, out index))
                {
                    name = segment.Substring(0, hash); indexed = true;
                }
                Transform found = null; var seen = 0;
                foreach (Transform child in current)
                    if (child.name == name && ++seen == index) { found = child; break; }
                if (found == null && indexed)
                    foreach (Transform child in current)
                        if (child.name == segment) { found = child; break; }
                if (found == null) return null;
                current = found;
            }
            return current;
        }

        // The garment instance root first (path ""), then every descendant, depth-first.
        internal static List<FitObject> CollectObjects(Transform root)
        {
            var objects = new List<FitObject>();
            void Visit(Transform t)
            {
                objects.Add(new FitObject { path = TransformPath(root, t), localPosition = FitVec3.From(t.localPosition),
                    localRotation = FitQuat.From(t.localRotation), localScale = FitVec3.From(t.localScale) });
                foreach (Transform child in t) Visit(child);
            }
            Visit(root);
            return objects;
        }

        internal static bool Moved(FitObject baseline, Transform current)
        {
            if (baseline.localPosition != null && Vector3.Distance(ToVector3(baseline.localPosition), current.localPosition) > FitMoveThreshold) return true;
            if (baseline.localRotation != null && Quaternion.Angle(ToQuaternion(baseline.localRotation), current.localRotation) > FitRotateThreshold) return true;
            if (baseline.localScale != null && Vector3.Distance(ToVector3(baseline.localScale), current.localScale) > FitMoveThreshold) return true;
            return false;
        }
        private static Vector3 ToVector3(FitVec3 value) => new Vector3(value.x, value.y, value.z);
        private static Quaternion ToQuaternion(FitQuat value) => new Quaternion(value.x, value.y, value.z, value.w);

        // Saved fits never fail an assembly: an object its path no longer resolves to is skipped.
        internal static List<string> ApplyFitObjects(Transform root, FitObject[] objects)
        {
            var skipped = new List<string>();
            if (objects == null) return skipped;
            foreach (var fit in objects)
            {
                if (fit == null) continue;
                var target = fit.path == "" ? root : FindByPath(root, fit.path);
                if (target == null) { skipped.Add(fit.path ?? ""); continue; }
                if (fit.localPosition != null) target.localPosition = ToVector3(fit.localPosition);
                if (fit.localRotation != null) target.localRotation = ToQuaternion(fit.localRotation);
                if (fit.localScale != null) target.localScale = ToVector3(fit.localScale);
            }
            return skipped;
        }

        // The comparison core, free of file IO so tests drive it with in-memory baselines. A null
        // root means the whole garment is gone. Returns null when nothing moved and nothing is missing.
        internal static FitChangeEntry CompareFitObjects(string sourceId, List<FitObject> baseline, Transform root)
        {
            var moved = new List<FitObject>(); var missing = new List<string>();
            foreach (var entry in baseline)
            {
                if (entry == null) continue;
                var path = entry.path ?? "";
                var target = root == null ? null : FindByPath(root, path);
                if (target == null) missing.Add(path);
                else if (Moved(entry, target))
                    moved.Add(new FitObject { path = path, localPosition = FitVec3.From(target.localPosition),
                        localRotation = FitQuat.From(target.localRotation), localScale = FitVec3.From(target.localScale) });
            }
            return moved.Count == 0 && missing.Count == 0 ? null
                : new FitChangeEntry { sourceId = sourceId, moved = moved, missing = missing };
        }

        private static List<FitChangeEntry> DetectFitChanges(Owners owners, FitBaseline baseline)
        {
            var changes = new List<FitChangeEntry>();
            if (owners == null || owners.objects == null || baseline == null || baseline.garments == null) return changes;
            foreach (var garment in baseline.garments)
            {
                if (garment == null || garment.objects == null) continue;
                var owner = owners.objects.SingleOrDefault(o => o.key == garment.ownerKey);
                var instance = owner == null ? null : FindOwned(owner.globalObjectId);
                var entry = CompareFitObjects(garment.sourceId, garment.objects, instance == null ? null : instance.transform);
                if (entry != null) changes.Add(entry);
            }
            return changes;
        }

        // Empty when there is no usable baseline for this run; the baseline file never breaks observation.
        private static List<FitChangeEntry> DetectFitChanges(Owners owners, string runId)
        {
            var root = Path.GetFullPath(Path.Combine(Application.dataPath, ".."));
            return TryReadFitBaseline(root, runId, out var baseline) ? DetectFitChanges(owners, baseline) : new List<FitChangeEntry>();
        }

        // True when a baseline for this run exists, so the scene-saved hook can tell "no changes"
        // apart from "no baseline" and only writes the report for the former.
        private static bool TryDetectFitChanges(string projectRoot, string runId, Owners owners, out List<FitChangeEntry> changes)
        {
            changes = new List<FitChangeEntry>();
            if (!TryReadFitBaseline(projectRoot, runId, out var baseline)) return false;
            changes = DetectFitChanges(owners, baseline);
            return true;
        }

        private static bool TryReadFitBaseline(string projectRoot, string runId, out FitBaseline baseline)
        {
            baseline = null;
            try
            {
                var path = Path.Combine(projectRoot, ".vua", "amf-fit-baseline.json");
                if (!File.Exists(path)) return false;
                var parsed = JsonUtility.FromJson<FitBaseline>(File.ReadAllText(path));
                if (parsed == null || parsed.runId != runId || parsed.garments == null) return false;
                baseline = parsed;
                return true;
            }
            catch (Exception) { return false; }
        }

        // The post-assembly baseline of every owned garment instance. A Recipe apply updates the
        // garments this assembly wrote and keeps the rest of the run's baseline; anything else
        // rewrites the file. A fresh assembly supersedes a pending change report.
        private static void CaptureFitBaseline(Owners owners, Request request)
        {
            var root = Path.GetFullPath(Path.Combine(Application.dataPath, ".."));
            var garments = new List<BaselineGarment>();
            foreach (var owner in owners.objects.Where(o => o.key.StartsWith(ScopedKey(owners, "outfit-"), StringComparison.Ordinal)))
            {
                var source = request.sources.SingleOrDefault(s => owner.key == ScopedKey(owners, "outfit-" + ShortHash(s.sourceId)));
                if (source == null) throw new InvalidOperationException("An owned object has no bound source identity.");
                var instance = FindOwned(owner.globalObjectId);
                if (instance == null) continue;
                garments.Add(new BaselineGarment { sourceId = source.sourceId, ownerKey = owner.key, objects = CollectObjects(instance.transform) });
            }
            var path = Path.Combine(root, ".vua", "amf-fit-baseline.json");
            if (request.applyTarget != null && TryReadFitBaseline(root, request.runId, out var retained))
            {
                var written = new HashSet<string>(garments.Select(g => g.sourceId));
                garments = retained.garments.Where(g => g != null && !written.Contains(g.sourceId)).Concat(garments).ToList();
            }
            AtomicWrite(path, JsonUtility.ToJson(new FitBaseline { schemaVersion = "0.1", runId = request.runId, garments = garments }, true));
            var changes = Path.Combine(root, ".vua", "bridge", "amf-fit-changes.json");
            if (File.Exists(changes)) File.Delete(changes);
        }

        // A player's edits in the open Editor are reported when the work scene is saved. The
        // report never interrupts the save: every failure is a warning. With no baseline for
        // the run there is nothing to compare against, and no file is written.
        private static void OnWorkSceneSaved(Scene scene)
        {
            try
            {
                if (scene.path != ScenePath) return;
                var root = Path.GetFullPath(Path.Combine(Application.dataPath, ".."));
                var runFile = Path.Combine(root, ".vua", "amf-run.txt");
                if (!File.Exists(runFile)) return;
                var runId = File.ReadAllText(runFile).Trim();
                if (!File.Exists(OwnershipPath)) return;
                var owners = JsonUtility.FromJson<Owners>(File.ReadAllText(OwnershipPath));
                if (owners == null) return;
                if (!TryDetectFitChanges(root, runId, owners, out var changes)) return;
                AtomicWrite(Path.Combine(root, ".vua", "bridge", "amf-fit-changes.json"), JsonUtility.ToJson(new FitChangesFile
                { runId = runId, writtenAt = DateTime.UtcNow.ToString("o", CultureInfo.InvariantCulture), fitChanges = changes }, true));
            }
            catch (Exception error) { Debug.LogWarning("[VUA] fit change report failed: " + error.GetType().Name + ": " + error.Message); }
        }
    }
}
