import { currentLocale } from "./current-table.ts";
import type { LocaleId } from "./locales.ts";

const themeNames = {
  grid: "Grid",
  aero: "Frutiger Aero",
  tenshi: "Tenshi Kaiwai",
  jirai: "Jirai Kei",
} as const;

const en = {
  workshop: { viewing: "Viewing: {step}", executing: "Running: {step}", idle: "No step is running", materials: "{count} materials",
    unknownAvatar: "Avatar awaiting inspection", details: "Material names and line details", lineId: "Line ID" },
  settings: { group: "Display & performance", family: "Theme style", familyHint: "Grid and Aero offer both modes; Tenshi Kaiwai is light and Jirai Kei is dark.", ...themeNames, appearanceHint: "Choose the appearance used throughout VUA.", fixedAppearanceHint: "This preset uses one appearance. Change theme style to choose another mode.", contrastHint: "Follow Windows or keep high contrast on." },
  repositories: { source: "Source", count: "Packages", actions: "Actions", use: "Use", uncached: "Not cached" },
  home: { continue: "Continue current work", environment: "Observed environment", utilities: "App & help" },
};
const copy: Partial<Record<LocaleId, typeof en>> = {
  en,
  "zh-CN": {
    workshop: { viewing: "正在查看：{step}", executing: "当前执行：{step}", idle: "当前没有步骤在执行", materials: "{count} 个素材",
      unknownAvatar: "Avatar 待识别", details: "素材名称与产线详情", lineId: "产线 ID" },
    settings: { group: "显示与性能", family: "主题风格", familyHint: "Grid 与 Aero 支持深浅色；Tenshi Kaiwai 为浅色，Jirai Kei 为深色。", ...themeNames, appearanceHint: "选择整个 VUA 使用的外观。", fixedAppearanceHint: "此预设固定外观；切换主题风格后可选择其他模式。", contrastHint: "跟随 Windows，或始终使用高对比度。" },
    repositories: { source: "来源", count: "包数量", actions: "操作", use: "启用", uncached: "未缓存" },
    home: { continue: "继续当前任务", environment: "已观察到的环境", utilities: "应用与帮助" },
  },
  "zh-TW": {
    workshop: { viewing: "正在檢視：{step}", executing: "目前執行：{step}", idle: "目前沒有步驟正在執行", materials: "{count} 個素材",
      unknownAvatar: "Avatar 待辨識", details: "素材名稱與產線詳情", lineId: "產線 ID" },
    settings: { group: "顯示與效能", family: "主題風格", familyHint: "Grid 與 Aero 支援深淺色；Tenshi Kaiwai 為淺色，Jirai Kei 為深色。", ...themeNames, appearanceHint: "選擇整個 VUA 使用的外觀。", fixedAppearanceHint: "此預設固定外觀；切換主題風格後可選擇其他模式。", contrastHint: "跟隨 Windows，或一律使用高對比。" },
    repositories: { source: "來源", count: "套件數量", actions: "操作", use: "啟用", uncached: "尚未快取" },
    home: { continue: "繼續目前工作", environment: "已觀察到的環境", utilities: "應用程式與說明" },
  },
  ja: {
    workshop: { viewing: "表示中：{step}", executing: "実行中：{step}", idle: "実行中の工程はありません", materials: "素材 {count} 件",
      unknownAvatar: "アバターの確認待ち", details: "素材名とラインの詳細", lineId: "ライン ID" },
    settings: { group: "表示とパフォーマンス", family: "テーマスタイル", familyHint: "Grid と Aero はダーク／ライト対応、Tenshi Kaiwai はライト、Jirai Kei はダークです。", ...themeNames, appearanceHint: "VUA 全体で使う外観を選びます。", fixedAppearanceHint: "このプリセットは外観固定です。別のモードはテーマスタイルを変更してください。", contrastHint: "Windows に従うか、高コントラストを常に使います。" },
    repositories: { source: "取得元", count: "パッケージ数", actions: "操作", use: "使用", uncached: "未取得" },
    home: { continue: "現在の作業を続ける", environment: "確認済みの環境", utilities: "アプリとヘルプ" },
  },
  ko: {
    workshop: { viewing: "보는 단계: {step}", executing: "실행 중: {step}", idle: "실행 중인 단계가 없습니다", materials: "소재 {count}개",
      unknownAvatar: "Avatar 확인 대기", details: "소재 이름 및 라인 세부 정보", lineId: "라인 ID" },
    settings: { group: "화면 및 성능", family: "테마 스타일", familyHint: "Grid와 Aero는 다크/라이트를 지원합니다. Tenshi Kaiwai는 라이트, Jirai Kei는 다크입니다.", ...themeNames, appearanceHint: "VUA 전체에 사용할 테마를 선택합니다.", fixedAppearanceHint: "고정 모드 프리셋입니다. 다른 모드는 테마 스타일을 변경하세요.", contrastHint: "Windows 설정을 따르거나 항상 고대비를 사용합니다." },
    repositories: { source: "출처", count: "패키지 수", actions: "작업", use: "사용", uncached: "캐시 없음" },
    home: { continue: "현재 작업 계속", environment: "관찰된 환경", utilities: "앱 및 도움말" },
  },
};

export const presentationCopy = copy[currentLocale] ?? en;
