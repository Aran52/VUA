/** Quitting while a task runs asks first (plan slice 28). The check only reads the task list, so every
 * provider keeps taking calls when the player chooses to stay; shutdown starts only after "stop and quit". */
import { APPLICATION_CONTRACT_VERSION, isTerminalTaskStateV01, type ApplicationRequestV01, type ApplicationResponseV01, type TaskSnapshotV01 } from "@vua/contracts";

export type QuitChoice = "stop" | "tray" | "cancel";
/** Button order in the question; "keep working in the tray" is the default. */
export const QUIT_CHOICES: readonly QuitChoice[] = ["stop", "tray", "cancel"];

export const quitCopy = {
  en: {
    message: "VUA is still working on a task.",
    detail: "Quitting stops it. Avatar work keeps its project and materials and continues from a new check.",
    stop: "Stop and quit", tray: "Keep working in the tray", cancel: "Cancel",
  },
  "zh-CN": {
    message: "VUA 还有任务在运行。",
    detail: "退出会停止它。Avatar 的工作工程和素材会保留，重新检查后即可继续。",
    stop: "停止并退出", tray: "在托盘中继续", cancel: "取消",
  },
  "zh-TW": {
    message: "VUA 還有任務在執行。",
    detail: "退出會停止它。Avatar 的工作專案和素材會保留，重新檢查後即可繼續。",
    stop: "停止並退出", tray: "在系統匣中繼續", cancel: "取消",
  },
  ja: {
    message: "VUA でタスクを実行中です。",
    detail: "終了するとタスクは止まります。アバターの作業プロジェクトと素材は残り、再チェックから続けられます。",
    stop: "停止して終了", tray: "トレイで続ける", cancel: "キャンセル",
  },
  ko: {
    message: "VUA에서 작업이 진행 중이에요.",
    detail: "종료하면 작업이 멈춰요. 아바타 작업 프로젝트와 자료는 남고, 다시 확인하면 이어갈 수 있어요.",
    stop: "멈추고 종료", tray: "트레이에서 계속", cancel: "취소",
  },
} as const;

export function quitStrings(locale: string | null | undefined): typeof quitCopy[keyof typeof quitCopy] {
  if (locale && Object.hasOwn(quitCopy, locale)) return quitCopy[locale as keyof typeof quitCopy];
  const language = (locale ?? "").toLowerCase();
  return /^zh-(hant|tw|hk|mo)(-|$)/.test(language) ? quitCopy["zh-TW"] : language.startsWith("zh") ? quitCopy["zh-CN"] : language.startsWith("ja") ? quitCopy.ja : language.startsWith("ko") ? quitCopy.ko : quitCopy.en;
}

/** Tasks that have not reached a final state; a provider that does not answer in time counts none. */
export async function runningTaskCount(
  provider: { invoke(request: ApplicationRequestV01): Promise<ApplicationResponseV01> },
  timeoutMs = 2_000,
): Promise<number> {
  const request: ApplicationRequestV01 = { contractVersion: APPLICATION_CONTRACT_VERSION, requestId: crypto.randomUUID(),
    correlationId: crypto.randomUUID(), kind: "query", method: "task.list", params: {} };
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise<null>(resolve => { timer = setTimeout(() => resolve(null), timeoutMs); });
  const answer = await Promise.race([provider.invoke(request).catch(() => null), late]);
  if (timer !== undefined) clearTimeout(timer);
  if (!answer?.ok || answer.value === null || typeof answer.value !== "object" || !("tasks" in answer.value)) return 0;
  return (answer.value.tasks as readonly TaskSnapshotV01[]).filter(task => !isTerminalTaskStateV01(task.state)).length;
}
