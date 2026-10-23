//! Candidate Recipe v0.5 (with retained v0.4 reads): portable declarations for the actual MA production path.
//! Paid files and machine-local identities are never part of this document.
use crate::amf_production::{production_error, valid_lighting_value, AssemblySelection, BindingMode, FitObject, OutfitSelection, ProductionRun};
use crate::AppErrorV1;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeMaterial {
    pub id: String,
    pub package_name: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "optional_text")]
    pub booth_product_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipePrefab {
    pub material_id: String,
    pub prefab_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "optional_text")]
    pub prefab_guid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeOutfit {
    pub material_id: String,
    pub prefab_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "optional_text")]
    pub prefab_guid: Option<String>,
    pub mode: BindingMode,
    pub bone: String,
    pub toggle_name: String,
    pub active: bool,
}

/// The saved fit of one material's garment: every adjusted object with its local Transform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeFit {
    pub material_id: String,
    pub objects: Vec<FitObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeLighting {
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "optional_text")]
    pub material_id: Option<String>,
    pub material: String,
    pub min: String,
    pub max: String,
}
impl From<crate::amf_production::MaterialLighting> for RecipeLighting {
    fn from(entry: crate::amf_production::MaterialLighting) -> Self {
        Self { material_id:None, material:entry.material, min:entry.min, max:entry.max }
    }
}
impl RecipeLighting {
    pub fn values(&self) -> crate::amf_production::MaterialLighting {
        crate::amf_production::MaterialLighting {material:self.material.clone(),min:self.min.clone(),max:self.max.clone()}
    }
}

/// VUA's own adjustments to the assembled line, replayed after the Recipe's first assembly:
/// the brightness limits written into each material, the SDK fixes run, named by the SDK's
/// exact words, and the fits each garment received. Older Recipes carry no `adjustments`;
/// the section is optional, as is `fits` inside it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeAdjustments {
    pub lighting: Vec<RecipeLighting>,
    pub sdk_fixes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fits: Vec<RecipeFit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Optional portable text may be absent, but an explicitly null field is not a declaration.
pub(crate) fn optional_text<'de,D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>,D::Error> {
    String::deserialize(deserializer).map(Some)
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

/// A fit object's path: empty names the garment instance root, otherwise a `/`-joined
/// child-name path with no empty segments.
fn fit_object_path(path: &str) -> bool {
    path.chars().count() <= 512 && (path.is_empty() || path.split('/').all(|segment| !segment.is_empty()))
}

