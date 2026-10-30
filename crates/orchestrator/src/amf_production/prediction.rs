//! Pre-assembly cost prediction (freeze row F5, slice 35 item 2). The plan lists, for the
//! Avatar and every selected outfit, the size, triangles, material slots, texture memory,
//! PhysBones and synced parameter bits the assembled Avatar is expected to carry. Every
//! count is read from the imported prefab before assembly; texture memory reuses the
//! texture-memory report's decoded-size rules (slice 9) and parameter bits reuse the
//! parameter-budget estimate (slice 33). The SDK's own measurement after the build remains
//! the authoritative number — see the tolerance table in docs/protocols/amf-production-v0.1.md.
use super::parameter_budget::estimate_parameters;
use super::*;

/// The platform whose imported texture settings the prediction decodes.
pub const PREDICTION_PLATFORM: &str = "pc";

/// Plausible bounds for one observation; a receipt beyond them is a contract violation, not
/// a number to display.
const MAX_TRIANGLES: u64 = 500_000_000;
const MAX_RENDERERS: u32 = 4096;
const MAX_MATERIAL_SLOTS: u32 = 8192;
const MAX_BYTES: u64 = 1 << 40;
const MAX_PHYS_BONES: u32 = 65536;

pub fn footprint_valid(footprint: &CandidateFootprint) -> bool {
    footprint.triangles <= MAX_TRIANGLES
        && footprint.meshes <= MAX_RENDERERS
        && footprint.skinned_meshes <= footprint.meshes
        && footprint.material_slots <= MAX_MATERIAL_SLOTS
        && footprint.texture_bytes <= MAX_BYTES
        && footprint.mesh_bytes <= MAX_BYTES
        && footprint.phys_bones <= MAX_PHYS_BONES
}

/// Every candidate's footprint must stay inside its contract before it is stored.
pub fn validate_prefab_footprints(prefabs: &[PrefabCandidate]) -> Result<(), AppErrorV1> {
    if prefabs.iter().any(|prefab| prefab.footprint.as_ref().is_some_and(|footprint| !footprint_valid(footprint))) {
        return Err(production_error(
            "receipt_invalid",
            "Footprint evidence exceeds its contract.",
        ));
    }
    Ok(())
}

/// What one candidate contributes: the six values plus whether every one of them was decided.
struct PredictedRow {
    source_id: String,
    name: String,
    role: &'static str,
    triangles: u64,
    material_slots: u32,
    texture_bytes: u64,
    phys_bones: u32,
    parameter_bits: Option<u32>,
    uncompressed_bytes: u64,
    complete: bool,
}

impl PredictedRow {
    fn json(&self) -> serde_json::Value {
        serde_json::json!({"sourceId":self.source_id,"name":self.name,"role":self.role,
            "triangles":self.triangles,"materialSlots":self.material_slots,
            "textureBytes":self.texture_bytes,"physBones":self.phys_bones,
            "parameterBits":self.parameter_bits,"uncompressedBytes":self.uncompressed_bytes,
            "complete":self.complete})
    }
}

