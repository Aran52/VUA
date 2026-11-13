using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        [Serializable] private sealed class LocalTestRequest
        {
            public string schemaVersion; public string runId; public string testId; public string avatarGlobalObjectId;
        }
        [Serializable] private sealed class LocalTestJournal
        {
            public string schemaVersion; public string runId; public string testId; public string avatarGlobalObjectId;
            public string blueprintId; public bool hadPipeline;
        }
        [Serializable] private sealed class LocalTestLine
        {
            public string schemaVersion = "0.1"; public string runId; public string testId; public string stage; public string status = "";
        }
        [Serializable] private sealed class LocalTestReceipt
        {
            public string schemaVersion = "0.1"; public string runId; public string testId; public string outcome = "failed";
            public string reason = ""; public string detail = ""; public List<string> errors = new List<string>();
            public bool canPublishAvatars; public bool canPublishAvatarsObserved; public string sdkSignIn = "panel";
            public SdkReview sdkReview = new SdkReview();
        }

        private static bool localTestRecoveryBlocked;

        [InitializeOnLoadMethod] private static void InstallLocalTestObserver()
        {
            if (Application.isBatchMode) return;
            double next = 0;
            EditorApplication.update += () =>
            {
                if (uploadRunning || localTestRecoveryBlocked || EditorApplication.timeSinceStartup < next
                    || EditorApplication.isCompiling || EditorApplication.isUpdating) return;
                next = EditorApplication.timeSinceStartup + 0.5;
                var folder = Path.GetFullPath(Path.Combine(Application.dataPath, "..", ".vua", "bridge"));
                // Recovery is driven by the journal even if VUA timed out or removed the request.
                if (!File.Exists(Path.Combine(folder, "amf-test.started")) && !File.Exists(Path.Combine(folder, "amf-test.json"))) return;
                if (File.Exists(Path.Combine(folder, "amf-upload.started"))) return;
                uploadRunning = true;
                RunLocalTest(folder);
            };
        }

        private static bool LocalTestBound(string folder, string schemaVersion, string runId, string testId, string avatarId)
        {
            var marker = Path.Combine(folder, "..", "amf-run.txt");
            return schemaVersion == "0.1" && !string.IsNullOrEmpty(testId) && testId.Length <= 100
                && testId.All(c => c < 128 && (char.IsLetterOrDigit(c) || c == '-'))
                && !string.IsNullOrEmpty(avatarId) && File.Exists(marker) && File.ReadAllText(marker).Trim() == runId;
        }

        private static bool OpenLocalTestScene()
        {
            if (SceneManager.GetActiveScene().path == ScenePath) return true;
            if (!EditorSceneManager.SaveCurrentModifiedScenesIfUserWantsTo()) return false;
            EditorSceneManager.OpenScene(ScenePath, OpenSceneMode.Single);
            return true;
        }

        private static async void RunLocalTest(string folder)
        {
            var file = Path.Combine(folder, "amf-test.json");
            var started = Path.Combine(folder, "amf-test.started");
            var receipt = new LocalTestReceipt();
            LocalTestJournal journal = null;
            LocalTestRequest request = null;
            bool restored = false;
            try
            {
                if (File.Exists(started))
                {
                    var saved = JsonUtility.FromJson<LocalTestJournal>(File.ReadAllText(started));
                    if (saved == null || !LocalTestBound(folder, saved.schemaVersion, saved.runId, saved.testId, saved.avatarGlobalObjectId))
                        throw new InvalidOperationException("The local test recovery journal does not match this work project.");
                    journal = saved;
                    receipt.runId = journal.runId; receipt.testId = journal.testId;
                    receipt.reason = "interrupted"; receipt.detail = "Unity reloaded during the local test. The original Avatar identity was restored; start a new test when ready.";
                    return;
                }
                request = JsonUtility.FromJson<LocalTestRequest>(File.ReadAllText(file));
                if (request == null || !LocalTestBound(folder, request.schemaVersion, request.runId, request.testId, request.avatarGlobalObjectId))
                { TryDelete(file); return; }
                receipt.runId = request.runId; receipt.testId = request.testId;

                void Line(string stage, string status = "") => AtomicWrite(Path.Combine(folder, "amf-test.progress.json"),
                    JsonUtility.ToJson(new LocalTestLine { runId = request.runId, testId = request.testId, stage = stage, status = status ?? "" }, true));
                bool StopRequested()
                {
                    try
                    {
                        var cancel = JsonUtility.FromJson<LocalTestRequest>(File.ReadAllText(Path.Combine(folder, "amf-test.cancel")));
                        return cancel != null && cancel.schemaVersion == "0.1" && cancel.runId == request.runId && cancel.testId == request.testId;
                    }
                    catch (Exception) { return false; }
                }
                bool Stop()
                {
                    if (!StopRequested()) return false;
                    receipt.outcome = "cancelled"; receipt.reason = "cancelled"; return true;
                }

                Line("sdk_panel");
                if (Stop()) return;
                if (!OpenLocalTestScene()) { receipt.reason = "scene_not_saved"; receipt.detail = "Save the work scene before testing the Avatar."; return; }
                var avatar = FindOwned(request.avatarGlobalObjectId);
                if (avatar == null) { receipt.reason = "avatar_missing"; return; }
                if (EditorUserBuildSettings.activeBuildTarget != BuildTarget.StandaloneWindows64)
                { receipt.reason = "platform_unsupported"; receipt.detail = "Local Avatar testing requires the Windows PC build target."; return; }
                var sdk = SdkBuilder.Load();
                var handoff = ReadSessionHandoff(folder);
                if (handoff != null)
                {
                    try { if (await sdk.ApplySession(handoff.username, handoff.auth, handoff.twoFactorAuth)) receipt.sdkSignIn = "vua_session"; }
                    catch (Exception error) { Debug.LogWarning("VUA local test sign-in: " + Unwrap(error).Message); }
                }
                UnityEditor.Selection.activeGameObject = avatar;
                EditorApplication.ExecuteMenuItem("VRChat SDK/Show Control Panel");
                object builder;
                while (true)
                {
                    if (Stop()) return;
                    builder = sdk.Builder();
                    if (builder != null && sdk.SignedIn()) break;
                    Line(builder == null ? "sdk_panel" : "sign_in");
                    await Task.Delay(1000);
                }
                receipt.canPublishAvatars = sdk.CanPublishAvatars(); receipt.canPublishAvatarsObserved = true;
                sdk.Select(builder, avatar);
                // A clean saved scene lets recovery persist the restored identity without saving
                // unrelated unsaved player edits after a domain reload.
                if (avatar.scene.isDirty) { receipt.reason = "scene_not_saved"; receipt.detail = "Save the work scene before testing the Avatar."; return; }
                if (Stop()) return;
                var pipeline = sdk.ExistingPipeline(avatar);
                journal = new LocalTestJournal { schemaVersion = "0.1", runId = request.runId, testId = request.testId,
                    avatarGlobalObjectId = request.avatarGlobalObjectId, hadPipeline = pipeline != null,
                    blueprintId = pipeline == null ? "" : sdk.BlueprintId(pipeline) ?? "" };
                AtomicWrite(started, JsonUtility.ToJson(journal, true));
                if (pipeline == null) pipeline = sdk.Pipeline(avatar);
                if (string.IsNullOrEmpty(journal.blueprintId))
                {
                    using (var hash = SHA256.Create())
                        sdk.SetBlueprintId(pipeline, "avtr_" + new Guid(hash.ComputeHash(Encoding.UTF8.GetBytes("vua-local-test/" + request.runId)).Take(16).ToArray()).ToString("D"));
                }
                Line("building");
                // The public SDK method has no cancellation parameter. Its result, followed by
                // restoration below, determines the outcome even if VUA receives a stop request.
                await sdk.BuildAndTest(builder, avatar);
                Line("finishing");
                receipt.outcome = "succeeded";
            }
            catch (Exception error)
            {
                var reason = Classify(error, receipt.errors);
                receipt.reason = new[] { "validation", "build", "build_blocked", "cancelled" }.Contains(reason) ? reason : "unknown";
                receipt.outcome = receipt.reason == "cancelled" ? "cancelled" : "failed";
                receipt.detail = Unwrap(error).Message;
            }
            finally
            {
                try
                {
                    if (journal != null)
                    {
                        if (!OpenLocalTestScene()) throw new InvalidOperationException("Open the saved work scene to restore the local test identity.");
                        var avatar = FindOwned(journal.avatarGlobalObjectId);
                        if (avatar == null) throw new InvalidOperationException("The Avatar is missing; its local test identity could not be restored.");
                        var sdk = SdkBuilder.Load();
                        var pipeline = sdk.ExistingPipeline(avatar);
                        if (journal.hadPipeline)
                        {
                            if (pipeline == null) throw new InvalidOperationException("The original Avatar pipeline is missing.");
                            if (sdk.BlueprintId(pipeline) != journal.blueprintId) sdk.SetBlueprintId(pipeline, journal.blueprintId);
                        }
                        else if (pipeline != null) UnityEngine.Object.DestroyImmediate(pipeline);
                        // A test build may have saved its temporary ID. Persist the original one
                        // before publishing success or clearing the durable recovery marker.
                        EditorSceneManager.MarkSceneDirty(avatar.scene);
                        if (!EditorSceneManager.SaveScene(avatar.scene)) throw new InvalidOperationException("The restored Avatar identity could not be saved.");
                    }
                    restored = true;
                }
                catch (Exception error)
                {
                    localTestRecoveryBlocked = true;
                    receipt.outcome = "failed"; receipt.reason = "scene_not_saved";
                    receipt.detail = "Local test recovery is pending. Save and reopen this work project in Unity: " + Unwrap(error).Message;
                }
                if (!string.IsNullOrEmpty(receipt.testId))
                {
                    try
                    {
                        var avatar = FindOwned(journal != null ? journal.avatarGlobalObjectId : request.avatarGlobalObjectId);
                        if (avatar != null) receipt.sdkReview = ReviewWithSdk(avatar);
                    }
                    catch (Exception error) { Debug.LogWarning("VUA local test SDK check: " + Unwrap(error).Message); }
                    // Clear the marker first: a Provider that sees success can safely admit the next task.
                    if (restored) { TryDelete(started); TryDelete(file); TryDelete(Path.Combine(folder, "amf-test.cancel")); }
                    try { AtomicWrite(Path.Combine(folder, "amf-test.result.json"), JsonUtility.ToJson(receipt, true)); }
                    catch (Exception error) { Debug.LogWarning("VUA local test receipt: " + error.Message); }
                }
                else if (File.Exists(started)) localTestRecoveryBlocked = true;
                else TryDelete(file); // An unreadable/unbound request never reaches the SDK.
                uploadRunning = false;
            }
        }
    }
}
