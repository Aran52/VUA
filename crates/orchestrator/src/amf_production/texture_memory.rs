//! Platform texture memory estimates and digest-bound importer changes in the managed project.
use super::*;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TexturePlatform { Pc, Android }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextureRank { Excellent, Good, Medium, Poor, VeryPoor, Unknown }

impl TextureRank {
    pub fn budget(self, platform: TexturePlatform) -> Option<u64> {
        let limits = match platform { TexturePlatform::Pc => [40, 75, 110, 150], TexturePlatform::Android => [10, 18, 25, 40] };
        let index = match self { Self::Excellent => 0, Self::Good => 1, Self::Medium => 2, Self::Poor => 3, _ => return None };
        Some(limits[index] * 1024 * 1024)
    }
}

pub fn texture_rank(bytes: u64, platform: TexturePlatform) -> TextureRank {
    [TextureRank::Excellent, TextureRank::Good, TextureRank::Medium, TextureRank::Poor]
        .into_iter().find(|rank| bytes <= rank.budget(platform).expect("bounded rank")).unwrap_or(TextureRank::VeryPoor)
}

/// Full mip chains, with each compressed mip rounded up to whole blocks. Crunch is decoded
/// to its underlying GPU format; file size and the optional CPU-readable copy are excluded.
pub fn estimate_texture_bytes(width: u32, height: u32, mipmap: bool, format: &str, faces: u32) -> Option<u64> {
    if width == 0 || height == 0 || width > 16384 || height > 16384 || ![1, 6].contains(&faces) { return None; }
    let (block, bytes) = match format {
        "DXT1" | "DXT1Crunched" | "BC4" | "ETC_RGB4" | "ETC_RGB4Crunched" | "ETC2_RGB" | "ETC2_RGBA1" | "EAC_R" | "EAC_R_SIGNED" => (4, 8),
        "DXT5" | "DXT5Crunched" | "BC5" | "BC6H" | "BC7" | "ETC2_RGBA8" | "ETC2_RGBA8Crunched" | "EAC_RG" | "EAC_RG_SIGNED" => (4, 16),
        "ASTC_4x4" | "ASTC_RGB_4x4" | "ASTC_RGBA_4x4" => (4, 16),
        "ASTC_5x5" => (5, 16), "ASTC_6x6" => (6, 16), "ASTC_8x8" => (8, 16), "ASTC_10x10" => (10, 16), "ASTC_12x12" => (12, 16),
        "Alpha8" | "R8" => (1, 1), "RGB16" | "RGBA16" | "ARGB16" | "R16" | "RHalf" | "RG16" => (1, 2),
        "RGB24" => (1, 3), "RGBA32" | "ARGB32" | "BGRA32" | "RFloat" | "RGHalf" => (1, 4),
        "RGBAHalf" | "RGFloat" => (1, 8), "RGBAFloat" => (1, 16), _ => return None,
    };
    let (mut w, mut h, mut total) = (width as u64, height as u64, 0);
    loop {
        total += w.div_ceil(block) * h.div_ceil(block) * bytes * faces as u64;
        if !mipmap || (w == 1 && h == 1) { return Some(total); }
        w = (w / 2).max(1); h = (h / 2).max(1);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextureInfo {
    pub id: String, pub path: String, pub width: u32, pub height: u32, pub format: String,
    pub mipmap: bool, pub faces: u32, pub estimated_bytes: u64, pub known: bool,
    pub max_size: u32, pub editable: bool, pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextureMemoryReport {
    pub platform: TexturePlatform, pub textures: Vec<TextureInfo>, pub total_bytes: u64,
    pub complete: bool, pub rank: TextureRank,
}

pub fn safe_texture_path(path: &str) -> bool {
    path.starts_with("Assets/") && path.len() <= 4096 && !path.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
        && !path.chars().any(char::is_control) && path.split('/').all(|s| !s.is_empty() && s != "." && s != ".." && !s.ends_with(['.', ' ']))
        && !path.to_ascii_lowercase().ends_with(".meta")
}

/// Both consumers reject malformed or internally inconsistent new facts. Absent facts are
/// handled by the caller so older runs remain readable and request a fresh inspection to plan.
pub fn validate_texture_memory(value: &serde_json::Value) -> Result<Vec<TextureMemoryReport>, AppErrorV1> {
    let invalid = || production_error("result_invalid", "Texture memory facts are malformed.");
    let reports: Vec<TextureMemoryReport> = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
    if reports.is_empty() { return Ok(reports); }
    if reports.len() != 2 || reports[0].platform != TexturePlatform::Pc || reports[1].platform != TexturePlatform::Android { return Err(invalid()); }
    for report in &reports {
        let mut ids = BTreeSet::new();
        let mut editable_paths = BTreeSet::new();
        if report.textures.len() > 2048 { return Err(invalid()); }
        for texture in &report.textures {
            let computed = estimate_texture_bytes(texture.width, texture.height, texture.mipmap, &texture.format, texture.faces);
            if texture.id.is_empty() || texture.id.len() > 256 || !ids.insert(&texture.id) || texture.path.len() > 4096
                || texture.reason.len() > 128 || texture.format.len() > 64 || ![1, 6].contains(&texture.faces)
                || texture.width > 16384 || texture.height > 16384 || texture.max_size > 16384
                || (texture.known && computed != Some(texture.estimated_bytes)) || (!texture.known && texture.estimated_bytes != 0)
                || (texture.editable && (!safe_texture_path(&texture.path) || !texture.known || texture.faces != 1
                    || !texture.width.is_power_of_two() || !texture.height.is_power_of_two() || !texture.max_size.is_power_of_two()
                    || texture.max_size < 32 || !editable_paths.insert(&texture.path))) { return Err(invalid()); }
        }
        let complete = report.textures.iter().all(|texture| texture.known);
        let total = report.textures.iter().map(|texture| texture.estimated_bytes).sum::<u64>();
        if complete != report.complete || total != report.total_bytes || report.rank != if complete { texture_rank(total, report.platform) } else { TextureRank::Unknown } { return Err(invalid()); }
    }
    Ok(reports)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextureImportChange {
    pub path: String, pub platform: TexturePlatform, pub max_size: u32, pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TexturePlanEntry {
    pub current: TextureInfo, pub proposed: TextureImportChange, pub estimated_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextureMemoryPlan {
    pub platform: TexturePlatform, pub target_rank: TextureRank, pub current: TextureMemoryReport,
    pub entries: Vec<TexturePlanEntry>, pub estimated_total_bytes: u64, pub estimated_rank: TextureRank,
    pub reaches_target: bool,
}

/// Version comparisons retain stable importer facts, excluding generated texture identities and
/// ordering that can change across Editor sessions without any file change.
pub(super) fn texture_revision_state(run: &ProductionRun) -> serde_json::Value {
    let Some(reports) = run.result["facts"]["textureMemory"].as_array() else { return serde_json::Value::Null; };
    let mut state = BTreeMap::new();
    for report in reports {
        for texture in report["textures"].as_array().into_iter().flatten() {
            let Some(path) = texture["path"].as_str().filter(|p| safe_texture_path(p)) else { continue };
            state.insert(format!("{}:{path}", report["platform"]), serde_json::json!({
                "width":texture["width"],"height":texture["height"],"format":texture["format"],
                "maxSize":texture["maxSize"],"mipmap":texture["mipmap"]}));
        }
    }
    if state.is_empty() { serde_json::Value::Null } else { serde_json::json!(state) }
}

/// Replay reproduces materials and assembly, not importer settings. It is refused only when a
/// texture both states carry has different settings now; textures that came or went with other
/// outfits, and versions recorded before texture facts existed, stay replayable.
pub(super) fn texture_replay_conflict(recorded: &serde_json::Value, current: &serde_json::Value) -> bool {
    let (Some(recorded), Some(current)) = (recorded.as_object(), current.as_object()) else { return false };
    recorded.iter().any(|(key, settings)| current.get(key).is_some_and(|now| now != settings))
}

pub fn compute_texture_plan(report: &TextureMemoryReport, target: TextureRank) -> Result<TextureMemoryPlan, AppErrorV1> {
    let budget = target.budget(report.platform).ok_or_else(|| production_error("params_invalid", "Choose Excellent, Good, Medium or Poor."))?;
    let mut entries: BTreeMap<String, TexturePlanEntry> = BTreeMap::new();
    let mut total = report.total_bytes;
    let preferred = match report.platform { TexturePlatform::Pc => "BC7", TexturePlatform::Android => "ASTC_4x4" };
    // Try compression before reducing resolution. Never increase an already smaller format.
    for texture in &report.textures {
        if total <= budget { break; }
        if !texture.editable { continue; }
        let estimate = estimate_texture_bytes(texture.width, texture.height, texture.mipmap, preferred, 1).expect("supported format");
        if estimate >= texture.estimated_bytes { continue; }
        total -= texture.estimated_bytes - estimate;
        entries.insert(texture.path.clone(), TexturePlanEntry { current: texture.clone(),
            proposed: TextureImportChange { path: texture.path.clone(), platform: report.platform, max_size: texture.max_size, format: preferred.into() }, estimated_bytes: estimate });
    }
    // One halving at a time, greatest saving first, with a conservative 512-pixel floor.
    while total > budget {
        let candidate = report.textures.iter().filter(|t| t.editable).filter_map(|texture| {
            let entry = entries.get(&texture.path);
            let max = entry.map_or(texture.max_size, |e| e.proposed.max_size).min(texture.width.max(texture.height));
            if max <= 512 { return None; }
            let new_max = max / 2;
            let scale = texture.width.max(texture.height) / new_max;
            let format = entry.map_or(texture.format.as_str(), |e| e.proposed.format.as_str());
            let bytes = estimate_texture_bytes((texture.width / scale).max(1), (texture.height / scale).max(1), texture.mipmap, format, 1)?;
            let old = entry.map_or(texture.estimated_bytes, |e| e.estimated_bytes);
            Some((old - bytes, texture, new_max, format.to_owned(), bytes))
        }).max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.path.cmp(&a.1.path)));
        let Some((saving, texture, max_size, format, bytes)) = candidate else { break; };
        if saving == 0 { break; }
        total -= saving;
        entries.insert(texture.path.clone(), TexturePlanEntry { current: texture.clone(),
            proposed: TextureImportChange { path: texture.path.clone(), platform: report.platform, max_size, format }, estimated_bytes: bytes });
    }
    Ok(TextureMemoryPlan { platform: report.platform, target_rank: target, current: report.clone(), entries: entries.into_values().collect(),
        estimated_total_bytes: total, estimated_rank: if report.complete { texture_rank(total, report.platform) } else { TextureRank::Unknown },
        reaches_target: report.complete && total <= budget })
}

impl AmfProductionEngine {
    fn texture_plan(&self, run: &ProductionRun, platform: TexturePlatform, target: TextureRank) -> Result<(TextureMemoryPlan, String), AppErrorV1> {
        if run.phase != "succeeded" { return Err(production_error("result_not_ready", "Assemble the Avatar before tuning textures.")); }
        let value = run.result["facts"].get("textureMemory").ok_or_else(|| production_error("inspect_required", "Reinspect the work project to measure texture memory."))?;
        let reports = validate_texture_memory(value)?;
        let (same, fingerprint) = self.project_state(run)?;
        if !same { return Err(production_error("project_drift", "Reinspect changed work before planning textures.")); }
        let report = reports.iter().find(|r| r.platform == platform).ok_or_else(|| production_error("inspect_required", "Inspect an assembled Avatar to measure texture memory."))?;
        let plan = compute_texture_plan(report, target)?;
        let value = serde_json::json!({"operation":"texture_memory","runId":run.run_id,"revision":run.revision,
            "projectRoot":run.project_root,"projectFingerprint":fingerprint,"plan":plan});
        let digest = Sha256::digest(serde_json::to_vec(&value).map_err(io_error)?).iter().map(|b| format!("{b:02x}")).collect();
        Ok((plan, digest))
    }

    pub fn plan_texture_memory(&self, run_id: &str, platform: TexturePlatform, target: TextureRank) -> Result<serde_json::Value, AppErrorV1> {
        let run = self.get(run_id)?;
        let (plan, digest) = self.texture_plan(&run, platform, target)?;
        Ok(serde_json::json!({"schemaVersion":"0.1","runId":run_id,"planDigest":digest,"plan":plan}))
    }

    /// The player's explicit path list is the reviewed confirmation. Ask mode sends one path;
    /// list/full modes send the reviewed subset/all. The engine never applies unlisted entries.
    pub fn apply_texture_memory(&self, run_id: &str, platform: TexturePlatform, target: TextureRank, digest: &str, paths: &[String], ctx: &TaskContext) -> Result<(), AppErrorV1> {
        let mut run = self.get(run_id)?;
        let (plan, actual) = self.texture_plan(&run, platform, target)?;
        if actual != digest { return Err(production_error("plan_drift", "Review the changed texture plan again.")); }
        let mut named = BTreeSet::new();
        let mut chosen = Vec::new();
        for path in paths {
            if !named.insert(path) { return Err(production_error("textures_invalid", "Name each reviewed texture once.")); }
            chosen.push(plan.entries.iter().find(|e| e.proposed.path == *path).ok_or_else(|| production_error("textures_invalid", "Choose reviewed texture entries."))?.clone());
        }
        if chosen.is_empty() { return Err(production_error("textures_invalid", "Choose at least one reviewed texture.")); }
        self.ensure_project_matches(&mut run)?;
        let observed_before = (run.prefabs.clone(), run.result.clone(), run.unity_fingerprint.clone(), run.project_fingerprint.clone(), run.revision);
        run.task_id = ctx.task_id().into(); run.phase = "assembling".into(); run.error = None; run.recovery = None;
        let (mut snapshot, mut revision_id) = (None, None);
        let result = (|| {
            self.step(&mut run, ctx, "snapshot", "Protect the work project")?;
            snapshot = Some(self.platform.snapshot(&run)?); run.snapshot_id = snapshot.clone();
            run.snapshot_selection = run.selection.clone(); run.snapshot_sources = None;
            revision_id = Some(self.record_revision(&mut run, RevisionKind::TextureMemory, false)); self.save(&run)?;
            self.step(&mut run, ctx, "texture_memory", "Apply reviewed texture import settings")?;
            let changes = chosen.iter().map(|e| e.proposed.clone()).collect::<Vec<_>>();
            let observed = self.platform.texture_import(&run, &changes, &self.watch(&run, ctx))?;
            if let Some(error) = ctx.cancellation_error("amf-production") { return Err(error); }
            if observed.editor_version != run.editor.version { return Err(production_error("editor_drift", "The actual Editor version changed.")); }
            run.prefabs = observed.prefabs; run.result = observed.result; run.unity_fingerprint = observed.unity_fingerprint;
            self.step(&mut run, ctx, "inspect", "Verify texture memory in Unity")?;
            self.update_observations(&mut run, ctx)?;
            if let Some(error) = ctx.cancellation_error("amf-production") { return Err(error); }
            let reports = validate_texture_memory(&run.result["facts"]["textureMemory"])?;
            let after = reports.iter().find(|r| r.platform == platform).ok_or_else(|| production_error("texture_verification_failed", "The texture recheck has no active Avatar."))?;
            for entry in &chosen {
                if !after.textures.iter().any(|t| t.path == entry.proposed.path && t.known && t.format == entry.proposed.format
                    && t.max_size == entry.proposed.max_size && t.estimated_bytes == entry.estimated_bytes) {
                    return Err(production_error("texture_verification_failed", "Unity did not retain the reviewed texture settings."));
                }
            }
            if chosen.len() == plan.entries.len() && plan.reaches_target
                && (!after.complete || after.total_bytes > target.budget(platform).expect("validated target")) {
                return Err(production_error("texture_verification_failed", "The complete reviewed texture plan did not reach its predicted target."));
            }
            run.phase = "succeeded".into(); run.confirmed_digest = None; Ok(())
        })();
        if result.is_err() {
            if let Some(snapshot) = snapshot { run.recovery = Some(match self.platform.restore(&run, &snapshot) {
                Ok(()) => { (run.prefabs, run.result, run.unity_fingerprint, run.project_fingerprint, run.revision) = observed_before; "restored".into() },
                Err(error) => format!("restore_failed: {}", error.code),
            }); }
            if !matches!(run.recovery.as_deref(), Some(r) if r.starts_with("restore_failed")) {
                if let Some(id) = revision_id { run.revisions.retain(|r| r.id != id); }
            }
        }
        let outcome = self.finish(&mut run, result); self.prune_snapshots(&run); outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reports() -> Vec<TextureMemoryReport> {
        let value: serde_json::Value = serde_json::from_slice(include_bytes!("../../../../schemas/unity-bridge/v5/examples/texture-memory.valid.result.json")).unwrap();
        validate_texture_memory(&value["data"]["textureMemory"]).unwrap()
    }
    #[test]
    fn estimates_decode_crunch_round_every_mip_and_count_cube_faces() {
        assert_eq!(estimate_texture_bytes(4096, 4096, true, "RGBA32", 1), Some(89_478_484));
        assert_eq!(estimate_texture_bytes(4, 4, true, "DXT1", 1), Some(24));
        assert_eq!(estimate_texture_bytes(4, 4, true, "DXT1Crunched", 1), Some(24));
        assert_eq!(estimate_texture_bytes(7, 5, false, "ASTC_6x6", 1), Some(32));
        assert_eq!(estimate_texture_bytes(4, 4, false, "BC7", 6), Some(96));
        assert_eq!(estimate_texture_bytes(4, 4, false, "Unknown", 1), None);
        assert_eq!(estimate_texture_bytes(0, 4, false, "RGBA32", 1), None);
    }
    #[test]
    fn platform_thresholds_are_inclusive_and_the_next_byte_drops_a_rank() {
        for platform in [TexturePlatform::Pc, TexturePlatform::Android] {
            for rank in [TextureRank::Excellent, TextureRank::Good, TextureRank::Medium, TextureRank::Poor] {
                let budget = rank.budget(platform).unwrap();
                assert_eq!(texture_rank(budget, platform), rank);
                assert_ne!(texture_rank(budget + 1, platform), rank);
            }
        }
    }
    #[test]
    fn plans_try_compression_then_halve_until_the_target_without_removing_mips() {
        let mut report = reports().remove(0);
        let plan = compute_texture_plan(&report, TextureRank::Good).unwrap();
        assert_eq!(plan.entries[0].proposed.format, "BC7");
        assert_eq!(plan.entries[0].proposed.max_size, 4096);
        assert_eq!(plan.estimated_rank, TextureRank::Excellent);
        assert!(plan.reaches_target);
        report.textures[0].format = "DXT1".into();
        report.textures[0].width = 16384; report.textures[0].height = 16384; report.textures[0].max_size = 16384;
        report.textures[0].estimated_bytes = estimate_texture_bytes(16384, 16384, true, "DXT1", 1).unwrap();
        report.total_bytes = report.textures[0].estimated_bytes;
        let plan = compute_texture_plan(&report, TextureRank::Excellent).unwrap();
        assert_eq!(plan.entries[0].proposed.format, "DXT1", "do not increase a smaller compressed format");
        assert_eq!(plan.entries[0].proposed.max_size, 4096);
        assert!(plan.reaches_target);
    }
    #[test]
    fn unknown_and_ineligible_textures_never_claim_a_reachable_rank() {
        let mut report = reports().remove(1);
        report.textures[0].editable = false;
        assert!(!compute_texture_plan(&report, TextureRank::Good).unwrap().reaches_target);
        report.textures[0].known = false; report.textures[0].estimated_bytes = 0;
        report.total_bytes = 0; report.complete = false; report.rank = TextureRank::Unknown;
        let plan = compute_texture_plan(&report, TextureRank::Good).unwrap();
        assert_eq!(plan.estimated_rank, TextureRank::Unknown);
        assert!(!plan.reaches_target);
    }
    #[test]
    fn replay_is_refused_only_where_a_shared_texture_changed_its_settings() {
        let state = |entries: &[(&str, &str)]| serde_json::json!(entries.iter()
            .map(|(key, format)| ((*key).to_owned(), serde_json::json!({"format": format, "maxSize": 2048})))
            .collect::<serde_json::Map<_, _>>());
        let tuned = state(&[("\"pc\":Assets/Coat.png", "BC7"), ("\"pc\":Assets/Body.png", "DXT5")]);
        assert!(texture_replay_conflict(&state(&[("\"pc\":Assets/Coat.png", "RGBA32")]), &tuned), "a tuned texture cannot replay back");
        assert!(!texture_replay_conflict(&state(&[("\"pc\":Assets/Hat.png", "RGBA32")]), &tuned), "another outfit's textures do not block");
        assert!(!texture_replay_conflict(&state(&[("\"pc\":Assets/Body.png", "DXT5")]), &tuned));
        assert!(!texture_replay_conflict(&serde_json::Value::Null, &tuned), "versions before texture facts stay replayable");
        assert!(!texture_replay_conflict(&tuned, &serde_json::Value::Null));
    }
    #[test]
    fn consumers_reject_false_totals_bad_paths_and_unknown_bytes() {
        for field in ["totalBytes", "rank", "estimatedBytes", "path"] {
            let mut value = serde_json::json!(reports());
            match field {
                "totalBytes" => value[0][field] = serde_json::json!(0),
                "rank" => value[0][field] = serde_json::json!("excellent"),
                "estimatedBytes" => value[0]["textures"][0][field] = serde_json::json!(0),
                _ => value[0]["textures"][0][field] = serde_json::json!("Assets/../Library/source.png"),
            }
            assert!(validate_texture_memory(&value).is_err(), "{field}");
        }
    }
}
