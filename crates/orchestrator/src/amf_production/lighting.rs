//! lilToon brightness limits: align each outfit material's lower and upper light limits with
//! the Avatar's, so outfit and body brighten and darken together across worlds. The Bridge's
//! `materialUse` facts say which renderer uses which material; the work project's `.mat` files
//! hold the current values. Only the two value lines of a chosen material change.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// lilToon materials carry their version as a saved float property.
const LILTOON_MARKER: &str = "_lilToonVersion";
/// lilToon's brightness limit properties in a serialized material.
pub const LIGHT_MIN_LIMIT: &str = "_LightMinLimit";
pub const LIGHT_MAX_LIMIT: &str = "_LightMaxLimit";
/// lilToon's own defaults when a property line is absent from the file.
pub const DEFAULT_LIGHT_MIN: &str = "0.05";
pub const DEFAULT_LIGHT_MAX: &str = "1";
/// The Bridge reports at most this many material uses; a run whose facts exceed it is malformed.
const MAX_MATERIAL_USE: usize = 512;

/// One renderer's use of one material, as the Bridge reports it in `materialUse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialUse {
    /// `avatar` for the Avatar itself, otherwise the owning outfit's source identity.
    pub owner: String,
    /// The renderer's hierarchy path relative to the Avatar root.
    pub renderer: String,
    /// The material asset path under `Assets/`.
    pub material: String,
}

/// A material file's brightness limits: the serialized value text of each property when the
/// file carries exactly one such line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaterialLimits {
    pub liltoon: bool,
    pub min: Option<String>,
    pub max: Option<String>,
}

/// A pair of brightness limits as serialized in the material: the exact value text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LightingLimits {
    pub min: String,
    pub max: String,
}

/// One outfit material whose limits differ from the reference, with current and proposed values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LightingEntry {
    pub material: String,
    pub source_id: String,
    pub current: LightingLimits,
    pub proposed: LightingLimits,
}

/// A material the plan deliberately leaves unchanged, with the reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightingSkip {
    pub material: String,
    pub reason: String,
}

/// The reviewed lighting plan: the Avatar's reference limits, the outfit entries to change and
/// the shared materials left alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LightingPlanData {
    pub reference: Option<LightingLimits>,
    /// Why there is no reference: `no_avatar_liltoon` or `tie`.
    pub reference_reason: Option<&'static str>,
    pub entries: Vec<LightingEntry>,
    pub skipped: Vec<LightingSkip>,
}

/// Two serialized values are the same setting when they parse to the same number.
fn same_value(actual: &str, reference: &str) -> bool {
    match (actual.parse::<f64>(), reference.parse::<f64>()) {
        (Ok(actual), Ok(reference)) => actual == reference,
        _ => actual == reference,
    }
}

/// The renderer's own name: the last segment of its hierarchy path ("" for the Avatar root).
fn renderer_name(renderer: &str) -> &str {
    renderer.rsplit('/').next().unwrap_or(renderer)
}

/// The serialized value tokens of one property's lines, wherever they sit in the file. A
/// property line looks like `    - _LightMinLimit: 0.05`.
fn property_tokens(bytes: &[u8], property: &str) -> Vec<String> {
    let needle = format!("- {property}:");
    let mut tokens = Vec::new();
    for raw in bytes.split(|byte| *byte == b'\n') {
        let content = raw.strip_suffix(b"\r").unwrap_or(raw);
        let start = content.iter().take_while(|byte| **byte == b' ' || **byte == b'\t').count();
        let Some(rest) = content[start..].strip_prefix(needle.as_bytes()) else { continue };
        let token_start = rest.iter().take_while(|byte| **byte == b' ' || **byte == b'\t').count();
        let token = &rest[token_start..];
        let end = token.iter().rev().take_while(|byte| **byte == b' ' || **byte == b'\t').count();
        tokens.push(String::from_utf8_lossy(&token[..token.len() - end]).into_owned());
    }
    tokens
}

