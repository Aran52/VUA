import type { LocaleId } from "./index.ts";

const en = {
  "notice": {
    "title": "Welcome to VUA",
    "purposeTitle": "What VUA does",
    "purpose": "Set up the software VRChat needs, see clear next steps, and assemble avatars from assets you own. Account sign-ins and final SDK uploads always stay in your hands.",
    "changesTitle": "What's new in this release",
    "changes": "Unified sidebar navigation, a Home view showing pending items, action buttons that state the exact next step, and in-app software reinstallation.",
    "changelog": "View changelog",
    "sourceTitle": "Free & open source",
    "source": "VUA is open-source software under the Apache 2.0 license and free to download on GitHub.",
    "github": "Source code & downloads",
    "licensesTitle": "Third-party licenses",
    "licenses": "Bundled components and external tools remain governed by their respective licenses.",
    "licensesLink": "View third-party licenses",
    "enter": "Take the VUA feature tour",
    "beginner": "New to VRChat? Start here"
  },
  "vrchat": {
    "id": "what-is-vrchat",
    "title": "What is VRChat?",
    "paragraphs": [
      "VRChat is an online social platform filled with community-made worlds and custom avatars, where people hang out, explore, and join live events together.",
      "You don't need a VR headset to play—any Windows PC can run VRChat in desktop mode with a monitor, keyboard, and mouse. Starting on desktop is an easy way to learn the basics before adding VR gear."
    ]
  },
  "wizard": {
    "title": "VRChat Starter Guide",
    "progress": "Page {current} of {total}",
    "want": "Open {page}",
    "next": "Next",
    "finish": "Done",
    "previous": "Back",
    "article": "Read full article",
    "unavailable": "This article couldn't be loaded. Open Help to browse other topics."
  }
};

