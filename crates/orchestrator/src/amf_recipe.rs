//! Candidate Recipe v0.4: portable declarations for the actual MA production path.
//! Paid files and machine-local identities are never part of this document.
use crate::amf_production::{production_error, valid_lighting_value, AssemblySelection, BindingMode, MaterialLighting, OutfitSelection, ProductionRun};
use crate::AppErrorV1;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeMaterial {
    pub id: String,
    pub package_name: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipePrefab {
    pub material_id: String,
    pub prefab_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeOutfit {
    pub material_id: String,
    pub prefab_path: String,
    pub mode: BindingMode,
    pub bone: String,
    pub toggle_name: String,
    pub active: bool,
}

/// VUA's own adjustments to the assembled line, replayed after the Recipe's first assembly:
/// the brightness limits written into each material and the SDK fixes run, named by the SDK's
/// exact words. Older Recipes carry no `adjustments`; the section is optional.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeAdjustments {
    pub lighting: Vec<MaterialLighting>,
    pub sdk_fixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionRecipe {
    pub format_version: String,
    pub recipe_id: String,
    pub title: String,
    pub editor_version: String,
    pub materials: Vec<RecipeMaterial>,
    pub avatar: RecipePrefab,
    pub outfits: Vec<RecipeOutfit>,
    pub packages: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustments: Option<RecipeAdjustments>,
}

fn invalid(detail: impl Into<String>) -> AppErrorV1 { production_error("recipe_invalid", detail) }
fn label(text: &str, max: usize) -> bool { !text.trim().is_empty() && text.chars().count() <= max && !text.chars().any(char::is_control) }
fn asset_path(path: &str) -> bool {
    path.len() <= 2048 && path.starts_with("Assets/") && path.ends_with(".prefab")
        && !path.contains(['\\', ':', '\0']) && path.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}
/// A material path in a Recipe's adjustments: the same grammar as a prefab path, for `.mat`.
fn material_path(path: &str) -> bool {
    path.len() <= 2048 && path.starts_with("Assets/") && path.ends_with(".mat")
        && !path.contains(['\\', ':', '\0']) && path.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

impl ProductionRecipe {
    pub fn validate(&self) -> Result<(), AppErrorV1> {
        if self.format_version != "0.4" || self.recipe_id.is_empty() || self.recipe_id.len() > 100
            || !self.recipe_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !label(&self.title, 120) || !matches!(self.editor_version.as_str(), "2022.3.22f1" | "2022.3.22f1c1")
            || self.materials.is_empty() || self.materials.len() > 64 || self.outfits.is_empty() || self.outfits.len() > 64 {
            return Err(invalid("Unsupported or malformed Recipe header."));
        }
        let mut ids = BTreeSet::new();
        for material in &self.materials {
            if material.sha256.len() != 64 || !material.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || material.id != format!("m-{}", material.sha256) || !ids.insert(material.id.as_str())
                || !label(&material.package_name, 240) || !material.package_name.to_ascii_lowercase().ends_with(".unitypackage")
                || material.package_name.contains(['/', '\\', ':']) {
                return Err(invalid("Recipe material references must identify bounded original packages."));
            }
        }
        if !ids.contains(self.avatar.material_id.as_str()) || !asset_path(&self.avatar.prefab_path) { return Err(invalid("Invalid Avatar reference.")); }
        let mut outfits = BTreeSet::new();
        for outfit in &self.outfits {
            if !ids.contains(outfit.material_id.as_str()) || !outfits.insert(outfit.material_id.as_str())
                || !asset_path(&outfit.prefab_path) || outfit.bone.len() > 80 || outfit.bone.chars().any(char::is_control)
                || outfit.toggle_name.chars().count() > 80 || outfit.toggle_name.chars().any(char::is_control)
                || (outfit.mode == BindingMode::AttachToBone && outfit.bone.is_empty()) {
                return Err(invalid("Invalid or ambiguous outfit declaration."));
            }
        }
        if self.packages.len() > 64 || !self.packages.contains_key("com.vrchat.avatars") || !self.packages.contains_key("nadena.dev.modular-avatar")
            || self.packages.iter().any(|(id, version)| id.len() > 128 || !id.contains('.') || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
                || version.is_empty() || version.len() > 80 || !version.bytes().all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))) {
            return Err(invalid("Recipe dependencies require exact observed package versions."));
        }
        if let Some(adjustments) = &self.adjustments {
            if adjustments.lighting.len() > 512 || adjustments.sdk_fixes.len() > crate::amf_production::SDK_FIX_LIMIT {
                return Err(invalid("Recipe adjustments exceed their bounds."));
            }
            if adjustments.lighting.iter().any(|entry| !material_path(&entry.material)
                || !valid_lighting_value(&entry.min) || !valid_lighting_value(&entry.max)) {
                return Err(invalid("Recipe lighting adjustments name bounded material paths and serialized values."));
            }
            if adjustments.sdk_fixes.iter().any(|fix| fix.is_empty() || fix.chars().count() > 600 || fix.contains('\0')) {
                return Err(invalid("Recipe SDK fixes name the SDK's exact words."));
            }
        }
        Ok(())
    }

    pub fn from_run(run: &ProductionRun, recipe_id: &str, title: &str, adjustments: Option<RecipeAdjustments>) -> Result<Self, AppErrorV1> {
        let selection: AssemblySelection = serde_json::from_value(run.result["facts"]["observedSelection"].clone())
            .map_err(|_| production_error("recipe_scene_unverified", "Reinspect a successfully assembled Avatar before saving a Recipe."))?;
        let source_by_id = run.sources.iter().map(|s| (s.source_id.as_str(), s)).collect::<BTreeMap<_, _>>();
        let avatar_sources = run.prefabs.iter().filter(|p| p.is_avatar && p.path == selection.avatar_prefab)
            .filter_map(|p| source_by_id.get(p.source_id.as_str()).copied()).collect::<Vec<_>>();
        if avatar_sources.len() != 1 { return Err(invalid("The Avatar input is ambiguous.")); }
        let mut materials = BTreeMap::new();
        let used: Option<Vec<String>> = serde_json::from_value(run.result["facts"]["usedSourceIds"].clone()).ok();
        for source in run.sources.iter().filter(|source| source.source_id == avatar_sources[0].source_id
            || selection.outfits.iter().any(|outfit| outfit.source_id == source.source_id)
            || used.as_ref().is_none_or(|ids| ids.contains(&source.source_id))) {
            let digest = source.sha256.strip_prefix("sha256:").unwrap_or(&source.sha256);
            let id = format!("m-{digest}");
            let name = source.name.rsplit(['/', '\\']).next().unwrap_or(&source.name).to_owned();
            materials.entry(id.clone()).or_insert(RecipeMaterial { id, package_name: name, sha256: digest.into() });
        }
        let outfits = selection.outfits.iter().map(|outfit| {
            let source = source_by_id.get(outfit.source_id.as_str()).ok_or_else(|| invalid("An observed outfit has no retained input."))?;
            Ok(RecipeOutfit { material_id: format!("m-{}", source.sha256.strip_prefix("sha256:").unwrap_or(&source.sha256)), prefab_path: outfit.prefab_path.clone(), mode: outfit.mode,
                bone: outfit.bone.clone(), toggle_name: outfit.toggle_name.clone(), active: outfit.active })
        }).collect::<Result<Vec<_>, AppErrorV1>>()?;
        let recipe = Self { format_version: "0.4".into(), recipe_id: recipe_id.into(), title: title.trim().into(), editor_version: run.editor.version.clone(),
            materials: materials.into_values().collect(), avatar: RecipePrefab { material_id: format!("m-{}", avatar_sources[0].sha256.strip_prefix("sha256:").unwrap_or(&avatar_sources[0].sha256)), prefab_path: selection.avatar_prefab },
            outfits, packages: serde_json::from_value(run.result["packageVersions"].clone()).map_err(|_| invalid("No observed dependency versions."))?, adjustments };
        recipe.validate()?;
        Ok(recipe)
    }

    pub fn selection(&self, sources: &BTreeMap<String, String>) -> Result<AssemblySelection, AppErrorV1> {
        self.validate()?;
        if self.materials.iter().any(|m| !sources.contains_key(&m.id)) { return Err(production_error("recipe_material_missing", "Supply the missing Recipe materials before preparing a project.")); }
        Ok(AssemblySelection { avatar_prefab: self.avatar.prefab_path.clone(), outfits: self.outfits.iter().map(|outfit| OutfitSelection {
            source_id: sources[&outfit.material_id].clone(), prefab_path: outfit.prefab_path.clone(), mode: outfit.mode,
            bone: outfit.bone.clone(), toggle_name: if outfit.toggle_name.trim().is_empty() { String::new() } else { outfit.toggle_name.clone() }, active: outfit.active,
        }).collect() })
    }
}