/// Reads a serialized material: whether it is lilToon and its two limit lines, each present
/// exactly once. Materials with zero or repeated lines carry no value for that property.
pub fn parse_material_limits(bytes: &[u8]) -> MaterialLimits {
    let one = |property: &str| {
        let tokens = property_tokens(bytes, property);
        (tokens.len() == 1).then(|| tokens.into_iter().next().expect("one token"))
    };
    MaterialLimits {
        liltoon: bytes
            .windows(LILTOON_MARKER.len())
            .any(|window| window == LILTOON_MARKER.as_bytes()),
        min: one(LIGHT_MIN_LIMIT),
        max: one(LIGHT_MAX_LIMIT),
    }
}

/// A material's effective limits: its serialized lines, or lilToon's defaults where absent.
fn effective_limits(limits: &MaterialLimits) -> LightingLimits {
    LightingLimits {
        min: limits.min.clone().unwrap_or_else(|| DEFAULT_LIGHT_MIN.into()),
        max: limits.max.clone().unwrap_or_else(|| DEFAULT_LIGHT_MAX.into()),
    }
}

/// The plan over the observed material use and the current material files. The reference is the
/// Avatar's own lilToon limits: the `Body` renderer's pair, or the most common pair across the
/// Avatar's lilToon materials (one vote per material). An ambiguous Body pair, a tied vote and
/// an Avatar without lilToon materials all leave the plan without a reference and without
/// entries. Materials the Avatar and an outfit share are never changed; non-lilToon materials
/// and materials without both limit lines never appear.
pub fn compute_lighting_plan(
    uses: &[MaterialUse],
    materials: &BTreeMap<String, MaterialLimits>,
) -> LightingPlanData {
    let mut avatar_limits: BTreeMap<&str, LightingLimits> = BTreeMap::new();
    let mut outfit_owner: BTreeMap<&str, &str> = BTreeMap::new();
    for use_ in uses {
        let Some(limits) = materials.get(use_.material.as_str()).filter(|limits| limits.liltoon) else { continue };
        if use_.owner == "avatar" {
            avatar_limits.entry(use_.material.as_str()).or_insert_with(|| effective_limits(limits));
        } else {
            outfit_owner.entry(use_.material.as_str()).or_insert(use_.owner.as_str());
        }
    }
    let shared: BTreeSet<&str> = avatar_limits
        .keys()
        .copied()
        .filter(|material| outfit_owner.contains_key(material))
        .collect();
    let body: BTreeSet<&LightingLimits> = uses
        .iter()
        .filter(|use_| use_.owner == "avatar" && renderer_name(&use_.renderer).eq_ignore_ascii_case("body"))
        .filter_map(|use_| avatar_limits.get(use_.material.as_str()))
        .collect();
    let (reference, reference_reason) = if body.len() == 1 {
        (Some((*body.iter().next().expect("one Body pair")).clone()), None)
    } else if body.len() > 1 {
        (None, Some("tie"))
    } else if avatar_limits.is_empty() {
        (None, Some("no_avatar_liltoon"))
    } else {
        let mut votes: BTreeMap<&LightingLimits, usize> = BTreeMap::new();
        for limits in avatar_limits.values() {
            *votes.entry(limits).or_default() += 1;
        }
        let best = votes.values().copied().max().expect("a vote exists");
        let winners: Vec<&LightingLimits> = votes
            .iter()
            .filter_map(|(limits, count)| (*count == best).then_some(*limits))
            .collect();
        match winners.as_slice() {
            [reference] => (Some((*reference).clone()), None),
            _ => (None, Some("tie")),
        }
    };
    let mut plan = LightingPlanData { reference, reference_reason, ..Default::default() };
    for (material, owner) in outfit_owner {
        if shared.contains(material) {
            plan.skipped.push(LightingSkip { material: material.into(), reason: "shared_with_avatar".into() });
            continue;
        }
        let (Some(reference), Some(limits)) = (plan.reference.as_ref(), materials.get(material)) else { continue };
        let (Some(min), Some(max)) = (limits.min.as_ref(), limits.max.as_ref()) else { continue };
        let current = LightingLimits { min: min.clone(), max: max.clone() };
        if !same_value(&current.min, &reference.min) || !same_value(&current.max, &reference.max) {
            plan.entries.push(LightingEntry {
                material: material.into(),
                source_id: owner.into(),
                current,
                proposed: reference.clone(),
            });
        }
    }
    plan
}

