#if UNITY_EDITOR
using System;
using System.Collections.Generic;
using System.Linq;
using UnityEditor;
using UnityEngine;
using UnityEngine.Rendering;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        // Pre-assembly cost prediction (freeze row F5 / slice 35 item 2). The observation is
        // read from the imported prefab before any assembly: exact counts where the asset
        // states them, decoded sizes through the same rules as the texture-memory report
        // (slice 9), and an unpredicted remainder named by `complete`. The built result is
        // always the SDK's own measurement; this is source evidence only.
        [Serializable] internal sealed class Footprint
        {
            // Index counts divided by three, summed over every submesh of every renderer.
            public long triangles;
            // Material slots across every renderer.
            public int materialSlots;
            // Skin/static mesh renderers that contributed; evidence for the two counts above.
            public int meshes;
            public int skinnedMeshes;
            // Decoded PC texture memory of everything the prefab depends on.
            public long textureBytes;
            public bool textureComplete;
            // Vertex and index buffer bytes of the meshes above.
            public long meshBytes;
            public bool meshComplete;
            // VRCPhysBone components; physBonesKnown is false when the SDK type is absent.
            public int physBones;
            public bool physBonesKnown;
        }

        private const int MaxFootprintTextures = 2048;

        private static Type PhysBoneType() => AppDomain.CurrentDomain.GetAssemblies()
            .Select(assembly => assembly.GetType("VRC.SDK3.Dynamics.PhysBone.Components.VRCPhysBone", false))
            .FirstOrDefault(type => type != null);

        private static Mesh MeshOf(Renderer renderer)
        {
            if (renderer is SkinnedMeshRenderer skinned) return skinned.sharedMesh;
            return renderer.TryGetComponent(out MeshFilter filter) ? filter.sharedMesh : null;
        }

        private static int VertexFormatBytes(VertexAttributeFormat format)
        {
            switch (format)
            {
                case VertexAttributeFormat.Float32:
                case VertexAttributeFormat.UInt32:
                case VertexAttributeFormat.SInt32: return 4;
                case VertexAttributeFormat.Float16:
                case VertexAttributeFormat.UNorm16:
                case VertexAttributeFormat.SNorm16:
                case VertexAttributeFormat.UInt16:
                case VertexAttributeFormat.SInt16: return 2;
                default: return 1;
            }
        }

        // Vertex stride from the mesh's own layout plus its index buffer. Layouts are asset
        // metadata, so this works on non-readable meshes; a mesh without a readable layout
        // falls back to a stated 32-byte stride and marks itself incomplete.
        private static long MeshBufferBytes(Mesh mesh, out bool known)
        {
            known = true;
            if (mesh == null) return 0;
            long stride = 0;
            try
            {
                foreach (var attribute in mesh.GetVertexAttributes())
                    stride += (long)VertexFormatBytes(attribute.format) * Mathf.Max(1, attribute.dimension);
            }
            catch { known = false; }
            if (stride == 0 && mesh.vertexCount > 0) { stride = 32; known = false; }
            var indexBytes = mesh.indexFormat == IndexFormat.UInt16 ? 2L : 4L;
            long indices = 0;
            for (var submesh = 0; submesh < mesh.subMeshCount; submesh++)
                indices += indexBytes * Math.Max(0, mesh.GetIndexCount(submesh));
            return stride * mesh.vertexCount + indices;
        }

        internal static Footprint ObserveFootprint(GameObject prefab, string platform)
        {
            var footprint = new Footprint { textureComplete = true, meshComplete = true };
            foreach (var renderer in prefab.GetComponentsInChildren<Renderer>(true))
            {
                var mesh = MeshOf(renderer);
                if (mesh == null) continue;
                footprint.meshes++;
                if (renderer is SkinnedMeshRenderer) footprint.skinnedMeshes++;
                footprint.materialSlots += renderer.sharedMaterials.Length;
                for (var submesh = 0; submesh < mesh.subMeshCount; submesh++)
                    footprint.triangles += Math.Max(0, mesh.GetIndexCount(submesh)) / 3;
                footprint.meshBytes += MeshBufferBytes(mesh, out var meshKnown);
                footprint.meshComplete &= meshKnown;
            }
            var textures = EditorUtility.CollectDependencies(new UnityEngine.Object[] { prefab }).OfType<Texture>()
                .OrderBy(texture => AssetDatabase.GetAssetPath(texture), StringComparer.Ordinal)
                .ThenBy(texture => texture.name, StringComparer.Ordinal).ToList();
            // The bound keeps an inspection predictable; an oversized inventory stays unknown
            // rather than throwing away the whole observation.
            if (textures.Count > MaxFootprintTextures) footprint.textureComplete = false;
            else foreach (var texture in textures)
            {
                var info = new TextureInfo();
                FillTextureInfo(texture, platform, new HashSet<Texture>(), info);
                footprint.textureBytes += info.estimatedBytes;
                footprint.textureComplete &= info.known;
            }
            var physBone = PhysBoneType();
            footprint.physBonesKnown = physBone != null;
            footprint.physBones = physBone == null ? 0 : prefab.GetComponentsInChildren(physBone, true).Length;
            return footprint;
        }
    }
}
#endif
