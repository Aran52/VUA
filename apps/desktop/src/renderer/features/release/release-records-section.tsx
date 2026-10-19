import { formatDateTime } from "../../i18n/index.ts";
import { useCallback, useEffect, useState } from "react";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { EmptyState } from "../../components/primitives/EmptyState.tsx";
import { Skeleton } from "../../components/primitives/Skeleton.tsx";
import type { RecordListEntryV02 } from "@vua/contracts";
import { useGateway } from "../../gateway/index.ts";
import { format, strings, termLabel } from "../../i18n/index.ts";
import type { PageId } from "../../app/nav-model.ts";
import {
  narrowBuildRecordFacts,
  recordListStatusLabel,
  recordListStatusTone,
  recordStatusTone,
  sortRecordRowsByFinishedAtDesc,
  type BuildRecordFacts,
} from "./release-records-model.ts";
import { handoffAdmission } from "./release-handoff-model.ts";
import { HandoffPanel } from "./release-handoff-panel.tsx";
import { ProjectOpenPanel } from "./release-project-open-panel.tsx";

const copy = strings.release.records;
const handoffCopy = copy.handoff;

/**
 * 出厂页构建记录节(夜间任务 P2):消费 production-use-case v0.2
 * record.list / record.get(production-chain-port live 实现;核心路由已
 * 交付验收)。三态诚实:读取失败≠空列表(UI-08);空列表=服务端缺席根的
 * 真实空态(空态即终态)。真机数据流随 W25 窗口,本节不宣称端到端。
 *
 * 详情证据纪律:证据引用仅呈现计数(证据本体在生产证据库);检查证据面
 * (inspection-evidence,016 草案)尚未接入,本节不渲染任何"官方检查结论"
 * ——如实说明,不以演示替代(AC-13 同款)。
 */

type ListState =
  | { kind: "loading" }
  | { kind: "failed" }
  | { kind: "ok"; rows: readonly RecordListEntryV02[] };

type DetailState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "failed" }
  | { kind: "unexplainable" }
  | { kind: "ok"; facts: BuildRecordFacts };

/**
 * 交棒准入呈现区(U19 用户裁决 2026-09-21,BOARD U19 行规范源):记录状态 →
 * 呈现桶纯投影(handoffAdmission)驱动——
 * - allowed:交棒面板挂载(白名单两态;警告呈现保留——状态徽标照常在场,
 *   不因放行遮蔽);
 * - blocked(failed/cancelled/rolled_back):交棒动作不渲染,呈现禁用原因
 *   词面 + 诊断(检查页)/恢复与重新生产(车间页)入口链——复用纯页面
 *   导航原语(不跨页携带记录身份,023 纪律;onNavigate 缺席不渲染,零死
 *   按钮);
 * - blocked-recovered:禁用 +「先完成检视及后续生产流程」+ 检视入口
 *   (recovered ≠ 任务 inspect_required,两套状态不混用);
 * - 后端权威闸独立在路由准入序:直连调用被拒时由交棒面板 intent-failed
 *   臂呈现类型化拒绝词面,本呈现区不预断受理结果。
 * 独立「在 Unity 中打开以检查/修复」动作(U19 第二交付)与准入桶无关,
 * 可确认记录一律挂载(ProjectOpenPanel;不按记录状态闸)。
 */
function HandoffAdmissionArea({
  facts,
  onNavigate,
}: {
  facts: BuildRecordFacts;
  onNavigate?: ((target: PageId) => void) | undefined;
}) {
  const admission = handoffAdmission(facts.status);
  if (admission.kind === "allowed") {
    return <HandoffPanel buildId={facts.buildId} onNavigate={onNavigate} />;
  }
  if (admission.kind === "blocked") {
    const reason =
      admission.state === "failed"
        ? handoffCopy.blockedFailed
        : admission.state === "cancelled"
          ? handoffCopy.blockedCancelled
          : handoffCopy.blockedRolledBack;
    return (
      <div className="vua-page__stack" data-testid="handoff-blocked" role="alert">
        <div className="vua-page__actions">
          <Badge tone="error">{handoffCopy.blockedTitle}</Badge>
        </div>
        <p className="vua-caption vua-text-secondary">{reason}</p>
        {onNavigate !== undefined ? (
          <div className="vua-page__actions">
            <Button variant="subtle" onClick={() => onNavigate("workshop")}>
              {handoffCopy.entryWorkshop}
            </Button>
          </div>
        ) : null}
      </div>
    );
  }
  // blocked-recovered:检视入口唯一(后续生产流程在检视完成之后,入口链
  // 不预授生产跳转——裁决词面「先完成检视及后续生产流程」的呈现纪律)
  return (
    <div className="vua-page__stack" data-testid="handoff-blocked-recovered" role="alert">
      <div className="vua-page__actions">
        <Badge tone="error">{handoffCopy.blockedTitle}</Badge>
      </div>
      <p className="vua-caption vua-text-secondary">{handoffCopy.recoveredBlockNote}</p>
      {onNavigate !== undefined ? (
        <div className="vua-page__actions">
          <Button variant="subtle" onClick={() => onNavigate("workshop")}>
            {handoffCopy.entryWorkshop}
          </Button>
        </div>
      ) : null}
    </div>
  );
}