/// An editable material path: a `.mat` asset inside the work project's `Assets/`.
pub fn safe_material_path(path: &str) -> bool {
    path.len() <= 4096
        && path.starts_with("Assets/")
        && path.to_ascii_lowercase().ends_with(".mat")
        && !path.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".." && !part.ends_with(['.', ' ']))
}

/// Resolves a reported material path to its file, which stays inside the work project.
fn material_file(project: &Path, material: &str) -> Result<PathBuf, AppErrorV1> {
    let root = fs::canonicalize(project).map_err(|e| production_error("project_missing", e.to_string()))?;
    let file = material.split('/').fold(project.to_path_buf(), |path, part| path.join(part));
    let actual = fs::canonicalize(&file)
        .map_err(|e| production_error("material_unreadable", format!("{material}: {e}")))?;
    if !actual.starts_with(&root) {
        return Err(production_error(
            "result_invalid",
            format!("A material path leaves the work project: {material}"),
        ));
    }
    Ok(actual)
}

/// The run's reported material use, checked against the material path rules. Runs checked before
/// the Bridge reported material use need a fresh inspection first.
fn material_use(run: &ProductionRun) -> Result<Vec<MaterialUse>, AppErrorV1> {
    let Some(value) = run.result["facts"].as_object().and_then(|facts| facts.get("materialUse")) else {
        return Err(production_error(
            "inspect_required",
            "This run's check has no material use facts. Reinspect the work project.",
        ));
    };
    let items = value
        .as_array()
        .filter(|items| items.len() <= MAX_MATERIAL_USE)
        .ok_or_else(|| production_error("result_invalid", "Material use facts are malformed."))?;
    let mut uses = Vec::with_capacity(items.len());
    for item in items {
        let owner = item["owner"].as_str().filter(|owner| !owner.is_empty());
        let renderer = item["renderer"].as_str();
        let material = item["material"].as_str().filter(|material| safe_material_path(material));
        let (Some(owner), Some(renderer), Some(material)) = (owner, renderer, material) else {
            return Err(production_error("result_invalid", "Material use facts are malformed."));
        };
        uses.push(MaterialUse { owner: owner.into(), renderer: renderer.into(), material: material.into() });
    }
    Ok(uses)
}

/// The current limits of every material the reported use names, read from the work project.
fn read_material_limits(
    project: &Path,
    uses: &[MaterialUse],
) -> Result<BTreeMap<String, MaterialLimits>, AppErrorV1> {
    let mut materials = BTreeMap::new();
    for use_ in uses {
        if materials.contains_key(use_.material.as_str()) {
            continue;
        }
        let bytes = fs::read(material_file(project, &use_.material)?)
            .map_err(|e| production_error("material_unreadable", format!("{}: {e}", use_.material)))?;
        materials.insert(use_.material.clone(), parse_material_limits(&bytes));
    }
    Ok(materials)
}

