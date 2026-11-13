use super::*;
use serde_json::{json, Value};
use vua_orchestrator::{RecipeDocumentStore, RecipeSaveError};

fn document(version: &str, id: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../schemas/amf-recipe/v{version}/examples/recipe.valid.json"
    ));
    let mut value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    value["recipeId"] = id.into();
    value
}
fn store(h: &Harness) -> RecipeDocumentStore {
    RecipeDocumentStore::new_with_system_clock(h.root.join("production/recipes-v04"))
}
fn assembled(h: &Harness) {
    h.prepared();
    let digest = h.engine.plan("n3-test", &selection()).unwrap()["planDigest"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx)));
}

#[test]
fn rename_preserves_both_formats_and_conflicts_without_side_effects() {
    let h = Harness::new();
    for (version, id) in [("0.4", "recipe-old"), ("0.5", "recipe-new")] {
        let original = document(version, id);
        store(&h).save(id, &original, 0).unwrap();
        let renamed = h.engine.rename_recipe(id, 1, "New title").unwrap();
        let mut expected = original;
        expected["title"] = "New title".into();
        assert_eq!(renamed.recipe, expected);
        assert_eq!(renamed.revision, 2);
        let error = h.engine.rename_recipe(id, 1, "Stale title").unwrap_err();
        assert_eq!(error.code, "vua.amf_production.recipe_revision_conflict");
        assert_eq!(
            serde_json::to_value(error).unwrap()["params"]["currentRevision"],
            2.0
        );
        assert_eq!(h.engine.get_recipe(id).unwrap(), renamed);
        for title in ["".into(), " ".into(), "Bad\nname".into(), "😀".repeat(121)] {
            assert_eq!(
                h.engine.rename_recipe(id, 2, &title).unwrap_err().code,
                "vua.amf_production.recipe_invalid"
            );
            assert_eq!(h.engine.get_recipe(id).unwrap(), renamed);
        }
        assert_eq!(
            h.engine
                .rename_recipe(id, 2, &"😀".repeat(120))
                .unwrap()
                .revision,
            3
        );
    }
    let error = h.engine.rename_recipe("missing", 1, "Name").unwrap_err();
    assert_eq!(
        serde_json::to_value(error).unwrap()["params"]["currentRevision"],
        0.0
    );
}

#[test]
fn store_and_engine_deletion_require_the_exact_revision() {
    let h = Harness::new();
    let id = "recipe-delete";
    store(&h).save(id, &document("0.5", id), 0).unwrap();
    assert_eq!(
        store(&h).delete(id, 2),
        Err(RecipeSaveError::RevisionConflict {
            current_revision: 1
        })
    );
    assert_eq!(
        store(&h).delete("missing", 0),
        Err(RecipeSaveError::RevisionConflict {
            current_revision: 0
        })
    );
    assert_eq!(
        store(&h).delete("../escape", 1),
        Err(RecipeSaveError::InvalidId)
    );
    assert_eq!(
        h.engine.delete_recipe(id, 2).unwrap_err().code,
        "vua.amf_production.recipe_revision_conflict"
    );
    h.engine.delete_recipe(id, 1).unwrap();
    assert!(store(&h).get(id).unwrap().is_none());
    assert_eq!(
        h.engine.delete_recipe(id, 1).unwrap_err().code,
        "vua.amf_production.recipe_revision_conflict"
    );
}

