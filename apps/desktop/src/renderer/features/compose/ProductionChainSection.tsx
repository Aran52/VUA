import { formatDateTime } from "../../i18n/index.ts";
import { useCallback, useEffect, useState } from "react";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { EmptyState } from "../../components/primitives/EmptyState.tsx";
import type { PlanListEntryV02, RecordListEntryV02 } from "@vua/contracts";
import { useGateway, useTaskCenter } from "../../gateway/index.ts";
import { useComposeDraft } from "../../app/compose-draft-store.ts";
import {
  productionChainExecuteAcceptedAction,
  productionChainExecuteFailedAction,
  productionChainExecuteRequestedAction,
  productionChainRecordSeenAction,
  productionChainResolveAcceptedAction,
  productionChainResolveFailedAction,
  productionChainResolveRequestedAction,
  productionChainGate,
  useProductionChain,
} from "../../app/production-chain-store.ts";
import { chainRecordsForPlan, planRowsForDisplay } from "./production-chain-model.ts";
import { format, strings } from "../../i18n/index.ts";

/**
 * 生产链段(019 批 C,需求 §5 Production 分组;UI-04 主流程延续):
 * 保存配方后推进——解析 → 计划 → 批准 → 装配 → 记录。状态与身份在共享
 * 容器层(app/production-chain-store),本组件只是现有 UI 的呈现/操作面
 * (两套 UI 消费同一 store 与同一 Gateway 端口;029 A5 起配方页选中态
 * 双挂载同一链段——选择事实源动作把 recipe.get 回执文档身份写入链)。
 *
 * - AC-05:搭配草稿在场且内容偏离已保存修订(stale-draft 闸门)时警示旧
 *   授权并禁用推进——解析应基于保存后的文档;服务端版本锁守卫独立拒绝;
 *   草稿不在场时警示不成立(029 A4 判决:选择驱动的链无草稿在场即 ready);
 * - AC-06:审批/执行进行中禁用重复提交;失败如实呈现,不自动重试;
 * - AC-07:受理后展示任务身份与任务中心权威状态;取消经任务中心
 *   (Gateway);本组件无计时器,不存在本地计时成功跳转;
 * - AC-13:记录按本链执行计划身份匹配呈现——不用固定历史示例;检测证据
 *   面尚未接入,如实说明,不以演示替代(UI-08/批 C 验收标准);
 * - 「已受理」不显示为「执行成功」(UI-06)。计划/记录列表是查询结果缓存
 *   (UI-02),按需重拉不入容器层。
 */
const copy = strings.compose.chain;

/** 任务中心权威状态行(AC-07:快照驱动,不本地推断)。029 A6 起车间执行
 *  状态面复用本行(任务中心同源事实,同一呈现,不另起第二呈现系)。 */
export function ChainTaskLine({ taskId, missingNote }: { taskId: string; missingNote: string }) {
  const taskCenter = useTaskCenter();
  const task = taskCenter.tasks.find((item) => item.id === taskId);
  return (
    <p className="vua-caption vua-text-secondary" role="status">
      {task !== undefined
        ? `${format(copy.taskLine, { taskId })} · ${strings.taskStatus[task.status]}${
            task.cancellable ? ` · ${copy.taskCancelHint}` : ""
          }`
        : `${format(copy.taskLine, { taskId })} · ${missingNote}`}
    </p>
  );
}

