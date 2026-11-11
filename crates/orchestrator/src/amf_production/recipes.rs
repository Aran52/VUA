//! Local Recipe cards and revision-guarded management. No Unity or material resolution.
use super::*;
use crate::{amf_recipe::ProductionRecipe, RecipeListEntry, RecipeSaveError, StoredRecipeDocument};
use std::{collections::BTreeSet, io::{ErrorKind, Read}};

const MAX_SAFE_REVISION: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionRecipeFacts {
    pub avatar_name: String,
    pub outfit_count: usize,
    pub material_count: usize,
    pub package_count: usize,
    pub booth_product_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductionRecipeSummary {
    #[serde(flatten)]
    pub entry: RecipeListEntry,
    #[serde(flatten)]
    pub facts: Option<ProductionRecipeFacts>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 100 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn store_error(error: RecipeSaveError) -> AppErrorV1 {
    match error {
        RecipeSaveError::RevisionConflict { current_revision } => {
            let mut error = production_error("recipe_revision_conflict", "The saved Recipe changed. Read its current revision before trying again.")
                .with_param("currentRevision", ParamValue::Number(current_revision as f64));
            error.category = ErrorCategory::Conflict;
            error
        }
        RecipeSaveError::InvalidId => production_error("params_invalid", "Invalid Recipe identity."),
        other => io_error(other),
    }
}

/// The stream is bounded even when the file grows between metadata and the read.
fn bounded_png(path: &Path) -> Result<Option<Vec<u8>>, AppErrorV1> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(error)),
    };
    let invalid = || production_error("recipe_image_invalid", "The Recipe image is not a bounded PNG file.");
    if !metadata.is_file() || metadata.len() > MAX_PREVIEW_BYTES { return Err(invalid()); }
    let mut bytes = Vec::new();
    fs::File::open(path).map_err(io_error)?.take(MAX_PREVIEW_BYTES + 1).read_to_end(&mut bytes).map_err(io_error)?;
    if bytes.len() as u64 > MAX_PREVIEW_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") { return Err(invalid()); }
    Ok(Some(bytes))
}

impl AmfProductionEngine {
    fn checked_recipe_revision(&self, recipe_id: &str, expected_revision: u64) -> Result<StoredRecipeDocument, AppErrorV1> {
        if !valid_id(recipe_id) || !(1..=MAX_SAFE_REVISION).contains(&expected_revision) {
            return Err(production_error("params_invalid", "Expected a Recipe identity and a positive safe revision."));
        }
        let stored = self.recipe_store().get(recipe_id).map_err(store_error)?;
        let current_revision = stored.as_ref().map_or(0, |stored| stored.revision);
        if current_revision != expected_revision {
            return Err(store_error(RecipeSaveError::RevisionConflict { current_revision }));
        }
        stored.ok_or_else(|| store_error(RecipeSaveError::RevisionConflict { current_revision: 0 }))
    }

    pub fn rename_recipe(&self, recipe_id: &str, expected_revision: u64, title: &str) -> Result<StoredRecipeDocument, AppErrorV1> {
        let stored = self.checked_recipe_revision(recipe_id, expected_revision)?;
        if expected_revision == MAX_SAFE_REVISION {
            return Err(production_error("params_invalid", "This Recipe cannot increment its revision safely."));
        }
        let before: ProductionRecipe = serde_json::from_value(stored.recipe.clone()).map_err(io_error)?;
        before.validate()?;
        if before.recipe_id != recipe_id { return Err(production_error("recipe_invalid", "Recipe identity changed.")); }
        let mut after = before.clone();
        after.title = title.into();
        after.validate()?;
        // Preserve the original JSON spelling and absent/default fields; only the title changes.
        let mut document = stored.recipe;
        document["title"] = title.into();
        self.stage_recipe_rename_origin(&before, &after)?;
        let result = self.recipe_store().save(recipe_id, &document, expected_revision).map_err(store_error)?;
        self.finish_recipe_rename_origin(&after);
        Ok(result)
    }