#[test]
fn deletion_protects_unconfirmed_confirmed_and_unlisted_lines() {
    let h = Harness::new();
    assembled(&h);
    h.engine
        .save_recipe("n3-test", "recipe-used", "Avatar")
        .unwrap();
    h.engine
        .begin_recipe("n3-from-recipe", "recipe-used", "synthetic-editor")
        .unwrap();
    assert!(h.job(|engine, ctx| engine.prepare("n3-from-recipe", ctx)));
    assert!(h
        .engine
        .get("n3-from-recipe")
        .unwrap()
        .confirmed_digest
        .is_none());
    assert_eq!(
        h.engine.delete_recipe("recipe-used", 1).unwrap_err().code,
        "vua.amf_production.recipe_in_use"
    );
    let digest = h.engine.plan("n3-from-recipe", &selection()).unwrap()["planDigest"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-from-recipe", selection(), &digest, ctx)));
    let run = h.engine.get("n3-from-recipe").unwrap();
    assert!(run.confirmed_digest.is_some());
    let original = fs::read(h.root.join("production/runs/n3-from-recipe.json")).unwrap();
    for i in 0..31 {
        let mut other = h.engine.get("n3-test").unwrap();
        other.run_id = format!("n3-z{i:02}");
        other.project_root = h
            .root
            .join("production/projects")
            .join(&other.run_id)
            .to_string_lossy()
            .into();
        fs::write(
            h.root
                .join("production/runs")
                .join(format!("{}.json", other.run_id)),
            serde_json::to_vec(&other).unwrap(),
        )
        .unwrap();
    }
    assert!(!h
        .engine
        .list()
        .unwrap()
        .iter()
        .any(|row| row.run_id == run.run_id));
    assert_eq!(
        h.engine.delete_recipe("recipe-used", 1).unwrap_err().code,
        "vua.amf_production.recipe_in_use"
    );
    assert_eq!(
        fs::read(h.root.join("production/runs/n3-from-recipe.json")).unwrap(),
        original
    );
    assert!(h.engine.get_recipe("recipe-used").is_ok());
}

#[test]
fn unreadable_line_references_fail_closed() {
    let h = Harness::new();
    let id = "recipe-safe";
    store(&h).save(id, &document("0.5", id), 0).unwrap();
    fs::create_dir_all(h.root.join("production/runs")).unwrap();
    fs::write(h.root.join("production/runs/n3-broken.json"), b"{").unwrap();
    assert_eq!(
        h.engine.delete_recipe(id, 1).unwrap_err().code,
        "vua.amf_production.recipe_references_unreadable"
    );
    assert!(h.engine.get_recipe(id).is_ok());
}

#[test]
fn summaries_retain_both_formats_and_isolate_corruption() {
    let h = Harness::new();
    for (version, id) in [("0.4", "recipe-old"), ("0.5", "recipe-new")] {
        store(&h).save(id, &document(version, id), 0).unwrap();
    }
    store(&h)
        .save(
            "recipe-bad-body",
            &json!({"title":"Damaged recipe", "formatVersion":"unknown"}),
            0,
        )
        .unwrap();
    fs::write(
        h.root
            .join("production/recipes-v04/recipe-bad-envelope.json"),
        b"{",
    )
    .unwrap();
    let rows = serde_json::to_value(h.engine.list_recipes().unwrap()).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 3);
    for id in ["recipe-old", "recipe-new"] {
        let row = rows.iter().find(|row| row["recipeId"] == id).unwrap();
        let recipe = h.engine.get_recipe(id).unwrap().recipe;
        assert_eq!(
            row["materialCount"],
            recipe["materials"].as_array().unwrap().len()
        );
        assert_eq!(
            row["outfitCount"],
            recipe["outfits"].as_array().unwrap().len()
        );
        assert_eq!(
            row["packageCount"],
            recipe["packages"].as_object().unwrap().len()
        );
        assert_eq!(row["avatarName"], recipe["materials"][0]["packageName"]);
    }
    assert_eq!(
        rows.iter()
            .find(|row| row["recipeId"] == "recipe-new")
            .unwrap()["boothProductIds"],
        json!(["1000001"])
    );
    assert_eq!(
        rows.iter()
            .find(|row| row["recipeId"] == "recipe-old")
            .unwrap()["boothProductIds"],
        json!([])
    );
    let broken = rows
        .iter()
        .find(|row| row["recipeId"] == "recipe-bad-body")
        .unwrap();
    assert_eq!(broken["title"], "Damaged recipe");
    assert_eq!(broken["revision"], 1);
    for field in [
        "avatarName",
        "materialCount",
        "packageCount",
        "outfitCount",
        "boothProductIds",
    ] {
        assert!(broken.get(field).is_none());
    }
    assert!(h
        .root
        .join("production/recipes-v04/recipe-bad-envelope.json")
        .exists());
}

