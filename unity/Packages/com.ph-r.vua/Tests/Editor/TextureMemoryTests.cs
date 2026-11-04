using NUnit.Framework;

namespace Vua.Editor.Bridge.Tests
{
    public sealed class TextureMemoryTests
    {
        [TestCase(4096, 4096, true, "RGBA32", 1, 89478484L)]
        [TestCase(4, 4, true, "DXT1", 1, 24L)]
        [TestCase(4, 4, true, "DXT1Crunched", 1, 24L)]
        [TestCase(7, 5, false, "ASTC_6x6", 1, 32L)]
        [TestCase(4, 4, false, "BC7", 6, 96L)]
        [TestCase(4, 4, false, "Unknown", 1, -1L)]
        public void DecodedMemoryCountsBlocksMipsAndFaces(int width, int height, bool mipmap, string format, int faces, long bytes)
        {
            Assert.AreEqual(bytes, AmfProductionEntryPoint.TextureBytes(width, height, mipmap, format, faces));
        }

        [Test]
        public void PlatformThresholdsIncludeTheirBoundary()
        {
            Assert.AreEqual("excellent", AmfProductionEntryPoint.TextureRank(40 * 1024L * 1024, "pc"));
            Assert.AreEqual("good", AmfProductionEntryPoint.TextureRank(40 * 1024L * 1024 + 1, "pc"));
            Assert.AreEqual("good", AmfProductionEntryPoint.TextureRank(18 * 1024L * 1024, "android"));
            Assert.AreEqual("very_poor", AmfProductionEntryPoint.TextureRank(40 * 1024L * 1024 + 1, "android"));
        }
    }
}
