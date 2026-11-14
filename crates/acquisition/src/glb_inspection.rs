//! Candidate `glb-inspection v0.1`: AMF-owned pure-Rust streaming inspection of warehouse
//! `.glb` and `.vrm` binary glTF 2.0 artifacts, `VRMC_vrm` 1.0 humanoid readiness, legacy
//! VRM 0.x upgrade detection, and PH-R Stage 1 MR Object-Head `>= 5 cm` clearance.
//!
//! Safety & streaming invariants:
//! - Opens only warehouse-contained regular files verified `"present"` by [`PresenceCheck`].
//! - Streams only the 12-byte GLB header + 8-byte Chunk 0 header + JSON chunk (`<= 16 MiB`);
//!   never reads Chunk 1 (`BIN` geometry/texture buffer) into memory.
//! - Redacts host filesystem paths, emitting only normalized warehouse-relative file names.

use crate::library_download::{LibraryDownloadError, LibraryDownloadService};
use crate::library_view::{product_copy_ids, PresenceCheck};
use crate::terms::valid_product_id;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use vua_bdl_store::BdlStore;

pub const GLB_INSPECTION_SCHEMA: &str = "vua.glb-inspection/v0.1";
pub const REFERENCE_HEAD_HMD_METERS: [f64; 3] = [0.22, 0.26, 0.28];

