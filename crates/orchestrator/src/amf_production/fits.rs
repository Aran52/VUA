//! Saved fits: a named set of local-Transform adjustments of one garment on one Avatar,
//! kept across production lines. A fit applies only when both content identities and both
//! prefab paths match exactly. The Unity side writes the per-garment baseline after every
//! assembly and reports moved objects as `fitChanges`; saving a fit folds the moved
//! transforms into the baseline and stores the overall state in `fits/<fitId>.json`.

use super::*;

/// Fits retained per data root; saving beyond this is refused.
pub const FIT_STORE_LIMIT: usize = 500;
/// Objects one fit or one garment's reported changes may carry.
pub const FIT_OBJECTS_LIMIT: usize = 2048;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitQuat {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

/// One garment object's local Transform. `path` is empty for the garment instance root,
/// otherwise a `/`-joined child-name path; a `name#n` segment selects the n-th sibling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitObject {
    pub path: String,
    pub local_position: FitVec3,
    pub local_rotation: FitQuat,
    pub local_scale: FitVec3,
}

/// The adjustments one garment reports: objects moved from their baseline (carrying the new
/// local Transform) and baseline paths that no longer exist. An entry with both empty is
/// never reported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GarmentFitChanges {
    pub source_id: String,
    #[serde(default)]
    pub moved: Vec<FitObject>,
    #[serde(default)]
    pub missing: Vec<String>,
}

/// The identity a fit binds: exact source content hash and prefab path, with a display name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitIdentity {
    pub sha256: String,
    pub prefab_path: String,
    pub name: String,
}

/// One stored fit, as `<root>/fits/<fitId>.json` carries it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredFit {
    pub schema_version: String,
    pub fit_id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub garment: FitIdentity,
    pub avatar: FitIdentity,
    pub objects: Vec<FitObject>,
}

/// The per-garment local-Transform baseline the Unity side writes after every successful
/// assembly, as `<project>/.vua/amf-fit-baseline.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FitBaseline {
    pub schema_version: String,
    pub run_id: String,
    #[serde(default)]
    pub garments: Vec<BaselineGarment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BaselineGarment {
    pub source_id: String,
    pub owner_key: String,
    #[serde(default)]
    pub objects: Vec<FitObject>,
}

/// The editor save hook's report, as `<project>/.vua/bridge/amf-fit-changes.json`. It
/// overlays the run's recorded `fitChanges` when it is newer than the last observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FitChangesFile {
    pub schema_version: String,
    pub run_id: String,
    pub written_at: String,
    #[serde(default)]
    pub fit_changes: Vec<GarmentFitChanges>,
}

/// The fit objects one outfit's assembly receives, engine to platform to Bridge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutfitFit {
    pub source_id: String,
    pub objects: Vec<FitObject>,
}

/// A fit that produced the current assembled state: its objects, with the stored fit's
/// identity and name when it came from the fit store (a Recipe's own fits carry neither).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppliedFit {
    pub source_id: String,
    pub fit_id: Option<String>,
    pub name: Option<String>,
    pub objects: Vec<FitObject>,
}

/// How a plan or the run view names an applicable fit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitRef {
    pub source_id: String,
    pub fit_id: Option<String>,
    pub name: Option<String>,
}

/// A fit identity names a bounded plain file in the store.
fn fit_id_path(root: &Path, fit_id: &str) -> Option<PathBuf> {
    let rest = fit_id.strip_prefix("fit-")?;
    if rest.is_empty() || rest.len() > 72 || !rest.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')) {
        return None;
    }
    Some(root.join("fits").join(format!("{fit_id}.json")))
}

/// The store of saved fits under one AMF production data root.
pub struct FitStore {
    root: PathBuf,
}

