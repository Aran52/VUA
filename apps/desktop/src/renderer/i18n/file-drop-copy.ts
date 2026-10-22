const en = {
  drop: "Drop to import local assets", title: "Import local assets", cancel: "Cancel",
  needsAmf: "Importing needs {amf}",
  enable: "Enable {amf} and import", importing: "Starting import…", retry: "Try again", continue: "Import",
  failed: "Import did not start",
  unavailable: "Import unavailable. Restart VUA and retry.",
  limit: "Up to 200 files or folders at once", busy: "Drop files after the current action",
};
type Copy = { [K in keyof typeof en]: string };
export const fileDropCopy: Record<"en" | "zh-CN" | "zh-TW" | "ja" | "ko", Copy> = {
  en,
  "zh-CN": {
    drop: "松开鼠标，导入本地素材", title: "导入本地素材", cancel: "取消",
    needsAmf: "导入需启用 {amf}",
    enable: "启用 {amf} 并导入", importing: "正在开始导入…", retry: "重试", continue: "导入",
    failed: "导入未能开始",
    unavailable: "当前无法导入，重启 VUA 后重试。",
    limit: "单次最多 200 个文件或文件夹", busy: "当前操作完成后可拖入",
  },
  "zh-TW": {
    drop: "鬆開滑鼠，匯入本地素材", title: "匯入本地素材", cancel: "取消",
    needsAmf: "匯入需啟用 {amf}",
    enable: "啟用 {amf} 並匯入", importing: "正在開始匯入…", retry: "重試", continue: "匯入",
    failed: "匯入未能開始",
    unavailable: "目前無法匯入，重啟 VUA 後重試。",
    limit: "單次最多 200 個檔案或資料夾", busy: "目前操作完成後可拖入",
  },
  ja: {
    drop: "ドロップしてローカル素材を取り込む", title: "ローカル素材の取り込み", cancel: "キャンセル",
    needsAmf: "インポートには {amf} が必要です",
    enable: "{amf} を有効にして取り込む", importing: "取り込みを開始中…", retry: "再試行", continue: "取り込む",
    failed: "インポートを開始できませんでした",
    unavailable: "現在インポートできません。VUA を再起動して再試行してください。",
    limit: "一度に最大 200 個のファイルかフォルダー", busy: "現在の操作の完了後にドロップできます",
  },
  ko: {
    drop: "놓아서 로컬 에셋 가져오기", title: "로컬 에셋 가져오기", cancel: "취소",
    needsAmf: "가져오려면 {amf}가 필요해요",
    enable: "{amf} 활성화 후 가져오기", importing: "가져오기 시작 중…", retry: "다시 시도", continue: "가져오기",
    failed: "가져오기를 시작하지 못했어요",
    unavailable: "지금은 가져올 수 없어요. VUA를 다시 시작한 뒤 시도하세요.",
    limit: "한 번에 최대 200개의 파일이나 폴더", busy: "현재 작업이 끝나면 놓을 수 있어요",
  },
};
