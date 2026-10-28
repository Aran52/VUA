import { useEffect, useRef, useState } from "react";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { useEnvironmentView, useGateway, useTaskCenter } from "../../gateway/index.ts";
import { format, strings } from "../../i18n/index.ts";
import type { ImportCopyPlanV01, ImportCopyReceiptV01 } from "@vua/contracts";
import {
  confirmChainDecision,
  receiptLines,
  bytesText,
  type ConfirmChainTexts,
  type GuardTextTable,
} from "./project-compat-model.ts";
import {
  associationLabel,
  lockStatusKey,
  narrowEnvironmentSnapshot,
  narrowInspectAssociations,
  narrowVuaIdentity,
  type VuaIdentityNarrowed,
} from "./project-detection-model.ts";
import type { RegisteredProjectRow } from "../../gateway/index.ts";
import "./project-compat.css";

/**
 * 项目兼容分区(proposal 026 B,用户 2026-09-18 裁决):原「项目兼容」独立页
 * 并入包管理器页尾部分区;对 ALCOM/VCC 管理的项目只读兼容呈现与
 * 「导入为 VUA 管理的副本」确认链。五读面(environmentManagers/
 * inspectProject/lockStatus/setNote/importCopy,013/014/020/021 词表)原样
 * 随迁,零 wire 触碰。分区恒渲染,不挂 packages 引擎 capability 门控——
 * 本分区消费面走 projectOps 独立通道,包引擎未接入时分区照常可用(IA 并入
 * ＝选项卡级合并,非引擎能力级绑定)。
 *
 * 数据源两段(诚实纪律):
 * - 环境状态段:env 引擎 presence 模型(create 辖区版本轨道)先行消费,
 *   恒标注「VUA 侧检测,非 ALCOM/VCC 记录」;
 * - 项目检测段:project-inspection 读面 live 消费(021 接线批),管理器
 *   计数/识别/备注/锁状态照读面事实呈现,缺席与不可用如实区分;
 * - 副本导入确认链(014 语义冻结;#22 消费批起 live 走任务化通道):源路径
 *   (在册项目选择器优先,proposal 026 B:复用 P1 同款 project.listProjects
 *   消费面,零新词表;非在册路径手输回退——ALCOM/VCC 管理原项目不在册,
 *   导入守卫 source_not_registered 如实兜底)→目标表单→plan 要点确认面板→
 *   apply→receipt 呈现;守卫拒绝(七项闭集)按 guard 原样映射文案与 detail。
 *   live 下 plan/apply 均为任务化受理(proposal 020 result 回流):端口内
 *   等待任务终态并从快照 result 窄化结果文档,本组件保持既有 outcome 形状;
 *   等待期间按钮禁用,任务真实状态由任务中心呈现;失败/取消/超时 = 诚实
 *   unavailable 反馈。fixture(DEV)恒诚实不可用(与 setNote 同裁定)。
 *   确认链呈现决策与回执五项投影抽为纯函数(project-compat-model,裁决 1:
 *   B8/B9 两呈现段的测试范围),本组件只做状态迁移与渲染。
 *
 * 写操作交接:呈现 1.2.0 禁止清单要点与「交接给对应管理器」引导;
 * 交接的具体交互形态(外部拉起等)无词表与能力事实,不虚构,维持预告
 * 文案(诚实纪律 1;proposal 026 B 预留路径)。
 */
const copy = strings.projectCompat;

const guardTable: GuardTextTable = {
  target_exists: copy.guardTargetExists,
  target_inside_source: copy.guardTargetInsideSource,
  source_not_registered: copy.guardSourceNotRegistered,
  source_invalid: copy.guardSourceInvalid,
  insufficient_disk_space: copy.guardInsufficientDiskSpace,
  plan_drift: copy.guardPlanDrift,
  execution_failed: copy.guardExecutionFailed,
};

const confirmTexts: ConfirmChainTexts = {
  guardTable,
  guardFallback: copy.guardFallback,
  unavailable: copy.importUnavailable,
};

const receiptLabels = {
  target: copy.importReceiptTarget,
  bytes: copy.importReceiptBytes,
  copied: copy.importReceiptCopied,
  source: copy.importReceiptSource,
  inspect: copy.importReceiptInspect,
};