const GLB_MAGIC: u32 = 0x4654_6C67; // "glTF"
const GLB_VERSION_2: u32 = 2;
const GLB_CHUNK_JSON: u32 = 0x4E4F_534A; // "JSON"
const MAX_JSON_CHUNK_BYTES: usize = 16 * 1024 * 1024;
const MAX_INSPECTED_ITEMS: usize = 32;
const MAX_SCENE_DEPTH: usize = 128;
const CLEARANCE_MIN_MARGIN_METERS: f64 = 0.05;
const OBJECT_HEAD_MAX_DIM_METERS: f64 = 2.0;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlbVrmSummaryV01 {
    pub spec_version: Option<String>,
    pub has_humanoid_head_bone: bool,
    pub bone_count: usize,
    pub model_name: Option<String>,
    pub authors: Vec<String>,
    pub avatar_permission: Option<String>,
    pub commercial_usage: Option<String>,
    pub modification: Option<String>,
    pub license_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlbBoundsMetersV01 {
    pub size: [f64; 3],
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectHeadClearanceV01 {
    pub status: &'static str,
    pub reference_head_hmd_meters: [f64; 3],
    pub margins_meters: Option<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlbInspectedFileV01 {
    pub copy_id: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub format: &'static str,
    pub extensions_used: Vec<String>,
    pub vrm: Option<GlbVrmSummaryV01>,
    pub bounds_meters: Option<GlbBoundsMetersV01>,
    pub object_head_clearance: ObjectHeadClearanceV01,
    pub phr_readiness: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlbInspectionResultV01 {
    pub schema_version: &'static str,
    pub items: Vec<GlbInspectedFileV01>,
    pub truncated: bool,
}

enum TargetScope {
    Product(String),
    Entry(String),
}

fn parse_params(params: &Value) -> Result<TargetScope, LibraryDownloadError> {
    let Some(obj) = params.as_object() else {
        return Err(LibraryDownloadError("invalid_params"));
    };
    if obj.len() != 2 || obj.get("schemaVersion").and_then(Value::as_str) != Some("0.1") {
        return Err(LibraryDownloadError("invalid_params"));
    }
    if let Some(pid) = obj.get("productId") {
        let Some(s) = pid.as_str().filter(|s| valid_product_id(s)) else {
            return Err(LibraryDownloadError("invalid_params"));
        };
        return Ok(TargetScope::Product(s.to_owned()));
    }
    if let Some(eid) = obj.get("entryId") {
        let Some(s) = eid
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 128 && !s.contains(['\\', '/']))
        else {
            return Err(LibraryDownloadError("invalid_params"));
        };
        return Ok(TargetScope::Entry(s.to_owned()));
    }
    Err(LibraryDownloadError("invalid_params"))
}

pub fn inspect_glb(
    bdl: &BdlStore,
    root: &Path,
    downloads: &LibraryDownloadService,
    params: Value,
) -> Result<Value, LibraryDownloadError> {
    let scope = parse_params(&params)?;
    let removed = bdl.removed_local_entries()?;
    let all_copies = bdl
        .library_copy_evidence()?
        .into_iter()
        .filter(|copy| !removed.contains(&copy.copy.warehouse_item_id))
        .collect::<Vec<_>>();

    let scoped = match scope {
        TargetScope::Product(product_id) => {
            let ids = product_copy_ids(bdl, root, downloads, &product_id)?;
            all_copies
                .into_iter()
                .filter(|c| ids.contains(&c.copy.copy_id))
                .collect::<Vec<_>>()
        }
        TargetScope::Entry(entry_id) => all_copies
            .into_iter()
            .filter(|c| c.copy.warehouse_item_id == entry_id)
            .collect::<Vec<_>>(),
    };

    let mut presence_check = PresenceCheck::new(root);
    let mut candidates = scoped
        .into_iter()
        .filter(|c| is_glb_or_vrm(&c.copy.relative_path) && presence_check.presence(c) == "present")
        .map(|c| {
            let safe_name = sanitize_relative_file_name(&c.copy.relative_path);
            (safe_name, c)
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.copy.copy_id.cmp(&b.1.copy.copy_id))
    });

    let truncated = candidates.len() > MAX_INSPECTED_ITEMS;
    candidates.truncate(MAX_INSPECTED_ITEMS);

    let mut items = Vec::with_capacity(candidates.len());
    for (file_name, copy) in candidates {
        let parsed = match File::open(Path::new(&copy.copy.stored_path)) {
            Ok(mut file) => inspect_glb_reader(&mut file),
            Err(_) => ParsedGlb::invalid(),
        };
        items.push(GlbInspectedFileV01 {
            copy_id: copy.copy.copy_id,
            file_name,
            size_bytes: copy.size_bytes,
            format: parsed.format,
            extensions_used: parsed.extensions_used,
            vrm: parsed.vrm,
            bounds_meters: parsed.bounds_meters,
            object_head_clearance: parsed.object_head_clearance,
            phr_readiness: parsed.phr_readiness,
        });
    }

    let result = GlbInspectionResultV01 {
        schema_version: GLB_INSPECTION_SCHEMA,
        items,
        truncated,
    };
    Ok(serde_json::to_value(result).unwrap_or_else(|_| {
        json!({
            "schemaVersion": GLB_INSPECTION_SCHEMA,
            "items": [],
            "truncated": false
        })
    }))
}

fn is_glb_or_vrm(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".glb") || lower.ends_with(".vrm")
}

pub fn sanitize_relative_file_name(raw: &str) -> String {
    let normalized = raw.replace('\\', "/");
    let segments = normalized
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != ".." && !seg.contains(':'))
        .collect::<Vec<_>>();
    let joined = if segments.is_empty() {
        "artifact.glb".to_owned()
    } else {
        segments.join("/")
    };
    let mut out = String::new();
    let mut utf16 = 0usize;
    for ch in joined.chars() {
        let len = ch.len_utf16();
        if utf16 + len > 512 {
            break;
        }
        out.push(ch);
        utf16 += len;
    }
    if out.is_empty() {
        "artifact.glb".to_owned()
    } else {
        out
    }
}

struct ParsedGlb {
    format: &'static str,
    extensions_used: Vec<String>,
    vrm: Option<GlbVrmSummaryV01>,
    bounds_meters: Option<GlbBoundsMetersV01>,
    object_head_clearance: ObjectHeadClearanceV01,
    phr_readiness: &'static str,
}

impl ParsedGlb {
    fn invalid() -> Self {
        Self {
            format: "invalid_glb",
            extensions_used: Vec::new(),
            vrm: None,
            bounds_meters: None,
            object_head_clearance: ObjectHeadClearanceV01 {
                status: "unknown",
                reference_head_hmd_meters: REFERENCE_HEAD_HMD_METERS,
                margins_meters: None,
            },
            phr_readiness: "invalid_glb",
        }
    }
}

