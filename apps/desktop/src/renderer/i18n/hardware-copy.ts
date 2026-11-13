/** Reading content, not an automatic hardware-detection result. */
const en = {
  title: "Hardware Basics",
  intro: "Check the exact name of your gear first—brand, headset model, and connection method are three separate things.",
  sections: [
    { id: "identify-device", title: "Check the exact headset model", paragraphs: ["Look at the label on the headset, its retail box, or Settings → About inside the headset, and note both the brand and full model name. Controller markings alone often omit the exact headset model.", "One brand can sell very different headsets. Use this page as a lookup guide—VUA does not automatically identify plugged-in headsets here."] },
    { id: "screen-or-headset", title: "Desktop mode, standalone headset, or PC VR?", paragraphs: ["Desktop mode runs VRChat right on your PC monitor with keyboard and mouse—no VR headset needed.", "Standalone headsets can run mobile VR apps on their own, while PC VR streaming renders the game on your PC and sends the video to the headset. Dedicated PC VR headsets always require a PC, so verify your model before installing software."] },
    { id: "choose-device-route", title: "Pick your device and connection method", paragraphs: ["Select your device on the Launch page. If a headset supports multiple connection methods, pick how you want to connect and the checklist below updates to match. Installing PC software and linking the headset itself are separate steps.", "VUA can install and launch Desktop mode and PICO Connect directly. For other headsets and streaming apps, follow the checklist and VUA will open their official download pages."] },
    { id: "tracking-accessories", title: "Trackers and add-ons are optional", paragraphs: ["Controllers, base stations, and full-body trackers each serve different roles, and eye or face tracking depends on both your exact headset model and companion software.", "Tracking and translation utilities live under Tools. None of them are required before regular play, and your PC's VR runtime list will not show every connected accessory."] },
  ],
};
type Copy = typeof en;
const zh: Copy = {
  title: "硬件介绍",
  intro: "先确认手头设备的完整名称——品牌、头显具体型号和连接方式是三件不同的事。",
  sections: [
    { id: "identify-device", title: "确认头显的完整型号", paragraphs: ["查看头显机身标签、包装盒，或头显系统“设置 → 关于本机”，记下品牌和完整型号（比如区分标准版与 Pro 版），不要只看手柄上的标志。", "同一品牌旗下往往有多代、不同形态的头显。本页用于帮你核对设备型号，VUA 不会在这里自动识别已接入的硬件。"] },
    { id: "screen-or-headset", title: "桌面模式、一体机，还是电脑串流 VR？", paragraphs: ["桌面模式直接用当前电脑的显示器和键鼠游玩，没有头显也能随时进 VRChat。", "一体机既能脱离电脑独立运行移动版应用，也能通过电脑串流游玩 PC VR（由电脑负责渲染画面再传给头显）；而纯 PC VR 头显必须连接电脑才能使用。装软件前先分清手头设备属于哪一类。"] },
    { id: "choose-device-route", title: "选择设备与连接方式", paragraphs: ["在“启动”页点选你的设备；如果该头显支持多种连接方式（如有线 USB 或无线 Wi-Fi），选定后下方会自动列出对应的准备步骤。注意装好电脑端软件后，还需要在头显端完成配对连接。", "桌面模式和 PICO Connect 可由 VUA 直接引导安装与启动；其他头显与串流方案可按步骤操作，由 VUA 帮你打开对应的官方下载页。"] },
    { id: "tracking-accessories", title: "全身追踪与面捕属于可选扩展", paragraphs: ["手柄、定位基站和全身追踪器各司其职；眼动追踪与面部捕捉则同时取决于头显硬件型号（如 PICO 4 Pro）和配套转发软件。", "追踪与实时翻译类软件集中在“工具”页管理。普通游玩不需要先配齐这些外设，电脑上的运行时列表也不会列出所有外接追踪配件。"] },
  ],
};
const zhTW: Copy = {
  title: "硬體介紹",
  intro: "先確認手邊設備的完整名稱——品牌、頭戴式裝置型號與連線方式是三件不同的事。",
  sections: [
    { id: "identify-device", title: "確認頭戴式裝置的完整型號", paragraphs: ["請查看頭顯機身標籤、外盒，或頭顯內「設定 → 關於」頁面，記下品牌與完整型號，別只看控制器上的名稱。", "同一品牌常有不同世代與類型的頭戴式裝置。本頁用來協助你核對型號，VUA 不會在此自動偵測已接上的硬體。"] },
    { id: "screen-or-headset", title: "桌面模式、一體機，還是電腦串流 VR？", paragraphs: ["桌面模式直接使用這台電腦的螢幕與滑鼠鍵盤遊玩，沒有頭戴式裝置也能馬上開始。", "一體機（獨立型頭顯）可單獨執行行動版應用程式，也能透過電腦串流遊玩 PC VR（由電腦運算畫面再傳送至頭顯）；純 PC VR 頭顯則必須連接電腦使用。安裝軟體前請先確認完整型號。"] },
    { id: "choose-device-route", title: "選擇裝置與連線方式", paragraphs: ["在「啟動」頁點選你的裝置；若該頭顯支援多種連線方式（如 USB 有線或 Wi-Fi 無線），選定後下方會隨之列出對應的準備步驟。安裝好電腦端軟體後，仍需實際連接頭顯。", "桌面模式與 PICO Connect 可由 VUA 直接安裝與啟動；其他裝置組合請依步驟操作，VUA 會協助開啟對應的官方頁面。"] },
    { id: "tracking-accessories", title: "全身追蹤與臉部追蹤屬於選配功能", paragraphs: ["控制器、定位基地台與身體追蹤器各自負責不同部位；眼動與臉部追蹤則同時取決於頭顯具體型號與配套追蹤軟體。", "追蹤與即時翻譯工具集中在「工具」頁管理。一般遊玩不必事先備妥這些配件，電腦中的 VR 執行環境清單也不會列出所有外接配件。"] },
  ],
};
const ja: Copy = {
  title: "ハードウェアの基本",
  intro: "まずは手元の機器名を正確に確認しましょう。メーカー名・ヘッドセットの型番・接続方法はそれぞれ別の情報です。",
  sections: [
    { id: "identify-device", title: "ヘッドセットの正確な型番を確かめる", paragraphs: ["ヘッドセット本体のラベル、外箱、または機器内の「設定 → デバイス情報」などでメーカー名と正確な型番を確認してください。コントローラーの刻印だけでは機種を特定できないことがあります。", "同じメーカーでも世代や方式が異なる機種があります。このページは機種を見分けるための案内であり、接続中の機器を VUA が自動判別するものではありません。"] },
    { id: "screen-or-headset", title: "デスクトップモード・一体型（スタンドアロン）・PC VR の違い", paragraphs: ["デスクトップモードなら、今使っている PC のモニターとマウス・キーボードだけですぐに遊べます。VR ヘッドセットは不要です。", "一体型（スタンドアロン）ヘッドセットは単体でアプリを動かせるほか、PC で描画した映像を転送する PC VR ストリーミングにも対応します。一方、PC 接続専用のヘッドセットは単体では動作しないため、ソフトを入れる前に型番を確認しておきましょう。"] },
    { id: "choose-device-route", title: "デバイスと接続方法を選ぶ", paragraphs: ["「起動」ページで使うデバイスを選びます。USB や Wi-Fi など複数の接続方法がある機種では、使う方法を選ぶと下の手順リストが切り替わります。PC ソフトの導入とヘッドセット本体の接続は別ステップです。", "デスクトップモードと PICO Connect は VUA からそのまま導入・起動できます。その他の機種や接続ソフトは、案内手順に沿って VUA から公式ページを開けます。"] },
    { id: "tracking-accessories", title: "フルトラやフェイストラッキングは追加要素", paragraphs: ["コントローラー、ベースステーション、フルトラ用トラッカーはそれぞれ役割が異なります。アイトラッキングやフェイストラッキングも、対応するヘッドセット型番と連携ソフトの組み合わせが必要です。", "トラッキングや翻訳などの補助ソフトは「ツール」にまとまっています。通常のプレイを始める段階では不要で、PC 側のランタイム一覧にすべての周辺機器が表示されるわけでもありません。"] },
  ],
};
const ko: Copy = {
  title: "하드웨어 기본 안내",
  intro: "먼저 가지고 계신 기기의 정확한 이름부터 확인해 보세요. 브랜드, 헤드셋 모델명, 연결 방식은 서로 다른 정보예요.",
  sections: [
    { id: "identify-device", title: "정확한 헤드셋 모델명 확인하기", paragraphs: ["헤드셋 본체 라벨, 제품 상자, 또는 기기 설정의 ‘정보’ 화면에서 브랜드와 전체 모델명을 확인해 두세요. 컨트롤러에 적힌 이름만으로는 정확한 기종을 구분하기 어려워요.", "같은 브랜드라도 세대와 방식이 다른 여러 기종이 있어요. 이 페이지는 기종을 직접 구분하기 위한 안내이며, VUA가 연결된 기기를 자동으로 감지한 결과는 아니에요."] },
    { id: "screen-or-headset", title: "데스크톱 모드, 독립형 헤드셋, PC VR의 차이", paragraphs: ["데스크톱 모드는 지금 쓰는 PC 모니터와 키보드·마우스만으로 바로 플레이할 수 있어요. VR 헤드셋이 없어도 시작할 수 있어요.", "독립형(스탠드얼론) 헤드셋은 기기 단독으로도 앱을 실행할 수 있고, PC에서 실행한 게임 화면을 전송받는 PC VR 스트리밍도 지원해요. 반면 PC 전용 헤드셋은 반드시 PC에 연결해야 작동하므로 설치할 소프트웨어를 고르기 전에 정확한 모델명을 확인하세요."] },
    { id: "choose-device-route", title: "기기와 연결 방식 고르기", paragraphs: ["‘실행’ 페이지에서 사용할 기기를 선택하세요. 연결 방식이 여러 가지인 헤드셋은 원하는 방식을 고르면 아래 준비 단계가 그에 맞춰 바뀌어요. PC 소프트웨어 설치와 실제 헤드셋 연결은 별도 단계예요.", "데스크톱 모드와 PICO Connect는 VUA에서 바로 설치하고 실행할 수 있어요. 그 밖의 기기와 연결 방식은 안내 단계에 따라 VUA에서 공식 페이지를 열어 진행할 수 있어요."] },
    { id: "tracking-accessories", title: "트래커와 표정 추적은 선택 사항", paragraphs: ["컨트롤러, 베이스 스테이션, 풀트래킹용 신체 트래커는 각기 다른 장비예요. 시선 추적(아이트래킹)이나 페이스트래킹 역시 지원하는 정확한 헤드셋 기종과 연동 소프트웨어가 함께 필요해요.", "트래킹과 번역 보조 프로그램은 ‘도구’ 탭에 모여 있어요. 일반 플레이를 시작할 때 미리 갖춰 둘 필요는 없으며, PC의 런타임 목록에 연결된 모든 주변기기가 표시되지는 않아요."] },
  ],
};
export const hardwareCopy = { en, "zh-CN": zh, "zh-TW": zhTW, ja, ko };
