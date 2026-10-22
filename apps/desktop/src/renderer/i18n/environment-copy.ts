const en = {
  close: "Close",
  network: "Network", details: "See what's included in {name}", prepare: "Prepare {name}", start: "Start {name}", stop: "Close this session of {name}",
  inspect: "Check again", checking: "Checking…", installing: "Preparing…", ready: "Ready", missing: "Needs software", starting: "Starting…", running: "Running", stopping: "Closing…", attention: "Needs attention", unknown: "Not confirmed",
  included: "Includes", install: "Prepare or repair", launch: "Start setup", closeHint: "Closes only software this card started",
  runningHint: "Running; sign-in and headset connection happen in each app.", borrowed: "Already running; stays open", owned: "Started by this card", stopped: "Not running",
  runtime: "Other VR software",
  editors: "Unity editors", managers: "Hub and package managers", installed: "Detected", notFound: "Not found in known locations", configFound: "Configuration found", configMissing: "No configuration found", configFailed: "Configuration could not be read",
  otherEditor: "Version found. Avatar preparation uses Unity 2022.3.22f1 or 2022.3.22f1c1.", paths: "Detected locations", inventoryUnknown: "Editor detection is unavailable right now.", official: "Open official page", choose2022: "Prepare Unity 2022", connection: "Connect PICO", closeDetails: "Close details",
  issues: { close_pending: "Further launch requests stopped; Steam may still hold a pending launch. Cancel it in Steam.", not_installed: "Required software missing or incomplete; see the setup plan.", other_route_active: "Another play card is using this software; close that card's launch first.", start_failed: "Launch did not finish. Check the app, or close this launch and retry.", start_timeout: "Waiting for the software to start. Check Steam sign-in, downloads and prompts; if stopped here, also cancel the pending launch in Steam.", close_failed: "Some software did not close and keeps running; quit it from its menu.", close_timeout: "Some software still runs after the close request; quit it from its menu." },
};
type Copy = { [K in keyof typeof en]: typeof en[K] extends string ? string : { [P in keyof typeof en[K]]: string } };
const zh: Copy = {
  close: "关闭",
  network: "网络测试", details: "查看{name}包含的内容", prepare: "准备{name}", start: "启动{name}", stop: "关闭{name}的本次启动",
  inspect: "重新检测", checking: "正在检测…", installing: "正在准备…", ready: "已准备好", missing: "还缺软件", starting: "正在启动…", running: "运行中", stopping: "正在关闭…", attention: "需要处理", unknown: "尚未确认",
  included: "包含", install: "安装或修复", launch: "启动环境", closeHint: "仅关闭本卡启动的软件",
  runningHint: "软件已运行，登录和头显连接在对应软件中完成。", borrowed: "已在运行，将保留", owned: "由本卡启动", stopped: "未运行",
  runtime: "其他 VR 软件",
  editors: "Unity 编辑器", managers: "Hub 与包管理器", installed: "已检测到", notFound: "未在常用位置找到", configFound: "配置已找到", configMissing: "未找到配置", configFailed: "配置读取失败",
  otherEditor: "已找到此版本。当前 Avatar 准备路线使用 Unity 2022.3.22f1 或 2022.3.22f1c1。", paths: "检测到的位置", inventoryUnknown: "暂时无法检测编辑器。", official: "打开官方页面", choose2022: "准备 Unity 2022", connection: "连接 PICO", closeDetails: "收起详情",
  issues: { close_pending: "后续启动请求已停止，Steam 中可能仍有等待的启动，需在 Steam 中取消。", not_installed: "缺少必要软件或安装不完整，见准备计划。", other_route_active: "另一张游玩卡片正在使用这套软件，需先关闭其本次启动。", start_failed: "启动未完成。查看对应软件，或关闭本次启动后重试。", start_timeout: "等待软件启动中。查看 Steam 登录、下载和弹窗；在此停止时也需在 Steam 中取消等待的启动。", close_failed: "部分软件未关闭，仍在运行，可从软件菜单退出。", close_timeout: "部分软件在关闭请求后仍在运行，可从软件菜单退出。" },
};
const ja: Copy = {
  close: "閉じる",
  network: "ネットワーク", details: "{name}の内容を見る", prepare: "{name}を準備", start: "{name}を起動", stop: "{name}の今回の起動を閉じる",
  inspect: "再確認", checking: "確認中…", installing: "準備中…", ready: "準備完了", missing: "ソフトウェアが必要", starting: "起動中…", running: "実行中", stopping: "終了を要求中…", attention: "対応が必要", unknown: "未確認",
  included: "含まれるもの", install: "インストール・修復", launch: "環境を起動", closeHint: "このカードで起動したソフトだけを閉じます",
  runningHint: "実行中です。ログインとヘッドセット接続は各ソフトで行います。", borrowed: "実行中のため残します", owned: "このカードで起動", stopped: "実行していません",
  runtime: "その他の VR ソフトウェア",
  editors: "Unity エディター", managers: "Hub とパッケージ管理", installed: "検出済み", notFound: "一般的な場所では未検出", configFound: "設定を検出", configMissing: "設定が見つかりません", configFailed: "設定を読み取れません",
  otherEditor: "この版が見つかりました。アバター準備には Unity 2022.3.22f1 または 2022.3.22f1c1 を使います。", paths: "検出した場所", inventoryUnknown: "エディターを確認できません。", official: "公式ページを開く", choose2022: "Unity 2022 を準備", connection: "PICO を接続", closeDetails: "詳細を閉じる",
  issues: { close_pending: "以降の起動要求は止めました。Steam に待機中の起動が残っている場合があるため、Steam でキャンセルしてください。", not_installed: "必要なソフトがないか、インストールが不完全です。準備計画を参照してください。", other_route_active: "別のプレイカードがこのソフトを使用中です。先にそのカードの起動を閉じてください。", start_failed: "起動が完了しませんでした。該当のソフトを確認するか、今回の起動を閉じて再試行してください。", start_timeout: "ソフトの起動待ちです。Steam のログイン、ダウンロード、ポップアップを確認してください。ここで止めた場合は Steam でも待機中の起動をキャンセルしてください。", close_failed: "一部のソフトが閉じず実行中です。ソフトのメニューから終了できます。", close_timeout: "終了要求の後も一部のソフトが実行中です。ソフトのメニューから終了できます。" },
};
const ko: Copy = {
  close: "닫기",
  network: "네트워크", details: "{name} 구성 보기", prepare: "{name} 준비", start: "{name} 시작", stop: "{name}의 이번 실행 닫기",
  inspect: "다시 확인", checking: "확인 중…", installing: "준비 중…", ready: "준비됨", missing: "소프트웨어 필요", starting: "시작 중…", running: "실행 중", stopping: "종료 요청 중…", attention: "확인 필요", unknown: "확인되지 않음",
  included: "포함", install: "설치 또는 복구", launch: "환경 실행", closeHint: "이 카드가 실행한 소프트웨어만 닫아요",
  runningHint: "실행 중이에요. 로그인과 헤드셋 연결은 각 소프트웨어에서 해요.", borrowed: "이미 실행 중이라 유지", owned: "이 카드가 실행함", stopped: "실행 중 아님",
  runtime: "기타 VR 소프트웨어",
  editors: "Unity 에디터", managers: "Hub 및 패키지 관리자", installed: "감지됨", notFound: "일반적인 위치에서 찾지 못함", configFound: "설정 감지됨", configMissing: "설정을 찾지 못함", configFailed: "설정 읽기 실패",
  otherEditor: "이 버전을 찾았어요. 아바타 준비에는 Unity 2022.3.22f1 또는 2022.3.22f1c1을 써요.", paths: "감지된 위치", inventoryUnknown: "지금은 에디터를 확인할 수 없습니다.", official: "공식 페이지 열기", choose2022: "Unity 2022 준비", connection: "PICO 연결", closeDetails: "상세 닫기",
  issues: { close_pending: "추가 실행 요청은 멈췄어요. Steam에 대기 중인 실행이 남아 있을 수 있으니 Steam에서 취소하세요.", not_installed: "필요한 소프트웨어가 없거나 설치가 불완전해요. 준비 계획을 보세요.", other_route_active: "다른 플레이 카드가 이 소프트웨어를 쓰고 있어요. 먼저 그 카드의 실행을 닫으세요.", start_failed: "실행이 끝나지 않았어요. 해당 소프트웨어를 확인하거나 이번 실행을 닫고 다시 시도하세요.", start_timeout: "소프트웨어 실행을 기다리는 중이에요. Steam 로그인, 다운로드, 팝업을 확인하세요. 여기서 멈추면 Steam에서도 대기 중인 실행을 취소하세요.", close_failed: "일부 소프트웨어가 닫히지 않고 실행 중이에요. 해당 메뉴에서 종료하세요.", close_timeout: "종료 요청 후에도 일부 소프트웨어가 실행 중이에요. 해당 메뉴에서 종료하세요." },
};
export const environmentCopy = { en, "zh-CN": zh, ja, ko };
