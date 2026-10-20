using System.Linq;
using NUnit.Framework;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Vua.Editor.Bridge.Tests
{
    internal sealed class AmfFitTests
    {
        [SetUp]
        public void SetUp()
        {
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
        }

        [TearDown]
        public void TearDown()
        {
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
        }

        [Test]
        public void PathsRoundTripThroughNestedHierarchiesAndDuplicateNames()
        {
            var root = new GameObject("Garment");
            var armature = Child(root, "Armature");
            var hips = Child(armature, "Hips");
            var clip = Child(hips, "HairClip");
            Child(clip, "Comb");
            Child(clip, "Comb");
            var combs = clip.transform.Cast<Transform>().Where(c => c.name == "Comb").ToList();

            Assert.That(AmfProductionEntryPoint.TransformPath(root.transform, root.transform), Is.EqualTo(""));
            Assert.That(AmfProductionEntryPoint.TransformPath(root.transform, hips.transform), Is.EqualTo("Armature/Hips"));
            Assert.That(AmfProductionEntryPoint.TransformPath(root.transform, combs[0]), Is.EqualTo("Armature/Hips/HairClip/Comb#1"));
            Assert.That(AmfProductionEntryPoint.TransformPath(root.transform, combs[1]), Is.EqualTo("Armature/Hips/HairClip/Comb#2"));

            Assert.That(AmfProductionEntryPoint.FindByPath(root.transform, ""), Is.EqualTo(root.transform));
            Assert.That(AmfProductionEntryPoint.FindByPath(root.transform, "Armature/Hips"), Is.EqualTo(hips.transform));
            // A bare name resolves to the first sibling of that name.
            Assert.That(AmfProductionEntryPoint.FindByPath(root.transform, "Armature/Hips/HairClip/Comb"), Is.EqualTo(combs[0]));
            Assert.That(AmfProductionEntryPoint.FindByPath(root.transform, "Armature/Hips/HairClip/Comb#2"), Is.EqualTo(combs[1]));
            Assert.That(AmfProductionEntryPoint.FindByPath(root.transform, "Armature/Hips/HairClip/Comb#3"), Is.Null);
            Assert.That(AmfProductionEntryPoint.FindByPath(root.transform, "Armature/Nope"), Is.Null);
        }

        [Test]
        public void HashSegmentsSplitAtTheLastHashAndFallBackToLiteralNames()
        {
            var root = new GameObject("Garment");
            // A real object named "Foo#1" (no sibling named "Foo"): the indexed reading finds
            // nothing, so the literal name still resolves.
            var literal = Child(root, "Foo#1");
            Assert.That(AmfProductionEntryPoint.TransformPath(root.transform, literal.transform), Is.EqualTo("Foo#1"));
            Assert.That(AmfProductionEntryPoint.FindByPath(root.transform, "Foo#1"), Is.EqualTo(literal.transform));
            // Once a sibling named "Foo" exists, the indexed reading wins.
            var foo = Child(root, "Foo");
            Assert.That(AmfProductionEntryPoint.FindByPath(root.transform, "Foo#1"), Is.EqualTo(foo.transform));

            // The split happens at the last "#": "Bar#baz#2" is the second sibling named "Bar#baz".
            Child(root, "Bar#baz");
            Child(root, "Bar#baz");
            var bars = root.transform.Cast<Transform>().Where(c => c.name == "Bar#baz").ToList();
            Assert.That(AmfProductionEntryPoint.FindByPath(root.transform, "Bar#baz#2"), Is.EqualTo(bars[1]));
        }

        [Test]
        public void MovedUsesDocumentedThresholds()
        {
            var root = new GameObject("Garment");
            root.transform.localScale = Vector3.zero;
            var baseline = AmfProductionEntryPoint.CollectObjects(root.transform)[0];

            root.transform.localPosition = new Vector3(0.0001f, 0f, 0f);
            Assert.That(AmfProductionEntryPoint.Moved(baseline, root.transform), Is.False);
            root.transform.localPosition = new Vector3(0.0002f, 0f, 0f);
            Assert.That(AmfProductionEntryPoint.Moved(baseline, root.transform), Is.True);
            root.transform.localPosition = Vector3.zero;

            // Quaternion components are floats and Quaternion.Angle treats nearly-equal dot
            // products as zero: a 0.01° rotation reports as not moved; 0.2° survives and
            // reports as moved.
            root.transform.localRotation = Quaternion.AngleAxis(0.01f, Vector3.forward);
            Assert.That(AmfProductionEntryPoint.Moved(baseline, root.transform), Is.False);
            root.transform.localRotation = Quaternion.AngleAxis(0.2f, Vector3.forward);
            Assert.That(AmfProductionEntryPoint.Moved(baseline, root.transform), Is.True);
            root.transform.localRotation = Quaternion.identity;

            root.transform.localScale = new Vector3(0.0001f, 0f, 0f);
            Assert.That(AmfProductionEntryPoint.Moved(baseline, root.transform), Is.False);
            root.transform.localScale = new Vector3(0.0002f, 0f, 0f);
            Assert.That(AmfProductionEntryPoint.Moved(baseline, root.transform), Is.True);
        }

        [Test]
        public void MovedTreatsNegatedQuaternionAsUnchanged()
        {
            var root = new GameObject("Garment");
            var rotation = Quaternion.Euler(30f, 45f, 60f);
            root.transform.localRotation = rotation;
            var baseline = new AmfProductionEntryPoint.FitObject
            {
                path = "",
                localPosition = AmfProductionEntryPoint.FitVec3.From(root.transform.localPosition),
                localRotation = AmfProductionEntryPoint.FitQuat.From(new Quaternion(-rotation.x, -rotation.y, -rotation.z, -rotation.w)),
                localScale = AmfProductionEntryPoint.FitVec3.From(root.transform.localScale),
            };
            Assert.That(AmfProductionEntryPoint.Moved(baseline, root.transform), Is.False);
        }

        [Test]
        public void CollectObjectsCapturesRootAndDescendantsDepthFirst()
        {
            var root = new GameObject("Garment");
            root.transform.localPosition = new Vector3(1f, 2f, 3f);
            var a = Child(root, "A");
            a.transform.localPosition = new Vector3(0.1f, 0f, 0f);
            var b = Child(a, "B");
            b.transform.localRotation = Quaternion.Euler(10f, 20f, 30f);
            b.transform.localScale = new Vector3(2f, 2f, 2f);
            Child(root, "C");

            var objects = AmfProductionEntryPoint.CollectObjects(root.transform);
            Assert.That(objects.Select(o => o.path), Is.EqualTo(new[] { "", "A", "A/B", "C" }));
            Assert.That(objects[0].localPosition.x, Is.EqualTo(1f));
            Assert.That(objects[0].localPosition.y, Is.EqualTo(2f));
            Assert.That(objects[0].localPosition.z, Is.EqualTo(3f));
            Assert.That(objects[1].localPosition.x, Is.EqualTo(0.1f));
            Assert.That(objects[2].localScale.x, Is.EqualTo(2f));
            var recorded = objects[2].localRotation;
            var actual = new Quaternion(recorded.x, recorded.y, recorded.z, recorded.w);
            Assert.That(Quaternion.Angle(actual, b.transform.localRotation), Is.EqualTo(0f));
        }

        [Test]
        public void ApplyFitObjectsSetsLocalTransformsAndReportsSkippedPaths()
        {
            var root = new GameObject("Garment");
            var a = Child(root, "A");
            var b = Child(a, "B");
            var fit = new[]
            {
                Fit("", new Vector3(1f, 0f, 0f)),
                Fit("A/B", new Vector3(0f, 5f, 0f)),
                Fit("A/Missing", new Vector3(9f, 9f, 9f)),
            };
            var skipped = AmfProductionEntryPoint.ApplyFitObjects(root.transform, fit);
            Assert.That(skipped, Is.EqualTo(new[] { "A/Missing" }));
            Assert.That(root.transform.localPosition, Is.EqualTo(new Vector3(1f, 0f, 0f)));
            Assert.That(b.transform.localPosition, Is.EqualTo(new Vector3(0f, 5f, 0f)));
            Assert.That(a.transform.localPosition, Is.EqualTo(Vector3.zero));
        }

        [Test]
        public void CompareFitObjectsReportsMovedObjectsWithCurrentTransforms()
        {
            var root = new GameObject("Garment");
            var a = Child(root, "A");
            a.transform.localPosition = new Vector3(0.01f, 0f, 0f);
            var baseline = AmfProductionEntryPoint.CollectObjects(root.transform);

            Assert.That(AmfProductionEntryPoint.CompareFitObjects("src", baseline, root.transform), Is.Null);

            a.transform.localPosition = new Vector3(0.02f, 0f, 0f);
            var entry = AmfProductionEntryPoint.CompareFitObjects("src", baseline, root.transform);
            Assert.That(entry.sourceId, Is.EqualTo("src"));
            Assert.That(entry.missing, Is.Empty);
            Assert.That(entry.moved.Select(m => m.path), Is.EqualTo(new[] { "A" }));
            Assert.That(entry.moved[0].localPosition.x, Is.EqualTo(0.02f));
        }

        [Test]
        public void CompareFitObjectsReportsEveryPathMissingWhenTheGarmentIsGone()
        {
            var root = new GameObject("Garment");
            Child(root, "A");
            var baseline = AmfProductionEntryPoint.CollectObjects(root.transform);
            var gone = AmfProductionEntryPoint.CompareFitObjects("src", baseline, null);
            Assert.That(gone.moved, Is.Empty);
            Assert.That(gone.missing, Is.EqualTo(new[] { "", "A" }));

            Child(root, "B");
            Object.DestroyImmediate(root.transform.Find("A").gameObject);
            var partial = AmfProductionEntryPoint.CompareFitObjects("src", baseline, root.transform);
            Assert.That(partial.moved, Is.Empty);
            Assert.That(partial.missing, Is.EqualTo(new[] { "A" }));
        }

        private static GameObject Child(GameObject parent, string name)
        {
            var child = new GameObject(name);
            child.transform.SetParent(parent.transform);
            return child;
        }

        private static AmfProductionEntryPoint.FitObject Fit(string path, Vector3 position) => new AmfProductionEntryPoint.FitObject
        {
            path = path,
            localPosition = AmfProductionEntryPoint.FitVec3.From(position),
            localRotation = AmfProductionEntryPoint.FitQuat.From(Quaternion.identity),
            localScale = AmfProductionEntryPoint.FitVec3.From(Vector3.one),
        };
    }
}