impl FitStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn dir(&self) -> PathBuf {
        self.root.join("fits")
    }
    /// Every stored fit, newest first; unreadable or unparseable records are skipped.
    pub fn list(&self) -> Vec<StoredFit> {
        let Ok(entries) = fs::read_dir(self.dir()) else { return Vec::new() };
        let mut fits: Vec<StoredFit> = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name();
                let name = name.to_str()?;
                (name.starts_with("fit-") && name.ends_with(".json")).then(|| {
                    serde_json::from_slice::<StoredFit>(&fs::read(entry.path()).ok()?).ok()
                })?
            })
            .collect();
        fits.sort_by(|a, b| b.fit_id.cmp(&a.fit_id));
        fits
    }
    pub fn count(&self) -> usize {
        self.list().len()
    }
    pub fn get(&self, fit_id: &str) -> Option<StoredFit> {
        let path = fit_id_path(&self.root, fit_id)?;
        serde_json::from_slice(&fs::read(path).ok()?).ok()
    }
    /// The stored fit with this exact garment and Avatar identity, when one exists.
    pub fn find(&self, garment: &FitIdentity, avatar: &FitIdentity) -> Option<StoredFit> {
        self.list().into_iter().find(|fit| fit.garment == *garment && fit.avatar == *avatar)
    }
    pub fn save(&self, fit: &StoredFit) -> Result<(), AppErrorV1> {
        let Some(path) = fit_id_path(&self.root, &fit.fit_id) else {
            return Err(production_error("fit_invalid", "Invalid fit identity."));
        };
        fs::create_dir_all(path.parent().expect("fit parent")).map_err(io_error)?;
        let pending = path.with_extension("pending");
        fs::write(&pending, serde_json::to_vec_pretty(fit).map_err(io_error)?).map_err(io_error)?;
        fs::rename(&pending, &path).map_err(io_error)
    }
    /// Deletes the fit; false when no such fit exists.
    pub fn delete(&self, fit_id: &str) -> Result<bool, AppErrorV1> {
        let Some(path) = fit_id_path(&self.root, fit_id) else { return Ok(false) };
        match fs::remove_file(path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(io_error(error)),
        }
    }
    /// A fresh fit identity: timestamp-ordered and unique within the store.
    fn fresh_id(&self) -> String {
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0);
        let mut id = format!("fit-{nanos}");
        while fit_id_path(&self.root, &id).is_some_and(|path| path.exists()) {
            id.push('_');
        }
        id
    }
}

/// The name a saved fit gets when the player does not choose one: the garment and Avatar
/// names without any whitespace, capped at the name limit.
pub fn default_fit_name(garment_name: &str, avatar_name: &str) -> String {
    let name: String = format!("{garment_name}For{avatar_name}")
        .chars()
        .filter(|c| !c.is_whitespace())
        .take(80)
        .collect();
    if name.is_empty() { "Fit".into() } else { name }
}

/// A fit name is a single line of 1–80 characters without control characters that contains
/// a non-whitespace character.
pub fn valid_fit_name(name: &str) -> bool {
    (1..=80).contains(&name.chars().count())
        && !name.chars().any(char::is_control)
        && name.chars().any(|c| !c.is_whitespace())
}

/// Binds a reported `fitChanges` entry to the line and garment it was reviewed under. The
/// payload's keys are inserted in sorted order so the digest reads the same whether or not
/// the build keeps JSON object insertion order.
pub fn changes_digest(run_id: &str, source_id: &str, entry: &GarmentFitChanges) -> String {
    let value = serde_json::json!({
        "binding": "vua.amf-fit-changes.v1",
        "missing": entry.missing,
        "moved": entry.moved,
        "runId": run_id,
        "sourceId": source_id,
    });
    Sha256::digest(serde_json::to_string(&value).unwrap_or_default().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The current time as the fit files carry it: RFC 3339 UTC with exactly seven fractional
/// digits, so a plain string comparison against the Unity side's round-trip stamps orders.
pub(crate) fn fit_now() -> String {
    crate::time::rfc3339_100ns(SystemTime::now())
}

/// An RFC 3339 UTC `Z` timestamp normalized to at least seven fractional digits, so stamps
/// of either precision string-compare correctly. Malformed stamps normalize to None.
fn fit_timestamp_key(text: &str) -> Option<String> {
    let stamp = text.strip_suffix('Z')?;
    let (head, fraction) = match stamp.split_once('.') {
        Some((head, fraction)) => (head, fraction),
        None => (stamp, ""),
    };
    if head.len() != 19
        || !head.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            10 => byte == b'T',
            13 | 16 => byte == b':',
            _ => byte.is_ascii_digit(),
        })
        || fraction.bytes().any(|byte| !byte.is_ascii_digit())
    {
        return None;
    }
    let mut fraction = fraction.to_owned();
    while fraction.len() < 7 {
        fraction.push('0');
    }
    Some(format!("{head}.{fraction}Z"))
}

pub(crate) fn baseline_file(run: &ProductionRun) -> PathBuf {
    Path::new(&run.project_root).join(".vua/amf-fit-baseline.json")
}

pub(crate) fn changes_file(run: &ProductionRun) -> PathBuf {
    Path::new(&run.project_root).join(".vua/bridge/amf-fit-changes.json")
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), AppErrorV1> {
    fs::create_dir_all(path.parent().expect("fit file parent")).map_err(io_error)?;
    let pending = path.with_extension("pending");
    fs::write(&pending, serde_json::to_vec_pretty(value).map_err(io_error)?).map_err(io_error)?;
    fs::rename(&pending, path).map_err(io_error)
}