type Flow =
  | { readonly kind: "idle" }
  | { readonly kind: "form" }
  | { readonly kind: "plan"; readonly plan: ImportCopyPlanV01 }
  | { readonly kind: "receipt"; readonly receipt: ImportCopyReceiptV01 };

/* ---- 检测段(013 读面消费,T-B):environmentManagers 快照呈现＋B6 识别流 ---- */

type ManagerState =
  | { readonly kind: "loading" }
  | { readonly kind: "unavailable" }
  | { readonly kind: "loaded"; readonly narrowed: ReturnType<typeof narrowEnvironmentSnapshot> };

type IdentifyState =
  | { readonly kind: "idle" }
  | { readonly kind: "inspecting" }
  | {
      readonly kind: "identified";
      readonly path: string;
      readonly associations: readonly string[];
      readonly identity: VuaIdentityNarrowed | null;
    }
  | { readonly kind: "not-found" }
  | { readonly kind: "unavailable" };

/** 备注流状态(D-6):查看 / 编辑草稿 / 已受理等待任务终态(任务中心权威) */
type NoteFlow =
  | { readonly kind: "view" }
  | { readonly kind: "editing"; readonly draft: string }
  | { readonly kind: "waiting"; readonly taskId: string; readonly path: string; readonly startedAt: number };

