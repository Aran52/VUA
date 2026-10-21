/** Shared Steam-connection copy for the five non-VRCFT N2 tools; {tool} takes the card title. */
const en = {
  install: "Install in Steam", openApp: "Open {tool}", running: "Running", closeApp: "Close",
  unknown: "Couldn't check", checking: "Checking…",
  waitInstall: "Waiting for Steam", waitClose: "Closing…", waitOpen: "Opening…",
  recheck: "Check again",
  steamMissing: "This {tool} build installs and opens through Steam.", prepareSteam: "Prepare Steam",
  presence: { installed: "Steam installation found", missing: "{tool} not installed in Steam", incomplete: "{tool} install in Steam unfinished", unknown: "Couldn't read the Steam installation" },
  issues: { steam_missing: "Steam required.", not_installed: "Finish installing {tool} in Steam, then check again.", handoff_failed: "Steam did not open. Open it manually and retry.", start_timeout: "{tool} did not start. Check Steam and retry.", close_failed: "Could not close normally; quit from the {tool} window or tray.", close_timeout: "{tool} is still running. Quit it from its window or tray." },
  cancelWait: "Stop waiting", existing: "{tool} was already running outside VUA. Close it from its window or tray.",
};
type Copy<T> = { [K in keyof T]: T[K] extends string ? string : Copy<T[K]> };
export const toolConnectionCopy: Record<"en" | "zh-CN" | "ja" | "ko", Copy<typeof en>> = {
  en,
  "zh-CN": {
    install: "在 Steam 安装", openApp: "打开 {tool}", running: "运行中", closeApp: "关闭",
    unknown: "暂时无法检查", checking: "正在检查…",
    waitInstall: "等待 Steam 安装", waitClose: "正在关闭…", waitOpen: "正在打开…",
    recheck: "重新检查",
    steamMissing: "此版本的 {tool} 需通过 Steam 安装和打开。", prepareSteam: "准备 Steam",
    presence: { installed: "已找到 Steam 中的安装", missing: "Steam 中未安装 {tool}", incomplete: "Steam 中 {tool} 安装未完成", unknown: "暂时无法读取 Steam 中的安装" },
    issues: { steam_missing: "需要 Steam。", not_installed: "在 Steam 完成 {tool} 安装后重新检查。", handoff_failed: "Steam 未打开，手动打开后重试。", start_timeout: "{tool} 未启动，查看 Steam 后重试。", close_failed: "无法正常关闭，需从 {tool} 窗口或托盘退出。", close_timeout: "{tool} 仍在运行，需从其窗口或托盘退出。" },
    cancelWait: "停止等待", existing: "{tool} 已在 VUA 外运行，需从其窗口或托盘关闭。",
  },
  ja: {
    install: "Steam でインストール", openApp: "{tool} を開く", running: "実行中", closeApp: "閉じる",
    unknown: "確認できませんでした", checking: "確認中…",
    waitInstall: "Steam のインストール待ち", waitClose: "終了中…", waitOpen: "起動中…",
    recheck: "もう一度確認",
    steamMissing: "この版の {tool} は Steam でインストール・起動します。", prepareSteam: "Steam を準備",
    presence: { installed: "Steam のインストールを確認", missing: "Steam に {tool} が未インストール", incomplete: "Steam で {tool} のインストールが未完了", unknown: "Steam のインストールを読み取れません" },
    issues: { steam_missing: "Steam が必要です。", not_installed: "Steam で {tool} のインストールを完了してから再確認してください。", handoff_failed: "Steam を開けませんでした。手動で開いて再試行してください。", start_timeout: "{tool} が起動しませんでした。Steam を確認して再試行してください。", close_failed: "正常に閉じられません。{tool} のウィンドウかトレイから終了してください。", close_timeout: "{tool} はまだ実行中です。ウィンドウかトレイから終了してください。" },
    cancelWait: "待機をやめる", existing: "{tool} は VUA の外で実行中です。ウィンドウかトレイから閉じてください。",
  },
  ko: {
    install: "Steam에서 설치", openApp: "{tool} 열기", running: "실행 중", closeApp: "닫기",
    unknown: "확인하지 못했어요", checking: "확인 중…",
    waitInstall: "Steam 설치 대기 중", waitClose: "종료 중…", waitOpen: "시작 중…",
    recheck: "다시 확인",
    steamMissing: "이 버전의 {tool}은 Steam으로 설치하고 실행해요.", prepareSteam: "Steam 준비",
    presence: { installed: "Steam 설치 확인됨", missing: "Steam에 {tool} 미설치", incomplete: "Steam에서 {tool} 설치 미완료", unknown: "Steam 설치 정보를 읽지 못했어요" },
    issues: { steam_missing: "Steam이 필요해요.", not_installed: "Steam에서 {tool} 설치를 마친 뒤 다시 확인하세요.", handoff_failed: "Steam을 열지 못했어요. 직접 연 뒤 다시 시도하세요.", start_timeout: "{tool}이 시작되지 않았어요. Steam을 확인한 뒤 다시 시도하세요.", close_failed: "정상적으로 닫지 못했어요. {tool} 창이나 트레이에서 종료하세요.", close_timeout: "{tool}이 아직 실행 중이에요. 창이나 트레이에서 종료하세요." },
    cancelWait: "대기 중단", existing: "{tool}이 VUA 밖에서 실행 중이에요. 창이나 트레이에서 닫으세요.",
  },
};