/// The plan's pre-assembly prediction. The document is `available: false` with no rows when
/// a selected candidate carries no footprint (an AMF build that predates the observation),
/// so the page can stay honest instead of showing zeros.
pub fn plan_prediction(run: &ProductionRun, selection: &AssemblySelection) -> serde_json::Value {
    let parameters = estimate_parameters(run, selection);
    let avatar = run.prefabs.iter().find(|prefab| prefab.is_avatar && prefab.path == selection.avatar_prefab);
    let outfits: Vec<(&OutfitSelection, &PrefabCandidate)> = selection
        .outfits
        .iter()
        .filter_map(|outfit| {
            run.prefabs
                .iter()
                .find(|prefab| {
                    !prefab.is_avatar && prefab.source_id == outfit.source_id && prefab.path == outfit.prefab_path
                })
                .map(|prefab| (outfit, prefab))
        })
        .collect();
    let selected: Vec<(&'static str, &PrefabCandidate)> = avatar
        .map(|prefab| ("avatar", prefab))
        .into_iter()
        .chain(outfits.iter().map(|(_, prefab)| ("outfit", *prefab)))
        .collect();
    let tolerance = serde_json::json!({"counts":"exact","textureBytes":"decoded_estimate",
        "uncompressedBytes":"within_25_percent"});
    let document = |available: bool, rows: Vec<serde_json::Value>, total: Option<serde_json::Value>| {
        let mut document = serde_json::json!({"schemaVersion":"0.1","evidence":"unity_source_estimate",
            "platform":PREDICTION_PLATFORM,"available":available,"tolerance":tolerance,"rows":rows});
        if let Some(total) = total { document["total"] = total; }
        document
    };
    if selected.is_empty() || selected.iter().any(|(_, prefab)| prefab.footprint.is_none()) {
        return document(false, vec![], None);
    }
    // The Avatar's own parameter row is its Expression Parameters plus its MA Parameters;
    // every outfit's row is the bits that outfit adds, including VUA's own switch.
    let avatar_bits = parameters.known.then_some(parameters.expression_bits + parameters.avatar_ma_bits);
    let mut rows = Vec::with_capacity(selected.len());
    for (role, prefab) in &selected {
        let footprint = prefab.footprint.expect("checked above");
        let parameter_bits = if !parameters.known {
            None
        } else if *role == "avatar" {
            avatar_bits
        } else {
            parameters.outfits.iter().find(|(source_id, ..)| *source_id == prefab.source_id).map(|(.., bits, _)| *bits)
        };
        rows.push(PredictedRow {
            source_id: prefab.source_id.clone(),
            name: prefab.name.clone(),
            role,
            triangles: footprint.triangles,
            material_slots: footprint.material_slots,
            texture_bytes: footprint.texture_bytes,
            phys_bones: footprint.phys_bones,
            parameter_bits,
            // Everything the built Avatar is charged for that the import can state before
            // assembly: decoded textures plus mesh buffers.
            uncompressed_bytes: footprint.texture_bytes.saturating_add(footprint.mesh_bytes),
            complete: footprint.texture_complete && footprint.mesh_complete
                && footprint.phys_bones_known && parameter_bits.is_some(),
        });
    }
    let sum = |value: fn(&PredictedRow) -> u64| rows.iter().map(value).sum::<u64>();
    let parameter_total = rows.iter().map(|row| row.parameter_bits).try_fold(0u32, |total, bits| {
        bits.map(|bits| total + bits)
    });
    let total = serde_json::json!({
        "triangles": sum(|row| row.triangles),
        "materialSlots": rows.iter().map(|row| row.material_slots).sum::<u32>(),
        "textureBytes": sum(|row| row.texture_bytes),
        "physBones": rows.iter().map(|row| row.phys_bones).sum::<u32>(),
        "parameterBits": parameter_total,
        "uncompressedBytes": sum(|row| row.uncompressed_bytes),
        "complete": rows.iter().all(|row| row.complete),
    });
    document(true, rows.iter().map(PredictedRow::json).collect(), Some(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn footprint(triangles: u64, texture: u64, mesh: u64) -> CandidateFootprint {
        CandidateFootprint { triangles, material_slots: 2, meshes: 2, skinned_meshes: 1,
            texture_bytes: texture, texture_complete: true, mesh_bytes: mesh, mesh_complete: true,
            phys_bones: 3, phys_bones_known: true }
    }
    fn budget(names: &[(&str, u8)]) -> ParameterBudget {
        ParameterBudget { expression_parameters_observed: true, expression_parameters: vec![],
            ma_components: vec![ParameterComponent { path: String::new(),
                parameters: names.iter().map(|(name, bits)| ParameterEntry { name: (*name).into(), bits: *bits, unique: false }).collect() }],
            unresolved_prefix_configs: 0, truncated: false }
    }
    fn candidate(source: &str, name: &str, avatar: bool, made: Option<CandidateFootprint>, parameters: Option<ParameterBudget>) -> PrefabCandidate {
        PrefabCandidate { source_id: source.into(), path: format!("Assets/{source}.prefab"), name: name.into(),
            is_avatar: avatar, has_skinned_mesh: true, has_merge_armature: !avatar, has_missing_scripts: false,
            humanoid_bones: vec![], preview: None, parameter_budget: parameters, prefab_guid: None,
            object_paths: None, footprint: made }
    }
    fn run_of(prefabs: Vec<PrefabCandidate>) -> ProductionRun {
        ProductionRun { run_id: "n3-prediction".into(), prefabs, ..serde_json::from_value(serde_json::json!({
            "schemaVersion":"0.1","runId":"n3-prediction","phase":"succeeded","taskId":"t",
            "editor":{"path":"e","version":"2022.3.22f1c1"},
            "projectRoot":"projects/n3-prediction","choices":[],"sources":[],"prefabs":[],"steps":[],
            "selection":null,"projectFingerprint":"p","unityFingerprint":"u","confirmedDigest":null,
            "error":null,"recovery":null,"result":null})).expect("synthetic run") }
    }
    fn selection(pairs: &[(&str, &str)]) -> AssemblySelection {
        AssemblySelection { avatar_prefab: "Assets/avatar.prefab".into(),
            outfits: pairs.iter().map(|(source, toggle)| OutfitSelection {
                source_id: (*source).into(), prefab_path: format!("Assets/{source}.prefab"),
                mode: BindingMode::MergeArmature, bone: String::new(), toggle_name: (*toggle).into(),
                active: true, saved: true, group: String::new() }).collect() }
    }
    fn p02() -> ProductionRun {
        run_of(vec![
            candidate("avatar", "Meiyun", true, Some(footprint(70_000, 120 << 20, 20 << 20)), Some(budget(&[("VRCEmote", 8)]))),
            candidate("outfit", "TshirtShorts", false, Some(footprint(30_000, 60 << 20, 8 << 20)), Some(budget(&[("Shirt", 1)]))),
            candidate("costume", "Costume4", false, Some(footprint(25_000, 40 << 20, 6 << 20)), Some(budget(&[("Dress", 1)]))),
        ])
    }

    #[test]
    fn rows_cover_the_avatar_and_every_selected_outfit_with_all_six_values() {
        let run = p02();
        let plan = plan_prediction(&run, &selection(&[("outfit", "Tshirt"), ("costume", "Costume")]));
        assert_eq!(plan["available"], true);
        let rows = plan["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["role"], "avatar");
        assert_eq!(rows[1]["name"], "TshirtShorts");
        // Size is decoded textures plus mesh buffers, stated in bytes.
        assert_eq!(rows[1]["triangles"], 30_000u64);
        assert_eq!(rows[1]["materialSlots"], 2u32);
        assert_eq!(rows[1]["textureBytes"], 60u64 << 20);
        assert_eq!(rows[1]["uncompressedBytes"], (60u64 << 20) + (8u64 << 20));
        assert_eq!(rows[1]["physBones"], 3u32);
        // The Avatar's row carries its own Expression plus MA bits; an outfit's row carries
        // the bits it adds, including VUA's switch.
        assert_eq!(rows[0]["parameterBits"], 8u32);
        assert_eq!(rows[1]["parameterBits"], 2u32);
        assert!(rows.iter().all(|row| row["complete"] == true));
        let total = &plan["total"];
        assert_eq!(total["triangles"], 125_000u64);
        assert_eq!(total["materialSlots"], 6u32);
        assert_eq!(total["textureBytes"], (120u64 + 60 + 40) << 20);
        assert_eq!(total["uncompressedBytes"], (120u64 + 60 + 40 + 20 + 8 + 6) << 20);
        assert_eq!(total["parameterBits"], 12u32);
        assert_eq!(total["complete"], true);
    }

    #[test]
    fn the_prediction_reuses_the_parameter_estimate_rather_than_counting_again() {
        let run = p02();
        let selection = selection(&[("outfit", "Tshirt"), ("costume", "Costume")]);
        let plan = plan_prediction(&run, &selection);
        let budget = plan_parameter_budget(&run, &selection);
        assert_eq!(plan["total"]["parameterBits"], budget["estimateBits"]);
        let row_sum: u32 = plan["rows"].as_array().unwrap().iter().map(|row| row["parameterBits"].as_u64().unwrap() as u32).sum();
        assert_eq!(row_sum, budget["estimateBits"].as_u64().unwrap() as u32);
    }

    #[test]
    fn a_candidate_without_a_footprint_reports_the_prediction_as_unavailable() {
        let mut run = p02();
        run.prefabs[1].footprint = None;
        let plan = plan_prediction(&run, &selection(&[("outfit", "Tshirt")]));
        assert_eq!(plan["available"], false);
        assert_eq!(plan["rows"].as_array().unwrap().len(), 0);
        assert!(plan.get("total").is_none());
    }

    #[test]
    fn partial_evidence_keeps_the_numbers_and_marks_the_row_incomplete() {
        let mut run = p02();
        let mut footprint = run.prefabs[1].footprint.unwrap();
        footprint.texture_complete = false;
        run.prefabs[1].footprint = Some(footprint);
        let plan = plan_prediction(&run, &selection(&[("outfit", "Tshirt")]));
        let row = &plan["rows"][1];
        assert_eq!(row["textureBytes"], 60u64 << 20);
        assert_eq!(row["complete"], false);
        assert_eq!(plan["total"]["complete"], false);
    }

    #[test]
    fn an_unknown_parameter_estimate_leaves_the_bits_null() {
        let mut run = p02();
        run.prefabs[0].parameter_budget = None;
        let plan = plan_prediction(&run, &selection(&[("outfit", "Tshirt")]));
        assert_eq!(plan["available"], true);
        assert!(plan["rows"][0]["parameterBits"].is_null());
        assert!(plan["total"]["parameterBits"].is_null());
        assert_eq!(plan["rows"][0]["complete"], false);
    }

    #[test]
    fn impossible_observations_are_contract_violations() {
        let mut run = p02();
        let mut impossible = run.prefabs[1].footprint.unwrap();
        impossible.skinned_meshes = impossible.meshes + 1;
        run.prefabs[1].footprint = Some(impossible);
        assert_eq!(validate_prefab_footprints(&run.prefabs).unwrap_err().code, "vua.amf_production.receipt_invalid");
        run.prefabs[1].footprint = Some(CandidateFootprint { triangles: MAX_TRIANGLES + 1, ..impossible });
        assert!(validate_prefab_footprints(&run.prefabs).is_err());
        run.prefabs[1].footprint = Some(footprint(1, 1, 1));
        assert!(validate_prefab_footprints(&run.prefabs).is_ok());
    }
}
