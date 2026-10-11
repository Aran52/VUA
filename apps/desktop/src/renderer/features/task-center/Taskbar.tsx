import { useState } from "react";
import { format, strings } from "../../i18n/index.ts";
import type { PageId } from "../../app/nav-model.ts";
import type { TaskItem } from "../../gateway/index.ts";
import { NotificationList } from "./NotificationList.tsx";
import { isActiveStatus, taskRowOpenTarget } from "./notification-model.ts";
import { useNotificationCenter } from "./use-notification-center.ts";

const copy = strings.taskCenter;

/**
 * 任务中心(ui-ux-standard §4.4 任务栏,§7.4 九态):
 * 折叠条显示运行中任务摘要,点击展开列表浮层;列表提供"回到来源页"
 * 回跳与取消意图。状态事实来源在应用层(TaskPort),本组件只是视图:
 * 取消由端口裁决 ok / rejected,rejected 时行内提示,不自行推断。
 *
 * 入口显隐(§2.6):capability 非 ready 时整条不出现(而非禁用)。
 * 通知投影与意图转发与顶栏铃铛共用 useNotificationCenter / NotificationList。
 */
export function Taskbar({ navigate }: { navigate: (target: PageId) => void }) {
  const center = useNotificationCenter();
  const [expanded, setExpanded] = useState(false);

  // §2.6:任务引擎未接入(unavailable)时入口整条不出现
  if (center.capability?.state !== "ready") return null;

  const firstActive = center.tasks.find((task) => isActiveStatus(task.status));
  const summary =
    center.activeCount > 0 && firstActive
      ? format(copy.runningSummary, {
          title: firstActive.title,
          count: center.activeCount,
        })
      : copy.idleSummary;

  return (
    <div className="vua-shell__taskbar">
      {expanded && (center.notifications.length > 0 || center.replay) ? (
        <div className="vua-taskbar__panel" role="region" aria-label={copy.title}>
          <NotificationList
            notifications={center.notifications}
            rejectedId={center.rejectedId}
            showCompleted={center.showCompleted}
            onShowCompletedChange={center.setShowCompleted}
            onCancel={(id) => void center.cancel(id)}
            onRetry={(id) => void center.retry(id)}
            dismissible={center.dismissible}
            onBackToOrigin={(task: TaskItem) => {
              setExpanded(false);
              // 行打开语义经模型函数(D2):活动/终态两态一致回来源页
              navigate(taskRowOpenTarget(task));
            }}
            emptyText={null}
            replay={center.replay}
          />
        </div>
      ) : null}
      <button
        type="button"
        className="vua-taskbar__toggle"
        aria-expanded={expanded}
        aria-label={expanded ? copy.collapseAria : format(copy.expandAria, { count: center.tasks.length })}
        onClick={() => setExpanded((value) => !value)}
      >
        <span className="vua-taskbar__summary">{summary}</span>
      </button>
    </div>
  );
}
