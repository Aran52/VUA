/** Quitting while a task runs asks first (plan slice 28). The check only reads the task list, so every
 * provider keeps taking calls when the player chooses to stay; shutdown starts only after "stop and quit". */
import { APPLICATION_CONTRACT_VERSION, isTerminalTaskStateV01, type ApplicationRequestV01, type ApplicationResponseV01, type TaskSnapshotV01 } from "@vua/contracts";

export type QuitChoice = "stop" | "tray" | "cancel";
/** Button order in the question; "keep working in the tray" is the default. */
export const QUIT_CHOICES: readonly QuitChoice[] = ["stop", "tray", "cancel"];

export const quitCopy = {
  en: {
    message: "VUA is still working on a task.",
    detail: "Quitting now stops the task. Your Avatar work project and imported materials stay safe, and you can continue from a fresh check next time.",
    stop: "Stop and quit", tray: "Keep working in the tray", cancel: "Cancel",
  },
  "zh-CN": {
    message: "VUA 还有任务正在运行。",
    detail: "现在退出会中止当前任务。Avatar 的工作项目与已导入素材都会安全保留，下次重新检查后即可接着继续。",
    stop: "停止并退出", tray: "隐藏到托盘继续运行", cancel: "取消",
  },
  "zh-TW": {
    message: "VUA 還有工作正在執行。",
    detail: "現在結束會中止目前工作。Avatar 的工作專案與已匯入素材都會安全保留，下次重新檢查後即可接著繼續。",
    stop: "停止並結束", tray: "縮小至系統匣繼續執行", cancel: "取消",
  },
  ja: {
    message: "VUA で実行中のタスクがあります。",
    detail: "今終了すると処理が中断されます。アバターの作業プロジェクトと素材はそのまま残るため、次回は再チェックから再開できます。",
    stop: "停止して終了", tray: "トレイで続ける", cancel: "キャンセル",
  },
  ko: {
    message: "VUA에서 아직 진행 중인 작업이 있어요.",
    detail: "지금 종료하면 진행 중인 작업이 멈춰요. 아바타 작업 프로젝트와 소재는 그대로 유지되며, 다음에 다시 검사해 이어갈 수 있어요.",
    stop: "중지하고 종료", tray: "트레이에서 계속 실행", cancel: "취소",
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
