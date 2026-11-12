// Synthetic Unity/SDK boundaries. The linked production observer owns the behavior under test.
using System;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace UnityEngine.SceneManagement
{
    public sealed class Scene { public string path = "Assets/Work.unity"; public bool isDirty; }
    public static class SceneManager { public static Scene GetActiveScene() => Vua.Editor.Bridge.Fixture.Avatar.scene; }
}
namespace UnityEngine
{
    public class Object { public static void DestroyImmediate(Component component) { Vua.Editor.Bridge.Fixture.Avatar.pipeline = null; } }
    public class Component : Object { }
    public class GameObject : Object
    {
        public VRC.Core.PipelineManager pipeline = new VRC.Core.PipelineManager();
        public SceneManagement.Scene scene = new SceneManagement.Scene();
        public Component GetComponent(Type type) => type == typeof(VRC.Core.PipelineManager) ? pipeline : null;
    }
    public static class Mathf { public static float Clamp01(float value) => Math.Clamp(value, 0, 1); }
    public static class Application { public static bool isBatchMode; public static string dataPath; }
    public static class Debug { public static void LogWarning(string message) { } }
    public static class JsonUtility
    {
        private static readonly JsonSerializerOptions Options = new JsonSerializerOptions { IncludeFields = true };
        public static string ToJson(object value, bool pretty) => JsonSerializer.Serialize(value, value.GetType(), Options);
        public static T FromJson<T>(string value)
        {
            using var document = JsonDocument.Parse(value);
            var instance = (T)Activator.CreateInstance(typeof(T), true);
            foreach (var field in typeof(T).GetFields(BindingFlags.Public | BindingFlags.Instance))
                if (document.RootElement.TryGetProperty(field.Name, out var entry))
                    field.SetValue(instance, JsonSerializer.Deserialize(entry.GetRawText(), field.FieldType, Options));
            return instance;
        }
    }
}
namespace UnityEditor
{
    public sealed class InitializeOnLoadMethodAttribute : Attribute { }
    public enum BuildTarget { StandaloneWindows64, Android }
    public static class EditorUserBuildSettings { public static BuildTarget activeBuildTarget = BuildTarget.StandaloneWindows64; }
    public static class EditorApplication
    {
        public static double timeSinceStartup; public static bool isCompiling, isUpdating;
        public delegate void CallbackFunction();
        public static event CallbackFunction update;
        public static void ExecuteMenuItem(string path) { }
    }
    public static class Selection { public static UnityEngine.GameObject activeGameObject; }
    public static class EditorUtility { public static void SetDirty(UnityEngine.Object target) { } }
    public static class Undo
    {
        public static UnityEngine.Component AddComponent(UnityEngine.GameObject target, Type type) =>
            target.pipeline = (VRC.Core.PipelineManager)Activator.CreateInstance(type);
    }
}
namespace UnityEditor.SceneManagement
{
    public enum OpenSceneMode { Single }
    public static class EditorSceneManager
    {
        public static bool SaveCurrentModifiedScenesIfUserWantsTo() => true;
        public static void OpenScene(string path, OpenSceneMode mode) { Vua.Editor.Bridge.Fixture.Avatar.scene.path = path; }
        public static void MarkSceneDirty(UnityEngine.SceneManagement.Scene scene) { scene.isDirty = true; }
        public static bool SaveScene(UnityEngine.SceneManagement.Scene scene)
        {
            if (Vua.Editor.Bridge.Fixture.SaveFails) return false;
            scene.isDirty = false;
            Vua.Editor.Bridge.Fixture.SavedBlueprint = Vua.Editor.Bridge.Fixture.Avatar.pipeline?.blueprintId;
            return true;
        }
    }
}
namespace Vua.Editor.Bridge
{
    internal static class Fixture
    {
        public static UnityEngine.GameObject Avatar;
        public static bool SaveFails, CanPublish;
        public static string SavedBlueprint;
        public static int Builds, Uploads, RemoteReads;
        public static Func<Task> Build;
        public static void Reset(string root)
        {
            Avatar = new UnityEngine.GameObject(); SaveFails = false; CanPublish = false; Builds = 0; Uploads = 0; RemoteReads = 0;
            SavedBlueprint = ""; Build = () => Task.CompletedTask;
            UnityEngine.Application.dataPath = Path.Combine(root, "Assets");
            UnityEditor.EditorUserBuildSettings.activeBuildTarget = UnityEditor.BuildTarget.StandaloneWindows64;
            AmfProductionEntryPoint.ResetForTest();
        }
    }
    internal static partial class AmfProductionEntryPoint
    {
        private const string ScenePath = "Assets/Work.unity";
        internal static void ResetForTest() { uploadRunning = false; localTestRecoveryBlocked = false; }
        internal static void ObserveForTest(string folder) { RunLocalTest(folder); }
        internal static void UploadForTest(string folder) { RunUpload(Path.Combine(folder, "amf-upload.json")); }
        internal static bool RecoveryBlockedForTest => localTestRecoveryBlocked;
        private static UnityEngine.GameObject FindOwned(string id) => id == "owned-avatar" ? Fixture.Avatar : null;
        private static void AtomicWrite(string path, string text) { File.WriteAllText(path, text); }
        [Serializable] private sealed class SdkReview { public string state = "checked"; }
        private static SdkReview ReviewWithSdk(UnityEngine.GameObject avatar) => new SdkReview();
        private sealed class Result { public ResultData data = new ResultData(); public System.Collections.Generic.List<string> changedPaths = new System.Collections.Generic.List<string>(); }
        private sealed class ResultData { public string avatarGlobalObjectId; public TurntableFacts turntable; }
    }
    internal sealed class BridgeCommand { public string commandId; }
    internal static class BridgePreviewBake { public static void RenderTurntable(UnityEngine.GameObject target, int size, int count, Func<int, string> path) { } }
}