/// The garment and Avatar identity a selected outfit binds a fit to, when every piece is
/// observed. Anything missing means no fit applies to the selection — silently.
pub fn fit_identity(
    run: &ProductionRun,
    selection: &AssemblySelection,
    source_id: &str,
) -> Option<(FitIdentity, FitIdentity)> {
    let outfit = selection.outfits.iter().find(|outfit| outfit.source_id == source_id)?;
    let garment_source = run.sources.iter().find(|source| source.source_id == source_id)?;
    let avatar_candidate = run
        .prefabs
        .iter()
        .find(|candidate| candidate.path == selection.avatar_prefab && candidate.is_avatar)?;
    let avatar_source = run
        .sources
        .iter()
        .find(|source| source.source_id == avatar_candidate.source_id)?;
    let bare = |sha256: &str| sha256.strip_prefix("sha256:").unwrap_or(sha256).to_owned();
    Some((
        FitIdentity {
            sha256: bare(&garment_source.sha256),
            prefab_path: outfit.prefab_path.clone(),
            name: garment_source.item_name.clone(),
        },
        FitIdentity {
            sha256: bare(&avatar_source.sha256),
            prefab_path: selection.avatar_prefab.clone(),
            name: avatar_source.item_name.clone(),
        },
    ))
}

impl AmfProductionEngine {
    /// The fit store of this data root.
    pub fn fit_store(&self) -> FitStore {
        FitStore::new(self.root.clone())
    }

    /// The fits an assembly of `selection` applies: a Recipe's own fits where it declares
    /// them for the garment's material (no store identity), stored fits for the other
    /// outfits whose garment and Avatar identity match one.
    pub(crate) fn planned_fits(
        &self,
        run: &ProductionRun,
        selection: &AssemblySelection,
        recipe_fits: &[crate::amf_recipe::RecipeFit],
    ) -> Vec<AppliedFit> {
        let store = self.fit_store();
        selection
            .outfits
            .iter()
            .filter_map(|outfit| {
                let source = run.sources.iter().find(|source| source.source_id == outfit.source_id);
                let material_id = run.recipe_material_sources.iter().find(|(_,id)| **id == outfit.source_id).map(|(material,_)| material.clone()).or_else(|| source.map(|source| {
                    format!("m-{}", source.sha256.strip_prefix("sha256:").unwrap_or(&source.sha256))
                }));
                if let Some(recipe_fit) =
                    material_id.as_ref().and_then(|id| recipe_fits.iter().find(|fit| fit.material_id == *id))
                {
                    return Some(AppliedFit {
                        source_id: outfit.source_id.clone(),
                        fit_id: None,
                        name: None,
                        objects: recipe_fit.objects.clone(),
                    });
                }
                let (garment, avatar) = fit_identity(run, selection, &outfit.source_id)?;
                store.find(&garment, &avatar).map(|fit| AppliedFit {
                    source_id: outfit.source_id.clone(),
                    fit_id: Some(fit.fit_id.clone()),
                    name: Some(fit.name.clone()),
                    objects: fit.objects,
                })
            })
            .collect()
    }