function ProjectDetectionSection({ onMigrate }: { onMigrate: (sourcePath: string) => void }) {
  const [managerState, setManagerState] = useState<ManagerState>({ kind: "loading" });
  const [reloadKey, setReloadKey] = useState(0);
  const [pickedPath, setPickedPath] = useState<string | null>(null);
  const [identify, setIdentify] = useState<IdentifyState>({ kind: "idle" });
  const [lockLine, setLockLine] = useState<string | null>(null);

  /* ---- D-6 备注区(project-ops v0.2 setNote;裁定 A:行内查看+轻量编辑) ----
   * 诚实纪律:保存经 Gateway 命令唯一路径;任务化受理后等任务终态,再以
   * inspectProject 读面刷新确认存储事实——成功判定 = 读面 note 与提交值
   * 一致;拒绝按读面三态推导呈现(任务面不携带拒绝 detail,不猜测)。
   * absent 不呈现备注入口;unreadable 只读+如实说明;identity 收窄失败
   * (null,不可解释)同 absent 不呈现——不猜测。 */
  const noteCopy = copy.note;
  const gateway = useGateway();
  const taskCenter = useTaskCenter();
  const [noteFlow, setNoteFlow] = useState<NoteFlow>({ kind: "view" });
  const [noteLine, setNoteLine] = useState<string | null>(null);
  /** 等待读面确认的提交值(null = 无在途提交) */
  const notePendingRef = useRef<string | null>(null);

  const reloadManagers = () => setReloadKey((key) => key + 1);

  useEffect(() => {
    let active = true;
    setManagerState({ kind: "loading" });
    void window.vua?.gateway
      .invoke({
        schemaVersion: 1,
        requestId: crypto.randomUUID(),
        method: "project.environmentManagers",
        params: {},
      })
      .then((result) => {
        if (!active) return;
        if (!result.ok) {
          setManagerState({ kind: "unavailable" });
          return;
        }
        // 021 接线批修正(收敛点 1):wire 值 = project-inspection 查询信封
        // {schemaVersion, operation, result},environment-managers 快照本体在
        // 内层 result——此前误读信封顶层,editors 计数呈现恒 '—'。先解包
        // 再 narrow,零协议变更
        const envelope = result.value as { result?: unknown };
        setManagerState({ kind: "loaded", narrowed: narrowEnvironmentSnapshot(envelope.result) });
      });
    return () => {
      active = false;
    };
  }, [reloadKey]);

  const inspect = (projectPath: string) => {
    setIdentify({ kind: "inspecting" });
    setLockLine(null);
    void window.vua?.gateway
      .invoke({
        schemaVersion: 1,
        requestId: crypto.randomUUID(),
        method: "project.inspectProject",
        params: { projectPath },
      })
      .then((result) => {
        if (result.ok) {
          const payload = result.value as {
            path?: unknown;
            associations?: unknown;
            vuaIdentity?: unknown;
          };
          if (typeof payload.path === "string" && Array.isArray(payload.associations)) {
            setIdentify({
              kind: "identified",
              path: payload.path,
              associations: narrowInspectAssociations(payload.associations),
              // vuaIdentity 三态(D-6 备注写读面);字段收不齐 = null(不可
              // 解释,不猜测——备注区不呈现编辑入口)
              identity: narrowVuaIdentity(payload.vuaIdentity),
            });
            return;
          }
          setIdentify({ kind: "unavailable" });
          return;
        }
        const applicationCode =
          result.error.code === "application"
            ? ((result.error as { application?: { code?: string } }).application?.code ?? "")
            : "";
        setIdentify(
          applicationCode === "vua.project.project_not_found"
            ? { kind: "not-found" }
            : { kind: "unavailable" },
        );
      });
  };

  const pickAndInspect = () => {
    void window.vua?.dialog.pickWarehouseFolders().then((folders) => {
      const first = folders?.[0];
      if (first === undefined) return;
      setPickedPath(first);
      inspect(first);
    });
  };

  const viewOnly = (projectPath: string) => {
    void window.vua?.gateway
      .invoke({
        schemaVersion: 1,
        requestId: crypto.randomUUID(),
        method: "project.lockStatus",
        params: { projectPath },
      })
      .then((result) => {
        const mutationStatus = result.ok
          ? (result.value as { mutationStatus?: unknown }).mutationStatus
          : undefined;
        const key = lockStatusKey(mutationStatus);
        setLockLine(key === null ? copy.lockUnreadable : copy[key]);
      });
  };

  /** inspect 结果窄化复用(初次识别与备注保存后的读面刷新同源) */
  const narrowIdentified = (payload: {
    path?: unknown;
    associations?: unknown;
    vuaIdentity?: unknown;
  }): IdentifyState | null => {
    if (typeof payload.path !== "string" || !Array.isArray(payload.associations)) return null;
    return {
      kind: "identified",
      path: payload.path,
      associations: narrowInspectAssociations(payload.associations),
      identity: narrowVuaIdentity(payload.vuaIdentity),
    };
  };

  const refreshAfterNote = (path: string) => {
    void window.vua?.gateway
      .invoke({
        schemaVersion: 1,
        requestId: crypto.randomUUID(),
        method: "project.inspectProject",
        params: { projectPath: path },
      })
      .then((result) => {
        if (!result.ok) {
          const applicationCode =
            result.error.code === "application"
              ? ((result.error as { application?: { code?: string } }).application?.code ?? "")
              : "";
          if (applicationCode === "vua.project.project_not_found") {
            setIdentify({ kind: "not-found" });
            setNoteLine(noteCopy.rejectedNotFound);
          } else {
            setNoteLine(noteCopy.unconfirmed);
          }
          notePendingRef.current = null;
          setNoteFlow({ kind: "view" });
          return;
        }
        const identified = narrowIdentified(
          result.value as { path?: unknown; associations?: unknown; vuaIdentity?: unknown },
        );
        if (identified === null || identified.kind !== "identified") {
          setNoteLine(noteCopy.unconfirmed);
          notePendingRef.current = null;
          setNoteFlow({ kind: "view" });
          return;
        }
        setIdentify(identified);
        const fresh = identified.identity;
        if (
          fresh !== null &&
          fresh.status === "present" &&
          fresh.note === notePendingRef.current
        ) {
          setNoteLine(noteCopy.savedConfirmed);
        } else if (fresh !== null && fresh.status === "absent") {
          setNoteLine(noteCopy.rejectedAbsent);
        } else if (fresh !== null && fresh.status === "unreadable") {
          setNoteLine(noteCopy.rejectedUnreadable);
        } else {
          setNoteLine(noteCopy.unconfirmed);
        }
        notePendingRef.current = null;
        setNoteFlow({ kind: "view" });
      });
  };

  // 备注任务终态等待(任务中心权威,AC-07 同款):终态或有界超时后经读面
  // 刷新确认;呈现不以受理回执或本地推断伪造
  useEffect(() => {
    if (noteFlow.kind !== "waiting") return;
    const item = taskCenter.tasks.find((entry) => entry.id === noteFlow.taskId);
    const settled =
      item !== undefined &&
      (item.status === "completed" ||
        item.status === "completedWithWarnings" ||
        item.status === "failed" ||
        item.status === "cancelled");
    const timedOut = Date.now() - noteFlow.startedAt > 20_000;
    if (settled || timedOut) {
      if (timedOut && !settled) setNoteLine(noteCopy.waitTimeout);
      refreshAfterNote(noteFlow.path);
    }
    // 刷新依据 noteFlow/taskCenter 快照;refreshAfterNote 为稳定闭包
  }, [noteFlow, taskCenter.tasks]);

  const saveNote = (note: string | null) => {
    if (identify.kind !== "identified") return;
    setNoteLine(null);
    void gateway.projectOps.setNote({ projectPath: identify.path, note }).then((outcome) => {
      if (!outcome.ok) {
        setNoteLine(noteCopy.unavailable);
        return;
      }
      notePendingRef.current = note;
      setNoteFlow({
        kind: "waiting",
        taskId: outcome.accepted.taskId,
        path: identify.path,
        startedAt: Date.now(),
      });
    });
  };


  const managerLine = (snapshot: ReturnType<typeof narrowEnvironmentSnapshot>): string => {
    if (snapshot === null) return copy.detectionUnavailable;
    // 计数＝列表纯派生量(核心表态):vcc/alcom 的 userProjects.length 与
    // editors/projects 计数直接投影(021 接线批解包修正后可用;缺失仍诚实 '—')
    const vccCount = snapshot.vcc?.userProjectsCount ?? null;
    const alcomCount = snapshot.alcom?.userProjectsCount ?? null;
    return format(copy.detectionManagersLine, {
      vccN: vccCount === null ? "—" : String(vccCount),
      alcomN: alcomCount === null ? "—" : String(alcomCount),
    });
  };

  return (
    <div className="vua-project-compat__detection">
      <p className="vua-caption vua-text-secondary">{copy.detectionWired}</p>
      {managerState.kind === "loading" ? (
        <p className="vua-caption vua-text-secondary">{copy.detectionUnavailable}</p>
      ) : null}
      {managerState.kind === "loaded" ? (
        <ul className="vua-project-compat__specs">
          <li>{managerLine(managerState.narrowed)}</li>
        </ul>
      ) : null}
      <div className="vua-project-compat__row">
        <Button variant="default" onClick={pickAndInspect}>
          {copy.detectionPickCta}
        </Button>
        <Button variant="subtle" disabled={pickedPath === null} onClick={reloadManagers}>
          {copy.detectionReload}
        </Button>
      </div>
      <p className="vua-caption vua-text-secondary">{copy.detectionPickNote}</p>

      {identify.kind === "inspecting" ? (
        <p className="vua-caption vua-text-secondary">{copy.detectionInspectCta}…</p>
      ) : null}
      {identify.kind === "not-found" ? (
        <p className="vua-caption vua-text-secondary" role="alert">
          {copy.detectionNotFound}
        </p>
      ) : null}
      {identify.kind === "unavailable" ? (
        <p className="vua-caption vua-text-secondary" role="alert">
          {copy.detectionUnavailable}
        </p>
      ) : null}
      {identify.kind === "identified" ? (
        <div
          className="vua-project-compat__form"
          role="group"
          aria-label={copy.associationsTitle}
        >
          <p>
            <strong>{identify.path}</strong>
          </p>
          <p className="vua-caption vua-text-secondary">{copy.associationsTitle}</p>
          <ul className="vua-project-compat__specs">
            {identify.associations.map((association) => (
              <li key={association}>{associationLabel(association, {
                vcc: copy.associationVcc,
                alcom: copy.associationAlcom,
              })}</li>
            ))}
          </ul>
          {lockLine !== null ? (
            <p className="vua-caption vua-text-secondary" role="status">
              {lockLine}
            </p>
          ) : null}
          {identify.identity !== null && identify.identity.status === "present" ? (
            /* D-6 备注区(present 态):行内查看+轻量编辑;保存唯一路径 =
             * project.setNote,任务化受理+读面刷新确认 */
            <div className="vua-project-compat__form" role="group" aria-label={noteCopy.title}>
              <p className="vua-caption vua-text-secondary">
                {noteCopy.title}
                {identify.identity.markedAt !== null
                  ? ` · ${format(noteCopy.markedAt, { markedAt: identify.identity.markedAt })}`
                  : ""}
              </p>
              {noteFlow.kind === "waiting" ? (
                <p className="vua-caption vua-text-secondary" role="status">
                  {format(noteCopy.saving, { taskId: noteFlow.taskId })}
                </p>
              ) : noteFlow.kind === "editing" ? (
                <>
                  <input
                    type="text"
                    className="vua-text-input"
                    value={noteFlow.draft}
                    maxLength={2000}
                    placeholder={noteCopy.placeholder}
                    onChange={(event) =>
                      setNoteFlow({ kind: "editing", draft: event.target.value })
                    }
                  />
                  <div className="vua-project-compat__row">
                    <Button variant="default" onClick={() => setNoteFlow({ kind: "view" })}>
                      {noteCopy.cancel}
                    </Button>
                    <Button
                      variant="primary"
                      disabled={noteFlow.draft.trim() === ""}
                      onClick={() => saveNote(noteFlow.draft)}
                    >
                      {noteCopy.save}
                    </Button>
                  </div>
                </>
              ) : (
                <>
                  <p>{identify.identity.note !== null ? identify.identity.note : noteCopy.empty}</p>
                  <div className="vua-project-compat__row">
                    <Button
                      variant="subtle"
                      onClick={() =>
                        setNoteFlow({ kind: "editing", draft: identify.identity?.note ?? "" })
                      }
                    >
                      {noteCopy.edit}
                    </Button>
                    {identify.identity.note !== null ? (
                      <Button variant="subtle" onClick={() => saveNote(null)}>
                        {noteCopy.clear}
                      </Button>
                    ) : null}
                  </div>
                </>
              )}
              {noteLine !== null ? (
                <p className="vua-caption vua-text-secondary" role="status">
                  {noteLine}
                </p>
              ) : null}
              <p className="vua-caption vua-text-secondary">{noteCopy.listOnly}</p>
            </div>
          ) : identify.identity !== null && identify.identity.status === "unreadable" ? (
            <p className="vua-caption vua-text-secondary" role="note">
              {noteCopy.unreadableNote}
            </p>
          ) : null}
          {/* identity.absent / identity = null(不可解释):不呈现备注入口——
              备注只属于可读的 VUA 原生身份,不猜测 */}
          <div className="vua-project-compat__row">
            <Button variant="subtle" onClick={() => viewOnly(identify.path)}>
              {copy.viewOnlyCta}
            </Button>
            <Button variant="primary" onClick={() => onMigrate(identify.path)}>
              {copy.migrateCta}
            </Button>
          </div>
        </div>
      ) : null}
    </div>
  );
}

