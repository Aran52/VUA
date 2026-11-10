const en = {
  title: "Modules", choose: "Choose your avatar workflow", amf: "{amf}",
  enable: "Enable {amf}", disable: "Disable {amf}", retry: "Try starting again", open: "Open Materials",
  retained: "Your materials, recipes, and work projects stay intact. Finish or cancel any active tasks first.",
  absent: "Not enabled", starting: "Starting…", ready: "Enabled", failed: "{amf} couldn't start. Play features remain available.", stopping: "Stopping…",
  busy: "{amf} has active tasks running. Wait for them to finish or cancel them first.", changeFailed: "Couldn't change module state. Your data is untouched.",
  wait: "Just a moment…", manage: "Manage modules", loading: "Opening {amf}…", developing: "In development",
  disableConfirm: "Disable {amf}?",
};
type Copy = { [Key in keyof typeof en]: string };
export const moduleCopy: Record<"en" | "zh-CN" | "zh-TW" | "ja" | "ko", Copy> = {
  en,
  "zh-CN": {
    title: "功能模块", choose: "选择改模工作流", amf: "{amf}",
    enable: "启用 {amf}", disable: "停用 {amf}", retry: "重新尝试启动", open: "打开素材库",
    retained: "停用后素材、配方与改模工程都会保留；若有正在运行的任务，请先等它完成或手动取消。",
    absent: "尚未启用", starting: "正在启动…", ready: "已启用", failed: "{amf} 未能正常启动，游玩相关功能仍可照常使用。", stopping: "正在停止…",
    busy: "{amf} 当前有任务正在运行，请等任务结束或取消后再操作。", changeFailed: "状态切换未能完成，原有数据均已妥善保留。",
    wait: "请稍候…", manage: "管理功能模块", loading: "正在打开 {amf}…", developing: "开发中",
    disableConfirm: "确认停用 {amf}？",
  },
  "zh-TW": {
    title: "功能模組", choose: "選擇改模方式", amf: "{amf}",
    enable: "啟用 {amf}", disable: "停用 {amf}", retry: "重試啟動", open: "開啟素材庫",
    retained: "停用後素材、配方與專案皆會保留；若有進行中的任務，請先等待完成或取消。",
    absent: "尚未啟用", starting: "正在啟動…", ready: "已啟用", failed: "{amf} 無法啟動，遊玩相關功能仍可正常使用。", stopping: "正在停止…",
    busy: "{amf} 仍有進行中的任務，請於完成或取消後再試。", changeFailed: "操作未完成，所有資料均已保留。",
    wait: "請稍候…", manage: "管理功能模組", loading: "正在開啟 {amf}…", developing: "開發中",
    disableConfirm: "確定要停用 {amf}？",
  },
  ja: {
    title: "機能モジュール", choose: "アバター改変の方法を選択", amf: "{amf}",
    enable: "{amf} を有効にする", disable: "{amf} を無効にする", retry: "起動を再試行", open: "素材ライブラリを開く",
    retained: "無効にしても素材・レシピ・プロジェクトはそのまま残ります。実行中のタスクがある場合は先に完了または中止してください。",
    absent: "無効", starting: "起動中…", ready: "有効", failed: "{amf} を起動できませんでした。プレイ機能はそのまま利用できます。", stopping: "停止中…",
    busy: "{amf} で実行中のタスクがあります。完了または中止してからやり直してください。", changeFailed: "切り替えを完了できませんでした。データは保持されています。",
    wait: "少々お待ちください…", manage: "モジュール管理", loading: "{amf} を開いています…", developing: "開発中",
    disableConfirm: "{amf} を無効にしますか？",
  },
  ko: {
    title: "기능 모듈", choose: "아바타 제작 방식 선택", amf: "{amf}",
    enable: "{amf} 켜기", disable: "{amf} 끄기", retry: "다시 시작해 보기", open: "소재 라이브러리 열기",
    retained: "모듈을 꺼도 소재와 레시피, 작업 프로젝트는 그대로 남아요. 진행 중인 작업이 있다면 먼저 마치거나 취소해 주세요.",
    absent: "꺼짐", starting: "시작 중…", ready: "켜짐", failed: "{amf}를 시작하지 못했어요. 플레이 기능은 평소처럼 쓸 수 있어요.", stopping: "중지 중…",
    busy: "{amf}에서 진행 중인 작업이 있어요. 끝내거나 취소한 뒤에 다시 시도해 주세요.", changeFailed: "설정을 바꾸지 못했어요. 기존 데이터는 안전하게 유지돼요.",
    wait: "잠시만 기다려 주세요…", manage: "모듈 관리", loading: "{amf} 여는 중…", developing: "개발 중",
    disableConfirm: "{amf} 모듈을 끌까요?",
  },
};
