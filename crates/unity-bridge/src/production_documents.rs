//! Production document faces (M3/T1, condition C1): the renderer-visible
//! views over the AMF inspection and plan documents, pinned by
//! `schemas/amf-production/v0.2/`. These are PROJECTIONS — the stored
//! documents remain the authority; the projections add the vocabulary the
//! production-use-case adopted from the renderer (findings, plannability,
//! plan diffs) and answer honestly where B3 produces nothing (diffs are
//! empty, duration is null).

use crate::material_intake::MaterialIntakePlanV01;
use serde::Serialize;
use vua_orchestrator::{ExecutableRiskKind, SourceFolderInspectionV01};

/// The closed finding vocabulary (production-use-case): `compat` /
/// `missing` / `conflict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionFindingKind {
    Compat,
    Missing,
    Conflict,
}

/// The plannability verdict of one inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Plannability {
    Plannable,
    NeedsAttention,
    NotPlannable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionFinding {
    pub kind: InspectionFindingKind,
    pub summary: String,
    pub recoverable: bool,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionDocument {
    pub inspection_id: String,
    pub inspected_at: String,
    pub display_name: String,
    pub source_fingerprint: String,
    pub risk_fingerprint: String,
    pub packages: Vec<SourcePackageView>,
    pub findings: Vec<InspectionFinding>,
    pub plannability: Plannability,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePackageView {
    pub relative_path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDocument {
    pub plan_id: String,
    pub revision: u64,
    pub inspection_id: String,
    pub mode: vua_orchestrator::MaterialEntryMode,
    pub project_id: String,
    pub project_fingerprint: String,
    /// Workflow stages in execution order (the unity-bridge lifecycle).
    pub stages: Vec<&'static str>,
    pub risk_decision_required: bool,
    pub risks: Vec<InspectionFinding>,
    /// B3 produces no plan diffs; the closed vocabulary (added / changed /
    /// resolved) is pinned for the v0.3 production extension.
    pub diffs: Vec<serde_json::Value>,
    pub estimated_duration_ms: Option<u64>,
}

/// Builds the renderer-facing inspection document. Executable-content
/// risks surface as `compat` findings carrying recoverable/retryable
/// markers; an inspection with no importable packages is honestly
/// `not_plannable`, and one with executable risks is `needs_attention`.
pub fn build_inspection_document(
    inspection_id: &str,
    inspected_at: &str,
    inspection: &SourceFolderInspectionV01,
) -> InspectionDocument {
    let findings: Vec<InspectionFinding> = inspection
        .executable_risks
        .iter()
        .map(|risk| InspectionFinding {
            kind: InspectionFindingKind::Compat,
            summary: format!(
                "executable content in {} ({}): {}",
                risk.package_path,
                risk_kind_name(risk.kind),
                risk.asset_path
            ),
            recoverable: true,
            retryable: true,
        })
        .collect();
    let plannability = if inspection.packages.is_empty() {
        Plannability::NotPlannable
    } else if inspection.executable_risks.is_empty() {
        Plannability::Plannable
    } else {
        Plannability::NeedsAttention
    };
    InspectionDocument {
        inspection_id: inspection_id.to_owned(),
        inspected_at: inspected_at.to_owned(),
        display_name: inspection.display_name.clone(),
        source_fingerprint: inspection.source_fingerprint.clone(),
        risk_fingerprint: inspection.risk_fingerprint.clone(),
        packages: inspection
            .packages
            .iter()
            .map(|package| SourcePackageView {
                relative_path: package.relative_path.clone(),
                size_bytes: package.size_bytes,
                sha256: package.sha256.clone(),
            })
            .collect(),
        findings,
        plannability,
    }
}

/// Builds the renderer-facing plan document over the stored plan. The
/// plan's own revision travels with the registry binding, not here.
pub fn build_plan_document(
    inspection_id: &str,
    revision: u64,
    plan: &MaterialIntakePlanV01,
) -> PlanDocument {
    let inspection_view = build_inspection_document(inspection_id, "", &plan.source);
    PlanDocument {
        plan_id: plan.plan_id.clone(),
        revision,
        inspection_id: inspection_id.to_owned(),
        mode: plan.mode,
        project_id: plan.project_id.clone(),
        project_fingerprint: plan.project_fingerprint.clone(),
        stages: vua_orchestrator::PRODUCTION_STAGES.to_vec(),
        risk_decision_required: plan.risk_decision_required,
        risks: inspection_view.findings,
        diffs: Vec::new(),
        estimated_duration_ms: None,
    }
}

fn risk_kind_name(kind: ExecutableRiskKind) -> &'static str {
    match kind {
        ExecutableRiskKind::CSharpSource => "csharp source",
        ExecutableRiskKind::ManagedAssembly => "managed assembly",
        ExecutableRiskKind::NativePlugin => "native plugin",
        ExecutableRiskKind::EditorContent => "editor content",
        ExecutableRiskKind::BuildEntryPoint => "build entry point",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vua_orchestrator::{
        ExecutableRiskEvidence, SourceFolderInspectionV01, SourcePackageEvidenceV01,
    };

    fn inspection(with_risk: bool, packages: usize) -> SourceFolderInspectionV01 {
        let mut risks = Vec::new();
        if with_risk {
            risks.push(ExecutableRiskEvidence {
                package_path: "pack.unitypackage".into(),
                asset_path: "Assets/bad.dll".into(),
                kind: ExecutableRiskKind::ManagedAssembly,
            });
        }
        SourceFolderInspectionV01 {
            schema_version: "vua.material-intake-inspection/v0.1".into(),
            display_name: "Fixture".into(),
            source_fingerprint: "sha256:aaa".into(),
            risk_fingerprint: "sha256:bbb".into(),
            packages: (0..packages)
                .map(|index| SourcePackageEvidenceV01 {
                    relative_path: format!("pack-{index}.unitypackage"),
                    size_bytes: 10,
                    sha256: format!("sha256:000{index}"),
                    asset_paths: vec![],
                })
                .collect(),
            executable_risks: risks,
            declared_dependencies: vec![],
        }
    }

    #[test]
    fn inspection_document_maps_risks_to_compat_findings_and_plannability() {
        let document =
            build_inspection_document("insp-1", "2026-09-06T09:00:00.000Z", &inspection(true, 1));
        assert_eq!(document.findings.len(), 1);
        assert_eq!(document.findings[0].kind, InspectionFindingKind::Compat);
        assert!(document.findings[0].recoverable);
        assert_eq!(document.plannability, Plannability::NeedsAttention);
        assert_eq!(document.inspected_at, "2026-09-06T09:00:00.000Z");

        let empty = build_inspection_document("insp-2", "t", &inspection(false, 0));
        assert_eq!(empty.plannability, Plannability::NotPlannable);
        let clean = build_inspection_document("insp-3", "t", &inspection(false, 2));
        assert_eq!(clean.plannability, Plannability::Plannable);
        assert!(clean.findings.is_empty());
    }

    #[test]
    fn plan_document_answers_honestly_where_b3_produces_nothing() {
        let plan = crate::material_intake::MaterialIntakePlanV01 {
            schema_version: "vua.material-intake-plan/v0.1".into(),
            plan_id: "plan-1".into(),
            plan_hash: "sha256:ccc".into(),
            mode: vua_orchestrator::MaterialEntryMode::DirectUnityPackage,
            project_id: "project".into(),
            project_fingerprint: "sha256:ddd".into(),
            source: inspection(false, 1),
            risk_decision_required: false,
            steps: vec![],
        };
        let document = build_plan_document("insp-1", 3, &plan);
        assert_eq!(document.revision, 3);
        assert_eq!(document.inspection_id, "insp-1");
        assert!(document.diffs.is_empty(), "B3 produces no plan diffs");
        assert_eq!(document.estimated_duration_ms, None);
        assert_eq!(document.stages.len(), 5);
    }
}