    pub fn delete_recipe(&self, recipe_id: &str, expected_revision: u64) -> Result<(), AppErrorV1> {
        self.checked_recipe_revision(recipe_id, expected_revision)?;
        // list() is capped at thirty. A deletion guard must inspect every durable record.
        match fs::read_dir(self.root.join("runs")) {
            Ok(entries) => for entry in entries {
                let path = entry.map_err(io_error)?.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("json") { continue; }
                let id = path.file_stem().and_then(|id| id.to_str())
                    .ok_or_else(|| production_error("recipe_references_unreadable", "A line record cannot be checked for Recipe references."))?;
                let run = self.get(id).map_err(|_| production_error("recipe_references_unreadable", "A line record cannot be checked for Recipe references."))?;
                if run.recipe_id.as_deref() == Some(recipe_id) {
                    return Err(production_error("recipe_in_use", "A retained line still uses this Recipe. Keep the Recipe while that line exists."));
                }
            },
            Err(error) if error.kind() == ErrorKind::NotFound => {},
            Err(error) => return Err(io_error(error)),
        }
        match fs::remove_file(self.recipe_still(recipe_id)) {
            Ok(()) => {},
            Err(error) if error.kind() == ErrorKind::NotFound => {},
            Err(error) => return Err(io_error(error)),
        }
        self.recipe_store().delete(recipe_id, expected_revision).map_err(store_error)
    }

    pub fn list_recipes(&self) -> Result<Vec<ProductionRecipeSummary>, AppErrorV1> {
        let entries = match fs::read_dir(self.root.join("recipes-v04")) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(io_error(error)),
        };
        let store = self.recipe_store();
        let mut rows = Vec::new();
        for entry in entries {
            let path = entry.map_err(io_error)?.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") { continue; }
            let Some(id) = path.file_stem().and_then(|id| id.to_str()).filter(|id| valid_id(id)) else { continue };
            // Keep unreadable files for repair; no fabricated revision or timestamp.
            let Ok(Some(stored)) = store.get(id) else { continue };
            if !(1..=MAX_SAFE_REVISION).contains(&stored.revision) || stored.updated_at.trim().is_empty() { continue; }
            let facts = serde_json::from_value::<ProductionRecipe>(stored.recipe.clone()).ok()
                .filter(|recipe| recipe.recipe_id == id && recipe.validate().is_ok()).map(|recipe| ProductionRecipeFacts {
                    avatar_name: recipe.materials.iter().find(|material| material.id == recipe.avatar.material_id)
                        .expect("validated Avatar material").package_name.clone(),
                    outfit_count: recipe.outfits.len(), material_count: recipe.materials.len(), package_count: recipe.packages.len(),
                    booth_product_ids: recipe.materials.iter().filter_map(|material| material.booth_product_id.clone())
                        .collect::<BTreeSet<_>>().into_iter().collect(),
                });
            rows.push(ProductionRecipeSummary { entry: RecipeListEntry {
                recipe_id: id.into(), title: stored.recipe["title"].as_str().filter(|title| !title.trim().is_empty()).unwrap_or(id).into(),
                revision: stored.revision, updated_at: stored.updated_at,
            }, facts });
        }
        rows.sort_by(|a, b| b.entry.updated_at.cmp(&a.entry.updated_at).then_with(|| a.entry.recipe_id.cmp(&b.entry.recipe_id)));
        Ok(rows)
    }

    fn recipe_still(&self, recipe_id: &str) -> PathBuf { self.root.join("recipe-stills").join(format!("{recipe_id}.png")) }

    pub fn recipe_image(&self, recipe_id: &str) -> Result<Option<Vec<u8>>, AppErrorV1> {
        self.get_recipe(recipe_id)?;
        bounded_png(&self.recipe_still(recipe_id))
    }

    pub(super) fn keep_recipe_still(&self, run: &ProductionRun, recipe_id: &str) {
        let Some(path) = run.result["facts"]["previews"].as_array().into_iter().flatten()
            .find(|view| view["view"] == "front").and_then(|view| view["path"].as_str()).filter(|path| is_preview_path(path)) else { return };
        let Ok(Some(bytes)) = bounded_png(&self.kept_still(&run.run_id, path)) else { return };
        let target = self.recipe_still(recipe_id);
        if fs::create_dir_all(target.parent().expect("Recipe still parent")).is_err() { return; }
        let pending = target.with_extension("pending");
        if fs::write(&pending, bytes).is_ok() { let _ = fs::rename(&pending, target); }
    }
}
