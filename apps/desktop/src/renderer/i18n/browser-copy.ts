const en = {
  assets: "Browse assets", wiki: "Official VRChat Wiki", shortcuts: "Website shortcuts",
  loading: "Loading website…", failed: "The site did not open",
  unavailable: "Built-in browser unavailable. Restart VUA.",
  retry: "Try again", reopen: "Open BOOTH",
  tabs: "Browser tabs", newTab: "New tab", closeTab: "Close {title}", tabLimit: "Up to 8 tabs",
};
type Copy = { [K in keyof typeof en]: string };
export const browserCopy: Record<"en" | "zh-CN" | "ja" | "ko", Copy> = {
  en,
  "zh-CN": {
    assets: "素材浏览", wiki: "VRChat 官方 Wiki", shortcuts: "网站快捷方式",
    loading: "正在加载网页…", failed: "网站未能打开",
    unavailable: "内嵌浏览器不可用，重启 VUA 后重试。",
    retry: "重试", reopen: "打开 BOOTH",
    tabs: "浏览器标签页", newTab: "新建标签页", closeTab: "关闭 {title}", tabLimit: "最多 8 个标签页",
  },
  ja: {
    assets: "素材を探す", wiki: "VRChat 公式 Wiki", shortcuts: "サイトのショートカット",
    loading: "サイトを読み込み中…", failed: "サイトを開けませんでした",
    unavailable: "内蔵ブラウザーは利用できません。VUA を再起動してください。",
    retry: "再試行", reopen: "BOOTH を開く",
    tabs: "ブラウザーのタブ", newTab: "新しいタブ", closeTab: "{title} を閉じる", tabLimit: "タブは最大 8 個",
  },
  ko: {
    assets: "에셋 둘러보기", wiki: "VRChat 공식 Wiki", shortcuts: "웹사이트 바로가기",
    loading: "웹사이트 불러오는 중…", failed: "사이트를 열지 못했어요",
    unavailable: "내장 브라우저를 쓸 수 없어요. VUA를 다시 시작하세요.",
    retry: "다시 시도", reopen: "BOOTH 열기",
    tabs: "브라우저 탭", newTab: "새 탭", closeTab: "{title} 닫기", tabLimit: "탭은 최대 8개",
  },
};
