/** The play page's device detection and the one-button route flow. Labels stay short: the button
 * itself says what happens next. */
const en = {
  refreshDevices: "Rescan devices", detecting: "Scanning devices…", devicesNone: "No VR headset detected. You can still play in Desktop mode.",
  phases: {
    unavailable: "Unavailable", check: "Check setup", checking: "Checking…", confirm: "Review", execute: "Install",
    running: "Installing", handoff: "Open official page", retry: "Retry", launch: "Launch", starting: "Starting…", stop: "Close",
  },
  cancel: "Cancel", connection: "Connection", region: "Download region", toInstall: "Will install", accounts: "Manage accounts",
  checklist: "Review setup", repair: "Having trouble with an app?", steps: "Steps", confirmContinue: "Confirm & continue", regionNeeded: "Select a download region first",
};
type Copy = { [K in keyof typeof en]: typeof en[K] extends string ? string : { [P in keyof typeof en[K]]: string } };
const zh: Copy = {
  refreshDevices: "重新检测设备", detecting: "正在检测设备…", devicesNone: "未检测到 VR 头显，可直接用电脑屏幕游玩。",
  phases: {
    unavailable: "暂不可用", check: "检查环境", checking: "检查中…", confirm: "请确认", execute: "安装",
    running: "安装中", handoff: "打开官方页面", retry: "重试", launch: "启动", starting: "正在启动…", stop: "关闭",
  },
  cancel: "取消", connection: "连接方式", region: "下载地区", toInstall: "即将安装", accounts: "管理账号",
  checklist: "请确认", repair: "软件无法正常运行？", steps: "步骤", confirmContinue: "确认并继续", regionNeeded: "请先选择下载地区",
};
const zhTW: Copy = {
  refreshDevices: "重新偵測裝置", detecting: "正在偵測裝置…", devicesNone: "未偵測到 VR 裝置，可直接用電腦螢幕遊玩。",
  phases: {
    unavailable: "暫不可用", check: "檢查環境", checking: "檢查中…", confirm: "請確認", execute: "安裝",
    running: "安裝中", handoff: "開啟官方頁面", retry: "重試", launch: "啟動", starting: "正在啟動…", stop: "關閉",
  },
  cancel: "取消", connection: "連線方式", region: "下載地區", toInstall: "即將安裝", accounts: "管理帳號",
  checklist: "請確認", repair: "軟體無法正常運作？", steps: "步驟", confirmContinue: "確認並繼續", regionNeeded: "請先選擇下載地區",
};
const ja: Copy = {
  refreshDevices: "デバイスを再検出", detecting: "デバイスを検出中…", devicesNone: "VR 機器が見つかりません。PC 画面（デスクトップモード）で遊べます。",
  phases: {
    unavailable: "利用不可", check: "環境を確認", checking: "確認中…", confirm: "確認へ", execute: "インストール",
    running: "インストール中", handoff: "公式ページを開く", retry: "再試行", launch: "起動", starting: "起動中…", stop: "閉じる",
  },
  cancel: "キャンセル", connection: "接続方法", region: "ダウンロード地域", toInstall: "導入予定", accounts: "アカウント管理",
  checklist: "内容の確認", repair: "ソフトがうまく動かない場合", steps: "手順", confirmContinue: "確認して続行", regionNeeded: "先にダウンロード地域を選んでください",
};
const ko: Copy = {
  refreshDevices: "기기 다시 찾기", detecting: "기기 확인 중…", devicesNone: "감지된 VR 기기가 없어요. PC 화면(데스크톱 모드)으로 플레이할 수 있어요.",
  phases: {
    unavailable: "사용 불가", check: "환경 확인", checking: "확인 중…", confirm: "내용 확인", execute: "설치",
    running: "설치 중", handoff: "공식 페이지 열기", retry: "다시 시도", launch: "실행", starting: "실행 중…", stop: "닫기",
  },
  cancel: "취소", connection: "연결 방식", region: "다운로드 지역", toInstall: "설치 항목", accounts: "계정 관리",
  checklist: "내용 확인", repair: "프로그램이 잘 작동하지 않나요?", steps: "단계", confirmContinue: "확인 후 계속", regionNeeded: "먼저 다운로드 지역을 선택해 주세요",
};
export const playFlowCopy = { en, "zh-CN": zh, "zh-TW": zhTW, ja, ko };
