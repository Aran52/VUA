/** The play page's device detection and the one-button route flow. Labels stay short: the button
 * itself says what happens next. */
const en = {
  refreshDevices: "Refresh device detection", detecting: "Detecting devices…", devicesNone: "No VR device software found",
  phases: {
    unavailable: "Unavailable", check: "Check environment", checking: "Checking…", confirm: "Confirm checklist", execute: "Run",
    running: "Running", handoff: "Open official page", retry: "Retry", launch: "Launch", starting: "Starting…", stop: "Close",
  },
  cancel: "Cancel", connection: "Connection", region: "Download region", toInstall: "To install", accounts: "Accounts",
  checklist: "Checklist", regionNeeded: "Choose a download region first",
};
type Copy = { [K in keyof typeof en]: typeof en[K] extends string ? string : { [P in keyof typeof en[K]]: string } };
const zh: Copy = {
  refreshDevices: "刷新设备识别", detecting: "正在识别设备…", devicesNone: "未识别到 VR 设备软件",
  phases: {
    unavailable: "暂不可用", check: "检查环境", checking: "检查中…", confirm: "确认清单", execute: "执行",
    running: "执行中", handoff: "打开官方页面", retry: "重试", launch: "启动", starting: "正在启动…", stop: "关闭",
  },
  cancel: "取消", connection: "连接方式", region: "下载地区", toInstall: "将安装", accounts: "账号",
  checklist: "清单", regionNeeded: "先选择下载地区",
};
const zhTW: Copy = {
  refreshDevices: "重新整理裝置識別", detecting: "正在識別裝置…", devicesNone: "未識別到 VR 裝置軟體",
  phases: {
    unavailable: "暫不可用", check: "檢查環境", checking: "檢查中…", confirm: "確認清單", execute: "執行",
    running: "執行中", handoff: "開啟官方頁面", retry: "重試", launch: "啟動", starting: "正在啟動…", stop: "關閉",
  },
  cancel: "取消", connection: "連線方式", region: "下載地區", toInstall: "將安裝", accounts: "帳號",
  checklist: "清單", regionNeeded: "先選擇下載地區",
};
const ja: Copy = {
  refreshDevices: "デバイス認識を更新", detecting: "デバイスを認識中…", devicesNone: "VR デバイスのソフトが見つかりません",
  phases: {
    unavailable: "利用できません", check: "環境を確認", checking: "確認中…", confirm: "チェックリストを確認", execute: "実行",
    running: "実行中", handoff: "公式ページを開く", retry: "再試行", launch: "起動", starting: "起動中…", stop: "閉じる",
  },
  cancel: "キャンセル", connection: "接続方法", region: "ダウンロード地域", toInstall: "インストール予定", accounts: "アカウント",
  checklist: "チェックリスト", regionNeeded: "先にダウンロード地域を選択",
};
const ko: Copy = {
  refreshDevices: "기기 인식 새로고침", detecting: "기기 인식 중…", devicesNone: "VR 기기 소프트웨어를 찾지 못했어요",
  phases: {
    unavailable: "사용할 수 없음", check: "환경 확인", checking: "확인 중…", confirm: "체크리스트 확인", execute: "실행",
    running: "실행 중", handoff: "공식 페이지 열기", retry: "다시 시도", launch: "시작", starting: "시작 중…", stop: "닫기",
  },
  cancel: "취소", connection: "연결 방식", region: "다운로드 지역", toInstall: "설치 예정", accounts: "계정",
  checklist: "체크리스트", regionNeeded: "먼저 다운로드 지역을 선택하세요",
};
export const playFlowCopy = { en, "zh-CN": zh, "zh-TW": zhTW, ja, ko };
