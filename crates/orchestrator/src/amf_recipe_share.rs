//! Bounded Recipe sharing; source statements are not entitlement or authenticity checks.
use crate::{amf_production::{production_error, AmfProductionEngine, ProductionRun}, amf_recipe::ProductionRecipe, recipe::{base64_url_decode, base64_url_encode, canonical_json, document_digest}, AppErrorV1, StoredRecipeDocument};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs};

pub const SHARE_PREFIX: &str = "vuar0.5.";
pub const MAX_SHARE_BYTES: usize = 256 * 1024;
fn error(code: &str, detail: impl Into<String>) -> AppErrorV1 { production_error(code, detail) }
fn io(error: impl std::fmt::Display) -> AppErrorV1 { production_error("recipe_store_failed", error.to_string()) }
pub fn valid_booth_id(id: &str) -> bool { !id.is_empty() && id.len() <= 20 && !id.starts_with('0') && id.bytes().all(|b| b.is_ascii_digit()) }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RecipeSourceStatement { Legacy(String), Booth(BoothRecipeSource) }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoothRecipeSource {
    pub booth_product_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "crate::amf_recipe::optional_text")]
    pub shop_name: Option<String>,
}
impl RecipeSourceStatement {
    pub fn product_id(&self) -> &str { match self { Self::Legacy(id) => id, Self::Booth(source) => &source.booth_product_id } }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SharedRecipe {
    pub format_version: String,
    pub recipe: ProductionRecipe,
    /// Material identity -> native BOOTH item ID. No URL, local path or asset body.
    pub sources: BTreeMap<String, RecipeSourceStatement>,
}
impl SharedRecipe {
    pub fn validate(&self) -> Result<(), AppErrorV1> {
        self.recipe.validate()?;
        if !matches!(self.format_version.as_str(), "0.4" | "0.5") || self.format_version != self.recipe.format_version || self.sources.len() != self.recipe.materials.len()
            || self.recipe.materials.iter().any(|m| self.sources.get(&m.id).is_none_or(|id| !valid_booth_id(id.product_id()))) {
            return Err(error("recipe_share_invalid", "Every referenced material needs one BOOTH source statement."));
        }
        for material in &self.recipe.materials {
            let source = &self.sources[&material.id];
            match source {
                RecipeSourceStatement::Legacy(_) if self.format_version == "0.4" => {},
                RecipeSourceStatement::Booth(source) if self.format_version == "0.5" && source.shop_name.as_ref().is_none_or(|name| !name.trim().is_empty() && name.chars().count() <= 240 && !name.chars().any(char::is_control)) => {},
                _ => return Err(error("recipe_share_invalid", "Unsupported source statement.")),
            }
            if material.booth_product_id.as_ref().is_some_and(|id| id != source.product_id()) {
                return Err(error("recipe_share_invalid", "Material and source BOOTH IDs disagree."));
            }
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<String, AppErrorV1> {
        self.validate()?;
        let json = canonical_json(self)?;
        if json.len() > MAX_SHARE_BYTES { return Err(error("recipe_share_too_large", "Recipe exceeds the sharing size limit.")); }
        Ok(format!("vuar{}.{}", self.format_version, base64_url_encode(json.as_bytes())))
    }
    pub fn decode(text: &str) -> Result<Self, AppErrorV1> {
        if text.len() > MAX_SHARE_BYTES * 4 / 3 + 32 { return Err(error("recipe_share_too_large", "Recipe exceeds the sharing size limit.")); }
        let text = text.trim();
        let bytes = if let Some(payload) = text.strip_prefix(SHARE_PREFIX).or_else(|| text.strip_prefix("vuar0.4.")) {
            base64_url_decode(payload).ok_or_else(|| error("recipe_share_invalid", "The Recipe text is damaged or unsupported."))?
        } else { text.as_bytes().to_vec() };
        if bytes.len() > MAX_SHARE_BYTES { return Err(error("recipe_share_too_large", "Recipe exceeds the sharing size limit.")); }
        let shared: Self = serde_json::from_slice(&bytes).map_err(|_| error("recipe_share_invalid", "The Recipe contains unsupported content or has an invalid format."))?;
        shared.validate()?;
        Ok(shared)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RecipeOrigin {
    recipe_digest: String,
    /// Empty only for received declarations. Source IDs remain machine-local.
    inputs: BTreeMap<String, Vec<String>>,
    references_observed: bool,
    received_sources: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    received_statements: BTreeMap<String,RecipeSourceStatement>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeShareMaterial {
    pub material_id: String,
    pub package_name: String,
    pub candidates: Vec<String>,
    pub selected_product_id: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSharePlan {
    pub recipe_id: String,
    pub revision: u64,
    pub title: String,
    pub materials: Vec<RecipeShareMaterial>,
    pub plan_digest: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeShareExport { pub share_code: String, pub file_text: String, pub document: SharedRecipe }

impl AmfProductionEngine {
    fn origin_path(&self, recipe_id: &str) -> std::path::PathBuf { self.root.join("recipe-origins").join(format!("{recipe_id}.json")) }
    fn save_origin(&self, recipe_id: &str, origin: &RecipeOrigin) -> Result<(), AppErrorV1> {
        // The caller has already validated the Recipe identity.
        let path = self.origin_path(recipe_id); fs::create_dir_all(path.parent().unwrap()).map_err(io)?;
        let pending = path.with_extension("pending");
        fs::write(&pending, serde_json::to_vec_pretty(origin).map_err(io)?).map_err(io)?;
        fs::rename(pending, path).map_err(io)
    }
    pub(crate) fn retain_recipe_origin(&self, run: &ProductionRun, recipe: &ProductionRecipe) -> Result<(), AppErrorV1> {
        let observed = run.result["facts"]["usedSourceIds"].as_array();
        let inputs = recipe.materials.iter().map(|material| (material.id.clone(), run.sources.iter()
            .filter(|source| source.sha256.strip_prefix("sha256:").unwrap_or(&source.sha256) == material.sha256)
            .map(|source| source.source_id.clone()).collect())).collect();
        self.save_origin(&recipe.recipe_id, &RecipeOrigin { recipe_digest: document_digest(recipe)?, inputs,
            references_observed: observed.is_some(), received_sources: BTreeMap::new(), received_statements:BTreeMap::new() })
    }
    fn share_origin(&self, recipe: &ProductionRecipe) -> Result<RecipeOrigin, AppErrorV1> {
        let path = self.origin_path(&recipe.recipe_id);
        if !path.is_file() { return Err(error("recipe_reinspect_for_share", "Reinspect the source project and save this Recipe again to establish its referenced materials.")); }
        let origin: RecipeOrigin = serde_json::from_slice(&fs::read(path).map_err(io)?).map_err(io)?;
        if origin.recipe_digest != document_digest(recipe)? || (!origin.references_observed && origin.received_sources.is_empty()) {
            return Err(error("recipe_reinspect_for_share", "Reinspect the source project and save this Recipe again before sharing."));
        }
        Ok(origin)
    }
    fn confirmed_sources(&self) -> Result<BTreeMap<String, String>, AppErrorV1> {
        let path = self.root.join("recipe-source-choices.json");
        if !path.exists() { return Ok(BTreeMap::new()); }
        let choices: BTreeMap<String, String> = serde_json::from_slice(&fs::read(path).map_err(io)?).map_err(io)?;
        if choices.iter().any(|(hash,id)| hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) || !valid_booth_id(id)) {
            return Err(error("recipe_source_choices_invalid", "Saved source choices need repair."));
        }
        Ok(choices)
    }
    pub(crate) fn record_recipe_products(&self, run: &ProductionRun, recipe: &mut ProductionRecipe) -> Result<(),AppErrorV1> {
        let confirmed = self.confirmed_sources()?;
        let inputs = recipe.materials.iter().map(|m| (m.id.clone(), run.sources.iter().filter(|s| s.sha256.trim_start_matches("sha256:") == m.sha256).map(|s| s.source_id.clone()).collect())).collect();
        let candidates = self.platform.recipe_source_candidates(&recipe.materials, &inputs)?;
        for material in &mut recipe.materials {
            let mut ids = candidates.get(&material.id).cloned().unwrap_or_default(); ids.sort(); ids.dedup();
            material.booth_product_id = confirmed.get(&material.sha256).cloned().or_else(|| (ids.len() == 1).then(|| ids[0].clone()));
        }
        recipe.validate()
    }
    pub(crate) fn recipe_for_resolution(&self, id: &str) -> Result<ProductionRecipe,AppErrorV1> {
        let mut recipe: ProductionRecipe = serde_json::from_value(self.get_recipe(id)?.recipe).map_err(io)?;
        // The v0.4 source statements are author claims attached to this declaration only.
        let path = self.origin_path(id);
        if path.is_file() {
            let origin: RecipeOrigin = serde_json::from_slice(&fs::read(path).map_err(io)?).map_err(io)?;
            if origin.recipe_digest == document_digest(&recipe)? {
                for material in &mut recipe.materials {
                    if material.booth_product_id.is_none() { material.booth_product_id = origin.received_sources.get(&material.id).cloned(); }
                }
            }
        }
        // v0.4 remains hash-only: the old declaration did not opt into product substitution.
        if recipe.format_version == "0.4" { for m in &mut recipe.materials { m.booth_product_id = None; } }
        Ok(recipe)
    }
    pub fn recipe_share_plan(&self, recipe_id: &str) -> Result<RecipeSharePlan, AppErrorV1> {
        let stored = self.get_recipe(recipe_id)?;
        let recipe: ProductionRecipe = serde_json::from_value(stored.recipe).map_err(io)?;
        let origin = self.share_origin(&recipe)?;
        let candidates = self.platform.recipe_source_candidates(&recipe.materials, &origin.inputs)?;
        let remembered = self.confirmed_sources()?;
        let materials = recipe.materials.iter().map(|material| {
            let mut available = candidates.get(&material.id).cloned().unwrap_or_default();
            available.retain(|id| valid_booth_id(id)); available.sort(); available.dedup();
            let selected = material.booth_product_id.as_ref().or_else(|| remembered.get(&material.sha256)).or_else(|| origin.received_sources.get(&material.id)).cloned()
                .or_else(|| (available.len() == 1).then(|| available[0].clone()));
            RecipeShareMaterial { material_id: material.id.clone(), package_name: material.package_name.clone(), candidates: available, selected_product_id: selected }
        }).collect();
        let mut plan = RecipeSharePlan { recipe_id: recipe_id.into(), revision: stored.revision, title: recipe.title, materials, plan_digest: String::new() };
        plan.plan_digest = document_digest(&serde_json::json!({"recipe":origin.recipe_digest,"plan":plan}))?.trim_start_matches("sha256:").into();
        Ok(plan)
    }
    pub fn export_recipe(&self, recipe_id: &str, digest: &str, supplements: &BTreeMap<String, String>) -> Result<RecipeShareExport, AppErrorV1> {
        let plan = self.recipe_share_plan(recipe_id)?;
        if digest != plan.plan_digest { return Err(error("recipe_share_stale", "The Recipe or its sources changed. Review the source list again.")); }
        if supplements.len() > 64 || supplements.iter().any(|(material,id)| !valid_booth_id(id) || !plan.materials.iter().any(|m| &m.material_id == material)) {
            return Err(error("recipe_sources_invalid", "Source choices must name only this Recipe's materials and valid BOOTH items."));
        }
        let source_ids = plan.materials.iter().map(|material| {
            let source = supplements.get(&material.material_id).or(material.selected_product_id.as_ref())
                .ok_or_else(|| error("recipe_source_missing", format!("Choose a BOOTH source for {} before sharing.", material.package_name)))?;
            Ok((material.material_id.clone(), source.clone()))
        }).collect::<Result<BTreeMap<_,_>,AppErrorV1>>()?;
        let recipe: ProductionRecipe = serde_json::from_value(self.get_recipe(recipe_id)?.recipe).map_err(io)?;
        let origin = self.share_origin(&recipe)?;
        let sources = source_ids.into_iter().map(|(material,id)| {
            let statement = if recipe.format_version == "0.5" {
                RecipeSourceStatement::Booth(BoothRecipeSource {shop_name:self.platform.recipe_shop_name(&id)?.or_else(|| origin.received_statements.get(&material).and_then(|statement| match statement {
                    RecipeSourceStatement::Booth(source) if source.booth_product_id == id => source.shop_name.clone(), _ => None,
                })),booth_product_id:id})
            } else { RecipeSourceStatement::Legacy(id) };
            Ok((material,statement))
        }).collect::<Result<BTreeMap<_,_>,AppErrorV1>>()?;
        let mut recipe = recipe;
        if recipe.format_version == "0.5" { for material in &mut recipe.materials { material.booth_product_id = Some(sources[&material.id].product_id().into()); } }
        let shared = SharedRecipe { format_version: recipe.format_version.clone(), recipe, sources };
        let share_code = shared.encode()?;
        let file_text = serde_json::to_string_pretty(&shared).map_err(io)?;
        if file_text.len() > MAX_SHARE_BYTES { return Err(error("recipe_share_too_large", "Recipe file exceeds the sharing size limit.")); }
        if !supplements.is_empty() {
            let mut confirmed = self.confirmed_sources()?;
            for material in &shared.recipe.materials { if let Some(id) = supplements.get(&material.id) { confirmed.insert(material.sha256.clone(), id.clone()); } }
            let path = self.root.join("recipe-source-choices.json"); let pending = path.with_extension("pending");
            fs::write(&pending, serde_json::to_vec_pretty(&confirmed).map_err(io)?).map_err(io)?;
            fs::rename(pending,path).map_err(io)?;
        }
        Ok(RecipeShareExport { share_code, file_text, document: shared })
    }
    pub fn import_recipe(&self, text: &str) -> Result<StoredRecipeDocument, AppErrorV1> {
        let shared = SharedRecipe::decode(text)?;
        let digest = document_digest(&shared)?;
        let recipe_id = format!("shared-{}", digest.trim_start_matches("sha256:"));
        let mut recipe = shared.recipe; recipe.recipe_id = recipe_id.clone(); recipe.validate()?;
        let value = serde_json::to_value(&recipe).map_err(io)?;
        if let Some(existing) = self.recipe_store().get(&recipe_id).map_err(io)? {
            if existing.recipe != value { return Err(error("recipe_import_conflict", "An existing imported Recipe has changed. Keep it and import a revised declaration.")); }
            self.share_origin(&recipe)?;
            return Ok(existing);
        }
        self.save_origin(&recipe_id, &RecipeOrigin { recipe_digest: document_digest(&recipe)?, inputs: BTreeMap::new(), references_observed: false, received_sources: shared.sources.iter().map(|(id,source)| (id.clone(),source.product_id().into())).collect(), received_statements:shared.sources })?;
        self.recipe_store().save(&recipe_id, &value, 0).map_err(io)
    }
}
