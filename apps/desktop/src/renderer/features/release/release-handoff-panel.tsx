import { formatDateTime } from "../../i18n/index.ts";
import { useEffect, useRef, useState } from "react";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import type { ReleaseHandoffFactV02 } from "@vua/contracts";
import { useGateway } from "../../gateway/index.ts";
import { format, strings, termLabel } from "../../i18n/index.ts";
import type { PageId } from "../../app/nav-model.ts";
import { handoffIntentErrorText, taskStateLabelKey } from "./release-handoff-model.ts";

const copy = strings.release.records.handoff;

/**
 * Build Record 行「交接」主操作面板(023 消费切片,桌面表态 IA 第 2 点):
 * - 经 release.openForHandoff(tasked)发起;受理后按 taskId 轮询任务面九态,
 *   完成判定=Bridge handshake 到达(契约语义,呈现层不自行推断完成);
 * - 完成呈现=「已交接」事实(交接事实文档收窄,无上传状态字段)＋「最终
 *   上传在官方 SDK 中完成」如实说明;绝不渲染上传进度/结果(VUA 侧已终态,
 *   无事实可显——诚实纪律 1/2);
 * - 实现域未接线=路由恒答 vua.release_handoff.unavailable→缺席语义呈现
 *   (absent),不预接可用假象(wt-2 冻结批留言明确要求);
 * - 取消不经本面板:取消目标在任务中心任务卡(017 批 2 验收口径一致);
 * - upload_readiness 证据摘要不在本面板呈现:buildId→inspectionId 无权威
 *   关联路径(build-record 文档无检查身份,证据束按 avatarRef 寻址),跨源
 *   推导为投影纪律禁止——核心表态已裁选项②维持现状(023 线程):权威
 *   浏览面在检查页,本页不呈现摘要即最终形态;完成态仅按核心表态第 4 点
 *   以纯导航 IA 手段指引(不带任何检查身份,零跨源推导),onNavigate 未传
 *   时不渲染按钮(零死按钮);
 * - U19 交棒准入(用户裁决 2026-09-21):本面板仅在记录状态白名单桶
 *   (succeeded/succeeded_with_warnings,警告呈现保留)被挂载——分桶判定
 *   在 release-records-section 经 handoffAdmission 纯投影完成;后端权威
 *   闸独立在路由准入序,受理被拒(准入闸两码)时本面板 intent-failed 臂
 *   如实呈现类型化拒绝词面,不猜测不降标。
 */

/** 轮询间隔(ms):任务面权威快照 task.get;非终态继续,终态即停 */
const POLL_MS = 2000;

type HandoffPhase =
  | { kind: "idle" }
  | { kind: "absent" }
  | { kind: "intent-failed"; code: string | null; params: Readonly<Record<string, string | number | boolean>> }
  | { kind: "polling"; taskId: string; state: string | null }
  | { kind: "succeeded"; fact: ReleaseHandoffFactV02 }
  | { kind: "task-failed"; errorCode: string | null; messageKey: string | null }
  | { kind: "cancelled" }
  | { kind: "fact-unexplainable" };

/** intent-failed 词面(纯组合移入模型层,U19 准入闸两码词面同函数消费;
 *  词面表由 strings 喂入,测试侧同表对拍) */
function intentErrorText(
  code: string | null,
  params: Readonly<Record<string, string | number | boolean>>,
): string {
  return handoffIntentErrorText(code, params, {
    failedUnknown: copy.failedUnknown,
    failedWithCode: copy.failedWithCode,
    codeInvalidParams: copy.codeInvalidParams,
    codeBuildUnknown: copy.codeBuildUnknown,
    codeEditorUnresolved: copy.codeEditorUnresolved,
    stateBlocked: strings.errors.releaseHandoff.stateBlocked,
    stateUnknown: strings.errors.releaseHandoff.stateUnknown,
  });
}

