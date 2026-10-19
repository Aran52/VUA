//! Synthetic consumer of library-download v0.1, including real task workers,
//! parent publication, cancellation and filesystem/ledger rollback on Windows.
use serde_json::{json, Value};
use std::io::{BufReader, Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use vua_bdl_store::{
    ArtifactMode, BdlStore, DownloadEventConsumer, DownloadEventKind, DownloadEventV01,
};
use vua_orchestrator::SqliteTaskStore;
use vua_provider_host::{run_provider_host_with_services, DownloadConfig, WarehouseConfig};

/// The ceiling for a wire response or a task to finish. The waits poll and return as soon as
/// the state arrives, so a generous ceiling costs nothing; Windows CI runners under load have
/// taken longer than 5 seconds to finish a zip or removal task.
const WIRE_WAIT: Duration = Duration::from_secs(60);

fn zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        archive.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

#[test]
fn public_booth_search_vectors_cross_the_wire_without_mutating_the_account_library() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/booth-search/v0.1");
    let read = |name: &str| -> Value { serde_json::from_slice(&std::fs::read(root.join(name)).unwrap()).unwrap() };
    let request_schema = jsonschema::validator_for(&read("request.schema.json")).unwrap();
    let response_schema = jsonschema::validator_for(&read("response.schema.json")).unwrap();
    let world = World::new(); let mut host = Host::new(&world.base, world.bdl.clone());
    let before = host.call("library.list", json!({"schemaVersion":"0.1"}));
    for name in ["public", "invalid-source", "invalid-empty"] {
        let request = read(&format!("examples/{name}.request.json"));
        let valid = name == "public";
        assert_eq!(request_schema.is_valid(&request), valid, "{name}");
        let response = host.call("catalog.searchProducts", request["params"].clone());
        assert_eq!(response["ok"], valid, "{response}");
        if valid { assert!(response_schema.is_valid(&response["value"])); assert_eq!(response["value"], read("examples/public.response.json")); }
    }
    let mut mismatch = read("examples/public.request.json")["params"].clone();
    mismatch["query"] = json!("Different");
    assert_eq!(host.call("catalog.searchProducts", mismatch)["ok"], false);
    assert_eq!(host.call("library.list", json!({"schemaVersion":"0.1"}))["value"], before["value"]);
}

