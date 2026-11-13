/** Local shell copy. IPC accepts a locale identifier, never caller-provided dialog text. */
export const dialogCopy = {
  en: { libraryThumbnail: "Select a thumbnail", unityPackage: "Choose material folder", localVpm: "Select local VPM package folder", warehouse: "Import material folders", editorExecutable: "Select Unity Editor executable", editorDirectory: "Select Unity Editor folder" },
  "zh-CN": { libraryThumbnail: "选择缩略图", unityPackage: "选择素材文件夹", localVpm: "选择本地 VPM 插件包文件夹", warehouse: "导入素材文件夹", editorExecutable: "选择 Unity 编辑器程序", editorDirectory: "选择 Unity 编辑器文件夹" },
  "zh-TW": { libraryThumbnail: "選擇縮圖", unityPackage: "選擇素材資料夾", localVpm: "選擇本機 VPM 套件資料夾", warehouse: "匯入素材資料夾", editorExecutable: "選擇 Unity 編輯器程式", editorDirectory: "選擇 Unity 編輯器資料夾" },
  ja: { libraryThumbnail: "サムネイルを選択", unityPackage: "素材フォルダーを選択", localVpm: "ローカル VPM パッケージのフォルダーを選択", warehouse: "素材フォルダーをインポート", editorExecutable: "Unity エディターの実行ファイルを選択", editorDirectory: "Unity エディターのフォルダーを選択" },
  ko: { libraryThumbnail: "썸네일 선택", unityPackage: "소재 폴더 선택", localVpm: "로컬 VPM 패키지 폴더 선택", warehouse: "소재 폴더 가져오기", editorExecutable: "Unity 에디터 실행 파일 선택", editorDirectory: "Unity 에디터 폴더 선택" },
} as const;

/** Startup has no renderer-selected locale yet; use the operating-system language. */
export function startupFailureCopy(locale: string): string {
  const language = locale.toLowerCase();
  if (/^zh-(hant|tw|hk|mo)(-|$)/.test(language)) return "VUA 無法啟動。請將整個程式 ZIP 壓縮檔完整解壓縮後再試一次。";
  if (language.startsWith("zh")) return "VUA 无法启动。请将整个程序 ZIP 压缩包完整解压后再试一次。";
  if (language.startsWith("ja")) return "VUA を起動できませんでした。アプリの ZIP ファイルをすべて展開し直してから、もう一度お試しください。";
  if (language.startsWith("ko")) return "VUA를 실행하지 못했어요. 앱 ZIP 파일 전체의 압축을 다시 푼 뒤 다시 시도해 주세요.";
  return "VUA couldn't start. Extract the entire application ZIP archive again and try opening it once more.";
}

export function dialogStrings(locale: unknown): typeof dialogCopy[keyof typeof dialogCopy] {
  return typeof locale === "string" && Object.hasOwn(dialogCopy, locale)
    ? dialogCopy[locale as keyof typeof dialogCopy]
    : dialogCopy.en;
}
