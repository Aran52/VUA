//! Real synthetic filesystem + durable task authority; shared selected-download fixture.
use super::*;
use crate::library_maintenance::{draft_selection_status, LibraryMaintenance};
use crate::recipe_selection_drafts::RecipeSelectionDrafts;
use vua_bdl_store::CopyRole;
use vua_orchestrator::RecipeDocumentStore;

fn stored(w: &World, id: i64, name: &str) -> vua_bdl_store::ManagedLibraryFile {
    w.begin(name, &[id]);
    w.delivery(id, &format!("dl-{name}"), b"original bytes", "notes.pdf");
    w.observe(name, id, "settled", Some(&format!("dl-{name}")))
        .unwrap();
    w.wait(name);
    w.bdl.managed_library_file(id).unwrap().unwrap()
}
fn service(w: &World) -> LibraryMaintenance {
    LibraryMaintenance::new(
        w.store.clone(),
        w.bdl.clone(),
        w.runtime.clone(),
        w.base.join("warehouse"),
        w.service.clone(),
    )
}
fn preview(
    s: &LibraryMaintenance,
    drafts: &RecipeSelectionDrafts,
    recipes: Option<&RecipeDocumentStore>,
    ids: Option<Vec<String>>,
) -> Value {
    let mut p = json!({"schemaVersion":"0.1","target":{"kind":"product","id":"booth:90"}});
    if let Some(ids) = ids {
        p["copyIds"] = json!(ids);
    }
    s.apply("library.removalPreview", p, drafts, recipes)
        .unwrap()
}
fn command(preview: &Value, name: &str) -> Value {
    json!({"schemaVersion":"0.1","removalId":format!("library-removal-{name}"),"target":preview["target"],
        "copyIds":preview["files"].as_array().unwrap().iter().map(|file|file["copyId"].clone()).collect::<Vec<_>>(),"previewHash":preview["previewHash"]})
}
fn wait(s: &LibraryMaintenance, drafts: &RecipeSelectionDrafts, name: &str) -> Value {
    let start = std::time::Instant::now();
    loop {
        let value = s
            .apply(
                "library.removalStatus",
                json!({"schemaVersion":"0.1","removalId":format!("library-removal-{name}")}),
                drafts,
                None,
            )
            .unwrap();
        if value["state"] != "running" {
            return value;
        }
        assert!(
            start.elapsed() < crate::test_support::SETTLE_LIMIT,
            "removal stuck: {value}"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
fn draft(d: &RecipeSelectionDrafts, copy: &vua_bdl_store::ManagedLibraryFile) -> Value {
    d.apply("recipeDraft.save",json!({"schemaVersion":"0.1","draftId":"recipe-draft-local","title":"Synthetic selection","baseRevision":0,
        "selections":[{"identity":copy.artifact_sha256,"displayName":"Local PDF","source":"local","warehouseItemId":copy.warehouse_item_id}]})).unwrap()
}

#[test]
fn removal_keeps_catalog_bindings_and_references_then_download_restores_same_copy() {
    let w = World::new();
    let copy = stored(&w, 901, "initial");
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let original_draft = draft(&drafts, &copy);
    let recipes = RecipeDocumentStore::new_with_system_clock(w.base.join("recipes"));
    let mut recipe: Value = serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/recipe/v0.3/example.recipe.json"),
        )
        .unwrap(),
    )
    .unwrap();
    recipe["assets"][0]["sourceRef"] = json!({"provider":"booth","productId":"90"});
    recipes.save("synthetic-recipe", &recipe, 0).unwrap();
    let p = preview(&s, &drafts, Some(&recipes), None);
    assert_eq!(p["referenceCoverage"], "drafts_and_recipes");
    assert_eq!(p["references"].as_array().unwrap().len(), 2);
    assert_eq!(p["unresolvedRecipeAssets"], 1);
    assert!(p["references"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["missingAfterRemoval"] == true));
    assert!(!p
        .to_string()
        .contains(&w.base.to_string_lossy().to_string()));
    let request = command(&p, "accepted");
    let accepted = s
        .apply(
            "library.removeFiles",
            request.clone(),
            &drafts,
            Some(&recipes),
        )
        .unwrap();
    let result = wait(&s, &drafts, "accepted");
    assert_eq!(
        result["state"],
        "succeeded",
        "{result}; {:?}",
        w.store.task(result["taskId"].as_str().unwrap()).unwrap()
    );
    assert_eq!(result["files"][0]["phase"], "removed");
    assert!(!Path::new(&copy.stored_path).exists());
    assert!(w.bdl.catalog_detail("booth:90").unwrap().is_some());
    assert_eq!(
        w.bdl.managed_library_file(901).unwrap().unwrap().copy_id,
        copy.copy_id
    );
    assert_eq!(
        drafts
            .apply(
                "recipeDraft.get",
                json!({"schemaVersion":"0.1","draftId":"recipe-draft-local"})
            )
            .unwrap(),
        original_draft
    );
    assert_eq!(
        recipes.get("synthetic-recipe").unwrap().unwrap().recipe,
        recipe
    );
    let status = draft_selection_status(
        &w.bdl,
        &w.base.join("warehouse"),
        &drafts,
        json!({"schemaVersion":"0.1","draftId":"recipe-draft-local"}),
    )
    .unwrap();
    assert_eq!(status["items"][0]["state"], "missing");
    assert_eq!(status["revision"], original_draft["revision"]);
    let replay = s
        .apply(
            "library.removeFiles",
            request.clone(),
            &drafts,
            Some(&recipes),
        )
        .unwrap();
    assert_eq!(replay["taskId"], accepted["taskId"]);
    let mut conflict = request;
    conflict["previewHash"] = json!(format!("sha256:{}", "f".repeat(64)));
    assert_eq!(
        s.apply("library.removeFiles", conflict, &drafts, Some(&recipes))
            .unwrap_err()
            .0,
        "removal_conflict"
    );
    stored(&w, 901, "restored");
    let restored = w.bdl.managed_library_file(901).unwrap().unwrap();
    assert_eq!(restored.copy_id, copy.copy_id);
    assert_eq!(restored.warehouse_item_id, copy.warehouse_item_id);
    assert_eq!(
        std::fs::read(restored.stored_path).unwrap(),
        b"original bytes"
    );
    let restarted = TaskRuntime::with_sqlite(
        w.store.clone(),
        Arc::new(SystemClock),
        Arc::new(NanosTaskIdGenerator::default()),
    )
    .unwrap();
    let s2 = LibraryMaintenance::new(
        w.store.clone(),
        w.bdl.clone(),
        restarted,
        w.base.join("warehouse"),
        w.service.clone(),
    );
    assert_eq!(
        s2.apply(
            "library.removalStatus",
            json!({"schemaVersion":"0.1","removalId":"library-removal-accepted"}),
            &drafts,
            None
        )
        .unwrap()["state"],
        "succeeded"
    );
    assert!(Path::new(&copy.stored_path).exists());
}

#[test]
fn preview_drift_is_refused_and_same_size_content_drift_is_not_deleted() {
    let w = World::new();
    let copy = stored(&w, 901, "original");
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let p = preview(&s, &drafts, None, None);
    std::fs::remove_file(&copy.stored_path).unwrap();
    assert_eq!(
        s.apply("library.removeFiles", command(&p, "drift"), &drafts, None)
            .unwrap_err()
            .0,
        "preview_changed"
    );
    std::fs::write(&copy.stored_path, b"modified bytes").unwrap();
    assert_eq!(std::fs::metadata(&copy.stored_path).unwrap().len(), 14);
    let p = preview(&s, &drafts, None, None);
    assert_eq!(p["files"][0]["presence"], "present");
    s.apply("library.removeFiles", command(&p, "content"), &drafts, None)
        .unwrap();
    let result = wait(&s, &drafts, "content");
    assert_eq!(result["state"], "failed");
    assert_eq!(result["files"][0]["errorCode"], "file_changed");
    assert_eq!(std::fs::read(&copy.stored_path).unwrap(), b"modified bytes");
}

#[test]
fn selected_files_report_partial_success_and_already_missing_independently() {
    let w = World::new();
    let first = stored(&w, 901, "one");
    let second = stored(&w, 902, "two");
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    std::fs::write(&second.stored_path, b"modified bytes").unwrap();
    let p = preview(&s, &drafts, None, None);
    s.apply("library.removeFiles", command(&p, "partial"), &drafts, None)
        .unwrap();
    let result = wait(&s, &drafts, "partial");
    assert_eq!(result["state"], "succeeded_with_warnings");
    assert!(!Path::new(&first.stored_path).exists());
    assert!(Path::new(&second.stored_path).exists());
    let p = preview(&s, &drafts, None, Some(vec![first.copy_id.clone()]));
    s.apply("library.removeFiles", command(&p, "missing"), &drafts, None)
        .unwrap();
    assert_eq!(
        wait(&s, &drafts, "missing")["files"][0]["phase"],
        "already_missing"
    );
}

#[test]
fn reference_read_failure_is_not_an_empty_preview() {
    let w = World::new();
    stored(&w, 901, "one");
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    std::fs::create_dir_all(w.base.join("drafts")).unwrap();
    std::fs::write(w.base.join("drafts/recipe-draft-bad.json"), b"corrupt").unwrap();
    assert_eq!(
        s.apply(
            "library.removalPreview",
            json!({"schemaVersion":"0.1","target":{"kind":"product","id":"booth:90"}}),
            &drafts,
            None
        )
        .unwrap_err()
        .0,
        "reference_read_failed"
    );
}

#[cfg(any(windows, unix))]
fn alias_path(w: &World, copy: &vua_bdl_store::ManagedLibraryFile) -> PathBuf {
    #[cfg(windows)]
    let path = PathBuf::from(copy.stored_path.to_ascii_uppercase());
    #[cfg(unix)]
    let path = {
        let directory = w.base.join("warehouse/path-alias");
        std::os::unix::fs::symlink(Path::new(&copy.stored_path).parent().unwrap(), &directory)
            .unwrap();
        directory.join(Path::new(&copy.stored_path).file_name().unwrap())
    };
    let _ = w;
    assert!(
        path.is_file(),
        "the synthetic alias must resolve to the actual file"
    );
    path
}

#[cfg(any(windows, unix))]
#[test]
fn a_path_alias_reference_is_affected_even_when_only_one_copy_id_is_selected() {
    let w = World::new();
    let copy = stored(&w, 901, "original");
    let aliased = alias_path(&w, &copy);
    let entry = w
        .bdl
        .create_warehouse_item(
            "Synthetic alias",
            "imported_material",
            "2026-10-08T00:00:00Z",
        )
        .unwrap();
    let alias = w
        .bdl
        .record_artifact_copy(
            &entry.warehouse_item_id,
            &copy.artifact_sha256,
            "notes.pdf",
            aliased.to_str().unwrap(),
            CopyRole::Original,
            "2026-10-08T00:00:00Z",
        )
        .unwrap();
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let document = drafts.apply("recipeDraft.save", json!({"schemaVersion":"0.1","draftId":"recipe-draft-alias","title":"Synthetic alias reference","baseRevision":0,
        "selections":[{"source":"local","identity":alias.artifact_sha256,"warehouseItemId":alias.warehouse_item_id,"displayName":"Aliased file"}]})).unwrap();
    let s = service(&w);
    let p = preview(&s, &drafts, None, Some(vec![copy.copy_id.clone()]));
    assert_eq!(p["files"].as_array().unwrap().len(), 1);
    assert_eq!(p["references"].as_array().unwrap().len(), 1);
    assert_eq!(p["references"][0]["copyIds"], json!([alias.copy_id]));
    assert_eq!(p["references"][0]["missingAfterRemoval"], true);
    s.apply("library.removeFiles", command(&p, "alias"), &drafts, None)
        .unwrap();
    assert_eq!(wait(&s, &drafts, "alias")["state"], "succeeded");
    assert!(!aliased.exists());
    assert_eq!(
        drafts
            .apply(
                "recipeDraft.get",
                json!({"schemaVersion":"0.1","draftId":"recipe-draft-alias"})
            )
            .unwrap(),
        document
    );
    assert_eq!(
        w.bdl.entry_copies(&entry.warehouse_item_id).unwrap().len(),
        1
    );
}

#[cfg(any(windows, unix))]
#[test]
fn conflicting_hash_at_a_path_alias_refuses_deletion() {
    let w = World::new();
    let copy = stored(&w, 901, "original");
    let aliased = alias_path(&w, &copy);
    let conflicting_sha = format!("sha256:{}", "f".repeat(64));
    w.bdl
        .record_untrusted_artifact(&vua_bdl_store::NewLocalArtifact {
            artifact_sha256: conflicting_sha.clone(),
            size_bytes: 14,
            suggested_file_name: Some("notes.pdf".into()),
            download_id: None,
            first_seen_at: "2026-10-08T00:00:00Z".into(),
        })
        .unwrap();
    let entry = w
        .bdl
        .create_warehouse_item(
            "Conflicting synthetic alias",
            "imported_material",
            "2026-10-08T00:00:00Z",
        )
        .unwrap();
    w.bdl
        .record_artifact_copy(
            &entry.warehouse_item_id,
            &conflicting_sha,
            "notes.pdf",
            aliased.to_str().unwrap(),
            CopyRole::Original,
            "2026-10-08T00:00:00Z",
        )
        .unwrap();
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let s = service(&w);
    let p = preview(&s, &drafts, None, Some(vec![copy.copy_id]));
    s.apply(
        "library.removeFiles",
        command(&p, "alias-conflict"),
        &drafts,
        None,
    )
    .unwrap();
    let result = wait(&s, &drafts, "alias-conflict");
    assert_eq!(result["state"], "failed");
    assert_eq!(result["files"][0]["errorCode"], "copy_changed");
    assert_eq!(std::fs::read(aliased).unwrap(), b"original bytes");
}

#[test]
fn a_ledger_copy_outside_the_managed_root_is_never_deleted() {
    let w = World::new();
    let copy = stored(&w, 901, "one");
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let outside = w.base.join("outside-warehouse.pdf");
    std::fs::write(&outside, b"original bytes").unwrap();
    let foreign = w
        .bdl
        .record_artifact_copy(
            &copy.warehouse_item_id,
            &copy.artifact_sha256,
            "outside.pdf",
            outside.to_str().unwrap(),
            CopyRole::Original,
            "2026-10-08T00:00:00Z",
        )
        .unwrap();
    let p = preview(&s, &drafts, None, Some(vec![foreign.copy_id]));
    assert_eq!(p["files"][0]["presence"], "unreadable");
    s.apply("library.removeFiles", command(&p, "foreign"), &drafts, None)
        .unwrap();
    let result = wait(&s, &drafts, "foreign");
    assert_eq!(result["state"], "failed");
    assert_eq!(result["files"][0]["errorCode"], "outside_managed_root");
    assert_eq!(std::fs::read(&outside).unwrap(), b"original bytes");
    assert!(Path::new(&copy.stored_path).exists());
}

#[test]
fn removal_waits_for_active_download() {
    let w = World::new();
    let copy = stored(&w, 901, "one");
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let p = preview(&s, &drafts, None, None);
    w.begin("active", &[901]);
    assert_eq!(
        s.apply("library.removeFiles", command(&p, "busy"), &drafts, None)
            .unwrap_err()
            .0,
        "file_busy"
    );
    w.observe("active", 901, "initiation_failed", None).unwrap();
    assert!(Path::new(&copy.stored_path).exists());
}

#[test]
fn removal_product_scope_includes_zip_members_and_verified_migrated_files() {
    let w = World::new();
    let local = w.import_local(
        "Migrated deletion",
        &[("Avatar.unitypackage", b"avatar bytes")],
    );
    let archive = zip_bytes(&[
        ("Avatar.unitypackage", b"avatar bytes"),
        ("texture.psd", b"texture bytes"),
    ]);
    w.begin("zip-removal", &[902]);
    w.delivery(902, "dl-zip-removal", &archive, "bundle.zip");
    w.observe("zip-removal", 902, "settled", Some("dl-zip-removal"))
        .unwrap();
    w.wait("zip-removal");
    w.verify_sources("zip-removal-merge");
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let row = &library_view(&w, json!({"schemaVersion":"0.1"}))["items"][0];
    let ids: Vec<String> = serde_json::from_value(row["copyIds"].clone()).unwrap();
    let p = preview(&s, &drafts, None, Some(ids));
    assert_eq!(p["files"].as_array().unwrap().len(), 4);
    s.apply(
        "library.removeFiles",
        command(&p, "zip-members"),
        &drafts,
        None,
    )
    .unwrap();
    let done = wait(&s, &drafts, "zip-members");
    assert_eq!(done["state"], "succeeded");
    assert!(done["files"]
        .as_array()
        .unwrap()
        .iter()
        .all(|file| file["phase"] == "removed"));
    assert!(w
        .bdl
        .entry_copies(&local.warehouse_item_id)
        .unwrap()
        .iter()
        .all(|copy| !Path::new(&copy.stored_path).exists()));
}

#[test]
fn explicit_inspection_closes_interrupted_removal_without_unlinking_remaining_files() {
    let w = World::new();
    let missing = stored(&w, 901, "inspect-one");
    let remaining = stored(&w, 902, "inspect-two");
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let p = preview(&s, &drafts, None, None);
    let request = command(&p, "inspect");
    let evidence = w.bdl.library_copy_evidence().unwrap();
    let mut files = p["files"].clone();
    for file in files.as_array_mut().unwrap() {
        let copy = evidence
            .iter()
            .find(|copy| file["copyId"] == copy.copy.copy_id)
            .unwrap();
        file["storedPath"] = json!(copy.copy.stored_path);
        file["productIds"] = json!(["booth:90"]);
    }
    let plan = json!({"request":request,"files":files});
    w.store.accept_idempotent_task("library.removeFiles","library-removal-inspect",&plan.to_string(),&vua_orchestrator::NewTask {
        task_id:"task-inspect-removal".into(),correlation_id:"library-removal-inspect".into(),occurred_at:"t".into()
    },&json!({"schemaVersion":"0.1","taskId":"task-inspect-removal","acceptedRevision":1,"initialState":"queued"})).unwrap();
    std::fs::remove_file(&missing.stored_path).unwrap();
    let runtime = TaskRuntime::with_sqlite(
        w.store.clone(),
        Arc::new(SystemClock),
        Arc::new(NanosTaskIdGenerator::default()),
    )
    .unwrap();
    let resumed = LibraryMaintenance::new(
        w.store.clone(),
        w.bdl.clone(),
        runtime.clone(),
        w.base.join("warehouse"),
        w.service.clone(),
    );
    let pending = resumed
        .apply(
            "library.pendingRemovals",
            json!({"schemaVersion":"0.1","target":{"kind":"product","id":"booth:90"}}),
            &drafts,
            None,
        )
        .unwrap();
    assert_eq!(pending["items"].as_array().unwrap().len(), 1);
    assert_eq!(resumed.apply("library.resolveRemoval", json!({"schemaVersion":"0.1","removalId":"library-removal-inspect","observedRevision":2}), &drafts, None).unwrap_err().0, "removal_conflict");
    let resolve =
        json!({"schemaVersion":"0.1","removalId":"library-removal-inspect","observedRevision":1});
    let done = resumed
        .apply("library.resolveRemoval", resolve.clone(), &drafts, None)
        .unwrap();
    assert_eq!(done["state"], "cancelled");
    assert_eq!(done["inspectionResolved"], true);
    assert_eq!(done["recoveryDisposition"], "none");
    assert_eq!(
        done["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["copyId"] == missing.copy_id)
            .unwrap()["phase"],
        "missing_after_inspection"
    );
    assert_eq!(
        done["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["copyId"] == remaining.copy_id)
            .unwrap()["phase"],
        "kept"
    );
    assert_eq!(
        std::fs::read(&remaining.stored_path).unwrap(),
        b"original bytes"
    );
    assert!(!done.to_string().contains("storedPath"));
    assert_eq!(
        runtime.snapshot("task-inspect-removal").unwrap().state,
        TaskState::Cancelled
    );
    assert_eq!(
        runtime
            .snapshot("task-inspect-removal")
            .unwrap()
            .recovery_disposition,
        vua_orchestrator::TaskRecoveryDisposition::None
    );
    assert_eq!(
        resumed
            .apply("library.resolveRemoval", resolve.clone(), &drafts, None)
            .unwrap(),
        done
    );
    let restarted_runtime = TaskRuntime::with_sqlite(
        w.store.clone(),
        Arc::new(SystemClock),
        Arc::new(NanosTaskIdGenerator::default()),
    )
    .unwrap();
    let restarted = LibraryMaintenance::new(
        w.store.clone(),
        w.bdl.clone(),
        restarted_runtime,
        w.base.join("warehouse"),
        w.service.clone(),
    );
    assert_eq!(
        restarted
            .apply("library.resolveRemoval", resolve, &drafts, None)
            .unwrap(),
        done
    );
    assert!(restarted
        .apply(
            "library.pendingRemovals",
            json!({"schemaVersion":"0.1","target":{"kind":"product","id":"booth:90"}}),
            &drafts,
            None
        )
        .unwrap()["items"]
        .as_array()
        .unwrap()
        .is_empty());
    w.begin("unblocked-after-inspection", &[901]);
    w.observe("unblocked-after-inspection", 901, "initiation_failed", None)
        .unwrap();
}

#[test]
fn durable_interrupted_intent_replays_without_restarting_deletion() {
    let w = World::new();
    let copy = stored(&w, 901, "one");
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let p = preview(&s, &drafts, None, None);
    let request = command(&p, "restart");
    let mut files = p["files"].clone();
    files[0]["storedPath"] = json!(copy.stored_path);
    files[0]["productIds"] = json!(["booth:90"]);
    let plan = json!({"request":request,"files":files});
    w.store.accept_idempotent_task("library.removeFiles","library-removal-restart",&plan.to_string(),&vua_orchestrator::NewTask {
        task_id:"task-interrupted-removal".into(),correlation_id:"library-removal-restart".into(),occurred_at:"2026-10-08T00:00:00Z".into(),
    },&json!({"schemaVersion":"0.1","taskId":"task-interrupted-removal","acceptedRevision":1,"initialState":"queued"})).unwrap();
    let runtime = TaskRuntime::with_sqlite(
        w.store.clone(),
        Arc::new(SystemClock),
        Arc::new(NanosTaskIdGenerator::default()),
    )
    .unwrap();
    let resumed = LibraryMaintenance::new(
        w.store.clone(),
        w.bdl.clone(),
        runtime,
        w.base.join("warehouse"),
        w.service.clone(),
    );
    let replay = resumed
        .apply("library.removeFiles", request, &drafts, None)
        .unwrap();
    assert_eq!(replay["state"], "unconfirmed");
    assert_eq!(replay["recoveryDisposition"], "inspect_required");
    assert_eq!(replay["files"][0]["phase"], "pending");
    assert!(Path::new(&copy.stored_path).exists());
    assert_eq!(
        w.store
            .task("task-interrupted-removal")
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    assert_eq!(w.service.apply("library.beginDownload",json!({"schemaVersion":"0.1","batchId":"library-download-busy","productId":"booth:90","downloadableIds":[901]}),|_|{}).unwrap_err().0,"file_busy");

    // A later catalog binding may point at the same file under another path and
    // product. Plans persisted before alias IDs were recorded still fence it.
    #[cfg(any(windows, unix))]
    {
        let aliased = alias_path(&w, &copy);
        let fixture: Value = serde_json::from_slice(
            &std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../schemas/catalog-sync/v0.3/examples/page.request.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let page = crate::library_page::extract_library_page(
            &fixture["params"]["html"]
                .as_str()
                .unwrap()
                .replace("901", "91"),
        )
        .unwrap();
        let observation = crate::library_page::library_item_to_observation(
            &page.items[0],
            &format!("sha256:{}", "0".repeat(64)),
            "2026-10-08T00:00:01Z",
            None,
            Some("bought"),
        );
        w.bdl.record_product_observation(&observation).unwrap();
        w.bdl
            .upsert_product_downloadables(
                "booth:91",
                &[(903, "aliased-notes.pdf".into())],
                "2026-10-08T00:00:01Z",
                None,
                Some("bought"),
            )
            .unwrap();
        w.bdl
            .record_managed_library_delivery(&vua_bdl_store::ManagedLibraryDelivery {
                downloadable_id: 903,
                item_id: "entry-later-alias",
                display_name: "Synthetic alias binding",
                file_name: "aliased-notes.pdf",
                stored_path: aliased.to_str().unwrap(),
                sha: &copy.artifact_sha256,
                download_id: &copy.download_id,
                now: "2026-10-08T00:00:01Z",
            })
            .unwrap();
        assert_eq!(
            w.service
                .apply(
                    "library.beginDownload",
                    json!({"schemaVersion":"0.1","batchId":"library-download-alias-busy","productId":"booth:91","downloadableIds":[903]}),
                    |_| {},
                )
                .unwrap_err()
                .0,
            "file_busy"
        );
        assert!(Path::new(&copy.stored_path).exists());
    }
}

#[test]
fn changed_redownload_preserves_and_marks_old_vpm_but_equal_bytes_do_not() {
    let w = World::new();
    let copy = stored(&w, 901, "one");
    let path = w
        .base
        .join("warehouse")
        .join(&copy.warehouse_item_id)
        .join("old-vpm.zip");
    std::fs::write(&path, b"generated synthetic").unwrap();
    let sha = format!("sha256:{}", hex_lower(&sha256_file(&path).unwrap()));
    w.bdl
        .record_untrusted_artifact(&vua_bdl_store::NewLocalArtifact {
            artifact_sha256: sha.clone(),
            size_bytes: 19,
            suggested_file_name: Some("old-vpm.zip".into()),
            download_id: None,
            first_seen_at: "2026-10-08T00:00:00Z".into(),
        })
        .unwrap();
    let generated = w
        .bdl
        .record_artifact_copy(
            &copy.warehouse_item_id,
            &sha,
            "old-vpm.zip",
            path.to_str().unwrap(),
            CopyRole::GeneratedVpm,
            "2026-10-08T00:00:00Z",
        )
        .unwrap();
    stored(&w, 901, "same");
    assert!(!w
        .bdl
        .generated_copy_is_superseded(&copy.warehouse_item_id, &sha)
        .unwrap());
    w.begin("changed", &[901]);
    w.delivery(901, "dl-changed", b"new original bytes", "notes.pdf");
    w.observe("changed", 901, "settled", Some("dl-changed"))
        .unwrap();
    w.wait("changed");
    assert!(w
        .bdl
        .generated_copy_is_superseded(&copy.warehouse_item_id, &sha)
        .unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), b"generated synthetic");
    let copies = w.bdl.library_copy_evidence().unwrap();
    let old = copies
        .iter()
        .find(|c| c.copy.copy_id == generated.copy_id)
        .unwrap();
    assert!(old.superseded);
    assert!(old.product_ids.contains(&"booth:90".into()));
    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    assert!(preview(&s, &drafts, None, None)["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["superseded"] == true));
}

#[test]
fn draft_materials_for_line_verifies_on_disk_hashes_and_expanded_zips_without_role_inference() {
    let w = World::new();
    // Store an expanded ZIP under booth:90 (item 901) and a standalone .unitypackage local copy + a .pdf copy.
    let zip_payload = zip_bytes(&[
        (
            "Avatar/Base.unitypackage",
            b"synthetic avatar package bytes",
        ),
        ("Avatar/Readme.txt", b"notes"),
    ]);
    w.begin("zip-dl", &[901]);
    w.delivery(901, "dl-zip", &zip_payload, "bundle.zip");
    w.observe("zip-dl", 901, "settled", Some("dl-zip")).unwrap();
    w.wait("zip-dl");

    let copies = w.bdl.library_copy_evidence().unwrap();
    let zip_copy = copies
        .iter()
        .find(|c| c.copy.relative_path.ends_with("bundle.zip"))
        .unwrap();
    let exp_copy = copies
        .iter()
        .find(|c| c.copy.relative_path.ends_with("Base.unitypackage"))
        .unwrap();

    // Add a standalone .unitypackage and a .pdf entry to test tampered/missing/no_package cases.
    let standalone_dir = w
        .base
        .join("warehouse")
        .join(&zip_copy.copy.warehouse_item_id);
    let coat_path = standalone_dir.join("Coat.unitypackage");
    std::fs::write(&coat_path, b"synthetic coat package").unwrap();
    let coat_sha = format!("sha256:{}", hex_lower(&sha256_file(&coat_path).unwrap()));
    w.bdl
        .record_untrusted_artifact(&vua_bdl_store::NewLocalArtifact {
            artifact_sha256: coat_sha.clone(),
            size_bytes: 22,
            suggested_file_name: Some("Coat.unitypackage".into()),
            download_id: None,
            first_seen_at: "2026-11-11T00:00:00Z".into(),
        })
        .unwrap();
    let coat_copy = w
        .bdl
        .record_artifact_copy(
            &zip_copy.copy.warehouse_item_id,
            &coat_sha,
            "Coat.unitypackage",
            coat_path.to_str().unwrap(),
            CopyRole::Original,
            "2026-11-11T00:00:00Z",
        )
        .unwrap();

    let pdf_path = standalone_dir.join("Manual.pdf");
    std::fs::write(&pdf_path, b"synthetic manual").unwrap();
    let pdf_sha = format!("sha256:{}", hex_lower(&sha256_file(&pdf_path).unwrap()));
    w.bdl
        .record_untrusted_artifact(&vua_bdl_store::NewLocalArtifact {
            artifact_sha256: pdf_sha.clone(),
            size_bytes: 16,
            suggested_file_name: Some("Manual.pdf".into()),
            download_id: None,
            first_seen_at: "2026-11-11T00:00:00Z".into(),
        })
        .unwrap();
    w.bdl
        .record_artifact_copy(
            &zip_copy.copy.warehouse_item_id,
            &pdf_sha,
            "Manual.pdf",
            pdf_path.to_str().unwrap(),
            CopyRole::Original,
            "2026-11-11T00:00:00Z",
        )
        .unwrap();

    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    drafts
        .apply(
            "recipeDraft.save",
            json!({
                "schemaVersion": "0.1",
                "draftId": "recipe-draft-line-check",
                "title": "Synthetic line prefill",
                "baseRevision": 0,
                "selections": [
                    // 0: local selection pinned to expanded unitypackage hash -> must resolve to parent ZIP member sourceId!
                    {"identity": exp_copy.copy.artifact_sha256, "displayName": "Base Avatar", "source": "local", "warehouseItemId": exp_copy.copy.warehouse_item_id},
                    // 1: standalone coat unitypackage
                    {"identity": coat_sha, "displayName": "Coat", "source": "local", "warehouseItemId": coat_copy.warehouse_item_id},
                    // 2: non-package PDF
                    {"identity": pdf_sha, "displayName": "Manual", "source": "local", "warehouseItemId": zip_copy.copy.warehouse_item_id},
                    // 3: unstored cloud product
                    {"identity": "booth:999999", "displayName": "Unstored", "source": "cloud"}
                ]
            }),
        )
        .unwrap();

    let root = w.base.join("warehouse");
    let resolved = crate::library_maintenance::draft_materials_for_line(
        &w.bdl,
        &root,
        &drafts,
        json!({"schemaVersion": "0.1", "draftId": "recipe-draft-line-check"}),
    )
    .unwrap();
    let schema: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/recipe-selection-draft/v0.1/response.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        jsonschema::validator_for(&schema)
            .unwrap()
            .is_valid(&resolved),
        "{resolved}"
    );
    let sources = resolved["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2, "{resolved}");
    // Neither source carries an inferred role; ZIP member uses <zip_copy_id>-<24hex>.
    assert!(sources.iter().all(|s| s.get("role").is_none()));
    assert!(sources.iter().any(|s| {
        s["sourceId"]
            .as_str()
            .unwrap()
            .starts_with(&format!("{}-", zip_copy.copy.copy_id))
            && s["name"] == "Avatar/Base.unitypackage"
            && s["selectionIndices"] == json!([0])
    }));
    assert!(sources
        .iter()
        .any(|s| { s["sourceId"] == coat_copy.copy_id && s["selectionIndices"] == json!([1]) }));
    assert_eq!(
        resolved["missing"],
        json!([
            {"index": 2, "reason": "no_package"},
            {"index": 3, "reason": "not_stored"}
        ])
    );

    // Same-size tampering of Coat.unitypackage on disk must NOT pass via DB digest: reports content_changed!
    std::fs::write(&coat_path, b"TAMPERED coat package!").unwrap();
    let after_tamper = crate::library_maintenance::draft_materials_for_line(
        &w.bdl,
        &root,
        &drafts,
        json!({"schemaVersion": "0.1", "draftId": "recipe-draft-line-check"}),
    )
    .unwrap();
    assert!(
        after_tamper["missing"]
            .as_array()
            .unwrap()
            .contains(&json!({"index": 1, "reason": "content_changed"})),
        "{after_tamper}"
    );

    // Removing the parent ZIP while keeping the expanded copy falls back to exp_copy.copy_id!
    std::fs::remove_file(&zip_copy.copy.stored_path).unwrap();
    std::fs::remove_file(&coat_path).unwrap();
    let after_zip_removed = crate::library_maintenance::draft_materials_for_line(
        &w.bdl,
        &root,
        &drafts,
        json!({"schemaVersion": "0.1", "draftId": "recipe-draft-line-check"}),
    )
    .unwrap();
    assert_eq!(
        after_zip_removed["sources"][0]["sourceId"],
        exp_copy.copy.copy_id
    );
    assert!(after_zip_removed["missing"]
        .as_array()
        .unwrap()
        .contains(&json!({"index": 1, "reason": "missing"})));
}

#[test]
fn production_recipes_v05_are_protected_in_removal_preview_and_legacy_migration_is_idempotent() {
    let w = World::new();
    let zip_payload = zip_bytes(&[(
        "SyntheticAvatar.unitypackage",
        b"synthetic production package",
    )]);
    w.begin("prod-ref", &[901]);
    w.delivery(901, "dl-prod", &zip_payload, "avatar.zip");
    w.observe("prod-ref", 901, "settled", Some("dl-prod"))
        .unwrap();
    w.wait("prod-ref");

    let copies = w.bdl.library_copy_evidence().unwrap();
    let exp_copy = copies
        .iter()
        .find(|c| {
            c.copy
                .relative_path
                .ends_with("SyntheticAvatar.unitypackage")
        })
        .unwrap();
    let pkg_hex = exp_copy
        .copy
        .artifact_sha256
        .strip_prefix("sha256:")
        .unwrap();

    let s = service(&w);
    let drafts = RecipeSelectionDrafts::new(w.base.join("drafts"));
    let legacy_store = RecipeDocumentStore::new_with_system_clock(w.base.join("legacy-recipes"));
    let prod_store = RecipeDocumentStore::new_with_system_clock(w.base.join("recipes-v04"));

    // Save a valid ProductionRecipe v0.5 referencing pkg_hex.
    prod_store
        .save(
            "recipe-prod-1",
            &json!({
                "formatVersion": "0.5",
                "recipeId": "recipe-prod-1",
                "title": "Modern Production Recipe",
                "editorVersion": "2022.3.22f1",
                "materials": [{
                    "id": "material-1",
                    "packageName": "SyntheticAvatar.unitypackage",
                    "sha256": pkg_hex,
                    "boothProductId": "90"
                }],
                "avatar": {
                    "materialId": "material-1",
                    "prefabPath": "Assets/SyntheticAvatar.prefab"
                },
                "outfits": [{
                    "materialId": "material-1",
                    "prefabPath": "Assets/SyntheticOutfit.prefab",
                    "mode": "merge_armature",
                    "bone": "",
                    "toggleName": "Coat",
                    "active": true
                }],
                "packages": {
                    "com.vrchat.avatars": "3.7.0",
                    "nadena.dev.modular-avatar": "1.10.0"
                }
            }),
            0,
        )
        .unwrap();

    let preview_val = s
        .apply_with_production_recipes(
            "library.removalPreview",
            json!({"schemaVersion": "0.1", "target": {"kind": "product", "id": "booth:90"}}),
            &drafts,
            Some(&legacy_store),
            Some(&prod_store),
        )
        .unwrap();
    assert_eq!(preview_val["referenceCoverage"], "drafts_and_recipes");
    assert_eq!(preview_val["references"].as_array().unwrap().len(), 1);
    assert_eq!(preview_val["references"][0]["kind"], "recipe");
    assert_eq!(preview_val["references"][0]["id"], "recipe-prod-1");
    assert_eq!(preview_val["references"][0]["missingAfterRemoval"], true);

    // Corrupting a production Recipe document fails closed with reference_read_failed instead of claiming zero references.
    std::fs::write(w.base.join("recipes-v04").join("corrupt.json"), b"{bad").unwrap();
    assert_eq!(
        s.apply_with_production_recipes(
            "library.removalPreview",
            json!({"schemaVersion": "0.1", "target": {"kind": "product", "id": "booth:90"}}),
            &drafts,
            Some(&legacy_store),
            Some(&prod_store),
        )
        .unwrap_err()
        .0,
        "reference_read_failed"
    );
    std::fs::remove_file(w.base.join("recipes-v04").join("corrupt.json")).unwrap();

    // Save a legacy Recipe v0.3 document with one booth asset, one verified local asset, one entity-only asset, and graph settings.
    let legacy_doc = json!({
        "formatVersion": "0.3",
        "recipeId": "01950000-0000-7000-8000-000000000099",
        "revision": 1,
        "title": "Legacy Graph Recipe",
        "createdAt": "2026-09-10T00:00:00Z",
        "updatedAt": "2026-09-10T00:00:00Z",
        "environment": {"unityVersionConstraint": "2022.3.22f1"},
        "target": {"platforms": ["windows"], "avatarInstanceId": "base"},
        "assets": [
            {"id": "base", "role": "avatar_base", "label": "Synthetic Cloud Base", "sourceRef": {"provider": "booth", "productId": "90"}},
            {"id": "local_pkg", "role": "outfit", "label": "Synthetic LocalPkg", "sourceRef": {"warehouseItemId": exp_copy.copy.warehouse_item_id}},
            {"id": "mystery", "role": "hair", "entityRef": {"entityId": "01950000-0000-7000-8000-000000000123"}}
        ],
        "instances": [{"id": "base", "assetId": "base", "entrypoint": "Assets/Base.prefab"}],
        "relations": []
    });
    legacy_store
        .save("01950000-0000-7000-8000-000000000099", &legacy_doc, 0)
        .unwrap();

    let root = w.base.join("warehouse");
    let census = crate::library_maintenance::legacy_recipe_census(
        &w.bdl,
        &root,
        &drafts,
        Some(&legacy_store),
        json!({"schemaVersion": "0.1"}),
    )
    .unwrap();
    assert_eq!(census["totalDocuments"], 1);
    assert_eq!(census["parseableDocuments"], 1);
    assert_eq!(census["migratedDrafts"], 0);
    assert_eq!(census["documents"][0]["verifiableSelections"], 2);
    assert_eq!(
        census["documents"][0]["unresolvedAssets"],
        json!([{"assetId": "mystery", "reason": "entity_only"}])
    );
    assert_eq!(census["documents"][0]["hasGraphSettings"], true);

    let first_mig = crate::library_maintenance::migrate_legacy_recipes(
        &w.bdl,
        &root,
        &drafts,
        Some(&legacy_store),
        json!({"schemaVersion": "0.1"}),
    )
    .unwrap();
    assert_eq!(first_mig["migratedCount"], 1);
    assert_eq!(first_mig["items"][0]["status"], "created");
    assert_eq!(first_mig["items"][0]["draftRevision"], 1);

    // Repeating migration is strictly idempotent (unchanged, revision stays 1) and preserves original legacy Recipe v0.3 document!
    let second_mig = crate::library_maintenance::migrate_legacy_recipes(
        &w.bdl,
        &root,
        &drafts,
        Some(&legacy_store),
        json!({"schemaVersion": "0.1"}),
    )
    .unwrap();
    assert_eq!(second_mig["migratedCount"], 0);
    assert_eq!(second_mig["unchangedCount"], 1);
    assert_eq!(second_mig["items"][0]["status"], "unchanged");
    assert_eq!(second_mig["items"][0]["draftRevision"], 1);
    assert_eq!(
        legacy_store
            .get("01950000-0000-7000-8000-000000000099")
            .unwrap()
            .unwrap()
            .recipe,
        legacy_doc
    );
}