#[test]
fn library_intake_wire_keeps_files_expands_zip_and_replays_without_duplicate_entries() {
    let world = World::new();
    let source = world.base.join("migrated-source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("texture.psd"), b"synthetic psd").unwrap();
    std::fs::write(source.join("README"), b"synthetic instructions").unwrap();
    let archive = zip_bytes(&[("sub/Avatar.unitypackage", b"synthetic UnityPackage"), ("sub/preview.jpg", b"synthetic image")]);
    std::fs::write(source.join("package.zip"), &archive).unwrap();
    let params = json!({"schemaVersion":"0.1","sourceFolders":[source]});
    let mut host = Host::new(&world.base, world.bdl.clone());
    let accepted = host.call_command("library.importFolders", params.clone(), Some("import-replay"));
    assert_eq!(accepted["ok"], true, "{accepted}");
    let response_schema: Value = serde_json::from_slice(&std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/library-intake/v0.1/response.schema.json")).unwrap()).unwrap();
    assert!(jsonschema::validator_for(&response_schema).unwrap().is_valid(&accepted["value"]));
    let again = host.call_command("library.importFolders", params, Some("import-replay"));
    assert_eq!(again["value"]["taskId"], accepted["value"]["taskId"]);
    let task_id = accepted["value"]["taskId"].as_str().unwrap();
    let start = Instant::now();
    loop {
        let task = world.tasks.task(task_id).unwrap().unwrap();
        if task.state.is_terminal() { assert_eq!(task.state, vua_orchestrator::TaskState::Succeeded); break; }
        assert!(start.elapsed() < WIRE_WAIT);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(world.bdl.warehouse_entry_cards(ArtifactMode::UseOriginalUnitypackage).unwrap().len(), 1);
    assert_eq!(world.bdl.library_copy_evidence().unwrap().len(), 5);
    assert_eq!(std::fs::read(source.join("package.zip")).unwrap(), archive);
    let view = host.call("library.list", json!({"schemaVersion":"0.1","source":"local"}));
    assert_eq!(view["value"]["items"][0]["storage"]["presentCopies"], 5);
    for invalid in [json!({"schemaVersion":"0.1","sourceFolders":[]}), json!({"schemaVersion":"0.1","sourceFolders":["relative"]}), json!({"schemaVersion":"0.1","sourceFolders":[source],"unknown":true})] {
        assert_eq!(host.call("library.importFolders", invalid)["ok"], false);
    }
}

#[test]
fn mixed_file_folder_intake_v02_is_scoped_idempotent_and_consumes_schema_vectors() {
    let world = World::new();
    let folder = world.base.join("drop-folder"); std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("texture.psd"), b"synthetic texture").unwrap();
    let file = world.base.join("single.txt"); std::fs::write(&file, b"only this file").unwrap();
    std::fs::write(world.base.join("unselected.txt"), b"must not import this sibling").unwrap();
    let zip = world.base.join("single.zip");
    let archive = zip_bytes(&[("sub/avatar.unitypackage", b"synthetic package"), ("instructions.txt", b"synthetic instructions")]);
    std::fs::write(&zip, &archive).unwrap();
    let params = json!({"schemaVersion":"0.2","sourcePaths":[folder,file,zip]});
    let mut host = Host::new(&world.base, world.bdl.clone());
    let accepted = host.call_command("library.importPaths", params.clone(), Some("mixed-once"));
    assert_eq!(accepted["ok"], true, "{accepted}");
    let again = host.call_command("library.importPaths", params, Some("mixed-once"));
    assert_eq!(accepted["value"]["taskId"], again["value"]["taskId"]);
    let task_id = accepted["value"]["taskId"].as_str().unwrap(); let start = Instant::now();
    loop {
        let task = world.tasks.task(task_id).unwrap().unwrap();
        if task.state.is_terminal() { assert_eq!(task.state, vua_orchestrator::TaskState::Succeeded); break; }
        assert!(start.elapsed() < WIRE_WAIT); std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(world.bdl.warehouse_entry_cards(ArtifactMode::UseOriginalUnitypackage).unwrap().len(), 3);
    let copies = world.bdl.library_copy_evidence().unwrap();
    assert_eq!(copies.len(), 5);
    assert!(!copies.iter().any(|c| c.copy.relative_path.contains("unselected.txt")));
    assert!(copies.iter().any(|c| c.copy.relative_path.ends_with("sub/avatar.unitypackage")));
    assert_eq!(std::fs::read(&file).unwrap(), b"only this file"); assert_eq!(std::fs::read(&zip).unwrap(), archive);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/library-intake/v0.2");
    let read = |name: &str| -> Value { serde_json::from_slice(&std::fs::read(root.join(name)).unwrap()).unwrap() };
    assert!(jsonschema::validator_for(&read("response.schema.json")).unwrap().is_valid(&accepted["value"]));
    let request_schema = jsonschema::validator_for(&read("request.schema.json")).unwrap();
    for name in ["import", "invalid-relative", "invalid-empty", "invalid-null", "invalid-unknown", "invalid-duplicate"] {
        let request = read(&format!("examples/{name}.request.json"));
        assert_eq!(request_schema.is_valid(&request), name == "import", "{name}");
        if name != "import" { assert_eq!(host.call_command("library.importPaths", request["params"].clone(), Some(name))["ok"], false, "{name}"); }
    }
    // Versioned faces stay distinct; replay with a different selection is a conflict.
    for invalid in [json!({"schemaVersion":"0.1","sourcePaths":[file]}), json!({"schemaVersion":"0.2","sourceFolders":[folder]})] {
        assert_eq!(host.call_command("library.importPaths", invalid, Some("bad-face"))["ok"], false);
    }
    assert_eq!(host.call_command("library.importPaths", json!({"schemaVersion":"0.2","sourcePaths":[file]}), Some("mixed-once"))["ok"], false);
    assert_eq!(world.bdl.warehouse_entry_cards(ArtifactMode::UseOriginalUnitypackage).unwrap().len(), 3);
}

#[test]
fn mixed_intake_refuses_warehouse_recursion_before_creating_an_entry() {
    let world = World::new(); let mut host = Host::new(&world.base, world.bdl.clone());
    let accepted = host.call_command("library.importPaths", json!({"schemaVersion":"0.2","sourcePaths":[world.base]}), Some("recursive-drop"));
    assert_eq!(accepted["ok"], true);
    let start = Instant::now(); let task_id = accepted["value"]["taskId"].as_str().unwrap();
    loop {
        let task = world.tasks.task(task_id).unwrap().unwrap();
        if task.state.is_terminal() { assert_eq!(task.state, vua_orchestrator::TaskState::Failed); break; }
        assert!(start.elapsed() < WIRE_WAIT); std::thread::sleep(Duration::from_millis(5));
    }
    assert!(world.bdl.warehouse_entry_cards(ArtifactMode::UseOriginalUnitypackage).unwrap().is_empty());
}

#[test]
fn managed_zip_download_wire_exposes_members_and_original_separately() {
    let world = World::new();
    let mut host = Host::new(&world.base, world.bdl.clone());
    let archive = zip_bytes(&[("Avatar.unitypackage", b"synthetic UnityPackage"), ("PSD/texture.psd", b"synthetic psd")]);
    assert_eq!(host.batch("zip", &[902])["ok"], true);
    world.delivery_named(902, "dl-zip", &archive, "bundle.zip");
    assert_eq!(host.observe("zip", 902, "dl-zip")["ok"], true);
    assert_eq!(host.wait("zip", &world.tasks)["state"], "succeeded");
    let parent = world.bdl.managed_library_file(902).unwrap().unwrap();
    assert_eq!(std::fs::read(parent.stored_path).unwrap(), archive);
    assert_eq!(world.bdl.archive_members(&parent.copy_id).unwrap().len(), 2);
    let view = host.call("library.list", json!({"schemaVersion":"0.1"}));
    assert_eq!(view["value"]["items"][0]["storage"]["presentCopies"], 3);
    assert_eq!(view["value"]["items"][0]["storage"]["productionQualification"], "not_evaluated");
}

fn vector(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/library-download/v0.1")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn interrupted_removal_wire_discovers_and_resolves_without_deleting_the_file() {
    let world = World::new();
    let mut host = Host::new(&world.base, world.bdl.clone());
    host.batch("inspect-original", &[901]);
    world.delivery(901, "dl-inspect-original", b"synthetic original");
    host.observe("inspect-original", 901, "dl-inspect-original"); host.wait("inspect-original", &world.tasks);
    let stored = world.bdl.managed_library_file(901).unwrap().unwrap();
    let target = json!({"kind":"product","id":"booth:90"});
    let preview = host.call("library.removalPreview", json!({"schemaVersion":"0.1","target":target}));
    assert_eq!(preview["ok"], true, "{preview}");
    let remove = json!({"schemaVersion":"0.1","removalId":"library-removal-before-restart","target":target,"copyIds":[stored.copy_id],"previewHash":preview["value"]["previewHash"]});
    let mut file = preview["value"]["files"][0].clone();
    file["storedPath"] = json!(stored.stored_path); file["productIds"] = json!(["booth:90"]);
    let plan = json!({"request":remove,"files":[file]});
    drop(host);
    world.tasks.accept_idempotent_task("library.removeFiles","library-removal-before-restart",&plan.to_string(),&vua_orchestrator::NewTask {
        task_id:"task-wire-interrupted-removal".into(),correlation_id:"library-removal-before-restart".into(),occurred_at:"t".into()
    },&json!({"schemaVersion":"0.1","taskId":"task-wire-interrupted-removal","acceptedRevision":1,"initialState":"queued"})).unwrap();
    let mut host = Host::new(&world.base, world.bdl.clone());
    let pending = host.call("library.pendingRemovals", json!({"schemaVersion":"0.1","target":target}));
    assert_eq!(pending["value"]["items"][0]["state"], "unconfirmed");
    let params = json!({"schemaVersion":"0.1","removalId":"library-removal-before-restart","observedRevision":1});
    let resolved = host.call("library.resolveRemoval", params.clone());
    assert_eq!(resolved["ok"], true, "{resolved}");
    assert_eq!(resolved["value"]["inspectionResolved"], true);
    assert_eq!(resolved["value"]["files"][0]["phase"], "kept");
    assert_eq!(resolved["value"]["taskState"], "cancelled");
    let schema: Value = serde_json::from_slice(&std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/library-maintenance/v0.1/response.schema.json")).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&pending["value"]));
    assert!(validator.is_valid(&resolved["value"]));
    assert_eq!(host.call("library.resolveRemoval", params)["value"], resolved["value"]);
    assert_eq!(std::fs::read(&stored.stored_path).unwrap(), b"synthetic original");
    assert!(host.call("library.pendingRemovals", json!({"schemaVersion":"0.1","target":target}))["value"]["items"].as_array().unwrap().is_empty());
    assert_eq!(host.batch("unblocked", &[901])["ok"], true);
    assert_eq!(host.call("library.observeDownload", json!({"schemaVersion":"0.1","batchId":"library-download-unblocked","downloadableId":901,"outcome":"initiation_failed"}))["ok"], true);
}

#[test]
fn public_provenance_association_persists_without_account_membership_or_extra_cloud_card() {
    let world = World::new();
    let html = "<ul><li class='item-card' data-product-id='99' data-product-name='Public synthetic product'><h2 class='item-card__title'>Public synthetic product</h2></li></ul>";
    let page = vua_acquisition::extract_library_page(html).unwrap();
    world.bdl.record_product_observation(&vua_acquisition::library_item_to_observation(
        &page.items[0], &format!("sha256:{}", "1".repeat(64)), "t", None, None,
    )).unwrap();
    let entry = world.bdl.create_warehouse_item("Local folder", "imported_material", "t").unwrap();
    let mut host = Host::new(&world.base, world.bdl.clone());
    let result = host.call_command("library.updateEntryMetadata", json!({"schemaVersion":"0.1","entryId":entry.warehouse_item_id,"expectedRevision":0,"displayName":"Local folder","productId":"booth:99","thumbnailRef":null}), Some("public-source-only"));
    assert_eq!(result["ok"], true, "{result}");
    assert!(world.bdl.product_library_memberships("booth:99").unwrap().is_empty());
    let view = host.call("library.list", json!({"schemaVersion":"0.1"}));
    let rows = view["value"]["items"].as_array().unwrap();
    let local = rows.iter().find(|row| row["entry"]["warehouseItemId"] == entry.warehouse_item_id).unwrap();
    assert_eq!(local["sourceMatch"]["basis"], "mapping"); assert_eq!(local["sourceMatch"]["sources"], json!([]));
    assert!(!rows.iter().any(|row| row["kind"] == "product" && row["product"]["productId"] == "booth:99"));
    drop(host);
    let reopened = BdlStore::open(world.base.join("bdl.db")).unwrap();
    assert_eq!(reopened.library_entry_metadata(&entry.warehouse_item_id).unwrap().product_id.as_deref(), Some("booth:99"));
}

#[test]
fn metadata_wire_has_atomic_replay_and_persistent_local_only_edits() {
    let world = World::new();
    let entry = world.bdl.create_warehouse_item("Local folder", "imported_material", "t").unwrap();
    let account = world.bdl.create_warehouse_item("Account download", "downloaded_material", "t").unwrap();
    let mut host = Host::new(&world.base, world.bdl.clone());
    let params = json!({"schemaVersion":"0.1","entryId":entry.warehouse_item_id,"expectedRevision":0,"displayName":"Texture","productId":"booth:90","thumbnailRef":format!("vua-img://local/{}", "a".repeat(64))});
    let before = host.call("library.entryMetadata", json!({"schemaVersion":"0.1","entryId":entry.warehouse_item_id}));
    assert_eq!(before["value"]["revision"], 0);
    let edited = host.call_command("library.updateEntryMetadata", params.clone(), Some("source-edit"));
    assert_eq!(edited["ok"], true, "{edited}");
    let schema: Value = serde_json::from_slice(&std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/library-entry-metadata/v0.1/response.schema.json")).unwrap()).unwrap();
    assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&edited["value"]));
    assert_eq!(host.call_command("library.updateEntryMetadata", params.clone(), Some("source-edit"))["value"], edited["value"]);
    assert_eq!(host.call("library.updateEntryMetadata", params)["error"]["code"], "vua.library.metadata_conflict");
    let invalid_account = json!({"schemaVersion":"0.1","entryId":account.warehouse_item_id,"expectedRevision":0,"displayName":"Fake","productId":null,"thumbnailRef":null});
    assert_eq!(host.call("library.updateEntryMetadata", invalid_account)["error"]["code"], "vua.library.entry_not_local");
    assert_eq!(host.call("library.entryMetadata", json!({"schemaVersion":"0.1","entryId":entry.warehouse_item_id,"unknown":true}))["error"]["code"], "vua.library.invalid_params");
    let view = host.call("library.list", json!({"schemaVersion":"0.1"}));
    let local = view["value"]["items"].as_array().unwrap().iter().find(|row| row["entry"]["warehouseItemId"] == entry.warehouse_item_id).unwrap();
    assert_eq!(local["metadata"], edited["value"]);
    assert_eq!(local["sourceMatch"]["content"], "unverified");
    let view_schema: Value = serde_json::from_slice(&std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/library-view/v0.1/response.schema.json")).unwrap()).unwrap();
    assert!(jsonschema::validator_for(&view_schema).unwrap().is_valid(&view["value"]));
    drop(host);
    let reopened = BdlStore::open(world.base.join("bdl.db")).unwrap();
    assert_eq!(reopened.library_entry_metadata(&entry.warehouse_item_id).unwrap().revision, 1);
    assert_eq!(reopened.product_library_memberships("booth:90").unwrap(), vec!["bought"]);
}
struct Lines {
    receiver: mpsc::Receiver<Vec<u8>>,
    current: Cursor<Vec<u8>>,
}

#[test]
fn imported_source_and_card_removal_keep_files_refs_account_and_restart_tombstones() {
    let world = World::new();
    let entry = world.bdl.create_warehouse_item("An empty local import", "imported_material", "t").unwrap();
    let mut host = Host::new(&world.base, world.bdl.clone());
    assert_eq!(host.batch("card-test", &[901])["ok"], true);
    world.delivery(901, "dl-card-test", b"synthetic account download");
    assert_eq!(host.observe("card-test", 901, "dl-card-test")["ok"], true);
    assert_eq!(host.wait("card-test", &world.tasks)["state"], "succeeded");
    let copy = world.bdl.managed_library_file(901).unwrap().unwrap();
    let draft = host.call("recipeDraft.save", json!({"schemaVersion":"0.1","draftId":"recipe-draft-card-preserved","title":"Synthetic retained reference","baseRevision":0,
        "selections":[{"identity":"booth:90","displayName":"Synthetic account product","source":"cloud"}]}));
    assert_eq!(draft["ok"], true, "{draft}");
    let imported = host.call("library.list", json!({"schemaVersion":"0.1","source":"imported"}));
    assert_eq!(imported["value"]["total"], 1);
    assert_eq!(imported["value"]["items"][0]["entry"]["warehouseItemId"], entry.warehouse_item_id);
    assert_eq!(host.call("library.list", json!({"schemaVersion":"0.1","source":"local"}))["value"]["items"][0]["product"]["productId"], "booth:90");
    let before = host.call("library.list", json!({"schemaVersion":"0.1"}))["value"].clone();
    let invalid = json!({"schemaVersion":"0.2","targets":[{"kind":"entry","id":entry.warehouse_item_id},{"kind":"product","id":"booth:404"}]});
    assert_eq!(host.call_command("library.removeCards", invalid, Some("bad-card-removal"))["error"]["code"], "vua.library.card_not_found");
    assert_eq!(host.call("library.list", json!({"schemaVersion":"0.1"}))["value"], before);
    // A physically missing account file still has a removable product card.
    assert!(std::path::Path::new(&copy.stored_path).starts_with(&world.base));
    std::fs::remove_file(&copy.stored_path).unwrap();
    assert_eq!(host.call("library.list", json!({"schemaVersion":"0.1","state":"missing"}))["value"]["total"], 1);
    let schema_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/library-records/v0.2");
    let read = |name: &str| -> Value { serde_json::from_slice(&std::fs::read(schema_root.join(name)).unwrap()).unwrap() };
    let mut request = read("examples/remove.request.json"); request["params"]["targets"][0]["id"] = json!(entry.warehouse_item_id);
    assert!(jsonschema::validator_for(&read("request.schema.json")).unwrap().is_valid(&request));
    let removed = host.call_command("library.removeCards", request["params"].clone(), Some("remove-cards-once"));
    assert_eq!(removed["ok"], true, "{removed}");
    assert!(jsonschema::validator_for(&read("response.schema.json")).unwrap().is_valid(&removed["value"]));
    assert_eq!(host.call_command("library.removeCards", request["params"].clone(), Some("remove-cards-once"))["value"], removed["value"]);
    let changed = json!({"schemaVersion":"0.2","targets":[{"kind":"product","id":"booth:90"}]});
    assert_eq!(host.call_command("library.removeCards", changed, Some("remove-cards-once"))["error"]["code"], "vua.library.record_conflict");
    assert_eq!(host.call("library.list", json!({"schemaVersion":"0.1"}))["value"]["total"], 0);
    assert!(world.bdl.warehouse_entry_detail(&entry.warehouse_item_id, ArtifactMode::UseOriginalUnitypackage).unwrap().is_some());
    assert!(world.bdl.managed_library_file(901).unwrap().is_some());
    assert_eq!(world.bdl.product_library_memberships("booth:90").unwrap(), vec!["bought"]);
    assert_eq!(host.call("recipeDraft.get", json!({"schemaVersion":"0.1","draftId":"recipe-draft-card-preserved"}))["value"]["document"], draft["value"]["document"]);
    drop(host);
    let reopened = Arc::new(BdlStore::open(world.base.join("bdl.db")).unwrap());
    reopened.upsert_product_downloadables("booth:90", &[(901,"new product observation.pdf".into())], "t2", None, Some("bought")).unwrap();
    let mut restarted = Host::new(&world.base, reopened);
    assert_eq!(restarted.call("library.list", json!({"schemaVersion":"0.1"}))["value"]["total"], 0);
    assert_eq!(restarted.call_command("library.removeCards", request["params"].clone(), Some("remove-cards-once"))["value"], removed["value"]);
    assert_eq!(restarted.call("recipeDraft.get", json!({"schemaVersion":"0.1","draftId":"recipe-draft-card-preserved"}))["value"]["document"], draft["value"]["document"]);
}

#[test]
fn local_record_removal_wire_keeps_account_and_warehouse_identities() {
    let world = World::new();
    let entry = world.bdl.create_warehouse_item("Local", "imported_material", "t").unwrap();
    let account = world.bdl.create_warehouse_item("Account", "downloaded_material", "t").unwrap();
    let mut host = Host::new(&world.base, world.bdl.clone());
    let params = json!({"schemaVersion":"0.1","entryIds":[entry.warehouse_item_id]});
    let rejected = host.call("library.removeLocalEntries", json!({"schemaVersion":"0.1","entryIds":[entry.warehouse_item_id,account.warehouse_item_id]}));
    assert_eq!(rejected["error"]["code"], "vua.library.entry_not_local");
    assert!(world.bdl.removed_local_entries().unwrap().is_empty());
    let removed = host.call_command("library.removeLocalEntries", params.clone(), Some("remove-local"));
    assert_eq!(removed["ok"], true, "{removed}");
    let schema: Value = serde_json::from_slice(&std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/library-records/v0.1/response.schema.json")).unwrap()).unwrap();
    assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&removed["value"]));
    assert_eq!(host.call_command("library.removeLocalEntries", params, Some("remove-local"))["value"], removed["value"]);
    assert_eq!(host.call_command("library.removeLocalEntries", json!({"schemaVersion":"0.1","entryIds":[account.warehouse_item_id]}), Some("remove-local"))["error"]["code"], "vua.library.record_conflict");
    let view = host.call("library.list", json!({"schemaVersion":"0.1"}));
    assert!(!view["value"]["items"].as_array().unwrap().iter().any(|row| row["entry"]["warehouseItemId"] == entry.warehouse_item_id));
    assert!(world.bdl.warehouse_entry_detail(&entry.warehouse_item_id, ArtifactMode::UseOriginalUnitypackage).unwrap().is_some());
    assert_eq!(world.bdl.product_library_memberships("booth:90").unwrap(), vec!["bought"]);
}
impl Read for Lines {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let read = self.current.read(output)?;
            if read > 0 {
                return Ok(read);
            }
            match self.receiver.recv() {
                Ok(line) => self.current = Cursor::new(line),
                Err(_) => return Ok(0),
            }
        }
    }
}
#[derive(Clone)]
struct SharedOutput(Arc<Mutex<Vec<u8>>>);
impl Write for SharedOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct Host {
    sender: Option<mpsc::Sender<Vec<u8>>>,
    output: SharedOutput,
    handle: Option<std::thread::JoinHandle<()>>,
    sequence: u64,
}
impl Host {
    fn new(base: &std::path::Path, bdl: Arc<BdlStore>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let output = SharedOutput(Arc::new(Mutex::new(Vec::new())));
        let mut writer = output.clone();
        let database = base.join("tasks.db");
        let warehouse = WarehouseConfig {
            bdl: bdl.clone(),
            warehouse_root: base.join("warehouse"),
            global_default: ArtifactMode::UseOriginalUnitypackage,
            executor: None,
            dependencies_queries: None,
        };
        let handle = std::thread::spawn(move || {
            run_provider_host_with_services(
                BufReader::new(Lines {
                    receiver,
                    current: Cursor::new(Vec::new()),
                }),
                &mut writer,
                database,
                None,
                Some(DownloadConfig { bdl }),
                Some(warehouse),
                None,
            )
            .unwrap()
        });
        Self {
            sender: Some(sender),
            output,
            handle: Some(handle),
            sequence: 0,
        }
    }
    fn frames(&self) -> Vec<Value> {
        String::from_utf8(self.output.0.lock().unwrap().clone())
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
    fn call(&mut self, method: &str, params: Value) -> Value {
        self.call_command(method, params, None)
    }
    fn call_command(&mut self, method: &str, params: Value, command_id: Option<&str>) -> Value {
        self.sequence += 1;
        let id = format!("request-{}", self.sequence);
        let query = matches!(
            method,
            "library.downloadStatus"
                | "catalog.searchProducts"
                | "library.entryMetadata"
                | "library.list"
                | "library.productFiles"
                | "recipeDraft.list"
                | "recipeDraft.get"
                | "recipeDraft.selectionStatus"
                | "library.removalPreview"
                | "library.removalStatus"
                | "library.pendingRemovals"
                | "app.snapshot"
        );
        let mut request = json!({"contractVersion":"0.1","requestId":id,"correlationId":id,"kind":if query {"query"} else {"command"},"method":method,"params":params});
        if !query {
            request["commandId"] = json!(command_id.unwrap_or(&id));
        }
        let frame = json!({"frameVersion":"0.1","frameId":id,"kind":"request","payload":request});
        self.sender
            .as_ref()
            .unwrap()
            .send(format!("{frame}\n").into_bytes())
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(frame) = self
                .frames()
                .into_iter()
                .find(|frame| frame["kind"] == "response" && frame["payload"]["requestId"] == id)
            {
                return frame["payload"].clone();
            }
            assert!(
                start.elapsed() < WIRE_WAIT,
                "wire response missing: {method}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn batch(&mut self, name: &str, ids: &[i64]) -> Value {
        self.call("library.beginDownload", json!({"schemaVersion":"0.1","batchId":format!("library-download-{name}"),"productId":"booth:90","downloadableIds":ids}))
    }
    fn observe(&mut self, name: &str, id: i64, delivery: &str) -> Value {
        self.call("library.observeDownload", json!({"schemaVersion":"0.1","batchId":format!("library-download-{name}"),"downloadableId":id,"outcome":"settled","downloadId":delivery}))
    }
    fn wait(&mut self, name: &str, tasks: &SqliteTaskStore) -> Value {
        let start = Instant::now();
        let id = format!("library-download-{name}");
        loop {
            if tasks.task(&id).unwrap().unwrap().state.is_terminal() {
                return self.call(
                    "library.downloadStatus",
                    json!({"schemaVersion":"0.1","batchId":id}),
                )["value"]
                    .clone();
            }
            assert!(
                start.elapsed() < WIRE_WAIT,
                "parent completion was not published"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }
}
struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        if self.0.starts_with(std::env::temp_dir()) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
struct World {
    base: PathBuf,
    bdl: Arc<BdlStore>,
    tasks: SqliteTaskStore,
    _cleanup: Cleanup,
}
impl World {
    fn new() -> Self {
        // Windows clock resolution can repeat under parallel tests.
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "vua-library-wire-{}-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&base).unwrap();
        let bdl = Arc::new(BdlStore::open(base.join("bdl.db")).unwrap());
        let fixture: Value = serde_json::from_slice(
            &std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../schemas/catalog-sync/v0.3/examples/page.request.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let page = vua_acquisition::extract_library_page(
            &fixture["params"]["html"]
                .as_str()
                .unwrap()
                .replace("901", "90"),
        )
        .unwrap();
        bdl.record_product_observation(&vua_acquisition::library_item_to_observation(
            &page.items[0],
            &format!("sha256:{}", "0".repeat(64)),
            "2026-10-08T00:00:00Z",
            None,
            Some("bought"),
        ))
        .unwrap();
        bdl.upsert_product_downloadables(
            "booth:90",
            &[(901, "synthetic.pdf".into()), (902, "synthetic.zip".into())],
            "2026-10-08T00:00:00Z",
            None,
            Some("bought"),
        )
        .unwrap();
        let tasks = SqliteTaskStore::open(base.join("tasks.db")).unwrap();
        Self {
            base: base.clone(),
            bdl,
            tasks,
            _cleanup: Cleanup(base),
        }
    }
    fn delivery(&self, id: i64, delivery: &str, content: &[u8]) {
        self.delivery_named(id, delivery, content, "synthetic.pdf");
    }
    fn delivery_named(&self, id: i64, delivery: &str, content: &[u8], name: &str) {
        let root = self.base.join("downloads-staging");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join(delivery);
        std::fs::write(&file, content).unwrap();
        for kind in [DownloadEventKind::Started, DownloadEventKind::Completed] {
            DownloadEventConsumer::new(&self.bdl)
                .ingest(&DownloadEventV01 {
                    schema_version: "0.1".into(),
                    kind,
                    download_id: delivery.into(),
                    attempt: 1,
                    source_url: format!("https://booth.pm/downloadables/{id}"),
                    initiated_from_page_url: None,
                    url_chain: None,
                    suggested_file_name: Some(name.into()),
                    stored_path: if kind == DownloadEventKind::Completed {
                        Some(file.to_string_lossy().into())
                    } else {
                        None
                    },
                    expected_bytes: Some(content.len() as u64),
                    received_bytes: if kind == DownloadEventKind::Completed {
                        Some(content.len() as u64)
                    } else {
                        None
                    },
                    resumable: false,
                    failure_kind: None,
                    occurred_at: if kind == DownloadEventKind::Completed {
                        "2026-10-08T00:00:01Z".into()
                    } else {
                        "2026-10-08T00:00:00Z".into()
                    },
                })
                .unwrap();
        }
    }
}

#[test]
fn schemas_vectors_and_live_admission_agree() {
    let request = jsonschema::validator_for(&vector("request.schema.json")).unwrap();
    let response = jsonschema::validator_for(&vector("response.schema.json")).unwrap();
    for name in [
        "begin.request",
        "settled.request",
        "initiation-failed.request",
        "status.request",
    ] {
        assert!(request.is_valid(&vector(&format!("examples/{name}.json"))));
    }
    for name in [
        "invalid-session.request",
        "invalid-duplicate.request",
        "invalid-unbound.request",
    ] {
        assert!(!request.is_valid(&vector(&format!("examples/{name}.json"))));
    }
    assert!(response.is_valid(&vector("examples/snapshot.response.json")));
    assert!(!response.is_valid(&vector("examples/invalid-stored.response.json")));
    let world = World::new();
    let mut host = Host::new(&world.base, world.bdl.clone());
    for name in ["invalid-session.request", "invalid-duplicate.request"] {
        let v = vector(&format!("examples/{name}.json"));
        assert_eq!(
            host.call(v["method"].as_str().unwrap(), v["params"].clone())["error"]["code"],
            "vua.library.invalid_params"
        );
    }
    assert!(world
        .tasks
        .task("library-download-synthetic")
        .unwrap()
        .is_none());
    let v = vector("examples/begin.request.json");
    let accepted = host.call(v["method"].as_str().unwrap(), v["params"].clone());
    assert!(response.is_valid(&accepted["value"]));
    assert_eq!(accepted["value"]["files"].as_array().unwrap().len(), 2);
}

#[test]
fn managed_removal_schema_wire_references_finality_and_restart_agree() {
    let read = |name: &str| -> Value {
        serde_json::from_slice(
            &std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../schemas/library-maintenance/v0.1")
                    .join(name),
            )
            .unwrap(),
        )
        .unwrap()
    };
    let request_schema = jsonschema::validator_for(&read("request.schema.json")).unwrap();
    let response_schema = jsonschema::validator_for(&read("response.schema.json")).unwrap();
    for name in [
        "preview.request",
        "remove.request",
        "status.request",
        "invalid-null.request",
        "invalid-empty.request",
        "invalid-duplicate.request",
        "invalid-path.request",
    ] {
        assert_eq!(
            request_schema.is_valid(&read(&format!("examples/{name}.json"))),
            !name.starts_with("invalid"),
            "{name}"
        );
    }
    for name in [
        "preview.response",
        "status.response",
        "recovered.response",
        "invalid-path.response",
        "invalid-finality.response",
        "invalid-result.response",
    ] {
        assert_eq!(
            response_schema.is_valid(&read(&format!("examples/{name}.json"))),
            !name.starts_with("invalid"),
            "{name}"
        );
    }
    let world = World::new();
    world.delivery(901, "dl-removal", b"synthetic notes");
    let mut host = Host::new(&world.base, world.bdl.clone());
    host.batch("removal-source", &[901]);
    host.observe("removal-source", 901, "dl-removal");
    host.wait("removal-source", &world.tasks);
    let copy = world.bdl.managed_library_file(901).unwrap().unwrap();
    let draft=host.call("recipeDraft.save",json!({"schemaVersion":"0.1","draftId":"recipe-draft-removal","title":"Synthetic selection","baseRevision":0,
        "selections":[{"identity":copy.artifact_sha256,"source":"local","displayName":"Synthetic notes","warehouseItemId":copy.warehouse_item_id}]}));
    assert_eq!(draft["ok"], true, "{draft}");
    let p = host.call(
        "library.removalPreview",
        json!({"schemaVersion":"0.1","target":{"kind":"product","id":"booth:90"}}),
    );
    assert_eq!(p["ok"], true, "{p}");
    assert!(response_schema.is_valid(&p["value"]));
    assert_eq!(p["value"]["referenceCoverage"], "drafts_only");
    assert_eq!(p["value"]["references"][0]["id"], "recipe-draft-removal");
    let request = json!({"schemaVersion":"0.1","removalId":"library-removal-wire","target":p["value"]["target"],"copyIds":[copy.copy_id],"previewHash":p["value"]["previewHash"]});
    let accepted = host.call("library.removeFiles", request.clone());
    assert_eq!(accepted["ok"], true, "{accepted}");
    assert!(response_schema.is_valid(&accepted["value"]));
    let start = Instant::now();
    let final_result = loop {
        let response = host.call(
            "library.removalStatus",
            json!({"schemaVersion":"0.1","removalId":"library-removal-wire"}),
        );
        assert_eq!(response["ok"], true);
        assert!(response_schema.is_valid(&response["value"]), "{response}");
        if response["value"]["state"] != "running" {
            break response["value"].clone();
        }
        assert!(start.elapsed() < WIRE_WAIT);
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(final_result["state"], "succeeded");
    assert_eq!(final_result["files"][0]["phase"], "removed");
    assert_eq!(
        host.call("library.list", json!({"schemaVersion":"0.1"}))["value"]["items"][0]["storage"]
            ["state"],
        "missing"
    );
    let status = host.call(
        "recipeDraft.selectionStatus",
        json!({"schemaVersion":"0.1","draftId":"recipe-draft-removal"}),
    );
    assert_eq!(status["ok"], true, "{status}");
    assert_eq!(status["value"]["items"][0]["state"], "missing");
    let draft_schema: Value = serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/recipe-selection-draft/v0.1/response.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(jsonschema::validator_for(&draft_schema)
        .unwrap()
        .is_valid(&status["value"]));
    assert_eq!(
        host.call(
            "recipeDraft.get",
            json!({"schemaVersion":"0.1","draftId":"recipe-draft-removal"})
        )["value"],
        draft["value"]
    );
    drop(host);
    let mut restarted = Host::new(&world.base, world.bdl.clone());
    let replay = restarted.call("library.removeFiles", request);
    assert_eq!(replay["value"]["taskId"], accepted["value"]["taskId"]);
    assert_eq!(replay["value"]["state"], "succeeded");
    assert!(!std::path::Path::new(&copy.stored_path).exists());
}

#[test]
fn library_view_schema_vectors_and_real_queries_agree() {
    let read = |name: &str| -> Value {
        serde_json::from_slice(
            &std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../schemas/library-view/v0.1")
                    .join(name),
            )
            .unwrap(),
        )
        .unwrap()
    };
    let request = jsonschema::validator_for(&read("request.schema.json")).unwrap();
    let response = jsonschema::validator_for(&read("response.schema.json")).unwrap();
    for name in ["list.request", "files.request", "description-links.request", "category.request", "unknown-category.request"] {
        assert!(request.is_valid(&read(&format!("examples/{name}.json"))));
    }
    for name in ["invalid-session.request", "invalid-limit.request"] {
        assert!(!request.is_valid(&read(&format!("examples/{name}.json"))));
    }
    for name in ["list.response", "files.response", "local-source.response", "category.response"] {
        assert!(
            response.is_valid(&read(&format!("examples/{name}.json"))),
            "{name}"
        );
    }
    assert!(!response.is_valid(&read("examples/invalid-path.response.json")));
    assert!(!request.is_valid(&read("examples/invalid-category.request.json")));
    let world = World::new();
    let mut host = Host::new(&world.base, world.bdl.clone());
    let list = host.call("library.list", json!({"schemaVersion":"0.1"}));
    assert!(response.is_valid(&list["value"]), "{list}");
    assert_eq!(list["value"]["total"], 1);
    assert!(response.is_valid(
        &host.call(
            "library.productFiles",
            json!({"schemaVersion":"0.1","productId":"booth:90"})
        )["value"]
    ));
    world.delivery(901, "dl-view", b"synthetic");
    host.batch("view", &[901]);
    host.observe("view", 901, "dl-view");
    host.wait("view", &world.tasks);
    let present = host.call("library.list", json!({"schemaVersion":"0.1"}));
    assert!(response.is_valid(&present["value"]), "{present}");
    assert_eq!(present["value"]["items"][0]["storage"]["presentCopies"], 1);
    let files = host.call(
        "library.productFiles",
        json!({"schemaVersion":"0.1","productId":"booth:90"}),
    );
    assert!(response.is_valid(&files["value"]), "{files}");
    assert_eq!(
        host.call("library.list", json!({"schemaVersion":"0.1","limit":201}))["error"]["code"],
        "vua.library.invalid_params"
    );
}

#[test]
fn description_link_filter_and_variant_counts_cross_the_native_wire_and_clear() {
    let world = World::new(); let mut host = Host::new(&world.base, world.bdl.clone());
    let page = |body: &str| json!({"schemaVersion":"0.2","sourceUrl":"https://booth.pm/items/90","fetchedAt":"2026-10-12T00:00:00Z",
        "html":format!(r#"<div id="items" data-product-id="90"><article><div class="summary"><h2>Synthetic outfit</h2></div><div class="js-market-item-detail-description description">{body}</div></article></div>"#)});
    assert_eq!(host.call("catalog.ingestLibraryPage", page(r#"<a href="https://booth.pm/en/items/999"><img src="x"></a>"#))["ok"], true);
    let request = json!({"schemaVersion":"0.1","descriptionLinksTo":"booth:999"});
    let result = host.call("library.list",request.clone()); assert_eq!(result["ok"],true,"{result}"); assert_eq!(result["value"]["total"],1);
    assert_eq!(result["value"]["items"][0]["variantCount"],0);
    for invalid in [Value::Null, json!("booth:x"), json!("https://booth.pm/items/999")] {
        assert_eq!(host.call("library.list",json!({"schemaVersion":"0.1","descriptionLinksTo":invalid}))["error"]["code"],"vua.library.invalid_params");
    }
    assert_eq!(host.call("catalog.ingestLibraryPage",page(""))["ok"],true);
    assert_eq!(host.call("library.list",request)["value"]["total"],0);
}

#[test]
fn sync_finalization_and_download_completion_reconcile_migrated_local_files() {
    let world = World::new();
    let title = world.bdl.library_product_summaries().unwrap()[0].title.clone().unwrap();
    let source = world.base.join("import-source").join(&title);
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("local.zip"), b"synthetic").unwrap();
    let imported = vua_acquisition::WarehouseImporter::new(&world.bdl, &vua_orchestrator::SystemClock, world.base.join("warehouse")).import_folder(&source).unwrap();
    let mut host = Host::new(&world.base, world.bdl.clone());
    assert_eq!(host.call("library.list", json!({"schemaVersion":"0.1"}))["value"]["total"], 2);
    world.delivery(901, "dl-reconcile", b"synthetic");
    host.batch("reconcile", &[901]);
    host.observe("reconcile", 901, "dl-reconcile");
    host.wait("reconcile", &world.tasks);
    let wait_checks = |key: Option<&str>| {
        let started = Instant::now();
        loop {
            let tasks = world.tasks.idempotent_tasks("library.reconcileSources").unwrap();
            if tasks.iter().any(|row| key.is_none_or(|key| row.idempotency_key == key)
                && world.tasks.task(&row.task_id).unwrap().is_some_and(|task| task.state.is_terminal())) { break; }
            assert!(started.elapsed() < WIRE_WAIT, "reconciliation was not dispatched");
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    wait_checks(None);
    let merged = host.call("library.list", json!({"schemaVersion":"0.1"}));
    assert_eq!(merged["value"]["total"], 1);
    assert_eq!(merged["value"]["items"][0]["storage"]["presentCopies"], 2);
    let sync_id = "catalog-sync-migration";
    assert_eq!(host.call("catalog.beginLibrarySync", json!({"schemaVersion":"0.3","runId":sync_id,"libraryTypes":["bought"]}))["ok"], true);
    let page: Value = serde_json::from_slice(&std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/catalog-sync/v0.3/examples/page.request.json")).unwrap()).unwrap();
    let mut params = page["params"].clone(); params["runId"] = json!(sync_id);
    params["html"] = json!(params["html"].as_str().unwrap().replace("901", "90"));
    assert_eq!(host.call("catalog.ingestLibraryPage", params)["ok"], true);
    let finish = json!({"schemaVersion":"0.3","runId":sync_id,"outcome":"completed"});
    assert_eq!(host.call("catalog.finishLibrarySync", finish.clone())["ok"], true);
    wait_checks(Some(&format!("sync:{sync_id}")));
    let before = world.tasks.idempotent_tasks("library.reconcileSources").unwrap().len();
    assert_eq!(host.call("catalog.finishLibrarySync", finish)["ok"], true);
    assert_eq!(world.tasks.idempotent_tasks("library.reconcileSources").unwrap().len(), before);
    assert_eq!(host.call("catalog.beginLibrarySync", json!({"schemaVersion":"0.3","runId":"catalog-sync-cancel-migration","libraryTypes":["bought"]}))["ok"], true);
    let cancelled = host.call("catalog.finishLibrarySync", json!({"schemaVersion":"0.3","runId":"catalog-sync-cancel-migration","outcome":"cancelled"}));
    assert_eq!(cancelled["value"]["state"], "cancelled");
    assert_eq!(world.tasks.idempotent_tasks("library.reconcileSources").unwrap().len(), before, "cancelling sync must not start another verifier");
    assert_eq!(host.call("library.list", json!({"schemaVersion":"0.1"}))["value"]["total"], 1);
    assert!(PathBuf::from(&world.bdl.entry_copies(&imported.entry.warehouse_item_id).unwrap()[0].stored_path).exists());
    assert!(host.frames().iter().any(|frame| frame["kind"] == "event" && frame["payload"]["payload"]["operation"] == "library.reconcileSources"));
}

#[test]
fn recipe_draft_wire_is_separate_durable_and_reports_conflicts() {
    let world = World::new();
    let mut host = Host::new(&world.base, world.bdl.clone());
    assert_eq!(
        host.call("recipeDraft.list", json!({"schemaVersion":"0.1"}))["value"]["entries"],
        json!([])
    );
    let read = |name: &str| -> Value {
        serde_json::from_slice(
            &std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../schemas/recipe-selection-draft/v0.1")
                    .join(name),
            )
            .unwrap(),
        )
        .unwrap()
    };
    let schema = jsonschema::validator_for(&read("response.schema.json")).unwrap();
    let save = read("examples/save.request.json");
    let saved = host.call("recipeDraft.save", save["params"].clone());
    assert_eq!(saved["ok"], true, "{saved}");
    assert!(schema.is_valid(&saved["value"]));
    assert_eq!(
        host.call("recipeDraft.save", save["params"].clone())["error"]["code"],
        "vua.recipe_draft.revision_conflict"
    );
    let add = read("examples/add.request.json");
    assert_eq!(
        host.call("recipeDraft.addSelection", add["params"].clone())["value"]["existingCount"],
        2
    );
    drop(host);
    let mut restarted = Host::new(&world.base, world.bdl.clone());
    let get = read("examples/get.request.json");
    let got = restarted.call("recipeDraft.get", get["params"].clone());
    assert_eq!(got["value"]["revision"], 2);
    assert!(schema.is_valid(&got["value"]));
    let list = restarted.call("recipeDraft.list", json!({"schemaVersion":"0.1"}));
    assert!(schema.is_valid(&list["value"]));
    assert_eq!(list["value"]["entries"].as_array().unwrap().len(), 1);
}

#[test]
fn worker_completion_updates_parent_without_polling_or_global_download_guessing() {
    let world = World::new();
    world.delivery(901, "dl-one", b"synthetic content");
    let mut host = Host::new(&world.base, world.bdl.clone());
    assert_eq!(host.batch("one", &[901])["ok"], true);
    assert_eq!(host.observe("one", 901, "dl-one")["ok"], true);
    let done = host.wait("one", &world.tasks);
    assert_eq!(done["state"], "succeeded", "{done}");
    assert!(jsonschema::validator_for(&vector("response.schema.json"))
        .unwrap()
        .is_valid(&done));
    assert!(host.frames().iter().any(|frame| frame["kind"] == "event"
        && frame["payload"]["taskId"] == "library-download-one"
        && frame["payload"]["state"] == "succeeded"));
}

#[test]
fn metadata_failure_rolls_back_the_replaced_file_and_binding() {
    let world = World::new();
    world.delivery(901, "dl-old", b"old content");
    let mut host = Host::new(&world.base, world.bdl.clone());
    host.batch("old", &[901]);
    host.observe("old", 901, "dl-old");
    host.wait("old", &world.tasks);
    let old = world.bdl.managed_library_file(901).unwrap().unwrap();
    rusqlite::Connection::open(world.base.join("bdl.db")).unwrap().execute_batch("CREATE TRIGGER synthetic_commit_failure BEFORE UPDATE OF artifact_sha256 ON artifact_copies BEGIN SELECT RAISE(FAIL,'synthetic failure'); END;").unwrap();
    world.delivery(901, "dl-new", b"new content");
    host.batch("new", &[901]);
    host.observe("new", 901, "dl-new");
    let done = host.wait("new", &world.tasks);
    assert_eq!(done["state"], "failed");
    assert_eq!(done["files"][0]["errorCode"], "vua.library.store_failed");
    assert_eq!(std::fs::read(&old.stored_path).unwrap(), b"old content");
    let binding = world.bdl.managed_library_file(901).unwrap().unwrap();
    assert_eq!(binding.artifact_sha256, old.artifact_sha256);
    assert_eq!(binding.download_id, "dl-old");
}

#[test]
fn cancellation_before_inspection_keeps_the_prior_file() {
    let world = World::new();
    world.delivery(901, "dl-old", b"old content");
    let mut host = Host::new(&world.base, world.bdl.clone());
    host.batch("old", &[901]);
    host.observe("old", 901, "dl-old");
    host.wait("old", &world.tasks);
    let old = world.bdl.managed_library_file(901).unwrap().unwrap();
    host.batch("cancel", &[901, 902]);
    host.call(
        "task.requestCancellation",
        json!({"taskId":"library-download-cancel"}),
    );
    world.delivery(901, "dl-new", b"new content");
    host.observe("cancel", 901, "dl-new");
    host.call("library.observeDownload", json!({"schemaVersion":"0.1","batchId":"library-download-cancel","downloadableId":902,"outcome":"cancelled"}));
    assert_eq!(host.wait("cancel", &world.tasks)["state"], "cancelled");
    assert_eq!(std::fs::read(old.stored_path).unwrap(), b"old content");
}