#[test]
fn recipe_still_outlives_the_line_project_and_is_deleted_with_its_recipe() {
    let h = Harness::new();
    assembled(&h);
    let run = h.engine.get("n3-test").unwrap();
    h.engine.keep_stills(&run);
    let before = h
        .engine
        .preview_image("n3-test", ".vua/previews/cmd/front.png")
        .unwrap();
    h.engine
        .save_recipe("n3-test", "recipe-picture", "Avatar")
        .unwrap();
    assert_eq!(
        h.engine.recipe_image("recipe-picture").unwrap(),
        Some(before.clone())
    );
    // The synthetic project belongs only to this unique Harness root.
    assert!(Path::new(&run.project_root).starts_with(&h.root));
    fs::remove_dir_all(&run.project_root).unwrap();
    assert_eq!(
        h.engine.recipe_image("recipe-picture").unwrap(),
        Some(before)
    );
    assert!(h.engine.delete_recipe("recipe-picture", 2).is_err());
    assert!(h
        .root
        .join("production/recipe-stills/recipe-picture.png")
        .exists());
    h.engine.delete_recipe("recipe-picture", 1).unwrap();
    assert!(!h
        .root
        .join("production/recipe-stills/recipe-picture.png")
        .exists());
    assert!(h.engine.get("n3-test").is_ok());
}

#[test]
fn images_are_optional_and_bounded_and_save_does_not_require_a_picture() {
    let h = Harness::new();
    assembled(&h);
    let retained = h
        .root
        .join("production/stills/n3-test/.vua_previews_cmd_front.png");
    if retained.exists() {
        fs::remove_file(&retained).unwrap();
    }
    h.engine
        .save_recipe("n3-test", "recipe-no-picture", "Avatar")
        .unwrap();
    assert!(h
        .engine
        .recipe_image("recipe-no-picture")
        .unwrap()
        .is_none());
    let folder = h.root.join("production/recipe-stills");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("recipe-no-picture.png");
    for bytes in [b"not a PNG".to_vec(), vec![0; 640 * 1024 + 1]] {
        fs::write(&path, bytes).unwrap();
        assert_eq!(
            h.engine.recipe_image("recipe-no-picture").unwrap_err().code,
            "vua.amf_production.recipe_image_invalid"
        );
    }
    assert!(h.engine.recipe_image("../escape").is_err());
}

#[test]
fn rename_retains_share_provenance_even_when_document_save_is_interrupted() {
    let h = Harness::new();
    let shared = json!({"formatVersion":"0.5", "recipe":document("0.5", "recipe-import"), "sources":{"material-1":{"boothProductId":"1000001"}, "material-2":{"boothProductId":"1000001"}}});
    let saved = h.engine.import_recipe(&shared.to_string()).unwrap();
    let id = saved.recipe["recipeId"].as_str().unwrap();
    let before = h.engine.recipe_share_plan(id).unwrap();
    let pending = h
        .root
        .join("production/recipes-v04")
        .join(format!(".{id}.tmp"));
    fs::create_dir(&pending).unwrap();
    assert!(h.engine.rename_recipe(id, 1, "Interrupted").is_err());
    assert_eq!(h.engine.get_recipe(id).unwrap(), saved);
    assert_eq!(
        h.engine.recipe_share_plan(id).unwrap().plan_digest,
        before.plan_digest
    );
    fs::remove_dir(pending).unwrap();
    let renamed = h.engine.rename_recipe(id, 1, "My copy").unwrap();
    assert_eq!(renamed.revision, 2);
    assert_eq!(h.engine.recipe_share_plan(id).unwrap().title, "My copy");
    let origin: Value = serde_json::from_slice(
        &fs::read(
            h.root
                .join("production/recipe-origins")
                .join(format!("{id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(origin.get("pendingRenameDigest").is_none());
}
