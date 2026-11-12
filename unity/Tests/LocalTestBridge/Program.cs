using System;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;
using Vua.Editor.Bridge;

internal static class Program
{
    private static int assertions;
    private static void Check(bool condition, string message)
    {
        assertions++;
        if (!condition) throw new InvalidOperationException(message);
    }
    private static string Read(JsonElement value, string key) => value.GetProperty(key).GetString();
    private static void Request(string folder, string test = "test-1") => File.WriteAllText(Path.Combine(folder, "amf-test.json"),
        JsonSerializer.Serialize(new { schemaVersion = "0.1", runId = "n3-synthetic", testId = test, avatarGlobalObjectId = "owned-avatar" }));
    private static void Cancel(string folder) => File.WriteAllText(Path.Combine(folder, "amf-test.cancel"),
        JsonSerializer.Serialize(new { schemaVersion = "0.1", runId = "n3-synthetic", testId = "test-1" }));
    private static async Task<JsonElement> Observe(string folder, bool upload = false)
    {
        var path = Path.Combine(folder, upload ? "amf-upload.result.json" : "amf-test.result.json");
        File.Delete(path);
        if (upload) AmfProductionEntryPoint.UploadForTest(folder);
        else AmfProductionEntryPoint.ObserveForTest(folder);
        for (var i = 0; i < 200 && !File.Exists(path); i++) await Task.Delay(10);
        using var result = JsonDocument.Parse(File.ReadAllText(path));
        return result.RootElement.Clone();
    }
    private static async Task Case(string name, Func<string, Task> test)
    {
        var root = Path.Combine(Path.GetTempPath(), "vua-local-test-" + Guid.NewGuid().ToString("N"));
        var folder = Path.Combine(root, ".vua", "bridge");
        Directory.CreateDirectory(folder); Directory.CreateDirectory(Path.Combine(root, "Assets"));
        File.WriteAllText(Path.Combine(root, ".vua", "amf-run.txt"), "n3-synthetic");
        Fixture.Reset(root);
        try { await test(folder); Console.WriteLine("PASS " + name); }
        finally { Directory.Delete(root, true); }
    }
    public static async Task Main()
    {
        await Case("Visitor builds locally and the temporary ID is restored", async folder => {
            string first = null, second = null;
            Fixture.Build = () => { first = Fixture.Avatar.pipeline.blueprintId; return Task.CompletedTask; };
            Request(folder);
            var receipt = await Observe(folder);
            Check(Read(receipt, "outcome") == "succeeded", "Visitor must be allowed to test");
            Check(receipt.GetProperty("canPublishAvatarsObserved").GetBoolean() && !receipt.GetProperty("canPublishAvatars").GetBoolean(), "false must be an actual SDK observation");
            Check(first.StartsWith("avtr_") && Fixture.Avatar.pipeline.blueprintId == "" && Fixture.SavedBlueprint == "", "restore both in-memory and saved identity");
            Check(!File.Exists(Path.Combine(folder, "amf-test.started")), "successful restoration clears the journal");
            Fixture.Build = () => { second = Fixture.Avatar.pipeline.blueprintId; return Task.CompletedTask; };
            Request(folder, "test-2"); await Observe(folder);
            Check(first == second, "repeated local builds use the same per-run ID");
        });
        await Case("Existing published IDs stay unchanged", async folder => {
            const string id = "avtr_11111111-2222-3333-4444-555555555555";
            Fixture.Avatar.pipeline.blueprintId = id;
            Fixture.Build = () => { Check(Fixture.Avatar.pipeline.blueprintId == id, "do not substitute a published ID"); return Task.CompletedTask; };
            Request(folder); await Observe(folder);
            Check(Fixture.Avatar.pipeline.blueprintId == id, "keep existing ID after build");
        });
        await Case("SDK failure restores the ID", async folder => {
            Fixture.Build = () => throw new VRC.SDKBase.Editor.BuilderException("synthetic SDK build failure");
            Request(folder); var receipt = await Observe(folder);
            Check(Read(receipt, "reason") == "build" && Read(receipt, "outcome") == "failed", "report the actual SDK failure");
            Check(Fixture.Avatar.pipeline.blueprintId == "" && Fixture.SavedBlueprint == "", "failed builds restore the ID");
        });
        await Case("A temporary PipelineManager is removed", async folder => {
            Fixture.Avatar.pipeline = null;
            Request(folder); await Observe(folder);
            Check(Fixture.Avatar.pipeline == null, "restore absence of PipelineManager");
        });
        await Case("Domain reload recovers without a request file", async folder => {
            File.WriteAllText(Path.Combine(folder, "amf-test.started"), JsonSerializer.Serialize(new {
                schemaVersion = "0.1", runId = "n3-synthetic", testId = "test-interrupted", avatarGlobalObjectId = "owned-avatar", blueprintId = "", hadPipeline = true }));
            Fixture.Avatar.pipeline.blueprintId = "avtr_temporary";
            var receipt = await Observe(folder);
            Check(Read(receipt, "reason") == "interrupted", "reload reports interruption");
            Check(Fixture.Builds == 0 && Fixture.Avatar.pipeline.blueprintId == "", "recovery never repeats the build");
        });
        await Case("Cancellation before building does not build or infer eligibility", async folder => {
            Request(folder); Cancel(folder); var receipt = await Observe(folder);
            Check(Read(receipt, "outcome") == "cancelled" && Fixture.Builds == 0, "stop before SDK build");
            Check(!receipt.GetProperty("canPublishAvatarsObserved").GetBoolean(), "no sign-in observation yet");
        });
        await Case("Cancellation during building waits for the real outcome", async folder => {
            Fixture.Build = () => { Cancel(folder); return Task.CompletedTask; };
            Request(folder); var receipt = await Observe(folder);
            Check(Read(receipt, "outcome") == "succeeded" && Fixture.Avatar.pipeline.blueprintId == "", "uncancellable build completes and restores");
        });
        await Case("Dirty scenes and Android targets are refused", async folder => {
            Fixture.Avatar.scene.isDirty = true;
            Request(folder); Check(Read(await Observe(folder), "reason") == "scene_not_saved", "leave unsaved player changes alone");
            Fixture.Avatar.scene.isDirty = false;
            UnityEditor.EditorUserBuildSettings.activeBuildTarget = UnityEditor.BuildTarget.Android;
            Request(folder); Check(Read(await Observe(folder), "reason") == "platform_unsupported" && Fixture.Builds == 0, "no Android local build");
        });
        await Case("Save failure retains the recovery journal", async folder => {
            Fixture.SaveFails = true;
            Request(folder); var receipt = await Observe(folder);
            Check(Read(receipt, "outcome") == "failed" && Read(receipt, "reason") == "scene_not_saved", "do not claim success before saving restoration");
            Check(File.Exists(Path.Combine(folder, "amf-test.started")), "keep recovery evidence");
        });
        await Case("Unbound requests and journals do not change an Avatar", folder => {
            var request = Path.Combine(folder, "amf-test.json");
            foreach (var text in new[] { "{", "{\"runId\":\"n3-synthetic\",\"testId\":\"test-1\",\"avatarGlobalObjectId\":\"owned-avatar\"}" }) {
                File.WriteAllText(request, text); AmfProductionEntryPoint.ObserveForTest(folder);
                Check(!File.Exists(request) && Fixture.Builds == 0, "discard malformed or unversioned requests");
            }
            File.WriteAllText(Path.Combine(folder, "amf-test.started"), JsonSerializer.Serialize(new {
                schemaVersion = "0.1", runId = "another-run", testId = "test-1", avatarGlobalObjectId = "owned-avatar", blueprintId = "replacement", hadPipeline = true }));
            AmfProductionEntryPoint.ObserveForTest(folder);
            Check(AmfProductionEntryPoint.RecoveryBlockedForTest && Fixture.Avatar.pipeline.blueprintId == "", "preserve an unbound journal without applying it");
            return Task.CompletedTask;
        });
        await Case("SDK upload eligibility and first upload remain correct after testing", async folder => {
            Request(folder); await Observe(folder);
            void UploadRequest() => File.WriteAllText(Path.Combine(folder, "amf-upload.json"), JsonSerializer.Serialize(new {
                schemaVersion = "0.1", runId = "n3-synthetic", uploadId = "upload-1", avatarGlobalObjectId = "owned-avatar",
                rightsConfirmed = true, avatarName = "Synthetic Avatar", thumbnail = ".vua/previews/front.png" }));
            UploadRequest(); var denied = await Observe(folder, true);
            Check(Read(denied, "reason") == "no_publish_permission" && Fixture.Uploads == 0, "Visitor can test but cannot upload");
            Check(denied.GetProperty("canPublishAvatarsObserved").GetBoolean() && !denied.GetProperty("canPublishAvatars").GetBoolean(), "upload reports observed false");
            Fixture.CanPublish = true;
            var previews = Path.Combine(folder, "..", "previews"); Directory.CreateDirectory(previews);
            File.WriteAllText(Path.Combine(previews, "front.png"), "synthetic thumbnail");
            UploadRequest(); var uploaded = await Observe(folder, true);
            Check(Read(uploaded, "outcome") == "succeeded" && uploaded.GetProperty("firstUpload").GetBoolean(), "test must not turn the first upload into an update");
            Check(Fixture.Uploads == 1 && Fixture.RemoteReads == 0 && Fixture.SavedBlueprint == Read(uploaded, "blueprintId"), "first upload keeps its new published ID");
        });
        Console.WriteLine($"{assertions} assertions passed against the production SDK observers and reflection adapter.");
    }
}
