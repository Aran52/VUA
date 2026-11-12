const en = {
  assets: "Browse BOOTH", wiki: "Official VRChat Wiki", shortcuts: "Quick links",
  loading: "Loading page…", failed: "Couldn't load this page",
  unavailable: "Built-in browser unavailable. Please restart VUA.",
  retry: "Try again", reopen: "Open BOOTH",
  tabs: "Browser tabs", newTab: "New tab", closeTab: "Close {title}", tabLimit: "Up to 8 tabs open",
};
type Copy = { [K in keyof typeof en]: string };
export const browserCopy: Record<"en" | "zh-CN" | "zh-TW" | "ja" | "ko", Copy> = {
  en,
  "zh-CN": {
    assets: "浏览 BOOTH 素材", wiki: "VRChat 官方 Wiki", shortcuts: "常用网站",
    loading: "正在加载网页…", failed: "网页未能打开",
    unavailable: "内置浏览器暂不可用，请重启 VUA 后再试。",
    retry: "重试", reopen: "打开 BOOTH",
    tabs: "浏览器标签页", newTab: "新建标签页", closeTab: "关闭 {title}", tabLimit: "最多同时打开 8 个标签页",
  },
  "zh-TW": {
    assets: "瀏覽 BOOTH 素材", wiki: "VRChat 官方 Wiki", shortcuts: "常用網站捷徑",
    loading: "正在載入網頁…", failed: "無法開啟網站",
    unavailable: "內建瀏覽器目前無法使用，請重新啟動 VUA 後再試。",
    retry: "重試", reopen: "開啟 BOOTH",
    tabs: "瀏覽器分頁", newTab: "新增分頁", closeTab: "關閉 {title}", tabLimit: "最多同時開啟 8 個分頁",
  },
  ja: {
    assets: "BOOTH で素材を探す", wiki: "VRChat 公式 Wiki", shortcuts: "よく使うサイト",
    loading: "ページを読み込み中…", failed: "ページを開けませんでした",
    unavailable: "内蔵ブラウザーを利用できません。VUA を再起動してください。",
    retry: "再試行", reopen: "BOOTH を開く",
    tabs: "ブラウザータブ", newTab: "新しいタブ", closeTab: "{title} を閉じる", tabLimit: "タブは最大 8 個まで",
  },
  ko: {
    assets: "BOOTH 둘러보기", wiki: "VRChat 공식 Wiki", shortcuts: "자주 찾는 사이트",
    loading: "웹 페이지 불러오는 중…", failed: "사이트를 열지 못했어요",
    unavailable: "내장 브라우저를 사용할 수 없어요. VUA를 다시 시작해 주세요.",
    retry: "다시 시도", reopen: "BOOTH 열기",
    tabs: "브라우저 탭", newTab: "새 탭", closeTab: "{title} 닫기", tabLimit: "탭은 최대 8개까지 열 수 있어요",
  },
};
