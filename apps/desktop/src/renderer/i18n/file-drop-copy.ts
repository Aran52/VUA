const en = {
  drop: "Drop here to import local materials", title: "Import local materials", cancel: "Cancel",
  needsAmf: "Enable {amf} to import materials",
  enable: "Enable {amf} & import", importing: "Starting import…", retry: "Try again", continue: "Import",
  failed: "Couldn't start import",
  unavailable: "Import is unavailable right now. Restart VUA and try again.",
  limit: "Up to 200 files or folders at once", busy: "Wait for the current action to finish before dropping files",
};
type Copy = { [K in keyof typeof en]: string };
export const fileDropCopy: Record<"en" | "zh-CN" | "zh-TW" | "ja" | "ko", Copy> = {
  en,
  "zh-CN": {
    drop: "松开鼠标即可导入本地素材", title: "导入本地素材", cancel: "取消",
    needsAmf: "导入素材需要先启用 {amf}",
    enable: "启用 {amf} 并导入", importing: "正在启动导入…", retry: "重试", continue: "开始导入",
    failed: "导入未能启动",
    unavailable: "当前无法导入，请重启 VUA 后再试。",
    limit: "单次最多拖入 200 个文件或文件夹", busy: "请等当前操作完成后再拖入文件",
  },
  "zh-TW": {
    drop: "放開滑鼠即可匯入本機素材", title: "匯入本機素材", cancel: "取消",
    needsAmf: "匯入素材需先啟用 {amf}",
    enable: "啟用 {amf} 並匯入", importing: "正在開始匯入…", retry: "重試", continue: "開始匯入",
    failed: "無法開始匯入",
    unavailable: "目前無法匯入，請重新啟動 VUA 後再試。",
    limit: "單次最多可拖入 200 個檔案或資料夾", busy: "請等目前操作完成後再拖入檔案",
  },
  ja: {
    drop: "ここにドロップしてローカル素材を取り込む", title: "ローカル素材の取り込み", cancel: "キャンセル",
    needsAmf: "素材の取り込みには {amf} の有効化が必要です",
    enable: "{amf} を有効にして取り込む", importing: "取り込みを開始中…", retry: "再試行", continue: "取り込む",
    failed: "取り込みを開始できませんでした",
    unavailable: "現在インポートできません。VUA を再起動してからお試しください。",
    limit: "一度に最大 200 個のファイル・フォルダーまで対応", busy: "実行中の操作が終わってからドロップしてください",
  },
  ko: {
    drop: "여기에 놓아 로컬 소재 가져오기", title: "로컬 소재 가져오기", cancel: "취소",
    needsAmf: "소재를 가져오려면 {amf}를 켜야 해요",
    enable: "{amf} 켜고 가져오기", importing: "가져오기 시작 중…", retry: "다시 시도", continue: "가져오기",
    failed: "가져오기를 시작하지 못했어요",
    unavailable: "지금은 가져올 수 없어요. VUA를 다시 켠 뒤 시도해 주세요.",
    limit: "한 번에 파일·폴더 최대 200개까지 가능", busy: "진행 중인 작업이 끝난 뒤에 끌어다 놓아 주세요",
  },
};