pub fn inspect_glb_bytes(bytes: &[u8]) -> GlbInspectedFileV01 {
    let mut cursor = std::io::Cursor::new(bytes);
    let parsed = inspect_glb_reader(&mut cursor);
    GlbInspectedFileV01 {
        copy_id: "mem-copy".to_owned(),
        file_name: "model.glb".to_owned(),
        size_bytes: bytes.len() as u64,
        format: parsed.format,
        extensions_used: parsed.extensions_used,
        vrm: parsed.vrm,
        bounds_meters: parsed.bounds_meters,
        object_head_clearance: parsed.object_head_clearance,
        phr_readiness: parsed.phr_readiness,
    }
}

fn inspect_glb_reader<R: Read>(reader: &mut R) -> ParsedGlb {
    let mut header = [0u8; 20];
    if reader.read_exact(&mut header).is_err() {
        return ParsedGlb::invalid();
    }
    let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let version = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    let total_length = u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as u64;
    let chunk0_len = u32::from_le_bytes([header[12], header[13], header[14], header[15]]) as usize;
    let chunk0_type = u32::from_le_bytes([header[16], header[17], header[18], header[19]]);

    if magic != GLB_MAGIC
        || version != GLB_VERSION_2
        || chunk0_type != GLB_CHUNK_JSON
        || chunk0_len == 0
        || chunk0_len > MAX_JSON_CHUNK_BYTES
        || (20 + chunk0_len as u64) > total_length
    {
        return ParsedGlb::invalid();
    }

    let mut json_bytes = vec![0u8; chunk0_len];
    if reader.read_exact(&mut json_bytes).is_err() {
        return ParsedGlb::invalid();
    }
    while json_bytes.last() == Some(&0) {
        json_bytes.pop();
    }

    let Ok(gltf) = serde_json::from_slice::<Value>(&json_bytes) else {
        return ParsedGlb::invalid();
    };
    let is_gltf_2 = gltf
        .get("asset")
        .and_then(|a| a.get("version"))
        .and_then(Value::as_str)
        .is_some_and(|v| v.starts_with("2."));
    if !is_gltf_2 {
        return ParsedGlb::invalid();
    }

    let extensions_used = extract_extensions_used(&gltf);
    let nodes_len = gltf
        .get("nodes")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    let (format, vrm) = if let Some(vrmc) = gltf
        .pointer("/extensions/VRMC_vrm")
        .and_then(Value::as_object)
    {
        ("vrm_1_0", Some(parse_vrm_1_0(vrmc, nodes_len)))
    } else if let Some(vrm0) = gltf.pointer("/extensions/VRM").and_then(Value::as_object) {
        ("vrm_0_x", Some(parse_vrm_0_x(vrm0, nodes_len)))
    } else {
        ("glb_2_0", None)
    };

    let bounds_meters = compute_scene_bounds_meters(&gltf);
    let object_head_clearance = evaluate_object_head_clearance(bounds_meters.as_ref());

    let phr_readiness = match format {
        "vrm_1_0" => {
            if vrm.as_ref().is_some_and(|v| v.has_humanoid_head_bone) {
                "ready_vrm1_humanoid"
            } else {
                "missing_head_bone"
            }
        }
        "vrm_0_x" => "vrm0_needs_upgrade",
        "glb_2_0" => match object_head_clearance.status {
            "passes" => "ready_object_head",
            "likely_unscaled" => "scale_suspect",
            _ => "clearance_insufficient",
        },
        _ => "invalid_glb",
    };

    ParsedGlb {
        format,
        extensions_used,
        vrm,
        bounds_meters,
        object_head_clearance,
        phr_readiness,
    }
}

fn trunc_utf16(raw: &str, max_utf16: usize) -> String {
    let mut out = String::new();
    let mut used = 0usize;
    for ch in raw.chars() {
        let len = ch.len_utf16();
        if used + len > max_utf16 {
            break;
        }
        out.push(ch);
        used += len;
    }
    out
}

fn opt_str(v: Option<&Value>, max_utf16: usize) -> Option<String> {
    v.and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| trunc_utf16(s, max_utf16))
}

fn extract_extensions_used(gltf: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = gltf.get("extensionsUsed").and_then(Value::as_array) {
        for entry in arr.iter().filter_map(Value::as_str) {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                continue;
            }
            let bounded = trunc_utf16(trimmed, 128);
            if !out.contains(&bounded) {
                out.push(bounded);
                if out.len() >= 64 {
                    break;
                }
            }
        }
    }
    out
}