export function HandoffPanel({
  buildId,
  onNavigate,
}: {
  buildId: string;
  onNavigate?: ((target: PageId) => void) | undefined;
}) {
  const gateway = useGateway();
  const [phase, setPhase] = useState<HandoffPhase>({ kind: "idle" });
  const [readFailed, setReadFailed] = useState(false);
  /* phaseRef:轮询循环读取最新 phase 而不重启 interval 之外的副作用 */
  const phaseRef = useRef(phase);
  phaseRef.current = phase;

  useEffect(() => {
    if (phase.kind !== "polling") return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const schedule = () => {
      timer = setTimeout(poll, POLL_MS);
    };
    const poll = () => {
      const current = phaseRef.current;
      if (current.kind !== "polling") return;
      void gateway.releaseHandoff.taskSnapshot(current.taskId).then((view) => {
        if (!alive) return;
        if (view === null) {
          // 本轮读取失败(断连/响应不可解释):保持上一视图,继续轮询
          setReadFailed(true);
          schedule();
          return;
        }
        setReadFailed(false);
        if (view.kind === "running") {
          // state 实际变化才更新 phase(更新触发 effect 重启并立即重查);
          // 未变化按固定周期继续,不制造高频轮询
          if (current.state !== view.state) {
            setPhase({ kind: "polling", taskId: current.taskId, state: view.state });
          } else {
            schedule();
          }
          return;
        }
        if (view.kind === "succeeded") setPhase({ kind: "succeeded", fact: view.fact });
        else if (view.kind === "failed") {
          setPhase({ kind: "task-failed", errorCode: view.errorCode, messageKey: view.messageKey });
        } else if (view.kind === "cancelled") setPhase({ kind: "cancelled" });
        else setPhase({ kind: "fact-unexplainable" });
      });
    };
    poll();
    return () => {
      alive = false;
      if (timer !== null) clearTimeout(timer);
    };
  }, [gateway, phase]);

  const submit = () => {
    setReadFailed(false);
    void gateway.releaseHandoff.openForHandoff(buildId).then((intent) => {
      if (intent.kind === "accepted") {
        // state=null:尚无权威快照,呈现不预置九态文案(不猜测任务态)
        setPhase({ kind: "polling", taskId: intent.taskId, state: null });
      } else if (intent.kind === "absent") {
        setPhase({ kind: "absent" });
      } else {
        setPhase({ kind: "intent-failed", code: intent.code, params: intent.params });
      }
    });
  };

  const retry = (
    <Button variant="subtle" onClick={submit}>
      {copy.retry}
    </Button>
  );

  if (phase.kind === "idle") {
    return (
      <div className="vua-page__stack" data-testid="handoff-panel">
        <div className="vua-page__actions">
          <Button variant="primary" onClick={submit}>
            {copy.action}
          </Button>
        </div>
        <p className="vua-caption vua-text-secondary">{copy.actionNote}</p>
      </div>
    );
  }

  if (phase.kind === "absent") {
    return (
      <div className="vua-page__stack" data-testid="handoff-panel" role="note">
        <div className="vua-page__actions">
          <Badge tone="neutral">{copy.absentTitle}</Badge>
          {retry}
        </div>
        <p className="vua-caption vua-text-secondary">{copy.absentNote}</p>
      </div>
    );
  }

  if (phase.kind === "intent-failed") {
    return (
      <div className="vua-page__stack" data-testid="handoff-panel" role="alert">
        <div className="vua-page__actions">
          <Badge tone="error">{copy.failedTitle}</Badge>
          {retry}
        </div>
        <p className="vua-caption vua-text-secondary">{intentErrorText(phase.code, phase.params)}</p>
      </div>
    );
  }

  if (phase.kind === "polling") {
    const labelKey = phase.state !== null ? taskStateLabelKey(phase.state) : null;
    return (
      <div className="vua-page__stack" data-testid="handoff-panel" role="status">
        {/* 有权威快照才渲染状态徽标;词表外原词原样呈现,不猜测 */}
        {labelKey !== null || phase.state !== null ? (
          <div className="vua-page__actions">
            <Badge tone="neutral">
              {labelKey !== null ? strings.taskStatus[labelKey] : phase.state}
            </Badge>
          </div>
        ) : null}
        <p className="vua-caption vua-text-secondary">{copy.runningNote}</p>
        {readFailed ? (
          <p className="vua-caption vua-text-secondary" role="alert">
            {copy.readFailedNote}
          </p>
        ) : null}
        <p className="vua-caption vua-text-secondary">{copy.sdkNote}</p>
      </div>
    );
  }

  if (phase.kind === "succeeded") {
    return (
      <div className="vua-page__stack" data-testid="handoff-panel" role="status">
        <div className="vua-page__actions">
          <Badge tone="neutral">{copy.succeededTitle}</Badge>
        </div>
        <p className="vua-caption vua-text-secondary">
          {format(copy.succeededLine, {
            editorVersion: phase.fact.editor.version,
            occurredAt: formatDateTime(phase.fact.occurredAt),
          })}
        </p>
        <p className="vua-caption vua-text-secondary">
          {format(copy.projectLine, { projectId: phase.fact.projectId })}
        </p>
        <p className="vua-caption vua-text-secondary">{copy.sdkNote}</p>
        {/* 检查落在车间工位(检查页已退役);纯页面级导航,回调未接不渲染按钮 */}
        {onNavigate !== undefined ? (
          <div className="vua-page__actions">
            <Button variant="subtle" onClick={() => onNavigate("workshop")}>
              {copy.entryWorkshop}
            </Button>
          </div>
        ) : null}
      </div>
    );
  }

  if (phase.kind === "task-failed") {
    return (
      <div className="vua-page__stack" data-testid="handoff-panel" role="alert">
        <div className="vua-page__actions">
          <Badge tone="error">{strings.taskStatus.failed}</Badge>
          {retry}
        </div>
        {phase.errorCode !== null ? (
          <p className="vua-caption vua-text-secondary">
            {format(copy.taskErrorLine, { code: phase.errorCode })}
          </p>
        ) : (
          <p className="vua-caption vua-text-secondary">{copy.taskFailedNote}</p>
        )}
        <p className="vua-caption vua-text-secondary">{copy.sdkNote}</p>
      </div>
    );
  }

  if (phase.kind === "cancelled") {
    return (
      <div className="vua-page__stack" data-testid="handoff-panel" role="status">
        <div className="vua-page__actions">
          <Badge tone="neutral">{strings.taskStatus.cancelled}</Badge>
          {retry}
        </div>
        <p className="vua-caption vua-text-secondary">{copy.cancelledNote}</p>
      </div>
    );
  }

  // fact-unexplainable:任务终态成功但事实不可解释——如实呈现,不猜测
  return (
    <div className="vua-page__stack" data-testid="handoff-panel" role="alert">
      <div className="vua-page__actions">
        <Badge tone="error">{copy.factUnexplainableTitle}</Badge>
        {retry}
      </div>
      <p className="vua-caption vua-text-secondary">{copy.factUnexplainable}</p>
    </div>
  );
}