export function ProjectCompatSection() {
  const environment = useEnvironmentView();
  const gateway = useGateway();
  const [flow, setFlow] = useState<Flow>({ kind: "idle" });
  const [busy, setBusy] = useState(false);
  const [errorFeedback, setErrorFeedback] = useState<string | null>(null);
  // 表单草稿(014 语义不变;源路径优先经在册项目选择器填充,proposal 026 B)
  const [sourcePath, setSourcePath] = useState("");
  const [parentDir, setParentDir] = useState("");
  const [projectName, setProjectName] = useState("");
  // 在册项目清单(P1 同款 project.listProjects 消费面,零新词表;分区挂载即
  // 加载一次):null = 清单不可用(诚实注记,仅手输回退可用,不猜测原因)
  const [registered, setRegistered] = useState<readonly RegisteredProjectRow[] | null>(null);

  useEffect(() => {
    let active = true;
    void gateway.projectOps.listProjects().then((outcome) => {
      if (!active) return;
      if (outcome.ok) {
        setRegistered(outcome.projects);
      } else {
        setRegistered(null);
      }
    });
    return () => {
      active = false;
    };
  }, [gateway]);

  const startForm = () => {
    setFlow({ kind: "form" });
    setErrorFeedback(null);
  };

  const pickParent = () => {
    void window.vua?.dialog.pickWarehouseFolders().then((folders) => {
      if (folders !== null && folders.length > 0) setParentDir(folders[0]!);
    });
  };

  const applyDecision = (outcome: Parameters<typeof confirmChainDecision>[0]): void => {
    const decision = confirmChainDecision(outcome, confirmTexts);
    if (decision.kind === "plan") {
      setFlow({ kind: "plan", plan: decision.plan });
    } else if (decision.kind === "receipt") {
      setFlow({ kind: "receipt", receipt: decision.receipt });
    } else {
      setErrorFeedback(decision.feedback);
    }
  };

  const submitPlan = () => {
    setBusy(true);
    setErrorFeedback(null);
    void gateway.projectOps
      .importCopy({
        phase: "plan",
        sourcePath: sourcePath.trim(),
        targetParentDirectory: parentDir.trim(),
        targetProjectName: projectName.trim(),
      })
      .then((outcome) => {
        setBusy(false);
        applyDecision(outcome);
      });
  };

  const submitApply = (plan: ImportCopyPlanV01) => {
    setBusy(true);
    setErrorFeedback(null);
    void gateway.projectOps
      .importCopy({
        phase: "apply",
        sourcePath: plan.sourcePath,
        targetParentDirectory: parentDir.trim(),
        targetProjectName: projectName.trim(),
        confirmedPlanDigest: plan.planDigest,
      })
      .then((outcome) => {
        setBusy(false);
        applyDecision(outcome);
      });
  };

  return (
    <Card>
      <div className="vua-page__stack">
        <h2 className="vua-packages__section-title">{copy.title}</h2>
        <p className="vua-caption vua-text-secondary">{copy.subtitle}</p>

        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.readOnlyTitle}</h3>
          <p className="vua-text-secondary">{copy.readOnlyDesc}</p>
        </section>

        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.detectionTitle}</h3>
          <ProjectDetectionSection
            onMigrate={(sourcePath) => {
              // B6 识别→迁移:预填确认链源路径(014 语义不变)
              setSourcePath(sourcePath);
              startForm();
            }}
          />
        </section>

        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.envStatusTitle}</h3>
          <p className="vua-caption vua-text-secondary">{copy.envStatusSource}</p>
          {environment.versions.create.length > 0 ? (
            <ul className="vua-project-compat__env-list">
              {environment.versions.create.map((track) => (
                <li key={track.id}>
                  <strong>{track.title}</strong>{" "}
                  <span className="vua-caption vua-text-secondary">
                    {track.installed ?? "—"}
                  </span>
                </li>
              ))}
            </ul>
          ) : null}
        </section>

        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.handoverTitle}</h3>
          <p className="vua-text-secondary">{copy.handoverDesc}</p>
        </section>

        <section>
          <h3 className="vua-warehouse-detail__section-title">
            {copy.importTitle} <Badge tone="error">⚠</Badge>
          </h3>
          <p className="vua-text-secondary">{copy.importSpecIntro}</p>
          <ul className="vua-project-compat__specs">
            {copy.importSpecs.map((spec) => (
              <li key={spec}>{spec}</li>
            ))}
          </ul>
        </section>

        {flow.kind === "idle" ? (
          <Button variant="default" onClick={startForm}>
            {copy.importCta}
          </Button>
        ) : null}

        {flow.kind === "form" ? (
          <div className="vua-project-compat__form">
            {/* 在册项目选择器(proposal 026 B):P1 同款 project.listProjects
             * 消费面;清单不可用或为空时不渲染,手输回退仍在——不猜测 */}
            {registered !== null && registered.length > 0 ? (
              <label className="vua-project-compat__label">
                <span>{copy.importSourceRegisteredLabel}</span>
                <select
                  aria-label={copy.importSourceRegisteredLabel}
                  value={
                    registered.some((project) => project.path === sourcePath)
                      ? sourcePath
                      : ""
                  }
                  onChange={(event) => setSourcePath(event.target.value)}
                >
                  <option value="" disabled>
                    {copy.importSourceRegisteredLabel}
                  </option>
                  {[...registered]
                    .sort((a, b) => a.name.localeCompare(b.name))
                    .map((project) => (
                      <option
                        key={project.path}
                        value={project.path}
                        disabled={!project.pathPresent}
                      >
                        {project.unityVersion !== null
                          ? `${project.name} · Unity ${project.unityVersion}`
                          : project.name}
                      </option>
                    ))}
                </select>
              </label>
            ) : null}
            <label className="vua-project-compat__label">
              <span>{copy.importSourceLabel}</span>
              <input
                type="text"
                className="vua-text-input"
                value={sourcePath}
                onChange={(event) => setSourcePath(event.target.value)}
                placeholder={copy.importSourcePlaceholder}
              />
            </label>
            <p className="vua-caption vua-text-secondary">{copy.importSourceNote}</p>
            <label className="vua-project-compat__label">
              <span>{copy.importParentLabel}</span>
              <span className="vua-project-compat__row">
                <input type="text" className="vua-text-input" value={parentDir} readOnly placeholder="—" />
                <Button variant="default" onClick={pickParent}>
                  {copy.importParentPick}
                </Button>
              </span>
            </label>
            <label className="vua-project-compat__label">
              <span>{copy.importNameLabel}</span>
              <input
                type="text"
                className="vua-text-input"
                value={projectName}
                onChange={(event) => setProjectName(event.target.value)}
              />
            </label>
            {errorFeedback !== null ? (
              <p className="vua-caption vua-text-secondary" role="alert">
                {errorFeedback}
              </p>
            ) : null}
            <div className="vua-project-compat__row">
              <Button
                variant="default"
                disabled={busy}
                onClick={() => setFlow({ kind: "idle" })}
              >
                {copy.importCancel}
              </Button>
              <Button
                variant="primary"
                disabled={busy || sourcePath.trim() === "" || parentDir.trim() === "" || projectName.trim() === ""}
                onClick={submitPlan}
              >
                {copy.importPlanCta}
              </Button>
            </div>
          </div>
        ) : null}

        {flow.kind === "plan" ? (
          <div
            className="vua-project-compat__form"
            role="group"
            aria-label={copy.importPlanPanelTitle}
          >
            <h3 className="vua-warehouse-detail__section-title">{copy.importPlanPanelTitle}</h3>
            <ul className="vua-project-compat__specs">
              <li>
                {copy.importPlanTarget}: {flow.plan.targetPath}
              </li>
              <li>
                {copy.importPlanBytes}: {bytesText(flow.plan.estimatedBytes)}
              </li>
              <li>
                {copy.importPlanExcluded}: {flow.plan.excludedEntries.join(", ")}
              </li>
              <li>
                {copy.importPlanTopLevels}: {flow.plan.sourceTopLevels.join(", ")}
              </li>
              <li>
                {copy.importPlanDigest}: …{flow.plan.planDigest.slice(-8)}
              </li>
            </ul>
            {errorFeedback !== null ? (
              <p className="vua-caption vua-text-secondary" role="alert">
                {errorFeedback}
              </p>
            ) : null}
            <div className="vua-project-compat__row">
              <Button
                variant="default"
                disabled={busy}
                onClick={() => setFlow({ kind: "form" })}
              >
                {copy.importCancel}
              </Button>
              <Button variant="primary" disabled={busy} onClick={() => submitApply(flow.plan)}>
                {copy.importApplyCta}
              </Button>
            </div>
          </div>
        ) : null}

        {flow.kind === "receipt" ? (
          <div className="vua-project-compat__form" role="status">
            <h3 className="vua-warehouse-detail__section-title">{copy.importReceiptTitle}</h3>
            {/* B8 呈现五项(裁决 1):新项目路径/已复制数据/已复制内容/来源
                关系已记录/重新检查完成——投影纯函数见 project-compat-model */}
            <ul className="vua-project-compat__specs">
              {receiptLines(flow.receipt, receiptLabels).map((line) => (
                <li key={line.label}>
                  {line.value === "" ? line.label : `${line.label}: ${line.value}`}
                </li>
              ))}
            </ul>
          </div>
        ) : null}
      </div>
    </Card>
  );
}
