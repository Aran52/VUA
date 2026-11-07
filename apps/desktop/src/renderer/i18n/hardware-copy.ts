/** Reading content, not an automatic hardware-detection result. */
const en = {
  title: "Hardware introduction",
  intro: "Start with the name of your device. A brand, headset model and connection method describe different things.",
  sections: [
    { id: "identify-device", title: "Find the full model name", paragraphs: ["Look on the headset, its box, or the About page in its settings. Write down the brand and full model name, rather than just the name on a controller.", "The same brand can have several kinds of headset. These pages explain how to identify yours; they do not claim VUA detected connected hardware."] },
    { id: "screen-or-headset", title: "PC mode, a standalone headset, or PC VR?", paragraphs: ["PC mode uses this PC's monitor and mouse. You can start without a headset.", "A standalone headset can run its own apps. PC streaming instead runs the game on the computer and sends the view to the headset. Some headsets depend on a PC; check the exact model before choosing software."] },
    { id: "choose-device-route", title: "Choose your device and connection", paragraphs: ["Pick your device on Launch. A headset with several ways to connect asks which one, and the steps below follow your choice. Connecting a headset is a separate step from installing the software.", "VUA installs and starts PC mode and PICO Connect itself. For other combinations, follow the steps; VUA opens the official pages."] },
    { id: "tracking-accessories", title: "Tracking is a separate addition", paragraphs: ["Controllers, tracking stations and body trackers are different devices. Eye or face tracking also depends on the headset's exact model and supported tracking software.", "Tracking and translation tools belong in Tools. Optional additions do not need to be ready before ordinary play, and the computer's runtime list does not identify every connected device."] },
  ],
};
type Copy = typeof en;
const zh: Copy = {
  title: "硬件介绍",
  intro: "先找到设备的名字。品牌、头显型号和连接方式，是三件不同的事。",
  sections: [
    { id: "identify-device", title: "先找完整型号", paragraphs: ["看看头显机身、包装盒，或头显设置里的“关于”。记下品牌和完整型号，别只看手柄上的名字。", "同一个品牌也有不同类型的头显。这里帮你辨认设备，不代表 VUA 已经检测到你接上的硬件。"] },
    { id: "screen-or-headset", title: "PC 模式、一体机，还是电脑 VR？", paragraphs: ["PC 模式使用这台电脑的显示器和鼠标，不需要先买头显。", "一体机可以自己运行应用。电脑串流则由电脑运行游戏，再把画面传给头显。有些头显需要依靠电脑使用，选择软件前先确认完整型号。"] },
    { id: "choose-device-route", title: "选设备和连接方式", paragraphs: ["在“启动”页选择设备。有多种连接方式的头显会再问一次，下面的准备步骤跟着变化。软件装好后，还要实际连接头显。", "PC 模式和 PICO Connect 由 VUA 安装和启动。其他组合按步骤操作，VUA 打开对应的官方页面。"] },
    { id: "tracking-accessories", title: "追踪设备是另外一回事", paragraphs: ["手柄、定位基站和身体追踪器是不同的设备。眼动或面部追踪还取决于头显的具体型号，以及对应追踪软件的支持。", "追踪和翻译工具放在“工具”里。这些可选功能不会成为普通游玩的前提；电脑里的运行时列表也不能代表所有已连接设备。"] },
  ],
};
const zhTW: Copy = {
  title: "硬體介紹",
  intro: "先找到裝置的名字。品牌、頭戴裝置型號和連線方式，是三件不同的事。",
  sections: [
    { id: "identify-device", title: "先找完整型號", paragraphs: ["看看頭戴裝置機身、包裝盒，或頭戴裝置設定裡的“關於”。記下品牌和完整型號，別隻看控制器上的名字。", "同一個品牌也有不同型別的頭戴裝置。這裡幫你辨認裝置，不代表 VUA 已經檢測到你接上的硬體。"] },
    { id: "screen-or-headset", title: "PC 模式、一體機，還是電腦 VR？", paragraphs: ["PC 模式使用這臺電腦的顯示器和滑鼠，不需要先買頭戴裝置。", "一體機可以自己執行應用。電腦串流則由電腦執行遊戲，再把畫面傳給頭戴裝置。有些頭戴裝置需要依靠電腦使用，選擇軟體前先確認完整型號。"] },
    { id: "choose-device-route", title: "選裝置和連線方式", paragraphs: ["在「啟動」頁選擇裝置。有多種連線方式的頭戴裝置會再問一次，下面的準備步驟跟著變化。軟體裝好後，還要實際連線頭戴裝置。", "PC 模式和 PICO Connect 由 VUA 安裝和啟動。其他組合按步驟操作，VUA 開啟對應的官方頁面。"] },
    { id: "tracking-accessories", title: "追蹤裝置是另外一回事", paragraphs: ["控制器、定位基地台和身體追蹤器是不同的裝置。眼動或面部追蹤還取決於頭戴裝置的具體型號，以及對應追蹤軟體的支援。", "追蹤和翻譯工具放在“工具”裡。這些可選功能不會成為普通遊玩的前提；電腦裡的執行時列表也不能代表所有已連線裝置。"] },
  ],
};
const ja: Copy = {
  title: "ハードウェア紹介",
  intro: "まず機器の名前を確認しましょう。ブランド、ヘッドセットの型番、接続方法は別々の情報です。",
  sections: [
    { id: "identify-device", title: "正確な型番を確認する", paragraphs: ["ヘッドセット本体、箱、または設定の「情報」を確認して、ブランドと正確な型番を控えましょう。コントローラーの名前だけでは判断できません。", "同じブランドにも異なる種類があります。ここは機器を見分けるための説明で、VUA が接続機器を検出した結果ではありません。"] },
    { id: "screen-or-headset", title: "デスクトップモード、単体型、それとも PC VR？", paragraphs: ["デスクトップモードでは、この PC のモニターとマウスを使います。ヘッドセットは不要です。", "単体型は自身でアプリを実行できます。PC ストリーミングでは PC でゲームを実行し、映像をヘッドセットへ送ります。PC が必要な機種もあるため、ソフトを選ぶ前に正確な型番を確認しましょう。"] },
    { id: "choose-device-route", title: "デバイスと接続方法を選ぶ", paragraphs: ["「起動」ページでデバイスを選びます。接続方法が複数あるヘッドセットはどれを使うか選び、下の手順がそれに合わせて変わります。ソフトのインストールと実際の接続は別の手順です。", "デスクトップモードと PICO Connect は VUA がインストールして起動します。その他の組み合わせは手順に沿って進め、VUA が公式ページを開きます。"] },
    { id: "tracking-accessories", title: "トラッキング機器は追加の選択", paragraphs: ["コントローラー、ベースステーション、身体用トラッカーは別の機器です。アイトラッキングやフェイストラッキングも、正確な機種と対応ソフトに依存します。", "トラッキングや翻訳の機能は「ツール」にあります。通常のプレイに必須ではなく、PC のランタイム一覧だけではすべての接続機器は分かりません。"] },
  ],
};
const ko: Copy = {
  title: "하드웨어 소개",
  intro: "먼저 기기 이름을 확인하세요. 브랜드, 헤드셋 모델, 연결 방식은 서로 다른 정보예요.",
  sections: [
    { id: "identify-device", title: "정확한 모델명 찾기", paragraphs: ["헤드셋 본체, 상자 또는 설정의 기기 정보에서 브랜드와 전체 모델명을 확인하세요. 컨트롤러에 적힌 이름만으로는 구분하기 어려워요.", "같은 브랜드에도 여러 종류가 있어요. 이 페이지는 기기를 구분하는 설명이며, VUA가 연결된 하드웨어를 감지했다는 뜻은 아니에요."] },
    { id: "screen-or-headset", title: "데스크톱 모드, 독립형 헤드셋, PC VR?", paragraphs: ["데스크톱 모드는 이 컴퓨터의 모니터와 마우스를 사용해요. 헤드셋 없이 시작할 수 있어요.", "독립형 헤드셋은 자체적으로 앱을 실행해요. PC 스트리밍은 컴퓨터가 게임을 실행하고 헤드셋으로 화면을 보내는 방식이에요. PC가 필요한 기기도 있으니 소프트웨어를 선택하기 전에 정확한 모델을 확인하세요."] },
    { id: "choose-device-route", title: "기기와 연결 방식 선택하기", paragraphs: ["실행 페이지에서 기기를 고르세요. 연결 방식이 여러 개인 헤드셋은 어떤 방식을 쓸지 고르고, 아래 단계가 그에 맞게 바뀌어요. 설치와 실제 헤드셋 연결은 별도 단계예요.", "데스크톱 모드와 PICO Connect는 VUA가 설치하고 실행해요. 다른 조합은 단계를 따라 진행하고, VUA가 공식 페이지를 열어요."] },
    { id: "tracking-accessories", title: "트래킹 기기는 별도의 추가 기능", paragraphs: ["컨트롤러, 베이스 스테이션, 신체 트래커는 서로 다른 기기예요. 눈이나 얼굴 트래킹도 정확한 헤드셋 모델과 지원하는 트래킹 소프트웨어에 따라 달라요.", "트래킹과 번역 도구는 ‘도구’에 있어요. 일반 플레이를 시작하기 위해 먼저 준비할 필요는 없으며, 컴퓨터의 런타임 목록이 모든 연결 기기를 알려 주지는 않아요."] },
  ],
};
export const hardwareCopy = { en, "zh-CN": zh, "zh-TW": zhTW, ja, ko };