// All editions await native review; Traditional Chinese uses Taiwan terminology.
export const firstUseCopy: Record<LocaleId, typeof en> = {
  en,
  "zh-CN": {
    "notice": {
      "title": "欢迎使用 VUA",
      "purposeTitle": "VUA 能帮你做什么",
      "purpose": "一站式配齐 VRChat 游玩与改模所需软件，理清每一步操作，并用你手头的素材组装专属模型。账号登录与最终通过官方 SDK 上传始终由你本人确认完成。",
      "changesTitle": "本版更新亮点",
      "changes": "整合侧边导航目录、首页集中展示待办事项、操作按钮直接标明下一步动作，并支持在软件内重装环境组件。",
      "changelog": "查看更新日志",
      "sourceTitle": "开源且免费",
      "source": "VUA 基于 Apache 2.0 协议开源，可在 GitHub 免费获取。",
      "github": "源码与下载页",
      "licensesTitle": "第三方软件许可",
      "licenses": "随包组件与外部联动工具各自遵循其原有许可协议。",
      "licensesLink": "查看第三方许可证",
      "enter": "开始 VUA 功能导览",
      "beginner": "第一次接触 VRChat：入门向导"
    },
    "vrchat": {
      "id": "what-is-vrchat",
      "title": "什么是 VRChat？",
      "paragraphs": [
        "VRChat 是一个由玩家社区共同创造世界与虚拟形象（Avatar）的多人社交平台，大家可以在里面聊天聚会、逛地图拍照或参加各类活动。",
        "玩 VRChat 不一定要买 VR 头显——普通 Windows 电脑直接用显示器和键鼠就能进入桌面模式。新手很适合先用电脑屏幕熟悉基本操作，之后再按需添置 VR 设备。"
      ]
    },
    "wizard": {
      "title": "VRChat 入门向导",
      "progress": "第 {current} / {total} 页",
      "want": "前往“{page}”",
      "next": "下一页",
      "finish": "完成",
      "previous": "上一页",
      "article": "阅读完整说明",
      "unavailable": "当前说明暂无法显示，请前往“帮助”查看其他主题。"
    }
  },
  "zh-TW": {
    "notice": {
      "title": "歡迎使用 VUA",
      "purposeTitle": "VUA 能幫你做什麼",
      "purpose": "協助備妥 VRChat 遊玩與改模所需軟體、清楚提示下一步操作，並用手邊擁有的素材組裝 Avatar。帳號登入與官方 SDK 最終上傳皆由你親自操作確認。",
      "changesTitle": "本次版本更新",
      "changes": "整合側邊導覽目錄、首頁集中列出待處理項目、按鈕直接標示下一步動作，並支援在 VUA 內重新安裝環境軟體。",
      "changelog": "查看更新紀錄",
      "sourceTitle": "開放原始碼且免費",
      "source": "VUA 採用 Apache 2.0 授權開放原始碼，可於 GitHub 免費下載。",
      "github": "原始碼與下載",
      "licensesTitle": "第三方授權條款",
      "licenses": "隨附軟體與外部串接工具各自適用其原本的授權條款。",
      "licensesLink": "查看第三方授權條款",
      "enter": "開始 VUA 功能導覽",
      "beginner": "初次接觸 VRChat：入門精靈"
    },
    "vrchat": {
      "id": "what-is-vrchat",
      "title": "什麼是 VRChat？",
      "paragraphs": [
        "VRChat 是以玩家社群創作的世界與虛擬形象（Avatar）為核心的線上社交平台，玩家可以在裡面聊天聚會、探索世界與參與各式活動。",
        "沒有 VR 頭戴式裝置也能玩——一般 Windows 電腦直接透過螢幕與鍵盤滑鼠就能以桌面模式進入遊戲。建議新手先用桌面模式熟悉基本操作，之後再考慮是否添購 VR 設備。"
      ]
    },
    "wizard": {
      "title": "VRChat 入門精靈",
      "progress": "第 {current}／{total} 頁",
      "want": "前往「{page}」",
      "next": "下一頁",
      "finish": "完成",
      "previous": "上一頁",
      "article": "閱讀完整說明",
      "unavailable": "目前無法載入這則說明，請至「說明」選擇其他主題。"
    }
  },
  "ja": {
    "notice": {
      "title": "VUA へようこそ",
      "purposeTitle": "VUA でできること",
      "purpose": "VRChat のプレイや改変に必要なソフトの準備、次の手順の案内、手持ち素材からのアバター組み立てをまとめてサポートします。各種アカウントへのログインと公式 SDK での最終アップロードはご自身で行います。",
      "changesTitle": "このバージョンの主な変更点",
      "changes": "サイドバー構成の整理、対応待ち項目がひと目でわかるホーム画面、次の操作を明示したボタン表示、VUA 内からのソフト再インストールに対応しました。",
      "changelog": "更新履歴を見る",
      "sourceTitle": "オープンソース・無料配布",
      "source": "VUA は Apache 2.0 ライセンスのオープンソースソフトウェアとして、GitHub から無料でダウンロードできます。",
      "github": "ソースコードとダウンロード",
      "licensesTitle": "サードパーティーライセンス",
      "licenses": "同梱コンポーネントおよび連携する外部ツールには、それぞれのライセンスが適用されます。",
      "licensesLink": "サードパーティーライセンスを見る",
      "enter": "VUA 機能ツアーを始める",
      "beginner": "VRChat が初めての方：入門ガイド"
    },
    "vrchat": {
      "id": "what-is-vrchat",
      "title": "VRChat とは？",
      "paragraphs": [
        "VRChat は、ユーザーコミュニティが制作した無数のワールドやアバターを通じて交流できるソーシャルプラットフォームです。雑談やワールド巡り、音楽・展示イベントなどを一緒に楽しめます。",
        "VR ヘッドセットがなくても、Windows PC のモニターとマウス・キーボードだけでデスクトップモードとして遊べます。まずはデスクトップで基本操作に慣れてから VR 機器の導入を検討するのもおすすめです。"
      ]
    },
    "wizard": {
      "title": "VRChat 入門ガイド",
      "progress": "{current} / {total} ページ",
      "want": "「{page}」を開く",
      "next": "次へ",
      "finish": "完了",
      "previous": "前へ",
      "article": "詳しい解説を読む",
      "unavailable": "この解説を表示できません。ヘルプから別のトピックを選んでください。"
    }
  },
  "ko": {
    "notice": {
      "title": "VUA에 오신 것을 환영해요",
      "purposeTitle": "VUA가 도와주는 일",
      "purpose": "VRChat 플레이와 개변에 필요한 프로그램을 준비하고 다음 할 일을 차례대로 안내하며, 가지고 계신 에셋으로 아바타를 조립할 수 있게 도와줘요. 계정 로그인과 공식 SDK 최종 업로드는 언제나 직접 확인해 진행해요.",
      "changesTitle": "이번 버전 주요 변경점",
      "changes": "사이드바 메뉴를 하나로 정돈하고, 홈 화면에서 바로 처리할 항목을 보여 주며, 버튼에 다음 동작을 명확히 표시했어요. VUA 안에서 프로그램 재설치도 가능해요.",
      "changelog": "변경 기록 보기",
      "sourceTitle": "오픈 소스 및 무료 다운로드",
      "source": "VUA는 Apache 2.0 라이선스로 공개된 오픈 소스 프로그램이며 GitHub에서 무료로 내려받을 수 있어요.",
      "github": "소스 코드 및 다운로드",
      "licensesTitle": "타사 라이선스",
      "licenses": "함께 포함된 구성 요소와 연동 도구에는 각자의 라이선스가 적용돼요.",
      "licensesLink": "타사 라이선스 보기",
      "enter": "VUA 기능 둘러보기 시작",
      "beginner": "VRChat이 처음이라면: 시작 가이드"
    },
    "vrchat": {
      "id": "what-is-vrchat",
      "title": "VRChat이란?",
      "paragraphs": [
        "VRChat은 커뮤니티가 직접 만든 수많은 월드와 개성 있는 아바타로 어울리는 소셜 플랫폼이에요. 친구들과 대화하고 월드를 탐험하거나 다양한 이벤트에 함께 참여할 수 있어요.",
        "VR 헤드셋이 없어도 괜찮아요. Windows PC 모니터와 키보드·마우스만으로 데스크톱 모드에서 바로 플레이할 수 있으니, 먼저 화면 모드로 조작에 익숙해진 뒤 VR 기기를 준비해도 충분해요."
      ]
    },
    "wizard": {
      "title": "VRChat 시작 가이드",
      "progress": "{current} / {total}쪽",
      "want": "‘{page}’ 열기",
      "next": "다음",
      "finish": "완료",
      "previous": "이전",
      "article": "전체 설명 읽기",
      "unavailable": "이 설명을 불러올 수 없어요. 도움말에서 다른 주제를 선택해 주세요."
    }
  }
};
