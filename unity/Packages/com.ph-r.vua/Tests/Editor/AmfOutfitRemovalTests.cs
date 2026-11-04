using System;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using NUnit.Framework;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using nadena.dev.modular_avatar.core;

namespace Vua.Editor.Bridge.Tests
{
    // Synthetic saved scene only. No purchased prefab or SDK upload is involved.
    internal sealed class AmfOutfitRemovalTests
    {
        private Scene scene;
        private Scene previous;
        private string folder;
        private static string Id(UnityEngine.Object obj) => GlobalObjectId.GetGlobalObjectIdSlow(obj).ToString();
        private static GameObject Child(GameObject parent, string name)
        { var obj = new GameObject(name); obj.transform.SetParent(parent.transform, false); return obj; }
        private static string Key(string source)
        { using (var sha = SHA256.Create()) return string.Concat(sha.ComputeHash(Encoding.UTF8.GetBytes(source)).Take(8).Select(b => b.ToString("x2"))); }
        [SetUp] public void SetUp()
        {
            previous = SceneManager.GetActiveScene();
            scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Additive);
            SceneManager.SetActiveScene(scene);
            folder = "Assets/VuaRemovalTest-" + Guid.NewGuid().ToString("N");
            Directory.CreateDirectory(folder); AssetDatabase.Refresh();
        }
        [TearDown] public void TearDown()
        {
            if (previous.IsValid()) SceneManager.SetActiveScene(previous);
            if (scene.IsValid()) EditorSceneManager.CloseScene(scene, true);
            AssetDatabase.DeleteAsset(folder);
        }
        private void Save() => Assert.That(EditorSceneManager.SaveScene(scene, folder + "/Scene.unity"), Is.True);
        private static AmfProductionEntryPoint.OutfitRemoval Inventory(GameObject avatar, GameObject outfit, GameObject menu, GameObject other)
        {
            var json = "{\"runId\":\"synthetic\",\"activeAvatarKey\":\"\",\"objects\":["
                + "{\"key\":\"avatar\",\"globalObjectId\":\"" + Id(avatar) + "\"},"
                + "{\"key\":\"outfit-" + Key("outfit") + "\",\"globalObjectId\":\"" + Id(outfit) + "\"},"
                + "{\"key\":\"menu-" + Key("outfit") + "\",\"globalObjectId\":\"" + Id(menu) + "\"},"
                + "{\"key\":\"outfit-" + Key("other") + "\",\"globalObjectId\":\"" + Id(other) + "\"}]}";
            var type = typeof(AmfProductionEntryPoint);
            var owners = JsonUtility.FromJson(json, type.GetNestedType("Owners", BindingFlags.NonPublic));
            return (AmfProductionEntryPoint.OutfitRemoval)type.GetMethod("RemovalInventory", BindingFlags.NonPublic | BindingFlags.Static)
                .Invoke(null, new[] { owners, "outfit", avatar });
        }
        [Test] public void ExactIdsDeleteTheOwnedTreeAndMenuWhileSameNameObjectsRemain()
        {
            var avatar = new GameObject("Avatar"); var outfit = Child(avatar, "Same name");
            var other = Child(avatar, "Same name"); var menu = Child(avatar, "Same name");
            var added = Child(outfit, "Player addition"); menu.AddComponent<ModularAvatarObjectToggle>();
            Save();
            var inventory = Inventory(avatar, outfit, menu, other);
            Assert.That(inventory.blockedReason, Is.Empty);
            Assert.That(inventory.objects.Any(o => o.globalObjectId == Id(added) && o.manual), Is.True);
            Assert.That(inventory.objects.Any(o => o.typeName == typeof(ModularAvatarObjectToggle).FullName), Is.True);
            typeof(AmfProductionEntryPoint).GetMethod("DestroyRemovalRoots", BindingFlags.NonPublic | BindingFlags.Static).Invoke(null, new object[] { inventory });
            Assert.That(outfit == null && menu == null && added == null, Is.True);
            Assert.That(other != null && avatar != null, Is.True);
        }
        [Test] public void AnotherOwnedGarmentInsideTheDeletionRootBlocksTheInventory()
        {
            var avatar = new GameObject("Avatar"); var outfit = Child(avatar, "Coat");
            var other = Child(outfit, "Other owned outfit"); var menu = Child(avatar, "Switch");
            Save();
            Assert.That(Inventory(avatar, outfit, menu, other).blockedReason, Is.EqualTo("contains_owned_object"));
            Assert.That(other != null && outfit != null, Is.True);
        }
    }
}
