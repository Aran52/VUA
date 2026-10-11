const en = {
  drop: "Drop to import local assets", title: "Import local assets", cancel: "Cancel",
  needsAmf: "The built-in asset library needs {amf}. Enable it to import these files or folders. Your originals will stay where they are.",
  enable: "Enable {amf} and import", importing: "Starting import…", retry: "Try again", continue: "Import",
  failed: "The import could not start. Check that the files are still available, then try again.",
  unavailable: "File import is unavailable here. Reopen VUA to use its built-in asset library.",
  limit: "Drop up to 200 files or folders at a time.", busy: "Please wait for the current action to finish, then drop the files again.",
};
type Copy = { [K in keyof typeof en]: string };
export const fileDropCopy: Record<"en" | "zh-CN" | "ja" | "ko", Copy> = {
  en,
  "zh-CN": {
    drop: "松开鼠标，导入本地素材", title: "导入本地素材", cancel: "取消",
    needsAmf: "内置素材管理需要启用 {amf}。启用后就能导入这次拖入的文件或文件夹，原文件会保留在原处。",
    enable: "启用 {amf} 并导入", importing: "正在开始导入…", retry: "重试", continue: "导入",
    failed: "没能开始导入。请确认文件仍然可用，然后重试。",
    unavailable: "这里暂时无法导入文件，请重新打开 VUA 后使用内置素材管理。",
    limit: "一次最多拖入 200 个文件或文件夹。", busy: "请等当前操作完成，再拖入文件。",
  },
  ja: {
    drop: "ドロップしてローカル素材を取り込む", title: "ローカル素材の取り込み", cancel: "キャンセル",
    needsAmf: "内蔵の素材管理には {amf} が必要です。有効にすると、ドロップしたファイルやフォルダーを取り込めます。元のファイルはそのまま残ります。",
    enable: "{amf} を有効にして取り込む", importing: "取り込みを開始中…", retry: "再試行", continue: "取り込む",
    failed: "取り込みを開始できませんでした。ファイルが利用可能か確認して、再試行してください。",
    unavailable: "ここではファイルを取り込めません。VUA を開き直して内蔵の素材管理を使用してください。",
    limit: "一度にドロップできるのは 200 個のファイルまたはフォルダーまでです。", busy: "現在の操作が終わってから、もう一度ドロップしてください。",
  },
  ko: {
    drop: "놓아서 로컬 에셋 가져오기", title: "로컬 에셋 가져오기", cancel: "취소",
    needsAmf: "내장 에셋 관리에는 {amf}가 필요해요. 활성화하면 끌어온 파일이나 폴더를 가져올 수 있어요. 원본 파일은 그대로 보관돼요.",
    enable: "{amf} 활성화 후 가져오기", importing: "가져오기 시작 중…", retry: "다시 시도", continue: "가져오기",
    failed: "가져오기를 시작하지 못했어요. 파일을 사용할 수 있는지 확인한 후 다시 시도해 주세요.",
    unavailable: "여기서는 파일을 가져올 수 없어요. VUA를 다시 열어 내장 에셋 관리를 사용해 주세요.",
    limit: "한 번에 최대 200개의 파일이나 폴더를 끌어올 수 있어요.", busy: "현재 작업이 끝난 후 파일을 다시 끌어와 주세요.",
  },
};
