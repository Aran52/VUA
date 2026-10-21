using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Vua.Editor.Bridge
{
    internal static partial class AmfProductionEntryPoint
    {
        private const string FtSessionKey = "VUA.AMF.FaceTracking.Session.v1";
        [Serializable] internal sealed class FtPatch
        {
            public string menuId; public string avatarGlobalObjectId; public bool confirmTrustedCode;
        }
        [Serializable] private sealed class FtSession
        {
            public Request request; public Result result; public string output; public string avatarId;
            public string selectionJson; public FtMenu menu;
        }
        [Serializable] private sealed class FtMarker
        {
            public string schemaVersion = "0.1"; public string runId; public string commandId;
            public string menuId; public string sourceId; public string scriptPath; public string menuPath;
            public string appliedAt; public string replay = "manual";
        }
        private static void StartFtSession(Request request, Result result, string output)
        {
            var patch = request.faceTrackingPatch;
            if (patch == null || !patch.confirmTrustedCode || string.IsNullOrEmpty(patch.avatarGlobalObjectId) ||
                patch.avatarGlobalObjectId != result.data.avatarGlobalObjectId || result.data.observedSelection == null)
                throw new InvalidOperationException("The confirmed active Avatar and its assembled relationships are required.");
            var menu = result.data.faceTracking.patchMenus.SingleOrDefault(m => m.id == patch.menuId);
            if (menu == null) throw new InvalidOperationException("Reinspect and select a unique imported creator menu.");
            if (!string.IsNullOrEmpty(SessionState.GetString(FtSessionKey, "")))
                throw new InvalidOperationException("A patch session has already been dispatched.");
            var session = new FtSession { request = request, result = result, output = output,
                avatarId = patch.avatarGlobalObjectId, selectionJson = JsonUtility.ToJson(result.data.observedSelection), menu = menu };
            // Save BEFORE executing vendor code. An assembly reload reopens our completion window,
            // never executes the menu again. Closing/restarting the Editor cannot resume this state.
            SessionState.SetString(FtSessionKey, JsonUtility.ToJson(session));
            UnityEditor.Selection.activeGameObject = FindOwned(session.avatarId);
            FtSessionWindow.ShowSession();
            LogStage("face_tracking");
            if (!EditorApplication.ExecuteMenuItem(menu.menuPath))
            {
                SessionState.EraseString(FtSessionKey);
                throw new InvalidOperationException("The selected creator menu could not be dispatched.");
            }
        }
        [InitializeOnLoadMethod]
        private static void ResumeFtWindow()
        {
            if (!Application.isBatchMode)
                EditorApplication.delayCall += () => {
                    if (!string.IsNullOrEmpty(SessionState.GetString(FtSessionKey, ""))) FtSessionWindow.ShowSession();
                };
        }
        internal sealed class FtSessionWindow : EditorWindow
        {
            private string failure;
            internal static void ShowSession()
            {
                var window = GetWindow<FtSessionWindow>("VUA Face Tracking");
                window.minSize = new Vector2(420, 280);
                window.Show();
            }
            private void OnGUI()
            {
                var saved = SessionState.GetString(FtSessionKey, "");
                if (string.IsNullOrEmpty(saved)) { EditorGUILayout.LabelField("No active VUA patch session."); return; }
                var session = JsonUtility.FromJson<FtSession>(saved);
                EditorGUILayout.LabelField("Complete the creator's patch", EditorStyles.boldLabel);
                EditorGUILayout.LabelField(session.menu.menuPath, EditorStyles.wordWrappedLabel);
                EditorGUILayout.HelpBox("Follow the creator's steps on the selected VUA Avatar. Add required prefabs to that same Avatar. " +
                    "Finish saves this scene, closes this dedicated Editor and checks the result in VUA. " +
                    "Opaque patch changes need manual replay; Recipe export is unavailable afterwards.", MessageType.Info);
                EditorGUILayout.HelpBox("This is your copied work project. Use only trusted creator code. VUA's restore point cannot undo writes outside this project.", MessageType.Warning);
                if (!string.IsNullOrEmpty(failure)) EditorGUILayout.HelpBox(failure, MessageType.Error);
                using (new EditorGUI.DisabledScope(EditorApplication.isCompiling || EditorApplication.isUpdating))
                {
                    if (GUILayout.Button("Select the VUA Avatar")) UnityEditor.Selection.activeGameObject = FindOwned(session.avatarId);
                    if (GUILayout.Button("Finish and check")) Complete(session);
                    if (GUILayout.Button("Cancel and restore")) Cancel();
                }
            }
            private static void Cancel()
            {
                SessionState.EraseString(FtSessionKey);
                // No successful receipt: the supervising engine restores only after exit.
                EditorApplication.Exit(1);
            }
            private void Complete(FtSession session)
            {
                try
                {
                    if (SceneManager.sceneCount != 1 || SceneManager.GetActiveScene().path != ScenePath)
                        throw new InvalidOperationException("Return to the single VUA Production scene before finishing.");
                    if (FindOwned(session.avatarId) == null)
                        throw new InvalidOperationException("The original owned Avatar is missing. Cancel and restore if the patch replaced its root.");
                    AssetDatabase.SaveAssets();
                    if (!EditorSceneManager.SaveScene(SceneManager.GetActiveScene(), ScenePath))
                        throw new IOException("The VUA scene could not be saved.");
                    ObserveScene(session.request, session.result);
                    if (session.result.data.avatarGlobalObjectId != session.avatarId ||
                        JsonUtility.ToJson(session.result.data.observedSelection) != session.selectionJson)
                        throw new InvalidOperationException("The original Avatar/outfit relationships changed. Correct them or cancel and restore.");
                    var marker = new FtMarker { runId = session.request.runId, commandId = session.result.commandId,
                        menuId = session.menu.id, sourceId = session.menu.sourceId, scriptPath = session.menu.scriptPath,
                        menuPath = session.menu.menuPath, appliedAt = DateTime.UtcNow.ToString("o") };
                    AtomicWrite(FtMarkerPath, JsonUtility.ToJson(marker, true));
                    session.result.data.faceTracking = ObserveFaceTracking(session.request, session.avatarId);
                    session.result.data.projectFingerprint = ProjectFingerprint.Compute();
                    session.result.changedPaths.Add(ScenePath);
                    session.result.changedPaths.Add(FtMarkerPath);
                    session.result.status = "succeeded";
                    AtomicWrite(BridgeEntryPoint.SafeBridgePath(session.output), JsonUtility.ToJson(session.result, true));
                    SessionState.EraseString(FtSessionKey);
                    EditorApplication.Exit(0);
                }
                catch (Exception error) { failure = error.Message; }
            }
        }
    }
}
