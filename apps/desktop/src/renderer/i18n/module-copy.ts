const en = {
  title: "Modules", choose: "Choose how to create your Avatar", amf: "{amf}",
  enable: "Enable {amf}", disable: "Disable {amf}", retry: "Retry startup", open: "Open asset library",
  retained: "Materials, recipes and projects stay. Running tasks must finish or be cancelled first.",
  absent: "Not enabled", starting: "Starting…", ready: "Enabled", failed: "{amf} could not start. Play is still available.", stopping: "Stopping…",
  busy: "{amf} has running tasks. Finish or cancel them, then retry.", changeFailed: "Not completed; data kept.",
  wait: "One moment…", manage: "Manage modules", loading: "Opening {amf}…", developing: "In development",
  disableConfirm: "Disable {amf}?",
};
type Copy = { [Key in keyof typeof en]: string };
export const moduleCopy: Record<"en" | "zh-CN" | "ja" | "ko", Copy> = {
  en,
  "zh-CN": {
    title: "功能模块", choose: "想用哪种方式改模？", amf: "{amf}",
    enable: "启用 {amf}", disable: "停用 {amf}", retry: "重试启动", open: "打开素材库",
    retained: "素材、配方和项目将保留，进行中的任务需先完成或取消。",
    absent: "尚未启用", starting: "正在启动…", ready: "已启用", failed: "{amf} 启动失败，游玩仍可使用。", stopping: "正在停止…",
    busy: "{amf} 有任务进行中，完成或取消后重试。", changeFailed: "操作未完成，数据已保留。",
    wait: "稍候…", manage: "管理功能模块", loading: "正在打开 {amf}…", developing: "开发中",
    disableConfirm: "停用 {amf}？",
  },
  ja: {
    title: "機能モジュール", choose: "どの方法で Avatar を編集しますか？", amf: "{amf}",
    enable: "{amf} を有効にする", disable: "{amf} を無効にする", retry: "起動を再試行", open: "素材ライブラリを開く",
    retained: "素材、レシピ、プロジェクトは残ります。実行中のタスクは先に完了かキャンセルが必要です。",
    absent: "未有効", starting: "起動中…", ready: "有効", failed: "{amf} を起動できませんでした。プレイは利用できます。", stopping: "停止中…",
    busy: "{amf} に実行中のタスクがあります。完了かキャンセル後に再試行してください。", changeFailed: "完了しませんでした。データは保持されています。",
    wait: "お待ちください…", manage: "モジュールを管理", loading: "{amf} を開いています…", developing: "開発中",
    disableConfirm: "{amf} を無効にしますか？",
  },
  ko: {
    title: "기능 모듈", choose: "어떤 방식으로 Avatar를 편집할까요?", amf: "{amf}",
    enable: "{amf} 켜기", disable: "{amf} 끄기", retry: "시작 다시 시도", open: "에셋 라이브러리 열기",
    retained: "에셋, 레시피, 프로젝트는 남아요. 진행 중인 작업은 먼저 끝내거나 취소해야 해요.",
    absent: "꺼짐", starting: "시작 중…", ready: "켜짐", failed: "{amf}를 시작하지 못했습니다. 플레이는 계속 사용할 수 있습니다.", stopping: "중지 중…",
    busy: "{amf}에 진행 중인 작업이 있어요. 끝내거나 취소한 뒤 다시 시도하세요.", changeFailed: "완료되지 않았어요. 데이터는 유지돼요.",
    wait: "잠시만요…", manage: "모듈 관리", loading: "{amf} 여는 중…", developing: "개발 중",
    disableConfirm: "{amf}를 끌까요?",
  },
};