/// The digest binds the line, the current project fingerprint and the whole reviewed plan.
fn lighting_digest(
    run: &ProductionRun,
    current_fingerprint: &str,
    plan: &LightingPlanData,
) -> Result<String, AppErrorV1> {
    let value = serde_json::json!({
        "operation": "lighting",
        "runId": run.run_id,
        "revision": run.revision,
        "projectRoot": run.project_root,
        "projectFingerprint": current_fingerprint,
        "reference": plan.reference,
        "entries": plan.entries,
        "skipped": plan.skipped,
    });
    Ok(Sha256::digest(serde_json::to_vec(&value).map_err(io_error)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Replaces the value text of one property's single line. Any other current text, a missing
/// line or repeated lines refuse the change; everything else stays byte-identical, line endings
/// included.
fn rewrite_property(
    bytes: &[u8],
    property: &str,
    current: &str,
    proposed: &str,
) -> Result<Vec<u8>, AppErrorV1> {
    let needle = format!("- {property}:");
    let mut out = Vec::with_capacity(bytes.len());
    let mut replaced = 0usize;
    for (index, raw) in bytes.split(|byte| *byte == b'\n').enumerate() {
        if index > 0 {
            out.push(b'\n');
        }
        let content = raw.strip_suffix(b"\r").unwrap_or(raw);
        let start = content.iter().take_while(|byte| **byte == b' ' || **byte == b'\t').count();
        let Some(rest) = content[start..].strip_prefix(needle.as_bytes()) else {
            out.extend_from_slice(raw);
            continue;
        };
        let token_start = start + needle.len() + rest.iter().take_while(|byte| **byte == b' ' || **byte == b'\t').count();
        let mut end = content.len();
        while end > token_start && (content[end - 1] == b' ' || content[end - 1] == b'\t') {
            end -= 1;
        }
        if &content[token_start..end] != current.as_bytes() {
            return Err(production_error(
                "material_changed",
                format!("The {property} value changed since the review. Plan the lighting again."),
            ));
        }
        replaced += 1;
        out.extend_from_slice(&content[..token_start]);
        out.extend_from_slice(proposed.as_bytes());
        out.extend_from_slice(&content[end..]);
        if raw.len() > content.len() {
            out.push(b'\r');
        }
    }
    if replaced != 1 {
        return Err(production_error(
            "material_invalid",
            format!("A material must carry exactly one {property} line; found {replaced}."),
        ));
    }
    Ok(out)
}

/// Writes the reviewed limits into each chosen material. A property already equal to the
/// reference keeps its line untouched.
fn write_lighting(project: &Path, entries: &[LightingEntry]) -> Result<(), AppErrorV1> {
    for entry in entries {
        let file = material_file(project, &entry.material)?;
        let bytes = fs::read(&file)
            .map_err(|e| production_error("material_unreadable", format!("{}: {e}", entry.material)))?;
        let mut updated = bytes.clone();
        if !same_value(&entry.current.min, &entry.proposed.min) {
            updated = rewrite_property(&updated, LIGHT_MIN_LIMIT, &entry.current.min, &entry.proposed.min)?;
        }
        if !same_value(&entry.current.max, &entry.proposed.max) {
            updated = rewrite_property(&updated, LIGHT_MAX_LIMIT, &entry.current.max, &entry.proposed.max)?;
        }
        if updated != bytes {
            let pending = file.with_extension("pending");
            fs::write(&pending, &updated)
                .map_err(|e| production_error("material_unwritable", format!("{}: {e}", entry.material)))?;
            fs::rename(&pending, &file)
                .map_err(|e| production_error("material_unwritable", format!("{}: {e}", entry.material)))?;
        }
    }
    Ok(())
}

/// After the recheck, each written material must hold the reviewed values on disk.
fn verify_lighting(project: &Path, entries: &[LightingEntry]) -> Result<(), AppErrorV1> {
    for entry in entries {
        let bytes = fs::read(material_file(project, &entry.material)?)
            .map_err(|e| production_error("material_unreadable", format!("{}: {e}", entry.material)))?;
        let limits = parse_material_limits(&bytes);
        let holds = limits.min.as_deref().is_some_and(|min| same_value(min, &entry.proposed.min))
            && limits.max.as_deref().is_some_and(|max| same_value(max, &entry.proposed.max));
        if !holds {
            return Err(production_error(
                "lighting_unverified",
                format!("{} does not hold the reviewed lighting limits.", entry.material),
            ));
        }
    }
    Ok(())
}

impl AmfProductionEngine {
    /// The line's lighting plan as its assembled facts and current material files produce it.
    fn lighting_plan(&self, run: &ProductionRun) -> Result<(LightingPlanData, String), AppErrorV1> {
        if run.phase != "succeeded" {
            return Err(production_error(
                "result_not_ready",
                "Assemble the Avatar before aligning material lighting.",
            ));
        }
        let uses = material_use(run)?;
        let (unchanged, current) = self.project_state(run)?;
        if !unchanged {
            return Err(production_error(
                "project_drift",
                "The work project changed since its last check. Reinspect it before planning lighting.",
            ));
        }
        let materials = read_material_limits(Path::new(&run.project_root), &uses)?;
        let plan = compute_lighting_plan(&uses, &materials);
        let digest = lighting_digest(run, &current, &plan)?;
        Ok((plan, digest))
    }

    /// Reviews the lilToon brightness alignment of an assembled line.
    pub fn plan_lighting(&self, run_id: &str) -> Result<serde_json::Value, AppErrorV1> {
        let run = self.get(run_id)?;
        let (plan, digest) = self.lighting_plan(&run)?;
        Ok(serde_json::json!({
            "schemaVersion": "0.1",
            "runId": run.run_id,
            "planDigest": digest,
            "reference": plan.reference,
            "referenceReason": plan.reference_reason,
            "entries": plan.entries,
            "skipped": plan.skipped,
        }))
    }

    /// Aligns the chosen materials behind a restore point, then rechecks the project in Unity so
    /// the run's facts reflect the change. Any failure restores the protected project.
    pub fn apply_lighting(
        &self,
        run_id: &str,
        digest: &str,
        materials: &[String],
        ctx: &TaskContext,
    ) -> Result<(), AppErrorV1> {
        let mut run = self.get(run_id)?;
        let (plan, actual) = self.lighting_plan(&run)?;
        if actual != digest {
            return Err(production_error(
                "plan_drift",
                "The reviewed lighting plan changed. Review it again.",
            ));
        }
        let mut chosen = Vec::new();
        let mut named = BTreeSet::new();
        for material in materials {
            if !named.insert(material) {
                return Err(production_error("materials_invalid", "Name each material once."));
            }
            let Some(entry) = plan.entries.iter().find(|entry| entry.material == *material) else {
                return Err(production_error(
                    "materials_invalid",
                    format!("{material} is not a reviewed lighting entry."),
                ));
            };
            chosen.push(entry.clone());
        }
        if chosen.is_empty() {
            return Err(production_error("materials_invalid", "Choose at least one reviewed material."));
        }
        self.ensure_project_matches(&mut run)?;
        let phase = run.phase.clone();
        run.task_id = ctx.task_id().into();
        run.phase = "assembling".into();
        run.error = None;
        run.recovery = None;
        let mut snapshot = None;
        // The recheck replaces the observed facts; a restore brings these back with the files.
        let observed_before = (run.prefabs.clone(), run.result.clone(), run.unity_fingerprint.clone(), run.project_fingerprint.clone());
        let result = (|| {
            self.step(&mut run, ctx, "snapshot", "Protect the work project")?;
            snapshot = Some(self.platform.snapshot(&run)?);
            run.snapshot_id = snapshot.clone();
            run.snapshot_selection = run.selection.clone();
            run.snapshot_sources = None;
            self.step(&mut run, ctx, "lighting", "Align lilToon brightness limits")?;
            write_lighting(Path::new(&run.project_root), &chosen)?;
            if let Some(error) = ctx.cancellation_error("amf-production") {
                return Err(error);
            }
            self.step(&mut run, ctx, "inspect", "Recheck the work project in Unity")?;
            self.update_observations(&mut run, ctx)?;
            if let Some(error) = ctx.cancellation_error("amf-production") {
                return Err(error);
            }
            verify_lighting(Path::new(&run.project_root), &chosen)?;
            run.phase = phase.clone();
            run.confirmed_digest = None;
            Ok(())
        })();
        if result.is_err() {
            if let Some(snapshot) = snapshot {
                run.recovery = Some(match self.platform.restore(&run, &snapshot) {
                    Ok(()) => {
                        (run.prefabs, run.result, run.unity_fingerprint, run.project_fingerprint) = observed_before;
                        "restored".into()
                    }
                    Err(error) => format!("restore_failed: {}", error.code),
                });
            }
        }
        self.finish(&mut run, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mat(body: &str) -> Vec<u8> {
        format!("%YAML 1.1\n--- !u!21 &2100000\nMaterial:\n  m_SavedProperties:\n    m_Floats:\n{body}    m_TexEnvs: []\n").into_bytes()
    }

    #[test]
    fn parse_reads_the_two_limit_lines_once_each() {
        let limits = parse_material_limits(&mat("    - _LightMaxLimit: 1\n    - _LightMinLimit: 0.05\n    - _lilToonVersion: 44\n"));
        assert!(limits.liltoon);
        assert_eq!(limits.min.as_deref(), Some("0.05"));
        assert_eq!(limits.max.as_deref(), Some("1"));
        // A property written zero or two times carries no readable value.
        let missing = parse_material_limits(&mat("    - _lilToonVersion: 44\n"));
        assert_eq!((missing.min, missing.max), (None, None));
        let doubled = parse_material_limits(&mat("    - _LightMinLimit: 0.05\n    - _LightMinLimit: 0.1\n    - _lilToonVersion: 44\n"));
        assert_eq!(doubled.min, None);
        // The marker decides; a Standard material has no lilToon reading.
        assert!(!parse_material_limits(&mat("    - _Glossiness: 0.5\n")).liltoon);
    }

    #[test]
    fn rewrite_changes_only_the_value_text_and_keeps_the_line_ending() {
        let original = mat("    - _LightMinLimit: 0.05\r\n    - _LightMaxLimit: 1\r\n    - _lilToonVersion: 44\r\n");
        let rewritten = rewrite_property(&original, LIGHT_MIN_LIMIT, "0.05", "0.1").unwrap();
        assert_eq!(
            String::from_utf8(rewritten).unwrap(),
            String::from_utf8(original.clone()).unwrap().replace("    - _LightMinLimit: 0.05\r\n", "    - _LightMinLimit: 0.1\r\n"),
            "the CRLF line keeps its ending and indentation"
        );
        // The last line may carry no terminator.
        let tail = b"    - _LightMinLimit: 0.05";
        assert_eq!(rewrite_property(tail, LIGHT_MIN_LIMIT, "0.05", "1").unwrap(), b"    - _LightMinLimit: 1");
        // A different current value, a missing line and a repeated line all refuse.
        assert_eq!(rewrite_property(&original, LIGHT_MIN_LIMIT, "0.2", "0.1").unwrap_err().code, "vua.amf_production.material_changed");
        assert_eq!(rewrite_property(b"Material:\n", LIGHT_MIN_LIMIT, "0.05", "0.1").unwrap_err().code, "vua.amf_production.material_invalid");
        let doubled = mat("    - _LightMinLimit: 0.05\n    - _LightMinLimit: 0.05\n");
        assert!(rewrite_property(&doubled, LIGHT_MIN_LIMIT, "0.05", "0.1").is_err());
    }

    #[test]
    fn values_compare_as_numbers() {
        assert!(same_value("0.1", "0.10"));
        assert!(same_value("1", "1.0"));
        assert!(!same_value("0.05", "0.1"));
    }
}