    /// The run view's fit facts: the reported `fitChanges` (the editor save hook's file
    /// overlays the recorded ones when it is newer than the last observation) and the
    /// stored fits each selected outfit could apply on this Avatar.
    pub fn fit_view(&self, run: &ProductionRun) -> (Vec<GarmentFitChanges>, Vec<FitRef>) {
        let overlay = fs::read(changes_file(run))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<FitChangesFile>(&bytes).ok())
            .filter(|file| {
                file.run_id == run.run_id
                    && match (&run.observed_at, fit_timestamp_key(&file.written_at)) {
                        (_, None) => false,
                        (None, Some(_)) => true,
                        (Some(observed), Some(written)) => {
                            fit_timestamp_key(observed).is_some_and(|observed| written > observed)
                        }
                    }
            });
        let changes = overlay.map(|file| file.fit_changes).unwrap_or_else(|| run.fit_changes.clone());
        let store = self.fit_store();
        let available = run
            .selection
            .as_ref()
            .map(|selection| {
                selection
                    .outfits
                    .iter()
                    .filter_map(|outfit| {
                        let (garment, avatar) = fit_identity(run, selection, &outfit.source_id)?;
                        store.find(&garment, &avatar).map(|fit| FitRef {
                            source_id: outfit.source_id.clone(),
                            fit_id: Some(fit.fit_id),
                            name: Some(fit.name),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        (changes, available)
    }

    /// Saves the garment's reported adjustments as a named fit: the baseline transforms with
    /// the moved ones folded in and the missing ones dropped. The reviewed digest binds the
    /// exact reported entry. Afterwards the baseline carries the overall state and the
    /// garment no longer reports changes.
    pub fn save_fit(
        &self,
        run_id: &str,
        source_id: &str,
        name: &str,
        digest: &str,
    ) -> Result<StoredFit, AppErrorV1> {
        if !valid_fit_name(name) {
            return Err(production_error(
                "fit_invalid",
                "A fit name is a single line of 1-80 characters.",
            ));
        }
        let mut run = self.get(run_id)?;
        let (changes, _) = self.fit_view(&run);
        let entry = changes
            .iter()
            .find(|entry| entry.source_id == source_id)
            .ok_or_else(|| {
                production_error("fit_changes_missing", "This garment reports no adjustments to save. Check the line again.")
            })?;
        if changes_digest(&run.run_id, source_id, entry) != digest {
            return Err(production_error(
                "plan_drift",
                "The reported adjustments changed. Review them again.",
            ));
        }
        let selection = run.selection.clone().filter(|selection| {
            selection.outfits.iter().any(|outfit| outfit.source_id == source_id)
        });
        let selection = selection.ok_or_else(|| {
            production_error("fit_changes_missing", "This garment is not part of the line's current selection.")
        })?;
        let (garment, avatar) = fit_identity(&run, &selection, source_id).ok_or_else(|| {
            production_error("fit_changes_missing", "This garment's identity is not fully observed. Reinspect the line.")
        })?;
        let baseline_path = baseline_file(&run);
        let mut baseline: FitBaseline = fs::read(&baseline_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .ok_or_else(|| {
                production_error("fit_baseline_missing", "The assembly baseline is missing. Assemble the line again, then save the fit.")
            })?;
        let Some(baseline_garment) = baseline.garments.iter().find(|g| g.source_id == source_id) else {
            return Err(production_error(
                "fit_baseline_missing",
                "The assembly baseline does not cover this garment. Assemble the line again, then save the fit.",
            ));
        };
        let objects: Vec<FitObject> = baseline_garment
            .objects
            .iter()
            .filter(|object| !entry.missing.contains(&object.path))
            .map(|object| {
                entry
                    .moved
                    .iter()
                    .find(|moved| moved.path == object.path)
                    .cloned()
                    .unwrap_or_else(|| object.clone())
            })
            .collect();
        if objects.len() > FIT_OBJECTS_LIMIT {
            return Err(production_error(
                "fit_too_large",
                format!("A fit carries at most {FIT_OBJECTS_LIMIT} objects."),
            ));
        }
        let store = self.fit_store();
        let now = SystemClock.now_rfc3339();
        let fit = match store.find(&garment, &avatar) {
            Some(existing) => StoredFit {
                name: name.to_owned(),
                updated_at: now,
                objects,
                ..existing
            },
            None => {
                if store.count() >= FIT_STORE_LIMIT {
                    return Err(production_error(
                        "fit_limit",
                        format!("At most {FIT_STORE_LIMIT} fits are kept. Delete one before saving another."),
                    ));
                }
                StoredFit {
                    schema_version: "0.1".into(),
                    fit_id: store.fresh_id(),
                    name: name.to_owned(),
                    created_at: now.clone(),
                    updated_at: now,
                    garment,
                    avatar,
                    objects,
                }
            }
        };
        store.save(&fit)?;
        if let Some(garment) = baseline.garments.iter_mut().find(|g| g.source_id == source_id) {
            garment.objects = fit.objects.clone();
        }
        write_json_atomic(&baseline_path, &baseline)?;
        run.fit_changes.retain(|entry| entry.source_id != source_id);
        if let Ok(bytes) = fs::read(changes_file(&run)) {
            if let Ok(file) = serde_json::from_slice::<FitChangesFile>(&bytes) {
                let remaining: Vec<GarmentFitChanges> = file
                    .fit_changes
                    .into_iter()
                    .filter(|entry| entry.source_id != source_id)
                    .collect();
                let path = changes_file(&run);
                if remaining.is_empty() {
                    fs::remove_file(&path).map_err(io_error)?;
                } else {
                    write_json_atomic(&path, &FitChangesFile { fit_changes: remaining, ..file })?;
                }
            }
        }
        self.save(&run)?;
        Ok(fit)
    }

    /// Every saved fit, newest first.
    pub fn list_fits(&self) -> Result<Vec<StoredFit>, AppErrorV1> {
        Ok(self.fit_store().list())
    }

    pub fn rename_fit(&self, fit_id: &str, name: &str) -> Result<StoredFit, AppErrorV1> {
        if !valid_fit_name(name) {
            return Err(production_error(
                "fit_invalid",
                "A fit name is a single line of 1-80 characters.",
            ));
        }
        let store = self.fit_store();
        let mut fit = store
            .get(fit_id)
            .ok_or_else(|| production_error("fit_not_found", "There is no saved fit with this ID."))?;
        fit.name = name.to_owned();
        fit.updated_at = SystemClock.now_rfc3339();
        store.save(&fit)?;
        Ok(fit)
    }

    pub fn delete_fit(&self, fit_id: &str) -> Result<(), AppErrorV1> {
        if !self.fit_store().delete(fit_id)? {
            return Err(production_error("fit_not_found", "There is no saved fit with this ID."));
        }
        Ok(())
    }
}
