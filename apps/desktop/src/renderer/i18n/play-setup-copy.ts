/** The Launch page's words: choosing a platform, device and connection, and the steps that follow. */
const en = {
  platform: "Platform", device: "Device", connection: "Connection", detected: "Detected", official: "Official page",
  noPc: "No PC",
  stepsTitle: "Steps", ready: "All software ready", inHeadset: "Runs on the headset", changeHint: "Change platform, device or connection",
  officialSource: "Steps from official guides, November 2026",
  standaloneNote: "The headset shows Avatars that have a version for it. Others appear as a stand-in or a default figure.",
  actors: { vua: "VUA does it", open: "VUA opens it", player: "You do it" },
  steps: {
    steam: "Install Steam", "steam-sign-in": "Sign in to Steam", platform: "Install {platform}", launch: "Launch {platform}",
    "store-install": "Install VRChat from {store}", "store-launch": "Open VRChat in the headset",
  },
  stores: { pico: "the PICO Store", meta: "the Meta Horizon Store", viveport: "Viveport" },
  devices: {
    pc: { name: "PC mode", hint: "No headset needed" },
    pico: { name: "PICO", hint: "PICO 4 series" },
    quest: { name: "Meta Quest", hint: "Quest 2, 3, 3S, Pro" },
    "vive-standalone": { name: "VIVE standalone", hint: "Focus Vision, XR Elite" },
    "vive-wired": { name: "VIVE for PC", hint: "VIVE Pro, Cosmos" },
    "other-headset": { name: "Another model", hint: "Any headset SteamVR supports" },
    wmr: { name: "Windows MR", hint: "HP Reverb and others" },
  },
  connections: {
    "pico-connect": { tag: "Official · Free", summary: "PICO's own streaming, over a USB cable or Wi-Fi.", pc: "Install PICO Connect and SteamVR", headset: "Open PICO Connect in the headset and connect to the PC" },
    standalone: { tag: "Headset version", summary: "VRChat runs on the headset itself, with no computer involved." },
  },
};
interface Words { readonly tag: string; readonly summary: string; readonly pc?: string; readonly headset?: string }
type Copy = Omit<typeof en, "connections"> & { readonly connections: Record<keyof typeof en.connections, Words> };