export function ReleaseRecordsSection({
  onNavigate,
}: {
  onNavigate?: ((target: PageId) => void) | undefined;
}) {
  const gateway = useGateway();
  const [list, setList] = useState<ListState>({ kind: "loading" });
  const [reloadNonce, setReloadNonce] = useState(0);
  const [selectedBuildId, setSelectedBuildId] = useState<string | null>(null);
  const [detail, setDetail] = useState<DetailState>({ kind: "idle" });

  const refreshList = useCallback(() => {
    setList({ kind: "loading" });
    setSelectedBuildId(null);
    setDetail({ kind: "idle" });
    void gateway.productionChain
      .listRecords({})
      .then((result) => {
        // null = 提供方响应不可用/不可解释:如实失败态,不折叠为空列表
        setList(
          result === null
            ? { kind: "failed" }
            : { kind: "ok", rows: sortRecordRowsByFinishedAtDesc(result.entries) },
        );
      })
      .catch(() => setList({ kind: "failed" }));
  }, [gateway]);

  useEffect(() => {
    refreshList();
  }, [refreshList, reloadNonce]);

  // 选中条目 → record.get 详情(按需拉取;不轮询不猜测)
  useEffect(() => {
    if (selectedBuildId === null) return;
    let alive = true;
    setDetail({ kind: "loading" });
    void gateway.productionChain
      .getRecord(selectedBuildId)
      .then((result) => {
        if (!alive) return;
        if (result === null) {
          setDetail({ kind: "failed" });
          return;
        }
        const facts = narrowBuildRecordFacts(result.recordDocument);
        setDetail(facts === null ? { kind: "unexplainable" } : { kind: "ok", facts });
      })
      .catch(() => {
        if (alive) setDetail({ kind: "failed" });
      });
    return () => {
      alive = false;
    };
  }, [gateway, selectedBuildId]);

  return (
    <Card>
      <div className="vua-page__stack">
        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.title}</h3>
          <p className="vua-caption vua-text-secondary">{copy.subtitle}</p>
        </section>

        {list.kind === "loading" ? (
          <div className="vua-page__stack">
            <Skeleton width="70%" />
            <Skeleton width="50%" />
          </div>
        ) : list.kind === "failed" ? (
          <EmptyState
            title={copy.failedTitle}
            description={copy.failedDescription}
            action={
              <Button variant="primary" onClick={() => setReloadNonce((nonce) => nonce + 1)}>
                {copy.reload}
              </Button>
            }
          />
        ) : list.rows.length === 0 ? (
          <EmptyState title={copy.emptyTitle} description={copy.emptyDescription} />
        ) : (
          <>
            <ul className="vua-release-records__list" role="list">
              {list.rows.map((row) => (
                <li key={row.buildId}>
                  <button
                    type="button"
                    className="vua-release-records__row"
                    data-selected={selectedBuildId === row.buildId || undefined}
                    onClick={() =>
                      setSelectedBuildId((current) => (current === row.buildId ? null : row.buildId))
                    }
                  >
                    <span className="vua-release-records__cell">{row.buildId}</span>
                    <Badge tone={recordListStatusTone(row.status)}>
                      {recordListStatusLabel(row.status, copy.status)}
                    </Badge>
                    <span className="vua-caption vua-text-secondary">{formatDateTime(row.finishedAt)}</span>
                  </button>
                </li>
              ))}
            </ul>

            {selectedBuildId !== null ? (
              <section>
                <h4 className="vua-caption vua-text-secondary">{copy.detailTitle}</h4>
                {detail.kind === "loading" || detail.kind === "idle" ? (
                  <Skeleton width="60%" />
                ) : detail.kind === "failed" ? (
                  <>
                    <p className="vua-caption vua-text-secondary" role="alert">
                      {copy.detailFailed}
                    </p>
                    {/* U19 缺失/未知桶:记录无法确认 → 交棒拒绝词面(不猜测
                     *  状态;无记录身份,open 动作亦不挂载) */}
                    <p className="vua-caption vua-text-secondary" role="alert">
                      {handoffCopy.unconfirmedNote}
                    </p>
                  </>
                ) : detail.kind === "unexplainable" ? (
                  <>
                    <p className="vua-caption vua-text-secondary" role="alert">
                      {copy.detailUnexplainable}
                    </p>
                    <p className="vua-caption vua-text-secondary" role="alert">
                      {handoffCopy.unconfirmedNote}
                    </p>
                  </>
                ) : (
                  <div className="vua-page__stack">
                    <p className="vua-caption vua-text-secondary">
                      {format(copy.detailBuildLine, { buildId: detail.facts.buildId })}
                    </p>
                    <p className="vua-caption vua-text-secondary">
                      {format(copy.detailRecipeLine, {
                        recipeId: detail.facts.recipeId,
                        revision: String(detail.facts.recipeRevision),
                      })}
                      {" · "}
                      {format(copy.detailPlanLine, { planId: detail.facts.planId })}
                    </p>
                    <div className="vua-page__actions">
                      <Badge tone={recordStatusTone(detail.facts.status)}>
                        {copy.status[detail.facts.status]}
                      </Badge>
                      {detail.facts.status === "recovered" ? (
                        <Badge tone="neutral">{copy.recoveredBadge}</Badge>
                      ) : null}
                    </div>
                    {detail.facts.status === "recovered" ? (
                      <p className="vua-caption vua-text-secondary" role="note">
                        {copy.recoveredNote}
                      </p>
                    ) : null}
                    <dl className="vua-release-detail">
                      <div className="vua-release-detail__row">
                        <dt className="vua-caption vua-text-secondary">{copy.startedAt}</dt>
                        <dd>{formatDateTime(detail.facts.startedAt)}</dd>
                      </div>
                      <div className="vua-release-detail__row">
                        <dt className="vua-caption vua-text-secondary">{copy.jobsTitle}</dt>
                        <dd>
                          {format(copy.jobsLine, {
                            total: detail.facts.jobCounts.total,
                            succeeded: detail.facts.jobCounts.succeeded,
                            failed: detail.facts.jobCounts.failed,
                            rejected: detail.facts.jobCounts.rejected,
                          })}
                        </dd>
                      </div>
                      <div className="vua-release-detail__row">
                        <dt className="vua-caption vua-text-secondary">{copy.deviationsTitle}</dt>
                        <dd>{format(copy.deviationsLine, { count: detail.facts.planDeviationCount })}</dd>
                      </div>
                      <div className="vua-release-detail__row">
                        <dt className="vua-caption vua-text-secondary">{copy.evidenceTitle}</dt>
                        <dd>
                          {detail.facts.evidenceIdCount === null
                            ? copy.evidenceAbsent
                            : format(copy.evidenceLine, { count: detail.facts.evidenceIdCount })}
                        </dd>
                      </div>
                    </dl>
                    {/* 交棒准入呈现区(U19):状态白名单桶挂交棒面板,其余桶
                     *  呈现禁用原因＋入口链;独立「在 Unity 中打开以检查/
                     *  修复」动作与桶无关一律挂载(U19 第二交付,与交棒显
                     *  式分离);key=buildId 切换行时重置两面板状态 */}
                    <HandoffAdmissionArea
                      key={detail.facts.buildId}
                      facts={detail.facts}
                      onNavigate={onNavigate}
                    />
                    <ProjectOpenPanel
                      key={`open-${detail.facts.buildId}`}
                      buildId={detail.facts.buildId}
                    />
                  </div>
                )}
              </section>
            ) : null}
          </>
        )}
      </div>
    </Card>
  );
}