export function ProductionChainSection() {
  const draft = useComposeDraft();
  const chain = useProductionChain();
  const gateway = useGateway();

  const gate = productionChainGate(chain, {
    present: draft.items.length > 0,
    dirty: draft.dirty,
  });
  const stale = gate.kind === "stale-draft";

  // 计划列表(查询缓存:按需重拉,不入容器层——UI-02 缓存只是事实副本)
  const [plans, setPlans] = useState<readonly PlanListEntryV02[] | null>(null);
  const [plansFailed, setPlansFailed] = useState(false);
  const [plansLoading, setPlansLoading] = useState(false);
  // 装配记录(按本链执行计划身份过滤后的结果;AC-13)
  const [records, setRecords] = useState<readonly RecordListEntryV02[] | null>(null);
  const [recordsFailed, setRecordsFailed] = useState(false);
  const [recordsLoading, setRecordsLoading] = useState(false);
  // 批准进行中的计划(防重复提交:AC-06)
  const [approvingPlanId, setApprovingPlanId] = useState<string | null>(null);
  const [approveFailed, setApproveFailed] = useState(false);

  const refreshPlans = useCallback(() => {
    const recipeId = chain.recipe?.recipeId;
    if (recipeId === undefined) return;
    setPlansLoading(true);
    setPlansFailed(false);
    void gateway.productionChain
      .listPlans({ recipeId })
      .then((result) => {
        // null = 提供方响应不可解释/不可用:如实失败态,不折叠为空列表
        setPlans(result === null ? [] : planRowsForDisplay(result.entries));
        setPlansFailed(result === null);
      })
      .catch(() => setPlansFailed(true))
      .finally(() => setPlansLoading(false));
  }, [gateway, chain.recipe?.recipeId]);

  const refreshRecords = useCallback(() => {
    const recipeId = chain.recipe?.recipeId;
    const executed = chain.execute;
    if (recipeId === undefined || executed.kind !== "accepted") return;
    setRecordsLoading(true);
    setRecordsFailed(false);
    void gateway.productionChain
      .listRecords({ recipeId })
      .then((result) => {
        if (result === null) {
          // 失败≠空:如实失败态(UI-08 不得把失败当作空列表)
          setRecords([]);
          setRecordsFailed(true);
          return;
        }
        const mine = chainRecordsForPlan(result.entries, executed.planId);
        setRecords(mine);
        // AC-13:登记本链发现的记录身份(幂等;供跨 UI 消费)
        if (mine[0] !== undefined) productionChainRecordSeenAction(mine[0].buildId);
      })
      .catch(() => setRecordsFailed(true))
      .finally(() => setRecordsLoading(false));
  }, [gateway, chain.recipe?.recipeId, chain.execute]);

  // 解析已受理后自动查询一次计划(此后手动刷新;不轮询不猜测)
  useEffect(() => {
    if (chain.resolve.kind === "accepted") refreshPlans();
  }, [chain.resolve.kind, refreshPlans]);

  // 装配已受理后自动查询一次记录(AC-13;此后手动刷新)
  useEffect(() => {
    if (chain.execute.kind === "accepted") refreshRecords();
  }, [chain.execute.kind, refreshRecords]);

  if (gate.kind === "no-recipe") return null;

  const requestResolve = () => {
    const recipe = chain.recipe;
    if (recipe === null || chain.resolve.kind === "requesting") return;
    productionChainResolveRequestedAction();
    void gateway.productionChain
      .resolveRecipe(recipe.recipeId, recipe.revision)
      .then((result) => {
        if (result === null) {
          productionChainResolveFailedAction();
          return;
        }
        productionChainResolveAcceptedAction(result.taskId, result.correlationId);
      })
      .catch(() => productionChainResolveFailedAction());
  };

  const approvePlan = (planId: string) => {
    if (approvingPlanId !== null) return;
    setApprovingPlanId(planId);
    setApproveFailed(false);
    void gateway.productionChain
      .approvePlan(planId)
      .then((result) => {
        // draft→approved 幂等;null=不可解释,如实失败
        if (result === null) setApproveFailed(true);
        else refreshPlans();
      })
      .catch(() => setApproveFailed(true))
      .finally(() => setApprovingPlanId(null));
  };

  const executePlan = (planId: string) => {
    if (chain.execute.kind === "requesting") return;
    productionChainExecuteRequestedAction();
    void gateway.productionChain
      .executeJob(planId)
      .then((result) => {
        if (result === null) {
          productionChainExecuteFailedAction();
          return;
        }
        productionChainExecuteAcceptedAction(result.taskId, result.correlationId, planId);
      })
      .catch(() => productionChainExecuteFailedAction());
  };

  const resolveBlocked = stale || chain.resolve.kind === "requesting";
  const executed = chain.execute.kind === "accepted" ? chain.execute : null;

  return (
    <Card>
      <div className="vua-page__stack">
        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.title}</h3>
          <p className="vua-caption vua-text-secondary" role="status">
            {format(copy.recipeLine, {
              recipeId: chain.recipe?.recipeId ?? "",
              revision: String(chain.recipe?.revision ?? 0),
            })}
          </p>
          {stale ? (
            <p className="vua-caption vua-text-secondary" role="alert">
              {copy.staleWarning}
            </p>
          ) : null}
        </section>

        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.resolveTitle}</h3>
          <div className="vua-project-compat__row">
            <Button variant="default" disabled={resolveBlocked} onClick={requestResolve}>
              {chain.resolve.kind === "requesting" ? copy.resolveRequesting : copy.resolveCta}
            </Button>
          </div>
          {chain.resolve.kind === "accepted" ? (
            <ChainTaskLine taskId={chain.resolve.taskId} missingNote={copy.taskMissingNote} />
          ) : null}
          {chain.resolve.kind === "failed" ? (
            <p className="vua-caption vua-text-secondary" role="alert">
              {copy.resolveFailedNote}
            </p>
          ) : null}
        </section>

        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.planTitle}</h3>
          <div className="vua-project-compat__row">
            <Button variant="default" disabled={stale || plansLoading} onClick={refreshPlans}>
              {plansLoading ? copy.planLoading : copy.planRefreshCta}
            </Button>
          </div>
          {plansFailed ? (
            <p className="vua-caption vua-text-secondary" role="alert">
              {copy.planFailedNote}
            </p>
          ) : null}
          {approveFailed ? (
            <p className="vua-caption vua-text-secondary" role="alert">
              {copy.planApproveFailedNote}
            </p>
          ) : null}
          {plans !== null && plans.length === 0 ? (
            <EmptyState title={copy.planEmptyTitle} description={copy.planEmptyDesc} />
          ) : null}
          {plans !== null && plans.length > 0 ? (
            <ul className="vua-project-compat__specs">
              {plans.map((plan) => (
                <li key={plan.planId}>
                  <span title={plan.planId}>{format(copy.planLine, { planId: plan.planId })}</span>{" "}
                  <Badge tone={plan.status === "approved" ? "success" : "neutral"}>
                    {copy.planStatus[plan.status]}
                  </Badge>{" "}
                  {plan.status === "draft" ? (
                    <Button
                      variant="default"
                      disabled={stale || approvingPlanId !== null}
                      onClick={() => approvePlan(plan.planId)}
                    >
                      {approvingPlanId === plan.planId ? copy.planApproving : copy.planApproveCta}
                    </Button>
                  ) : null}{" "}
                  {plan.status === "approved" ? (
                    <Button
                      variant="default"
                      disabled={stale || chain.execute.kind === "requesting"}
                      onClick={() => executePlan(plan.planId)}
                    >
                      {chain.execute.kind === "requesting" ? copy.executeRequesting : copy.executeCta}
                    </Button>
                  ) : null}
                </li>
              ))}
            </ul>
          ) : null}
        </section>

        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.executeTitle}</h3>
          {executed !== null ? (
            <ChainTaskLine taskId={executed.taskId} missingNote={copy.taskMissingNote} />
          ) : null}
          {chain.execute.kind === "failed" ? (
            <p className="vua-caption vua-text-secondary" role="alert">
              {copy.executeFailedNote}
            </p>
          ) : null}
          {executed === null && chain.execute.kind !== "failed" ? (
            <p className="vua-caption vua-text-secondary">{copy.executePendingNote}</p>
          ) : null}
        </section>

        <section>
          <h3 className="vua-warehouse-detail__section-title">{copy.recordTitle}</h3>
          <div className="vua-project-compat__row">
            <Button
              variant="default"
              disabled={executed === null || recordsLoading}
              onClick={refreshRecords}
            >
              {recordsLoading ? copy.recordLoading : copy.recordCta}
            </Button>
          </div>
          {executed === null ? (
            <p className="vua-caption vua-text-secondary">{copy.recordPendingNote}</p>
          ) : null}
          {recordsFailed ? (
            <p className="vua-caption vua-text-secondary" role="alert">
              {copy.recordFailedNote}
            </p>
          ) : null}
          {records !== null && records.length === 0 ? (
            <EmptyState title={copy.recordEmptyTitle} description={copy.recordEmptyDesc} />
          ) : null}
          {records !== null && records.length > 0 ? (
            <ul className="vua-project-compat__specs">
              {records.map((record) => (
                <li key={record.buildId}>
                  <span title={record.buildId}>
                    {format(copy.recordLine, {
                      buildId: record.buildId,
                      status: record.status,
                      at: formatDateTime(record.finishedAt),
                    })}
                  </span>
                </li>
              ))}
            </ul>
          ) : null}
        </section>
      </div>
    </Card>
  );
}