export const playSetupCopy: Record<"en" | "zh-CN" | "zh-TW" | "ja" | "ko", Copy> = {
  en,
  "zh-CN": {
    platform: "平台", device: "设备", connection: "连接方式", detected: "已检测到", official: "官方页面",
    noPc: "不连电脑",
    stepsTitle: "准备步骤", ready: "软件都已就绪", inHeadset: "在头显上运行", changeHint: "更改平台、设备或连接方式",
    officialSource: "步骤依据官方说明整理，2026 年 11 月",
    standaloneNote: "头显上显示有对应版本的 Avatar，其他 Avatar 显示为替身或默认形象。",
    actors: { vua: "VUA 做", open: "VUA 打开", player: "你来做" },
    steps: {
      steam: "安装 Steam", "steam-sign-in": "登录 Steam", platform: "安装 {platform}", launch: "启动 {platform}",
      "store-install": "从{store}安装 VRChat", "store-launch": "在头显里打开 VRChat",
    },
    stores: { pico: "PICO 商店", meta: "Meta Horizon 商店", viveport: "Viveport" },
    devices: {
      pc: { name: "PC 模式", hint: "不需要头显" },
      pico: { name: "PICO", hint: "PICO 4 系列" },
      quest: { name: "Meta Quest", hint: "Quest 2、3、3S、Pro" },
      "vive-standalone": { name: "VIVE 一体机", hint: "Focus Vision、XR Elite" },
      "vive-wired": { name: "VIVE PC 头显", hint: "VIVE Pro、Cosmos" },
      "other-headset": { name: "其他型号", hint: "SteamVR 支持的头显都可以" },
      wmr: { name: "Windows MR", hint: "HP Reverb 等" },
    },
    connections: {
      "pico-connect": { tag: "官方 · 免费", summary: "PICO 自己的串流，可以用 USB 线或 Wi-Fi。", pc: "安装 PICO Connect 和 SteamVR", headset: "在头显里打开 PICO Connect，连接电脑" },
      standalone: { tag: "头显版", summary: "VRChat 直接在头显上运行，电脑不参与。" },
    },
  },
  "zh-TW": {
    platform: "平台", device: "裝置", connection: "連線方式", detected: "已偵測到", official: "官方頁面",
    noPc: "不連電腦",
    stepsTitle: "準備步驟", ready: "軟體都已就緒", inHeadset: "在頭戴裝置上執行", changeHint: "更改平台、裝置或連線方式",
    officialSource: "步驟依據官方說明整理，2026 年 11 月",
    standaloneNote: "頭戴裝置上顯示有對應版本的 Avatar，其他 Avatar 顯示為替身或預設形象。",
    actors: { vua: "VUA 做", open: "VUA 開啟", player: "你來做" },
    steps: {
      steam: "安裝 Steam", "steam-sign-in": "登入 Steam", platform: "安裝 {platform}", launch: "啟動 {platform}",
      "store-install": "從{store}安裝 VRChat", "store-launch": "在頭戴裝置裡開啟 VRChat",
    },
    stores: { pico: "PICO 商店", meta: "Meta Horizon 商店", viveport: "Viveport" },
    devices: {
      pc: { name: "PC 模式", hint: "不需要頭戴裝置" },
      pico: { name: "PICO", hint: "PICO 4 系列" },
      quest: { name: "Meta Quest", hint: "Quest 2、3、3S、Pro" },
      "vive-standalone": { name: "VIVE 一體機", hint: "Focus Vision、XR Elite" },
      "vive-wired": { name: "VIVE PC 頭戴裝置", hint: "VIVE Pro、Cosmos" },
      "other-headset": { name: "其他型號", hint: "SteamVR 支援的頭戴裝置都可以" },
      wmr: { name: "Windows MR", hint: "HP Reverb 等" },
    },
    connections: {
      "pico-connect": { tag: "官方 · 免費", summary: "PICO 自己的串流，可以用 USB 線或 Wi-Fi。", pc: "安裝 PICO Connect 和 SteamVR", headset: "在頭戴裝置裡開啟 PICO Connect，連線電腦" },
      standalone: { tag: "頭戴裝置版", summary: "VRChat 直接在頭戴裝置上執行，電腦不參與。" },
    },
  },
  ja: {
    platform: "プラットフォーム", device: "デバイス", connection: "接続方法", detected: "検出済み", official: "公式ページ",
    noPc: "PC を使わない",
    stepsTitle: "準備の手順", ready: "ソフトはすべて準備済み", inHeadset: "ヘッドセットで動作", changeHint: "プラットフォーム、デバイス、接続方法を変更",
    officialSource: "公式の案内に基づく手順（2026 年 11 月）",
    standaloneNote: "ヘッドセットには対応版のあるアバターが表示されます。それ以外は代替または標準の姿で表示されます。",
    actors: { vua: "VUA が行う", open: "VUA が開く", player: "自分で行う" },
    steps: {
      steam: "Steam をインストール", "steam-sign-in": "Steam にログイン", platform: "{platform} をインストール", launch: "{platform} を起動",
      "store-install": "{store}で VRChat をインストール", "store-launch": "ヘッドセットで VRChat を開く",
    },
    stores: { pico: "PICO ストア", meta: "Meta Horizon ストア", viveport: "Viveport" },
    devices: {
      pc: { name: "デスクトップモード", hint: "ヘッドセット不要" },
      pico: { name: "PICO", hint: "PICO 4 シリーズ" },
      quest: { name: "Meta Quest", hint: "Quest 2、3、3S、Pro" },
      "vive-standalone": { name: "VIVE スタンドアロン", hint: "Focus Vision、XR Elite" },
      "vive-wired": { name: "VIVE PC 用", hint: "VIVE Pro、Cosmos" },
      "other-headset": { name: "その他の機種", hint: "SteamVR 対応のヘッドセット" },
      wmr: { name: "Windows MR", hint: "HP Reverb など" },
    },
    connections: {
      "pico-connect": { tag: "公式・無料", summary: "PICO 純正のストリーミング。USB ケーブルまたは Wi-Fi で接続します。", pc: "PICO Connect と SteamVR をインストール", headset: "ヘッドセットで PICO Connect を開き、PC に接続" },
      standalone: { tag: "ヘッドセット版", summary: "VRChat がヘッドセット単体で動きます。PC は使いません。" },
    },
  },
  ko: {
    platform: "플랫폼", device: "기기", connection: "연결 방식", detected: "감지됨", official: "공식 페이지",
    noPc: "PC 없이",
    stepsTitle: "준비 단계", ready: "소프트웨어 준비 완료", inHeadset: "헤드셋에서 실행", changeHint: "플랫폼, 기기, 연결 방식 변경",
    officialSource: "공식 안내를 바탕으로 한 단계 (2026년 11월)",
    standaloneNote: "헤드셋에는 해당 버전이 있는 아바타가 보여요. 나머지는 대체 또는 기본 모습으로 보여요.",
    actors: { vua: "VUA가 해요", open: "VUA가 열어요", player: "직접 해요" },
    steps: {
      steam: "Steam 설치", "steam-sign-in": "Steam 로그인", platform: "{platform} 설치", launch: "{platform} 실행",
      "store-install": "{store}에서 VRChat 설치", "store-launch": "헤드셋에서 VRChat 열기",
    },
    stores: { pico: "PICO 스토어", meta: "Meta Horizon 스토어", viveport: "Viveport" },
    devices: {
      pc: { name: "데스크톱 모드", hint: "헤드셋 불필요" },
      pico: { name: "PICO", hint: "PICO 4 시리즈" },
      quest: { name: "Meta Quest", hint: "Quest 2, 3, 3S, Pro" },
      "vive-standalone": { name: "VIVE 독립형", hint: "Focus Vision, XR Elite" },
      "vive-wired": { name: "VIVE PC용", hint: "VIVE Pro, Cosmos" },
      "other-headset": { name: "다른 모델", hint: "SteamVR를 지원하는 헤드셋" },
      wmr: { name: "Windows MR", hint: "HP Reverb 등" },
    },
    connections: {
      "pico-connect": { tag: "공식 · 무료", summary: "PICO 자체 스트리밍이에요. USB 케이블이나 Wi-Fi로 연결해요.", pc: "PICO Connect와 SteamVR 설치", headset: "헤드셋에서 PICO Connect를 열고 PC에 연결" },
      standalone: { tag: "헤드셋 버전", summary: "VRChat이 헤드셋에서 바로 실행돼요. PC는 쓰지 않아요." },
    },
  },
};