fn parse_vrm_1_0(vrmc: &serde_json::Map<String, Value>, nodes_len: usize) -> GlbVrmSummaryV01 {
    let human_bones = vrmc
        .get("humanoid")
        .and_then(|h| h.get("humanBones"))
        .and_then(Value::as_object);
    let bone_count = human_bones.map_or(0, |m| m.len().min(512));
    let has_humanoid_head_bone = human_bones
        .and_then(|m| m.get("head"))
        .and_then(|h| h.get("node"))
        .and_then(Value::as_u64)
        .is_some_and(|idx| (idx as usize) < nodes_len);

    let meta = vrmc.get("meta").and_then(Value::as_object);
    let authors = meta
        .and_then(|m| m.get("authors"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .take(32)
                .map(|s| trunc_utf16(s, 256))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    GlbVrmSummaryV01 {
        spec_version: opt_str(vrmc.get("specVersion"), 32),
        has_humanoid_head_bone,
        bone_count,
        model_name: opt_str(meta.and_then(|m| m.get("name")), 256),
        authors,
        avatar_permission: opt_str(meta.and_then(|m| m.get("avatarPermission")), 64),
        commercial_usage: opt_str(meta.and_then(|m| m.get("commercialUsage")), 64),
        modification: opt_str(meta.and_then(|m| m.get("modification")), 64),
        license_url: opt_str(
            meta.and_then(|m| m.get("otherLicenseUrl").or_else(|| m.get("licenseUrl"))),
            2048,
        ),
    }
}

fn parse_vrm_0_x(vrm0: &serde_json::Map<String, Value>, nodes_len: usize) -> GlbVrmSummaryV01 {
    let bones_arr = vrm0
        .get("humanoid")
        .and_then(|h| h.get("humanBones"))
        .and_then(Value::as_array);
    let bone_count = bones_arr.map_or(0, |a| a.len().min(512));
    let has_humanoid_head_bone = bones_arr.is_some_and(|arr| {
        arr.iter().any(|entry| {
            entry
                .get("bone")
                .and_then(Value::as_str)
                .is_some_and(|b| b.eq_ignore_ascii_case("head"))
                && entry
                    .get("node")
                    .and_then(Value::as_u64)
                    .is_some_and(|idx| (idx as usize) < nodes_len)
        })
    });

    let meta = vrm0.get("meta").and_then(Value::as_object);
    let authors = opt_str(meta.and_then(|m| m.get("author")), 256)
        .into_iter()
        .collect::<Vec<_>>();

    GlbVrmSummaryV01 {
        spec_version: opt_str(
            vrm0.get("specVersion")
                .or_else(|| vrm0.get("exporterVersion")),
            32,
        ),
        has_humanoid_head_bone,
        bone_count,
        model_name: opt_str(meta.and_then(|m| m.get("title")), 256),
        authors,
        avatar_permission: opt_str(meta.and_then(|m| m.get("allowedUserName")), 64),
        commercial_usage: opt_str(
            meta.and_then(|m| {
                m.get("commercialUssageName")
                    .or_else(|| m.get("commercialUsageName"))
            }),
            64,
        ),
        modification: opt_str(meta.and_then(|m| m.get("licenseName")), 64),
        license_url: opt_str(
            meta.and_then(|m| {
                m.get("otherLicenseUrl")
                    .or_else(|| m.get("otherPermissionUrl"))
            }),
            2048,
        ),
    }
}

fn round6(v: f64) -> f64 {
    let r = (v * 1_000_000.0).round() / 1_000_000.0;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

fn evaluate_object_head_clearance(bounds: Option<&GlbBoundsMetersV01>) -> ObjectHeadClearanceV01 {
    let Some(bounds) = bounds else {
        return ObjectHeadClearanceV01 {
            status: "unknown",
            reference_head_hmd_meters: REFERENCE_HEAD_HMD_METERS,
            margins_meters: None,
        };
    };
    let margins = [
        round6((bounds.size[0] - REFERENCE_HEAD_HMD_METERS[0]) / 2.0),
        round6((bounds.size[1] - REFERENCE_HEAD_HMD_METERS[1]) / 2.0),
        round6((bounds.size[2] - REFERENCE_HEAD_HMD_METERS[2]) / 2.0),
    ];
    let max_dim = bounds.size[0].max(bounds.size[1]).max(bounds.size[2]);
    let status = if max_dim > OBJECT_HEAD_MAX_DIM_METERS {
        "likely_unscaled"
    } else if margins
        .iter()
        .all(|m| *m + 1e-9 >= CLEARANCE_MIN_MARGIN_METERS)
    {
        "passes"
    } else {
        "insufficient_margin"
    };
    ObjectHeadClearanceV01 {
        status,
        reference_head_hmd_meters: REFERENCE_HEAD_HMD_METERS,
        margins_meters: Some(margins),
    }
}

type Mat4 = [f64; 16]; // column-major 4x4

const IDENTITY_MAT4: Mat4 = [
    1.0, 0.0, 0.0, 0.0, // col 0
    0.0, 1.0, 0.0, 0.0, // col 1
    0.0, 0.0, 1.0, 0.0, // col 2
    0.0, 0.0, 0.0, 1.0, // col 3
];

fn mul_mat4(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [0.0f64; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[k * 4 + row] * b[col * 4 + k];
            }
            out[col * 4 + row] = sum;
        }
    }
    out
}

fn transform_point(m: &Mat4, p: [f64; 3]) -> Option<[f64; 3]> {
    let x = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
    let y = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
    let z = m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14];
    if x.is_finite() && y.is_finite() && z.is_finite() {
        Some([x, y, z])
    } else {
        None
    }
}