// Public SDK shapes, reached by the production SdkBuilder's reflection. No SDK implementation is copied.
public static class VRCSdkControlPanel
{
    public static object window = new object();
    public static bool TryGetBuilder<T>(out T builder) where T : class
    { builder = new SyntheticAvatarBuilder() as T; return builder != null; }
}
namespace VRC.Core
{
    public sealed class PipelineManager : UnityEngine.Component { public string blueprintId = ""; }
    public sealed class APIUser
    {
        public static bool IsLoggedIn => true;
        public static APIUser CurrentUser => new APIUser();
        public bool canPublishAvatars => Vua.Editor.Bridge.Fixture.CanPublish;
    }
}
namespace VRC.SDKBase.Editor
{
    public interface IVRCSdkBuilderApi { }
    public sealed class BuilderException : Exception { public BuilderException(string message) : base(message) { } }
}
namespace VRC.SDKBase.Editor.Api
{
    public sealed class VRCAvatar
    {
        public string Name { get; set; }
        public string Description { get; set; }
        public System.Collections.Generic.List<string> Tags { get; set; }
        public string ReleaseStatus { get; set; }
    }
    public static class VRCApi
    {
        public static Task<VRCAvatar> GetAvatar(string id, bool force, CancellationToken token)
        { Vua.Editor.Bridge.Fixture.RemoteReads++; return Task.FromResult(new VRCAvatar()); }
    }
}
namespace VRC.SDK3A.Editor
{
    public interface IVRCSdkAvatarBuilderApi : VRC.SDKBase.Editor.IVRCSdkBuilderApi
    {
        void SelectAvatar(UnityEngine.GameObject target);
        Task BuildAndTest(UnityEngine.GameObject target);
        Task BuildAndUpload(UnityEngine.GameObject target, VRC.SDKBase.Editor.Api.VRCAvatar content, string thumbnail, CancellationToken cancel);
    }
}
public sealed class SyntheticAvatarBuilder : VRC.SDK3A.Editor.IVRCSdkAvatarBuilderApi
{
    public void SelectAvatar(UnityEngine.GameObject target) { }
    public async Task BuildAndTest(UnityEngine.GameObject target)
    { Vua.Editor.Bridge.Fixture.Builds++; await Vua.Editor.Bridge.Fixture.Build(); }
    public Task BuildAndUpload(UnityEngine.GameObject target, VRC.SDKBase.Editor.Api.VRCAvatar content, string thumbnail, CancellationToken cancel)
    {
        Vua.Editor.Bridge.Fixture.Uploads++;
        target.pipeline.blueprintId = "avtr_11111111-2222-3333-4444-555555555555";
        return Task.CompletedTask;
    }
}
