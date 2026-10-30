#if UNITY_EDITOR
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEngine;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        [Serializable] internal sealed class TextureInfo
        {
            public string id, path, format, reason;
            public int width, height, faces = 1, maxSize;
            public bool mipmap, known, editable;
            public long estimatedBytes;
        }
        [Serializable] internal sealed class TextureMemoryReport
        {
            public string platform, rank;
            public List<TextureInfo> textures = new List<TextureInfo>();
            public long totalBytes;
            public bool complete = true;
        }
        [Serializable] internal sealed class TextureImportChange
        {
            public string path, platform, format;
            public int maxSize;
        }

        internal static string TextureRank(long bytes, string platform)
        {
            var limits = platform == "pc" ? new[] { 40, 75, 110, 150 } : new[] { 10, 18, 25, 40 };
            var names = new[] { "excellent", "good", "medium", "poor" };
            for (var i = 0; i < limits.Length; i++) if (bytes <= limits[i] * 1024L * 1024) return names[i];
            return "very_poor";
        }

        internal static long TextureBytes(int width, int height, bool mipmap, string format, int faces)
        {
            if (width <= 0 || height <= 0 || width > 16384 || height > 16384 || (faces != 1 && faces != 6)) return -1;
            int block = 1, bytes;
            switch (format)
            {
                case "DXT1": case "DXT1Crunched": case "BC4": case "ETC_RGB4": case "ETC_RGB4Crunched":
                case "ETC2_RGB": case "ETC2_RGBA1": case "EAC_R": case "EAC_R_SIGNED": block = 4; bytes = 8; break;
                case "DXT5": case "DXT5Crunched": case "BC5": case "BC6H": case "BC7": case "ETC2_RGBA8":
                case "ETC2_RGBA8Crunched": case "EAC_RG": case "EAC_RG_SIGNED":
                case "ASTC_4x4": case "ASTC_RGB_4x4": case "ASTC_RGBA_4x4": block = 4; bytes = 16; break;
                case "ASTC_5x5": block = 5; bytes = 16; break;
                case "ASTC_6x6": block = 6; bytes = 16; break;
                case "ASTC_8x8": block = 8; bytes = 16; break;
                case "ASTC_10x10": block = 10; bytes = 16; break;
                case "ASTC_12x12": block = 12; bytes = 16; break;
                case "Alpha8": case "R8": bytes = 1; break;
                case "RGB16": case "RGBA16": case "ARGB16": case "R16": case "RHalf": case "RG16": bytes = 2; break;
                case "RGB24": bytes = 3; break;
                case "RGBA32": case "ARGB32": case "BGRA32": case "RFloat": case "RGHalf": bytes = 4; break;
                case "RGBAHalf": case "RGFloat": bytes = 8; break;
                case "RGBAFloat": bytes = 16; break;
                default: return -1;
            }
            long total = 0;
            while (true)
            {
                total += ((width + block - 1L) / block) * ((height + block - 1L) / block) * bytes * faces;
                if (!mipmap || (width == 1 && height == 1)) return total;
                width = Math.Max(1, width / 2); height = Math.Max(1, height / 2);
            }
        }

        private static bool SafeTextureFile(string path)
        {
            if (!SafeAssetPath(path) || path.EndsWith(".meta", StringComparison.OrdinalIgnoreCase)) return false;
            // The work project must own real local files, including the metadata Unity changes.
            // Refuse symlinks/junctions rather than following one into an original material library.
            foreach (var candidate in new[] { path, path + ".meta" })
            {
                var root = Path.GetFullPath(Path.Combine(Application.dataPath, "..")).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
                var file = Path.GetFullPath(Path.Combine(root, candidate));
                if (!file.StartsWith(root, StringComparison.OrdinalIgnoreCase) || !File.Exists(file)) return false;
                if ((File.GetAttributes(root) & FileAttributes.ReparsePoint) != 0) return false;
                for (var current = file; current.Length >= root.Length; current = Path.GetDirectoryName(current))
                    if ((File.GetAttributes(current) & FileAttributes.ReparsePoint) != 0) return false;
            }
            return AssetDatabase.IsOpenForEdit(path);
        }

        private static HashSet<Texture> TextureDependencies(GameObject avatar) => new HashSet<Texture>(
            EditorUtility.CollectDependencies(new UnityEngine.Object[] { avatar }).OfType<Texture>());

        // One texture's decoded footprint on a platform, from the same importer rules the
        // assembled-Avatar report uses. `shared` only marks why a texture is read-only; it
        // never changes the decoded size. Known is false when the rules cannot decide.
        private static void FillTextureInfo(Texture texture, string platform, HashSet<Texture> shared, TextureInfo info)
        {
            var target = platform == "pc" ? "Standalone" : "Android";
            var path = AssetDatabase.GetAssetPath(texture);
            info.path = path; info.width = texture.width; info.height = texture.height; info.format = "Unknown"; info.reason = "unsupported_texture";
            info.id = AssetDatabase.TryGetGUIDAndLocalFileIdentifier(texture, out string guid, out long localId)
                ? guid + ":" + localId : "generated:" + texture.GetInstanceID();
            var importer = AssetImporter.GetAtPath(path) as TextureImporter;
            if (texture is Texture2D tex)
            {
                info.mipmap = tex.mipmapCount > 1;
                if (importer != null && importer.textureShape == TextureImporterShape.Texture2D)
                {
                    importer.GetSourceTextureWidthAndHeight(out int width, out int height);
                    var pot = Mathf.IsPowerOfTwo(width) && Mathf.IsPowerOfTwo(height);
                    var settings = importer.GetPlatformTextureSettings(target);
                    info.maxSize = settings.overridden ? settings.maxTextureSize : importer.maxTextureSize;
                    info.mipmap = importer.mipmapEnabled;
                    var effective = settings.overridden ? settings : importer.GetDefaultPlatformTextureSettings();
                    info.format = (effective.format == TextureImporterFormat.Automatic ? importer.GetAutomaticFormat(target) : effective.format).ToString();
                    // Non-POT import scaling and special shapes vary by importer; leave
                    // these unknown instead of guessing a favorable platform total.
                    if (pot)
                    {
                        while (Math.Max(width, height) > info.maxSize) { width = Math.Max(1, width / 2); height = Math.Max(1, height / 2); }
                        info.width = width; info.height = height;
                    }
                    else info.format = "Unknown";
                    info.reason = shared.Contains(texture) ? "shared_with_other_avatar"
                        : !pot ? "non_power_of_two"
                        : importer.textureType != TextureImporterType.Default || !importer.sRGBTexture || texture.filterMode == FilterMode.Point
                          || info.format.Contains("Half") || info.format.Contains("Float") || info.format.Contains("HDR") || info.format == "BC6H"
                          || new[] { ".exr", ".hdr" }.Contains(Path.GetExtension(path).ToLowerInvariant()) ? "specialized_texture"
                        : !SafeTextureFile(path) || AssetDatabase.LoadMainAssetAtPath(path) != texture ? "read_only_texture" : "";
                }
                else { info.format = tex.format.ToString(); info.reason = "no_texture_importer"; }
            }
            else if (texture is Cubemap cube)
            {
                info.faces = 6; info.mipmap = cube.mipmapCount > 1;
                // Only the active platform's imported cube dimensions/format are observed.
                if ((platform == "android") == (EditorUserBuildSettings.activeBuildTarget == BuildTarget.Android)) info.format = cube.format.ToString();
            }
            // A format unsupported on the target may be expanded by Unity. Its compressed
            // footprint must not be presented as that platform's decoded memory.
            var mobileFormat = info.format.StartsWith("ASTC", StringComparison.Ordinal) || info.format.StartsWith("ETC", StringComparison.Ordinal) || info.format.StartsWith("EAC", StringComparison.Ordinal);
            var desktopFormat = info.format.StartsWith("BC", StringComparison.Ordinal) || info.format.StartsWith("DXT", StringComparison.Ordinal);
            if ((platform == "pc" && mobileFormat) || (platform == "android" && desktopFormat)) info.format = "Unknown";
            var size = TextureBytes(info.width, info.height, info.mipmap, info.format, info.faces);
            info.known = size >= 0; info.estimatedBytes = Math.Max(0, size);
            if (!info.known && info.reason == "") info.reason = "unsupported_format";
            info.editable = info.known && info.reason == "";
        }

        internal static List<TextureMemoryReport> MeasureTextureMemory(string avatarId)
        {
            var avatar = string.IsNullOrEmpty(avatarId) ? null : FindOwned(avatarId);
            if (avatar == null) return new List<TextureMemoryReport>();
            var dependencies = TextureDependencies(avatar);
            if (dependencies.Count > 2048) throw new InvalidOperationException("The texture inventory exceeds its supported limit.");
            var shared = new HashSet<Texture>();
            foreach (var other in Resources.FindObjectsOfTypeAll<GameObject>().Where(go => go != avatar && go.scene == avatar.scene && IsAvatar(go)))
                shared.UnionWith(TextureDependencies(other));
            var reports = new List<TextureMemoryReport>();
            foreach (var platform in new[] { "pc", "android" })
            {
                var report = new TextureMemoryReport { platform = platform };
                foreach (var texture in dependencies.OrderBy(t => AssetDatabase.GetAssetPath(t), StringComparer.Ordinal).ThenBy(t => t.name, StringComparer.Ordinal))
                {
                    var info = new TextureInfo();
                    FillTextureInfo(texture, platform, shared, info);
                    report.textures.Add(info); report.totalBytes += info.estimatedBytes; report.complete &= info.known;
                }
                report.rank = report.complete ? TextureRank(report.totalBytes, platform) : "unknown";
                reports.Add(report);
            }
            return reports;
        }

        private static void ApplyTextureImports(Request request, Result result)
        {
            var changes = request.textureChanges;
            if (changes == null || changes.Length == 0 || changes.Length > 2048 || changes.Select(c => c?.path).Distinct().Count() != changes.Length)
                throw new InvalidOperationException("Choose distinct reviewed texture paths.");
            var reports = MeasureTextureMemory(result.data.avatarGlobalObjectId);
            // Validate every entry before the first importer write.
            foreach (var change in changes)
            {
                var report = reports.SingleOrDefault(r => r.platform == change?.platform);
                var texture = report?.textures.SingleOrDefault(t => t.path == change.path && t.editable);
                if (texture == null || !SafeTextureFile(change.path) || change.maxSize < 32 || (change.maxSize < 512 && change.maxSize != texture.maxSize) || change.maxSize > texture.maxSize || !Mathf.IsPowerOfTwo(change.maxSize)
                    || !Enum.TryParse(change.format, out TextureImporterFormat format)
                    || (change.format != texture.format && change.format != (change.platform == "pc" ? "BC7" : "ASTC_4x4")))
                    throw new InvalidOperationException("Texture import change is outside the reviewed safe scope.");
            }
            foreach (var change in changes)
            {
                var importer = (TextureImporter)AssetImporter.GetAtPath(change.path);
                var settings = importer.GetPlatformTextureSettings(change.platform == "pc" ? "Standalone" : "Android");
                settings.overridden = true; settings.maxTextureSize = change.maxSize;
                settings.format = (TextureImporterFormat)Enum.Parse(typeof(TextureImporterFormat), change.format);
                // Crunch affects the download, not decoded memory. Explicit formats here are
                // ordinary GPU compression; all other default/platform properties are retained.
                if (change.format == "BC7" || change.format == "ASTC_4x4") settings.crunchedCompression = false;
                importer.SetPlatformTextureSettings(settings); importer.SaveAndReimport();
                result.changedPaths.Add(change.path + ".meta");
            }
        }
    }
}
#endif