fn parse_f64_slice<const N: usize>(val: Option<&Value>) -> Option<[f64; N]> {
    let arr = val?.as_array()?;
    if arr.len() != N {
        return None;
    }
    let mut out = [0.0f64; N];
    for (i, item) in arr.iter().enumerate() {
        let num = item.as_f64()?;
        if !num.is_finite() {
            return None;
        }
        out[i] = num;
    }
    Some(out)
}

fn node_local_matrix(node: &Value) -> Mat4 {
    if let Some(mat) = parse_f64_slice::<16>(node.get("matrix")) {
        return mat;
    }
    let t = parse_f64_slice::<3>(node.get("translation")).unwrap_or([0.0, 0.0, 0.0]);
    let r = parse_f64_slice::<4>(node.get("rotation")).unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let s = parse_f64_slice::<3>(node.get("scale")).unwrap_or([1.0, 1.0, 1.0]);

    let norm_sq = r[0] * r[0] + r[1] * r[1] + r[2] * r[2] + r[3] * r[3];
    let (qx, qy, qz, qw) = if norm_sq > 1e-12 {
        let inv = 1.0 / norm_sq.sqrt();
        (r[0] * inv, r[1] * inv, r[2] * inv, r[3] * inv)
    } else {
        (0.0, 0.0, 0.0, 1.0)
    };

    let xx = qx * qx;
    let yy = qy * qy;
    let zz = qz * qz;
    let xy = qx * qy;
    let xz = qx * qz;
    let yz = qy * qz;
    let wx = qw * qx;
    let wy = qw * qy;
    let wz = qw * qz;

    let r00 = (1.0 - 2.0 * (yy + zz)) * s[0];
    let r10 = (2.0 * (xy + wz)) * s[0];
    let r20 = (2.0 * (xz - wy)) * s[0];

    let r01 = (2.0 * (xy - wz)) * s[1];
    let r11 = (1.0 - 2.0 * (xx + zz)) * s[1];
    let r21 = (2.0 * (yz + wx)) * s[1];

    let r02 = (2.0 * (xz + wy)) * s[2];
    let r12 = (2.0 * (yz - wx)) * s[2];
    let r22 = (1.0 - 2.0 * (xx + yy)) * s[2];

    [
        r00, r10, r20, 0.0, // col 0
        r01, r11, r21, 0.0, // col 1
        r02, r12, r22, 0.0, // col 2
        t[0], t[1], t[2], 1.0, // col 3
    ]
}

