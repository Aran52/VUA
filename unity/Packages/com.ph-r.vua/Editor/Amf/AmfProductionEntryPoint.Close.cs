using System;
using System.Globalization;
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
        [Serializable] private sealed class CloseRequest
        {
            public string schemaVersion; public string runId; public string closeId;
            public bool confirmed; public string expiresAt;
        }
        [Serializable] private sealed class CloseReceipt
        {
            public string schemaVersion = "0.1"; public string runId; public string closeId;
            public string outcome = "failed"; public bool scenesSaved; public bool assetsSaved;
            // Exit is observed by VUA after this receipt, never claimed by the Editor.
            public bool editorClosed; public string reason = "save_failed"; public string detail = "";
        }

        private static bool PlainCloseId(string value) => !string.IsNullOrEmpty(value) && value.Length <= 100
            && value.All(c => c < 128 && (char.IsLetterOrDigit(c) || c == '-'));

        // Runs on the Editor thread, before the existing SDK open observer. A consumed request
        // never runs twice after a domain reload. Expiry bounds pickup and the final exit.
        private static bool HandleCloseRequest(string folder)
        {
            var file = Path.Combine(folder, "amf-close.json");
            if (!File.Exists(file)) return false;
            try
            {
                if (new FileInfo(file).Length > 8192) { File.Delete(file); return true; }
                var request = JsonUtility.FromJson<CloseRequest>(File.ReadAllText(file));
                File.Delete(file);
                var marker = Path.Combine(folder, "..", "amf-run.txt");
                if (request == null || request.schemaVersion != "0.1" || !request.confirmed
                    || !PlainCloseId(request.runId) || !PlainCloseId(request.closeId)
                    || !File.Exists(marker) || File.ReadAllText(marker).Trim() != request.runId) return true;
                var receipt = new CloseReceipt { runId = request.runId, closeId = request.closeId };
                DateTimeOffset expires;
                var validTime = DateTimeOffset.TryParse(request.expiresAt, CultureInfo.InvariantCulture,
                    DateTimeStyles.AssumeUniversal, out expires);
                if (!validTime || expires <= DateTimeOffset.UtcNow || expires > DateTimeOffset.UtcNow.AddMinutes(3))
                {
                    receipt.reason = "expired"; receipt.detail = "The save and close request expired. Request it again in VUA.";
                }
                else if ((Application.unityVersion != "2022.3.22f1" && Application.unityVersion != "2022.3.22f1c1")
                    || EditorApplication.isPlayingOrWillChangePlaymode || uploadRunning
                    || PrefabStageUtility.GetCurrentPrefabStage() != null
                    || File.Exists(Path.Combine(folder, "amf-upload.json")) || File.Exists(Path.Combine(folder, "amf-open.json")))
                {
                    receipt.reason = "editor_busy"; receipt.detail = "Finish Play mode, prefab editing or the SDK handoff in Unity, then try again.";
                }
                else
                {
                    try { SaveForClose(receipt); }
                    catch (Exception error)
                    {
                        receipt.detail = error.Message.Length > 1000 ? error.Message.Substring(0, 1000) : error.Message;
                    }
                    if (receipt.scenesSaved && receipt.assetsSaved)
                    {
                        var cancel = Path.Combine(folder, "amf-close.cancel");
                        if (File.Exists(cancel) && File.ReadAllText(cancel).Trim() == request.closeId)
                        {
                            receipt.reason = "cancelled"; receipt.detail = "Unity saved, but closing was cancelled in VUA.";
                        }
                        else if (expires <= DateTimeOffset.UtcNow)
                        {
                            receipt.reason = "expired"; receipt.detail = "Unity saved, but the close request expired. Request closing again in VUA.";
                        }
                        else { receipt.outcome = "saved"; receipt.reason = ""; receipt.detail = ""; }
                    }
                }
                AtomicWrite(Path.Combine(folder, "amf-close.result.json"), JsonUtility.ToJson(receipt, true));
                // Exit skips Unity's save prompt, so every save must have succeeded and the
                // receipt must be on disk before calling it. A failed receipt write keeps Unity open.
                if (receipt.outcome == "saved") EditorApplication.Exit(0);
            }
            catch (Exception error) { Debug.LogWarning("VUA save and close: " + error.Message); }
            return true;
        }

        private static void SaveForClose(CloseReceipt receipt)
        {
            // An unnamed dirty scene needs the player's path choice. Do not open a modal save
            // dialog from the observer, or save other scenes before discovering this blocker.
            for (var i = 0; i < SceneManager.sceneCount; i++)
            {
                var scene = SceneManager.GetSceneAt(i);
                if (scene.isLoaded && scene.isDirty && string.IsNullOrEmpty(scene.path))
                {
                    receipt.reason = "untitled_scene";
                    throw new InvalidOperationException("Save the untitled scene in Unity first, then try again.");
                }
            }
            var saveError = false;
            Application.LogCallback onLog = (message, stack, type) =>
            {
                if (type == LogType.Error || type == LogType.Exception || type == LogType.Assert) saveError = true;
            };
            Application.logMessageReceived += onLog;
            try
            {
                receipt.reason = "scene_save_failed";
                for (var i = 0; i < SceneManager.sceneCount; i++)
                {
                    var scene = SceneManager.GetSceneAt(i);
                    if (scene.isLoaded && scene.isDirty && (!EditorSceneManager.SaveScene(scene) || scene.isDirty || saveError))
                        throw new InvalidOperationException("Unity could not save an open scene. Check its save errors before closing.");
                }
                receipt.scenesSaved = true;
                receipt.reason = "asset_save_failed";
                AssetDatabase.SaveAssets();
                var dirtyAsset = Resources.FindObjectsOfTypeAll<UnityEngine.Object>().Any(asset =>
                    !(asset is MonoScript) && EditorUtility.IsPersistent(asset) && EditorUtility.IsDirty(asset)
                    && IsProjectAsset(AssetDatabase.GetAssetPath(asset)));
                if (saveError || dirtyAsset)
                    throw new InvalidOperationException("Unity could not save all assets. Check its save errors before closing.");
                // Save hooks can dirty scenes again while assets are saved.
                for (var i = 0; i < SceneManager.sceneCount; i++)
                    if (SceneManager.GetSceneAt(i).isLoaded && SceneManager.GetSceneAt(i).isDirty)
                    {
                        receipt.scenesSaved = false; receipt.reason = "scene_save_failed";
                        throw new InvalidOperationException("A scene changed during saving. Save it in Unity before closing.");
                    }
                receipt.assetsSaved = true;
            }
            finally { Application.logMessageReceived -= onLog; }
        }

        // MonoScript reflection objects and read-only PackageCache importers can remain dirty
        // after SaveAssets. Check serialized assets/importers on disk inside this project;
        // source edits are already files, and script import settings belong to MonoImporter.
        private static bool IsProjectAsset(string path)
        {
            if (string.IsNullOrEmpty(path)) return false;
            if (path.StartsWith("Assets/", StringComparison.Ordinal)) return true;
            if (!path.StartsWith("Packages/", StringComparison.Ordinal)) return false;
            // Unity maps virtual Packages paths into Library/PackageCache for registry packages.
            var package = UnityEditor.PackageManager.PackageInfo.FindForAssetPath(path);
            if (package == null || string.IsNullOrEmpty(package.resolvedPath)) return false;
            var embeddedRoot = Path.GetFullPath(Path.Combine(Application.dataPath, "..", "Packages")) + Path.DirectorySeparatorChar;
            return Path.GetFullPath(package.resolvedPath).StartsWith(embeddedRoot, StringComparison.OrdinalIgnoreCase);
        }
    }
}
