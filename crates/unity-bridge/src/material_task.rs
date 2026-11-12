//! Registers the material intake execution as a TaskRuntime task type.
//!
//! B3 的取消/漂移/超时/回滚/幂等重放语义由此落到任务层：执行器仍是纯
//! 执行核，这里只做三件事——把确认与路径绑定为一次性任务闭包、把任务级
//! 取消请求桥接到本次执行私有的取消令牌（观察线程在步骤边界生效）、把
//! 运行报告映射为任务出口（Done 携带报告与回执指针，Cancelled/Failed 走
//! 对应出口）。运行时自身负责九态、事件、revision、commandId 幂等与超时
//! 强制结束。

use crate::material_exec::{MaterialCancelToken, MaterialExecutor};
use crate::material_intake::MaterialIntakeConfirmationV01;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use vua_orchestrator::ProjectRef;
use vua_orchestrator::{SubmitRequest, TaskExit, TaskJob, TaskRuntime};

/// 一个素材入口执行任务的全部绑定（确认 + 路径 + 产物输出根 + 本次执行
/// 私有的取消令牌）。令牌由提交层创建：一次提交一个，执行器不持有任何
/// 跨任务状态。
#[derive(Debug, Clone)]
pub struct MaterialIntakeTaskSpec {
    pub confirmation: MaterialIntakeConfirmationV01,
    pub source_folder: PathBuf,
    pub project: ProjectRef,
    pub artifact_output_root: PathBuf,
    pub token: MaterialCancelToken,
}

/// 执行报告的任务层载荷（含回执指针，供任务中心/详情跳转）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialTaskResult {
    pub plan_id: String,
    pub status: String,
    pub error_code: Option<String>,
    pub rollback: String,
    pub build_record_id: Option<String>,
    pub replayed: bool,
}

/// 组装任务闭包：观察线程把任务级取消桥接到本次执行的令牌（步骤边界
/// 生效），主路径执行并映射报告。
pub fn material_intake_job(
    executor: Arc<MaterialExecutor>,
    spec: Arc<MaterialIntakeTaskSpec>,
) -> TaskJob {
    Box::new(move |ctx| {
        let done = Arc::new(AtomicBool::new(false));

        let report = std::thread::scope(|scope| {
            let done_watcher = done.clone();
            let ctx_ref = &*ctx;
            let token = spec.token.clone();
            scope.spawn(move || loop {
                if done_watcher.load(Ordering::SeqCst) {
                    return;
                }
                if ctx_ref.check_cancel() {
                    token.cancel();
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            });
            let report = executor.execute(
                &spec.confirmation,
                &spec.source_folder,
                &spec.project,
                &spec.artifact_output_root,
                &spec.token,
            );
            done.store(true, Ordering::SeqCst);
            report
        });

        let result = MaterialTaskResult {
            plan_id: report.plan_id.clone(),
            status: match report.status {
                crate::MaterialExecutionStatus::Succeeded => "succeeded".to_owned(),
                crate::MaterialExecutionStatus::Cancelled => "cancelled".to_owned(),
                crate::MaterialExecutionStatus::Failed => "failed".to_owned(),
            },
            error_code: report.error_code.clone(),
            rollback: match report.rollback {
                crate::RollbackOutcome::NotNeeded => "not_needed".to_owned(),
                crate::RollbackOutcome::Restored => "restored".to_owned(),
                crate::RollbackOutcome::Failed => "failed".to_owned(),
            },
            build_record_id: report.build_record_id.clone(),
            replayed: report.replayed,
        };
        let payload = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
        let plan_id = result.plan_id.clone();

        match report.status {
            crate::MaterialExecutionStatus::Succeeded => Ok(TaskExit::Done(payload)),
            crate::MaterialExecutionStatus::Cancelled => Ok(TaskExit::Cancelled),
            crate::MaterialExecutionStatus::Failed => Err(vua_orchestrator::AppErrorV1::new(
                report
                    .error_code
                    .clone()
                    .unwrap_or_else(|| "vua.material.failed".to_owned()),
                vua_orchestrator::ErrorCategory::ExternalFailure,
                // 第 150 批：失败词面按类别分流——供给段失败命中桌面
                // 预留的 provisionFailed，其余维持 executionFailed。
                crate::material_exec::failure_message_key(report.error_code.as_deref()),
                &spec.confirmation.correlation_id,
            )
            .with_param("planId", vua_orchestrator::ParamValue::Text(plan_id))
            .with_recoverable(true)),
        }
    })
}

/// 便捷提交：以确认的 correlation id 创建任务，返回命令受理。
pub fn submit_material_intake(
    runtime: &TaskRuntime,
    executor: Arc<MaterialExecutor>,
    spec: MaterialIntakeTaskSpec,
    timeout: Option<Duration>,
) -> Result<vua_orchestrator::CommandAcceptedV1, vua_orchestrator::AppErrorV1> {
    let correlation_id = spec.confirmation.correlation_id.clone();
    runtime.submit(SubmitRequest {
        correlation_id: Some(correlation_id),
        timeout,
        job: material_intake_job(executor, Arc::new(spec)),
    })
}
