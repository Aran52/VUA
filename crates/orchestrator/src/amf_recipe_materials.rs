//! Recipe material resolution and reviewable mapping against Unity observations.
use crate::amf_production::{production_error, AssemblySelection, ProductionRun};
use crate::amf_recipe::{ProductionRecipe, RecipeAdjustments};
use crate::{AppErrorV1, ParamValue};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type MaterialChoices = BTreeMap<String, String>;

/// Optional Bridge evidence is validated before deserialization; old receipts may omit it.
pub fn validate_inspection(data: &serde_json::Value) -> Result<(), AppErrorV1> {
    let invalid = || {
        production_error(
            "receipt_invalid",
            "Invalid Recipe prefab or material inspection evidence.",
        )
    };
    if let Some(prefabs) = data["prefabs"].as_array() {
        for prefab in prefabs {
            if let Some(guid) = prefab.get("prefabGuid") {
                if !guid.as_str().is_some_and(|guid| {
                    guid.len() == 32
                        && guid
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                }) {
                    return Err(invalid());
                }
            }
            if let Some(paths) = prefab.get("objectPaths") {
                let paths = paths.as_array().ok_or_else(invalid)?;
                if paths.len() > 2048
                    || paths.iter().any(|p| {
                        !p.as_str().is_some_and(|p| {
                            p.chars().count() <= 512 && !p.chars().any(char::is_control)
                        })
                    })
                    || paths
                        .iter()
                        .filter_map(|p| p.as_str())
                        .collect::<BTreeSet<_>>()
                        .len()
                        != paths.len()
                {
                    return Err(invalid());
                }
            }
        }
    }
    if let Some(paths) = data.get("materialPaths") {
        let paths = paths.as_array().ok_or_else(invalid)?;
        if paths.len() > 4096
            || paths.iter().any(|p| {
                !p.as_str().is_some_and(|p| {
                    p.len() <= 2048
                        && p.starts_with("Assets/")
                        && p.ends_with(".mat")
                        && !p.contains(['\\', ':', '\0'])
                        && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
                })
            })
            || paths
                .iter()
                .filter_map(|p| p.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != paths.len()
        {
            return Err(invalid());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaterialPackage {
    pub source_id: String,
    pub package_name: String,
    pub sha256: String,
}

/// Local facts supplied by the adapter; neither a source claim nor a Recipe grants ownership.
#[derive(Debug, Clone)]
pub struct MaterialInventory {
    pub package: MaterialPackage,
    pub product_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaterialStatus {
    Ready,
    MaterialChoiceRequired,
    MaterialNotDownloaded,
    MaterialNotOwned,
    MaterialMissing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeMaterialResolution {
    pub material_id: String,
    pub package_name: String,
    pub booth_product_id: Option<String>,
    pub status: MaterialStatus,
    pub resolved: Option<MaterialPackage>,
    pub candidates: Vec<MaterialPackage>,
    pub download_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeMaterialPlan {
    pub recipe_id: String,
    pub materials: Vec<RecipeMaterialResolution>,
    pub material_choices: MaterialChoices,
    pub ready: bool,
    pub plan_digest: String,
}

pub fn resolve_materials(
    recipe: &ProductionRecipe,
    inventory: &[MaterialInventory],
    purchased: &BTreeMap<String, Vec<String>>,
    choices: &MaterialChoices,
) -> Result<Vec<RecipeMaterialResolution>, AppErrorV1> {
    recipe.validate()?;
    if choices.len() > 64
        || choices
            .keys()
            .any(|id| !recipe.materials.iter().any(|m| &m.id == id))
    {
        return Err(production_error(
            "material_choices_invalid",
            "Choose packages only for this Recipe's materials.",
        ));
    }
    let mut results = Vec::new();
    for material in &recipe.materials {
        let mut exact = inventory
            .iter()
            .filter(|item| item.package.sha256 == material.sha256)
            .map(|item| item.package.clone())
            .collect::<Vec<_>>();
        exact.sort_by(|a, b| a.source_id.cmp(&b.source_id));
        let mut candidates = inventory
            .iter()
            .filter(|item| {
                material
                    .booth_product_id
                    .as_ref()
                    .is_some_and(|id| item.product_ids.contains(id))
            })
            .map(|item| item.package.clone())
            .collect::<Vec<_>>();
        candidates
            .sort_by(|a, b| (&a.package_name, &a.source_id).cmp(&(&b.package_name, &b.source_id)));
        let named = candidates
            .iter()
            .filter(|package| package.package_name == material.package_name)
            .collect::<Vec<_>>();
        let explicit = choices.get(&material.id);
        // Exact bytes remain the first rule. An explicit selection cannot override them.
        let selected = if let Some(package) = exact.first() {
            Some(package.clone())
        } else if named.len() == 1 {
            Some(named[0].clone())
        } else if candidates.len() == 1 {
            Some(candidates[0].clone())
        } else if let Some(id) = explicit {
            Some(
                candidates
                    .iter()
                    .find(|package| &package.source_id == id)
                    .cloned()
                    .ok_or_else(|| {
                        production_error(
                            "material_choices_invalid",
                            "The selected package is not held for this BOOTH product.",
                        )
                    })?,
            )
        } else {
            None
        };
        if explicit.is_some_and(|id| {
            selected
                .as_ref()
                .is_some_and(|package| &package.source_id != id)
        }) {
            return Err(production_error("material_choices_invalid", "An exact original or uniquely resolved package takes precedence. Refresh the material list."));
        }
        let status = if selected.is_some() {
            MaterialStatus::Ready
        } else if !candidates.is_empty() {
            MaterialStatus::MaterialChoiceRequired
        } else if material
            .booth_product_id
            .as_ref()
            .is_some_and(|id| purchased.contains_key(id))
        {
            MaterialStatus::MaterialNotDownloaded
        } else if material.booth_product_id.is_some() {
            MaterialStatus::MaterialNotOwned
        } else {
            MaterialStatus::MaterialMissing
        };
        results.push(RecipeMaterialResolution {
            material_id: material.id.clone(),
            package_name: material.package_name.clone(),
            booth_product_id: material.booth_product_id.clone(),
            status,
            resolved: selected,
            candidates,
            download_names: material
                .booth_product_id
                .as_ref()
                .and_then(|id| purchased.get(id))
                .cloned()
                .unwrap_or_default(),
        });
    }
    Ok(results)
}

pub fn resolved_sources(
    materials: &[RecipeMaterialResolution],
) -> Result<BTreeMap<String, String>, AppErrorV1> {
    if materials.iter().any(|m| m.status != MaterialStatus::Ready) {
        return Err(production_error(
            "recipe_materials_unresolved",
            "Review the complete Recipe material list before preparing or applying it.",
        )
        .with_param(
            "materials",
            ParamValue::Text(serde_json::to_string(materials).unwrap_or_default()),
        )
        .with_param(
            "suggestedAction",
            ParamValue::Text("amf.production.recipeMaterialPlan".into()),
        ));
    }
    Ok(materials
        .iter()
        .filter_map(|m| {
            m.resolved
                .as_ref()
                .map(|p| (m.material_id.clone(), p.source_id.clone()))
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecipeDifferenceKind {
    MaterialFileChanged,
    PrefabMapped,
    PrefabMissing,
    FitObjectsMissing,
    LightingMaterialMissing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeDifference {
    pub kind: RecipeDifferenceKind,
    pub material_id: String,
    pub before: String,
    pub after: Option<String>,
    pub rule: Option<String>,
    pub paths: Vec<String>,
    pub count: usize,
}

pub struct MappedRecipe {
    pub selection: Option<AssemblySelection>,
    pub adjustments: Option<RecipeAdjustments>,
    pub differences: Vec<RecipeDifference>,
}

/// Map only candidates of the resolved input. Ambiguous matches are missing, never guesses.
pub fn map_recipe(
    recipe: &ProductionRecipe,
    run: &ProductionRun,
    materials: &[RecipeMaterialResolution],
) -> Result<MappedRecipe, AppErrorV1> {
    let resolved = resolved_sources(materials)?;
    let mut differences = Vec::new();
    let mut changed = BTreeSet::new();
    for material in &recipe.materials {
        let package = materials
            .iter()
            .find(|m| m.material_id == material.id)
            .and_then(|m| m.resolved.as_ref())
            .expect("resolved material");
        if package.sha256 != material.sha256 || package.package_name != material.package_name {
            changed.insert(material.id.as_str());
            differences.push(RecipeDifference {
                kind: RecipeDifferenceKind::MaterialFileChanged,
                material_id: material.id.clone(),
                before: material.package_name.clone(),
                after: Some(package.package_name.clone()),
                rule: None,
                paths: vec![],
                count: 1,
            });
        }
    }
    let mut complete = true;
    let mut selection = recipe.selection(&resolved)?;
    let mut targets = BTreeMap::new();
    for (id, path, guid, avatar) in std::iter::once((
        &recipe.avatar.material_id,
        &recipe.avatar.prefab_path,
        recipe.avatar.prefab_guid.as_deref(),
        true,
    ))
    .chain(recipe.outfits.iter().map(|o| {
        (
            &o.material_id,
            &o.prefab_path,
            o.prefab_guid.as_deref(),
            false,
        )
    })) {
        let candidates = run
            .prefabs
            .iter()
            .filter(|p| p.source_id == resolved[id] && (!avatar || p.is_avatar))
            .collect::<Vec<_>>();
        let by_guid = candidates
            .iter()
            .filter(|p| guid.is_some() && p.prefab_guid.as_deref() == guid)
            .copied()
            .collect::<Vec<_>>();
        let by_path = candidates
            .iter()
            .filter(|p| p.path == *path)
            .copied()
            .collect::<Vec<_>>();
        let by_name = candidates
            .iter()
            .filter(|p| p.path.rsplit('/').next() == path.rsplit('/').next())
            .copied()
            .collect::<Vec<_>>();
        let matched = if by_guid.len() == 1 {
            Some((by_guid[0], "guid"))
        } else if by_path.len() == 1 {
            Some((by_path[0], "path"))
        } else if by_name.len() == 1 {
            Some((by_name[0], "name"))
        } else {
            None
        };
        if let Some((prefab, rule)) = matched {
            if changed.contains(id.as_str())
                || prefab.path != *path
                || (guid.is_some() && prefab.prefab_guid.as_deref() != guid)
            {
                differences.push(RecipeDifference {
                    kind: RecipeDifferenceKind::PrefabMapped,
                    material_id: id.clone(),
                    before: path.clone(),
                    after: Some(prefab.path.clone()),
                    rule: Some(rule.into()),
                    paths: vec![],
                    count: 1,
                });
            }
            if avatar {
                selection.avatar_prefab = prefab.path.clone();
            } else if let Some(outfit) = selection
                .outfits
                .iter_mut()
                .find(|o| o.source_id == resolved[id])
            {
                outfit.prefab_path = prefab.path.clone();
            }
            targets.insert(id.clone(), prefab);
        } else {
            complete = false;
            differences.push(RecipeDifference {
                kind: RecipeDifferenceKind::PrefabMissing,
                material_id: id.clone(),
                before: path.clone(),
                after: None,
                rule: None,
                paths: vec![],
                count: 1,
            });
        }
    }
    let mut adjustments = recipe.adjustments.clone();
    if let Some(adjustments) = &mut adjustments {
        for fit in &mut adjustments.fits {
            let target = targets.get(&fit.material_id);
            let missing = fit
                .objects
                .iter()
                .filter(
                    |object| match target.and_then(|p| p.object_paths.as_ref()) {
                        Some(paths) => !paths.contains(&object.path),
                        None => changed.contains(fit.material_id.as_str()) || target.is_none(),
                    },
                )
                .map(|o| o.path.clone())
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                fit.objects.retain(|o| !missing.contains(&o.path));
                differences.push(RecipeDifference {
                    kind: RecipeDifferenceKind::FitObjectsMissing,
                    material_id: fit.material_id.clone(),
                    before: String::new(),
                    after: None,
                    rule: Some("path".into()),
                    count: missing.len(),
                    paths: missing,
                });
            }
        }
        adjustments.lighting.retain(|entry| {
            let in_source = entry.material_id.as_ref().is_none_or(|id| {
                resolved.get(id).is_some_and(|source_id| {
                    run.sources.iter().any(|s| {
                        s.source_id == *source_id && s.asset_paths.contains(&entry.material)
                    })
                })
            });
            let present = in_source
                && run.result["facts"]["materialPaths"]
                    .as_array()
                    .map(|paths| paths.iter().any(|p| p.as_str() == Some(&entry.material)))
                    .unwrap_or_else(|| {
                        entry
                            .material_id
                            .as_deref()
                            .is_none_or(|id| !changed.contains(id))
                            && run
                                .sources
                                .iter()
                                .any(|s| s.asset_paths.contains(&entry.material))
                    });
            if !present {
                differences.push(RecipeDifference {
                    kind: RecipeDifferenceKind::LightingMaterialMissing,
                    material_id: entry.material_id.clone().unwrap_or_default(),
                    before: entry.material.clone(),
                    after: None,
                    rule: Some("path".into()),
                    paths: vec![entry.material.clone()],
                    count: 1,
                });
            }
            present
        });
    }
    Ok(MappedRecipe {
        selection: complete.then_some(selection),
        adjustments,
        differences,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn recipe() -> ProductionRecipe {
        let mut recipe: ProductionRecipe = serde_json::from_str(include_str!(
            "../../../schemas/amf-recipe/v0.5/examples/recipe.valid.json"
        ))
        .unwrap();
        for (index, material) in recipe.materials.iter_mut().enumerate() {
            material.booth_product_id = Some(format!("100{}", index + 1));
        }
        recipe
    }
    fn inventory(recipe: &ProductionRecipe) -> Vec<MaterialInventory> {
        recipe
            .materials
            .iter()
            .map(|m| MaterialInventory {
                package: MaterialPackage {
                    source_id: format!("source-{}", m.id),
                    package_name: m.package_name.clone(),
                    sha256: m.sha256.clone(),
                },
                product_ids: m.booth_product_id.clone().into_iter().collect(),
            })
            .collect()
    }
    fn run(recipe: &ProductionRecipe, inventory: &[MaterialInventory]) -> ProductionRun {
        let mut run: ProductionRun = serde_json::from_value(json!({"schemaVersion":"0.1","runId":"n3-recipe","phase":"ready","taskId":"t",
            "editor":{"path":"synthetic","version":recipe.editor_version},"projectRoot":"synthetic","choices":[],"sources":[],"prefabs":[],"projectFingerprint":"p","unityFingerprint":"u",
            "steps":[],"selection":null,"confirmedDigest":null,"result":{"facts":{"materialPaths":[]}},"error":null,"recovery":null})).unwrap();
        for (index, material) in recipe.materials.iter().enumerate() {
            let package = &inventory[index].package;
            let prefab = if material.id == recipe.avatar.material_id {
                &recipe.avatar.prefab_path
            } else {
                &recipe.outfits[0].prefab_path
            };
            let guid = if material.id == recipe.avatar.material_id {
                &recipe.avatar.prefab_guid
            } else {
                &recipe.outfits[0].prefab_guid
            };
            run.sources.push(serde_json::from_value(json!({"sourceId":package.source_id,"role":"auto","itemName":"Synthetic","name":package.package_name,"sha256":package.sha256,"assetPaths":[prefab]})).unwrap());
            run.prefabs.push(serde_json::from_value(json!({"sourceId":package.source_id,"path":prefab,"name":"Synthetic","prefabGuid":guid,"objectPaths":[""],"isAvatar":material.id==recipe.avatar.material_id,
                "hasSkinnedMesh":true,"hasMergeArmature":true,"hasMissingScripts":false,"humanoidBones":[]})).unwrap());
        }
        run
    }
    #[test]
    fn checklist_reports_every_missing_state_and_all_product_packages_without_name_guessing() {
        let mut recipe = recipe();
        let mut inventory = inventory(&recipe);
        let outfit_id = recipe.outfits[0].material_id.clone();
        let item = inventory
            .iter_mut()
            .find(|item| item.package.source_id == format!("source-{outfit_id}"))
            .unwrap();
        item.package.sha256 = "e".repeat(64);
        item.package.package_name = "Coat-2.unitypackage".into();
        let mut other = item.clone();
        other.package.source_id = "other-package".into();
        other.package.package_name = "Coat-3.unitypackage".into();
        inventory.push(other);
        let result =
            resolve_materials(&recipe, &inventory, &BTreeMap::new(), &BTreeMap::new()).unwrap();
        let choice = result.iter().find(|m| m.material_id == outfit_id).unwrap();
        assert_eq!(choice.status, MaterialStatus::MaterialChoiceRequired);
        assert_eq!(choice.candidates.len(), 2);
        let choices = BTreeMap::from([(outfit_id.clone(), "other-package".into())]);
        let result = resolve_materials(&recipe, &inventory, &BTreeMap::new(), &choices).unwrap();
        assert_eq!(
            result
                .iter()
                .find(|m| m.material_id == outfit_id)
                .unwrap()
                .resolved
                .as_ref()
                .unwrap()
                .source_id,
            "other-package"
        );
        let purchased = BTreeMap::from([(
            recipe.materials[0].booth_product_id.clone().unwrap(),
            vec!["Recorded download.zip".into()],
        )]);
        let result = resolve_materials(&recipe, &[], &purchased, &BTreeMap::new()).unwrap();
        assert_eq!(result[0].status, MaterialStatus::MaterialNotDownloaded);
        assert_eq!(result[0].download_names, vec!["Recorded download.zip"]);
        assert_eq!(result[1].status, MaterialStatus::MaterialNotOwned);
        recipe.materials[1].booth_product_id = None;
        assert_eq!(
            resolve_materials(&recipe, &[], &purchased, &BTreeMap::new()).unwrap()[1].status,
            MaterialStatus::MaterialMissing
        );
        assert!(resolved_sources(&result)
            .unwrap_err()
            .params
            .unwrap()
            .contains_key("materials"));
    }
    #[test]
    fn exact_bytes_then_exact_filename_then_single_package_are_the_only_automatic_rules() {
        let recipe = recipe();
        let mut inventory = inventory(&recipe);
        let mut extra = inventory[1].clone();
        extra.package.sha256 = "f".repeat(64);
        extra.package.source_id = "new".into();
        inventory.push(extra);
        let resolve = |items: &[MaterialInventory]| {
            resolve_materials(&recipe, items, &BTreeMap::new(), &BTreeMap::new()).unwrap()
        };
        assert_eq!(
            resolve(&inventory)[1].resolved.as_ref().unwrap().sha256,
            recipe.materials[1].sha256
        );
        inventory[1].package.sha256 = "e".repeat(64);
        inventory[1].package.package_name = "Other.unitypackage".into();
        assert_eq!(
            resolve(&inventory)[1].resolved.as_ref().unwrap().source_id,
            "new"
        );
        inventory.pop();
        assert_eq!(resolve(&inventory)[1].status, MaterialStatus::Ready);
        let bad = BTreeMap::from([(recipe.materials[1].id.clone(), "unrelated-source".into())]);
        assert!(resolve_materials(&recipe, &inventory, &BTreeMap::new(), &bad).is_err());
    }
    #[test]
    fn moved_and_renamed_prefab_uses_guid_and_deleted_prefab_blocks_assembly_with_full_differences()
    {
        let recipe = recipe();
        let mut inventory = inventory(&recipe);
        inventory[1].package.sha256 = "f".repeat(64);
        inventory[1].package.package_name = "Updated.unitypackage".into();
        let materials =
            resolve_materials(&recipe, &inventory, &BTreeMap::new(), &BTreeMap::new()).unwrap();
        let mut run = run(&recipe, &inventory);
        run.prefabs[1].path = "Assets/Moved/Renamed.prefab".into();
        let mapped = map_recipe(&recipe, &run, &materials).unwrap();
        assert_eq!(
            mapped.selection.unwrap().outfits[0].prefab_path,
            "Assets/Moved/Renamed.prefab"
        );
        assert!(mapped
            .differences
            .iter()
            .any(|d| d.kind == RecipeDifferenceKind::PrefabMapped
                && d.rule.as_deref() == Some("guid")));
        assert!(mapped
            .differences
            .iter()
            .any(|d| d.kind == RecipeDifferenceKind::MaterialFileChanged
                && d.after.as_deref() == Some("Updated.unitypackage")));
        assert!(mapped
            .differences
            .iter()
            .any(|d| d.kind == RecipeDifferenceKind::FitObjectsMissing));
        run.prefabs.pop();
        let mapped = map_recipe(&recipe, &run, &materials).unwrap();
        assert!(mapped.selection.is_none());
        assert!(mapped
            .differences
            .iter()
            .any(|d| d.kind == RecipeDifferenceKind::PrefabMissing));
    }
    #[test]
    fn path_then_unique_filename_fallback_and_digest_bind_the_exact_choices_and_differences() {
        let recipe = recipe();
        let mut inventory = inventory(&recipe);
        inventory[1].package.sha256 = "f".repeat(64);
        let materials =
            resolve_materials(&recipe, &inventory, &BTreeMap::new(), &BTreeMap::new()).unwrap();
        let mut run = run(&recipe, &inventory);
        run.prefabs[1].prefab_guid = Some("c".repeat(32));
        let mapped = map_recipe(&recipe, &run, &materials).unwrap();
        assert!(mapped
            .differences
            .iter()
            .any(|d| d.rule.as_deref() == Some("path")));
        run.prefabs[1].path = format!(
            "Assets/Moved/{}",
            recipe.outfits[0].prefab_path.rsplit('/').next().unwrap()
        );
        let mapped = map_recipe(&recipe, &run, &materials).unwrap();
        let selection = mapped.selection.unwrap();
        assert!(mapped
            .differences
            .iter()
            .any(|d| d.rule.as_deref() == Some("name")));
        run.recipe_materials = materials;
        run.recipe_differences = mapped.differences;
        let first = crate::amf_production::plan_digest(&run, &selection).unwrap();
        run.material_choices.insert(
            recipe.outfits[0].material_id.clone(),
            inventory[1].package.source_id.clone(),
        );
        let second = crate::amf_production::plan_digest(&run, &selection).unwrap();
        assert_ne!(first, second);
        run.recipe_differences[0].before = "Another.unitypackage".into();
        assert_ne!(
            second,
            crate::amf_production::plan_digest(&run, &selection).unwrap()
        );
        let duplicate = run.prefabs[1].clone();
        run.prefabs.push(duplicate);
        assert!(map_recipe(&recipe, &run, &run.recipe_materials)
            .unwrap()
            .selection
            .is_none());
    }
}