pub fn valid_material_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 100 && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b,b'-' | b'_'))
}
impl ProductionRecipe {
    fn valid_guid(&self, guid: &Option<String>) -> bool {
        match guid { None => true, Some(guid) => self.format_version == "0.5" && guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }
    }
    pub fn validate(&self) -> Result<(), AppErrorV1> {
        if !matches!(self.format_version.as_str(), "0.4" | "0.5") || self.recipe_id.is_empty() || self.recipe_id.len() > 100
            || !self.recipe_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !label(&self.title, 120) || !matches!(self.editor_version.as_str(), "2022.3.22f1" | "2022.3.22f1c1")
            || self.materials.is_empty() || self.materials.len() > 64 || self.outfits.is_empty() || self.outfits.len() > 64 {
            return Err(invalid("Unsupported or malformed Recipe header."));
        }
        let mut ids = BTreeSet::new();
        for material in &self.materials {
            if material.sha256.len() != 64 || !material.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || (self.format_version == "0.4" && (material.id != format!("m-{}", material.sha256) || material.booth_product_id.is_some()))
                || !valid_material_id(&material.id) || !ids.insert(material.id.as_str())
                || material.booth_product_id.as_ref().is_some_and(|id| !crate::amf_recipe_share::valid_booth_id(id))
                || !label(&material.package_name, 240) || !material.package_name.to_ascii_lowercase().ends_with(".unitypackage")
                || material.package_name.contains(['/', '\\', ':']) {
                return Err(invalid("Recipe material references must identify bounded original packages."));
            }
        }
        if !ids.contains(self.avatar.material_id.as_str()) || !asset_path(&self.avatar.prefab_path) || !self.valid_guid(&self.avatar.prefab_guid) { return Err(invalid("Invalid Avatar reference.")); }
        let mut outfits = BTreeSet::new();
        for outfit in &self.outfits {
            if !ids.contains(outfit.material_id.as_str()) || !outfits.insert(outfit.material_id.as_str())
                || !asset_path(&outfit.prefab_path) || !self.valid_guid(&outfit.prefab_guid) || outfit.bone.len() > 80 || outfit.bone.chars().any(char::is_control)
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
                || (self.format_version == "0.5" && entry.material_id.as_deref().is_none_or(|id| !ids.contains(id)))
                || (self.format_version == "0.4" && entry.material_id.is_some())
                || !valid_lighting_value(&entry.min) || !valid_lighting_value(&entry.max)) {
                return Err(invalid("Recipe lighting adjustments name bounded material paths and serialized values."));
            }
            if adjustments.sdk_fixes.iter().any(|fix| fix.is_empty() || fix.chars().count() > 600 || fix.contains('\0')) {
                return Err(invalid("Recipe SDK fixes name the SDK's exact words."));
            }
            if adjustments.fits.len() > 64
                || adjustments.fits.iter().any(|fit| fit.objects.len() > 2048
                    || !ids.contains(fit.material_id.as_str())
                    || (self.format_version == "0.4" && (fit.material_id.len() != 66 || !fit.material_id.starts_with("m-")))
                    || fit.objects.iter().any(|object| !fit_object_path(&object.path))) {
                return Err(invalid("Recipe fits reference declared material IDs with bounded object paths."));
            }
        }
        Ok(())
    }

    pub fn from_run(run: &ProductionRun, recipe_id: &str, title: &str, mut adjustments: Option<RecipeAdjustments>) -> Result<Self, AppErrorV1> {
        let selection: AssemblySelection = serde_json::from_value(run.result["facts"]["observedSelection"].clone())
            .map_err(|_| production_error("recipe_scene_unverified", "Reinspect a successfully assembled Avatar before saving a Recipe."))?;
        let source_by_id = run.sources.iter().map(|s| (s.source_id.as_str(), s)).collect::<BTreeMap<_, _>>();
        let avatar_sources = run.prefabs.iter().filter(|p| p.is_avatar && p.path == selection.avatar_prefab)
            .filter_map(|p| source_by_id.get(p.source_id.as_str()).copied()).collect::<Vec<_>>();
        if avatar_sources.len() != 1 { return Err(invalid("The Avatar input is ambiguous.")); }
        let mut materials = BTreeMap::new();
        let mut source_materials = BTreeMap::new();
        let used: Option<Vec<String>> = serde_json::from_value(run.result["facts"]["usedSourceIds"].clone()).ok();
        for source in run.sources.iter().filter(|source| source.source_id == avatar_sources[0].source_id
            || selection.outfits.iter().any(|outfit| outfit.source_id == source.source_id)
            || used.as_ref().is_none_or(|ids| ids.contains(&source.source_id))) {
            let digest = source.sha256.strip_prefix("sha256:").unwrap_or(&source.sha256);
            let id = materials.get(digest).map(|m: &RecipeMaterial| m.id.clone()).unwrap_or_else(|| format!("material-{}", materials.len() + 1));
            source_materials.insert(source.source_id.clone(), id.clone());
            let name = source.name.rsplit(['/', '\\']).next().unwrap_or(&source.name).to_owned();
            materials.entry(digest.to_owned()).or_insert(RecipeMaterial { id, package_name: name, sha256: digest.into(), booth_product_id:None });
        }
        let outfits = selection.outfits.iter().map(|outfit| {
            let source = source_by_id.get(outfit.source_id.as_str()).ok_or_else(|| invalid("An observed outfit has no retained input."))?;
            Ok(RecipeOutfit { material_id: source_materials[&source.source_id].clone(), prefab_path: outfit.prefab_path.clone(),
                prefab_guid:run.prefabs.iter().find(|p| p.source_id == outfit.source_id && p.path == outfit.prefab_path).and_then(|p| p.prefab_guid.clone()), mode: outfit.mode,
                bone: outfit.bone.clone(), toggle_name: outfit.toggle_name.clone(), active: outfit.active })
        }).collect::<Result<Vec<_>, AppErrorV1>>()?;
        if let Some(adjustments) = &mut adjustments {
            for fit in &mut adjustments.fits {
                if let Some(material) = materials.get(fit.material_id.trim_start_matches("m-")) { fit.material_id = material.id.clone(); }
            }
            for entry in &mut adjustments.lighting {
                let owners = run.sources.iter().filter(|s| s.asset_paths.contains(&entry.material)).filter_map(|s| source_materials.get(&s.source_id)).collect::<BTreeSet<_>>();
                if owners.len() != 1 { return Err(invalid("A lighting material has no unique retained package.")); }
                entry.material_id = owners.first().map(|id| (*id).clone());
            }
        }
        let recipe = Self { format_version: "0.5".into(), recipe_id: recipe_id.into(), title: title.trim().into(), editor_version: run.editor.version.clone(),
            materials: materials.into_values().collect(), avatar: RecipePrefab { material_id: source_materials[&avatar_sources[0].source_id].clone(),
                prefab_guid:run.prefabs.iter().find(|p| p.source_id == avatar_sources[0].source_id && p.path == selection.avatar_prefab).and_then(|p| p.prefab_guid.clone()), prefab_path: selection.avatar_prefab },
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