fn compute_scene_bounds_meters(gltf: &Value) -> Option<GlbBoundsMetersV01> {
    let nodes = gltf.get("nodes").and_then(Value::as_array)?;
    if nodes.is_empty() {
        return None;
    }
    let meshes = gltf.get("meshes").and_then(Value::as_array)?;
    let accessors = gltf.get("accessors").and_then(Value::as_array)?;

    let root_indices: Vec<usize> = if let Some(scenes) = gltf
        .get("scenes")
        .and_then(Value::as_array)
        .filter(|s| !s.is_empty())
    {
        let active = gltf
            .get("scene")
            .and_then(Value::as_u64)
            .map(|i| i as usize)
            .filter(|i| *i < scenes.len())
            .unwrap_or(0);
        scenes[active]
            .get("nodes")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_u64)
                    .map(|i| i as usize)
                    .filter(|i| *i < nodes.len())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        let mut referenced = HashSet::new();
        for node in nodes {
            if let Some(children) = node.get("children").and_then(Value::as_array) {
                for child in children.iter().filter_map(Value::as_u64) {
                    referenced.insert(child as usize);
                }
            }
        }
        (0..nodes.len())
            .filter(|i| !referenced.contains(i))
            .collect()
    };

    let mut global_min = [f64::INFINITY; 3];
    let mut global_max = [f64::NEG_INFINITY; 3];
    let mut found_any = false;
    let mut active_path = HashSet::new();
    let mut visits = 0usize;
    const MAX_SCENE_VISITS: usize = 16_384;

    #[allow(clippy::too_many_arguments)]
    fn visit_node(
        node_idx: usize,
        parent_world: &Mat4,
        depth: usize,
        nodes: &[Value],
        meshes: &[Value],
        accessors: &[Value],
        active_path: &mut HashSet<usize>,
        visits: &mut usize,
        global_min: &mut [f64; 3],
        global_max: &mut [f64; 3],
        found_any: &mut bool,
    ) {
        if *visits >= MAX_SCENE_VISITS
            || depth > MAX_SCENE_DEPTH
            || node_idx >= nodes.len()
            || !active_path.insert(node_idx)
        {
            return;
        }
        *visits += 1;
        let node = &nodes[node_idx];
        let local = node_local_matrix(node);
        let world = mul_mat4(parent_world, &local);

        if let Some(mesh_idx) = node
            .get("mesh")
            .and_then(Value::as_u64)
            .map(|i| i as usize)
            .filter(|i| *i < meshes.len())
        {
            if let Some(primitives) = meshes[mesh_idx].get("primitives").and_then(Value::as_array) {
                for prim in primitives {
                    let Some(pos_idx) = prim
                        .get("attributes")
                        .and_then(|a| a.get("POSITION"))
                        .and_then(Value::as_u64)
                        .map(|i| i as usize)
                        .filter(|i| *i < accessors.len())
                    else {
                        continue;
                    };
                    let acc = &accessors[pos_idx];
                    if acc.get("type").and_then(Value::as_str) != Some("VEC3") {
                        continue;
                    }
                    let (Some(mn), Some(mx)) = (
                        parse_f64_slice::<3>(acc.get("min")),
                        parse_f64_slice::<3>(acc.get("max")),
                    ) else {
                        continue;
                    };
                    if mn[0] > mx[0] || mn[1] > mx[1] || mn[2] > mx[2] {
                        continue;
                    }
                    for &cx in &[mn[0], mx[0]] {
                        for &cy in &[mn[1], mx[1]] {
                            for &cz in &[mn[2], mx[2]] {
                                if let Some(tp) = transform_point(&world, [cx, cy, cz]) {
                                    for axis in 0..3 {
                                        global_min[axis] = global_min[axis].min(tp[axis]);
                                        global_max[axis] = global_max[axis].max(tp[axis]);
                                    }
                                    *found_any = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(children) = node.get("children").and_then(Value::as_array) {
            for child in children.iter().filter_map(Value::as_u64) {
                visit_node(
                    child as usize,
                    &world,
                    depth + 1,
                    nodes,
                    meshes,
                    accessors,
                    active_path,
                    visits,
                    global_min,
                    global_max,
                    found_any,
                );
            }
        }
        active_path.remove(&node_idx);
    }

    for root in root_indices {
        visit_node(
            root,
            &IDENTITY_MAT4,
            0,
            nodes,
            meshes,
            accessors,
            &mut active_path,
            &mut visits,
            &mut global_min,
            &mut global_max,
            &mut found_any,
        );
    }

    if !found_any {
        return None;
    }

    let min = [
        round6(global_min[0]),
        round6(global_min[1]),
        round6(global_min[2]),
    ];
    let max = [
        round6(global_max[0]),
        round6(global_max[1]),
        round6(global_max[2]),
    ];
    let size = [
        round6((global_max[0] - global_min[0]).max(0.0)),
        round6((global_max[1] - global_min[1]).max(0.0)),
        round6((global_max[2] - global_min[2]).max(0.0)),
    ];
    Some(GlbBoundsMetersV01 { size, min, max })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_synthetic_glb(json_doc: &Value, bin_tail_len: usize) -> Vec<u8> {
        let mut json_bytes = serde_json::to_vec(json_doc).unwrap();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }
        let bin_padded = (bin_tail_len + 3) & !3;
        let total_len = 12 + 8 + json_bytes.len() + if bin_padded > 0 { 8 + bin_padded } else { 0 };

        let mut out = Vec::with_capacity(total_len);
        out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        out.extend_from_slice(&GLB_VERSION_2.to_le_bytes());
        out.extend_from_slice(&(total_len as u32).to_le_bytes());
        out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&GLB_CHUNK_JSON.to_le_bytes());
        out.extend_from_slice(&json_bytes);
        if bin_padded > 0 {
            out.extend_from_slice(&(bin_padded as u32).to_le_bytes());
            out.extend_from_slice(&0x004E_4942u32.to_le_bytes()); // "BIN\0"
            out.resize(total_len, 0xAA);
        }
        out
    }

    #[test]
    fn inspects_vrm_1_0_humanoid_head_and_metadata_without_reading_bin() {
        let doc = json!({
            "asset": { "version": "2.0" },
            "extensionsUsed": ["VRMC_vrm", "VRMC_springBone"],
            "scene": 0,
            "scenes": [{ "nodes": [0] }],
            "nodes": [
                { "mesh": 0, "children": [1] },
                { "name": "HeadBone" }
            ],
            "meshes": [{
                "primitives": [{ "attributes": { "POSITION": 0 } }]
            }],
            "accessors": [{
                "type": "VEC3",
                "componentType": 5126,
                "count": 24,
                "min": [-0.23, 0.0, -0.15],
                "max": [0.23, 1.48, 0.16]
            }],
            "extensions": {
                "VRMC_vrm": {
                    "specVersion": "1.0",
                    "meta": {
                        "name": "Kipfel VRM1",
                        "authors": ["Studio Example"],
                        "avatarPermission": "onlyAuthor",
                        "commercialUsage": "personalNonProfit",
                        "modification": "allowModification",
                        "otherLicenseUrl": "https://vrm.dev/licenses/1.0/"
                    },
                    "humanoid": {
                        "humanBones": {
                            "hips": { "node": 0 },
                            "head": { "node": 1 }
                        }
                    }
                }
            }
        });
        let glb = pack_synthetic_glb(&doc, 4096);
        let res = inspect_glb_bytes(&glb);
        assert_eq!(res.format, "vrm_1_0");
        assert_eq!(res.phr_readiness, "ready_vrm1_humanoid");
        let vrm = res.vrm.expect("vrm summary");
        assert!(vrm.has_humanoid_head_bone);
        assert_eq!(vrm.bone_count, 2);
        assert_eq!(vrm.model_name.as_deref(), Some("Kipfel VRM1"));
        assert_eq!(vrm.authors, vec!["Studio Example"]);
        let bounds = res.bounds_meters.expect("bounds");
        assert_eq!(bounds.size, [0.46, 1.48, 0.31]);
    }

    #[test]
    fn detects_vrm1_missing_head_and_vrm0_upgrade_candidates() {
        let vrm1_no_head = pack_synthetic_glb(
            &json!({
                "asset": { "version": "2.0" },
                "nodes": [{ "name": "Hips" }],
                "extensions": {
                    "VRMC_vrm": {
                        "specVersion": "1.0",
                        "humanoid": { "humanBones": { "hips": { "node": 0 } } }
                    }
                }
            }),
            0,
        );
        let r1 = inspect_glb_bytes(&vrm1_no_head);
        assert_eq!(r1.format, "vrm_1_0");
        assert_eq!(r1.phr_readiness, "missing_head_bone");

        let vrm0 = pack_synthetic_glb(
            &json!({
                "asset": { "version": "2.0" },
                "nodes": [{ "name": "Head" }],
                "extensions": {
                    "VRM": {
                        "exporterVersion": "UniVRM-0.115.0",
                        "meta": { "title": "LegacyAvatar", "author": "CreatorA" },
                        "humanoid": { "humanBones": [{ "bone": "head", "node": 0 }] }
                    }
                }
            }),
            0,
        );
        let r0 = inspect_glb_bytes(&vrm0);
        assert_eq!(r0.format, "vrm_0_x");
        assert_eq!(r0.phr_readiness, "vrm0_needs_upgrade");
        assert!(r0.vrm.unwrap().has_humanoid_head_bone);
    }

    #[test]
    fn evaluates_object_head_5cm_clearance_trs_scale_and_unscaled_exports() {
        // 1) 40 cm cube modeled in centimeters [-20..20] with 0.01 root node TRS scale -> [0.40, 0.42, 0.44] m -> passes!
        let scaled_pass = pack_synthetic_glb(
            &json!({
                "asset": { "version": "2.0" },
                "scene": 0,
                "scenes": [{ "nodes": [0] }],
                "nodes": [{ "mesh": 0, "scale": [0.01, 0.01, 0.01] }],
                "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
                "accessors": [{
                    "type": "VEC3",
                    "min": [-20.0, -21.0, -22.0],
                    "max": [20.0, 21.0, 22.0]
                }]
            }),
            0,
        );
        let pass_res = inspect_glb_bytes(&scaled_pass);
        assert_eq!(pass_res.format, "glb_2_0");
        assert_eq!(
            pass_res.bounds_meters.as_ref().unwrap().size,
            [0.4, 0.42, 0.44]
        );
        assert_eq!(pass_res.object_head_status(), "passes");
        assert_eq!(pass_res.phr_readiness, "ready_object_head");

        // 2) Undersized [0.24, 0.26, 0.28] m -> margins [0.01, 0.0, 0.0] < 0.05 -> insufficient_margin!
        let tight = pack_synthetic_glb(
            &json!({
                "asset": { "version": "2.0" },
                "scene": 0,
                "scenes": [{ "nodes": [0] }],
                "nodes": [{ "mesh": 0 }],
                "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
                "accessors": [{
                    "type": "VEC3",
                    "min": [-0.12, -0.13, -0.14],
                    "max": [0.12, 0.13, 0.14]
                }]
            }),
            0,
        );
        let tight_res = inspect_glb_bytes(&tight);
        assert_eq!(
            tight_res.object_head_clearance.status,
            "insufficient_margin"
        );
        assert_eq!(tight_res.phr_readiness, "clearance_insufficient");

        // 3) Unscaled 40 m export -> likely_unscaled / scale_suspect!
        let unscaled = pack_synthetic_glb(
            &json!({
                "asset": { "version": "2.0" },
                "scene": 0,
                "scenes": [{ "nodes": [0] }],
                "nodes": [{ "mesh": 0 }],
                "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
                "accessors": [{
                    "type": "VEC3",
                    "min": [-20.0, -20.0, -20.0],
                    "max": [20.0, 20.0, 20.0]
                }]
            }),
            0,
        );
        let unscaled_res = inspect_glb_bytes(&unscaled);
        assert_eq!(unscaled_res.object_head_clearance.status, "likely_unscaled");
        assert_eq!(unscaled_res.phr_readiness, "scale_suspect");

        // 4) Invalid magic -> invalid_glb
        let bad = inspect_glb_bytes(b"NOT_A_GLB_FILE_AT_ALL_12345");
        assert_eq!(bad.format, "invalid_glb");
        assert_eq!(bad.phr_readiness, "invalid_glb");
    }

    impl GlbInspectedFileV01 {
        fn object_head_status(&self) -> &'static str {
            self.object_head_clearance.status
        }
    }

    #[test]
    fn sanitizes_relative_filenames_against_drive_and_traversal_prefixes() {
        assert_eq!(
            sanitize_relative_file_name("C:\\Users\\Alice\\Models\\Head.glb"),
            "Users/Alice/Models/Head.glb"
        );
        assert_eq!(
            sanitize_relative_file_name("..\\..\\Sub\\Avatar.vrm"),
            "Sub/Avatar.vrm"
        );
    }
}
