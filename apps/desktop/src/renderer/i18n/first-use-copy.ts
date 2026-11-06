import type { LocaleId } from "./index.ts";

const en = {
  "notice": {
    "title": "Welcome to VUA",
    "purposeTitle": "What VUA helps with",
    "purpose": "Prepare VRChat software, understand the next step, and assemble Avatars from materials you own. Sign-ins and the final SDK upload stay with you.",
    "changesTitle": "In this version",
    "changes": "One directory, a Home that lists what is waiting, buttons that name the next step, and reinstalling from inside VUA.",
    "changelog": "Read the changelog",
    "sourceTitle": "Open source and free",
    "source": "VUA is open source under Apache 2.0 and free to download on GitHub.",
    "github": "Source and downloads",
    "licensesTitle": "Third-party licenses",
    "licenses": "Bundled software and connected tools have their own licenses.",
    "licensesLink": "Read third-party licenses",
    "enter": "Start the VUA feature tour",
    "beginner": "New to VRChat: getting started"
  },
  "vrchat": {
    "id": "what-is-vrchat",
    "title": "What is VRChat?",
    "paragraphs": [
      "VRChat is a social platform built around worlds and Avatars created by its community. People meet, explore and take part in activities together.",
      "A Windows PC can run VRChat on its screen; a VR headset is optional. Start with desktop play to learn the controls before adding VR hardware."
    ]
  },
  "wizard": {
    "title": "VRChat getting started",
    "progress": "Page {current} of {total}",
    "want": "Go to {page}",
    "next": "Next page",
    "finish": "Finish",
    "previous": "Previous page",
    "article": "Read the full explanation",
        "unavailable": "This explanation is unavailable. Open Help to choose another topic."
  }
};

// All editions await native review; Traditional Chinese uses Taiwan terminology.
export const firstUseCopy: Record<LocaleId, typeof en> = {
  en,
  "zh-CN": {
  "notice": {
    "title": "欢迎使用 VUA",
    "purposeTitle": "VUA 能做什么",
    "purpose": "准备 VRChat 所需软件，解释下一步，并用已有素材组装模型。账号登录和 SDK 最终上传由玩家完成。",
    "changesTitle": "此版本更新",
    "changes": "目录合而为一，首页列出待处理的事项，按钮写明下一步，还能在 VUA 里重新安装软件。",
    "changelog": "查看更新日志",
    "sourceTitle": "开源与免费下载",
    "source": "VUA 以 Apache 2.0 开源，可在 GitHub 免费下载。",
    "github": "源码与下载",
    "licensesTitle": "第三方许可证",
    "licenses": "随附软件和连接的工具各自遵循其许可证。",
    "licensesLink": "查看第三方许可证",
    "enter": "开始 VUA 功能导览",
    "beginner": "还不了解 VRChat：入门向导"
  },
  "vrchat": {
    "id": "what-is-vrchat",
    "title": "什么是 VRChat？",
    "paragraphs": [
      "VRChat 是围绕社区制作的世界与模型展开的社交平台。玩家可以一起见面、探索和参加活动。",
      "Windows 电脑可以直接用屏幕游玩，不必先购买 VR 设备。先用桌面模式熟悉操作，再考虑是否使用头显。"
    ]
  },
  "wizard": {
    "title": "VRChat 入门向导",
    "progress": "第 {current} / {total} 页",
    "want": "跳转至 {page}",
    "next": "下一页",
    "finish": "完成",
    "previous": "上一页",
    "article": "查看完整说明",
        "unavailable": "这条说明暂不可用。可在帮助中选择其他主题。"
  }
},
  "zh-TW": {
  "notice": {
    "title": "歡迎使用 VUA",
    "purposeTitle": "VUA 能做什麼",
    "purpose": "準備 VRChat 所需軟體，說明下一步，並以已有素材組裝模型。帳號登入與 SDK 最終上傳由玩家完成。",
    "changesTitle": "此版本更新",
    "changes": "目錄合而為一，首頁列出待處理的事項，按鈕寫明下一步，還能在 VUA 裡重新安裝軟體。",
    "changelog": "查看更新紀錄",
    "sourceTitle": "開放原始碼與免費下載",
    "source": "VUA 以 Apache 2.0 開放原始碼，可在 GitHub 免費下載。",
    "github": "原始碼與下載",
    "licensesTitle": "第三方授權條款",
    "licenses": "隨附軟體與連接的工具各自遵循其授權條款。",
    "licensesLink": "查看第三方授權條款",
    "enter": "開始 VUA 功能導覽",
    "beginner": "還不了解 VRChat：入門精靈"
  },
  "vrchat": {
    "id": "what-is-vrchat",
    "title": "什麼是 VRChat？",
    "paragraphs": [
      "VRChat 是圍繞社群製作的世界與模型展開的社交平台。玩家可以一起見面、探索與參加活動。",
      "Windows 電腦可以直接使用螢幕遊玩，不必先購買 VR 裝置。先以桌面模式熟悉操作，再考慮是否使用頭戴式裝置。"
    ]
  },
  "wizard": {
    "title": "VRChat 入門精靈",
    "progress": "第 {current} / {total} 頁",
    "want": "跳轉至 {page}",
    "next": "下一頁",
    "finish": "完成",
    "previous": "上一頁",
    "article": "查看完整說明",
        "unavailable": "這則說明暫時無法使用。可在說明中選擇其他主題。"
  }
},
  "ja": {
  "notice": {
    "title": "VUA へようこそ",
    "purposeTitle": "VUA でできること",
    "purpose": "VRChat に必要なソフトを準備し、次の手順を案内し、手持ちの素材からアバターを組み立てます。ログインと SDK での最終アップロードはプレイヤーが行います。",
    "changesTitle": "このバージョンの変更",
    "changes": "目次を一つにまとめ、ホームに対応待ちの項目を表示し、ボタンに次の操作を記載しました。VUA から再インストールもできます。",
    "changelog": "変更履歴を読む",
    "sourceTitle": "オープンソース・無料配布",
    "source": "VUA は Apache 2.0 のオープンソースソフトウェアです。GitHub から無料でダウンロードできます。",
    "github": "ソースコードとダウンロード",
    "licensesTitle": "サードパーティーのライセンス",
    "licenses": "同梱ソフトウェアと連携ツールには、それぞれのライセンスが適用されます。",
    "licensesLink": "サードパーティーのライセンスを読む",
    "enter": "VUA 機能ツアーを開始",
    "beginner": "VRChat が初めて：入門ガイド"
  },
  "vrchat": {
    "id": "what-is-vrchat",
    "title": "VRChat とは？",
    "paragraphs": [
      "VRChat は、コミュニティが作ったワールドとアバターで交流するソーシャルプラットフォームです。一緒に会話したり、探索したり、イベントに参加したりできます。",
      "Windows パソコンの画面だけでも遊べるため、VR 機器は必須ではありません。まずデスクトップモードで操作に慣れてから、VR 機器の利用を考えられます。"
    ]
  },
  "wizard": {
    "title": "VRChat 入門ガイド",
    "progress": "{total} ページ中 {current} ページ",
    "want": "{page} へ移動",
    "next": "次のページ",
    "finish": "完了",
    "previous": "前のページ",
    "article": "詳しい説明を読む",
        "unavailable": "この説明は現在利用できません。ヘルプから別の項目を選んでください。"
  }
},
  "ko": {
  "notice": {
    "title": "VUA에 오신 것을 환영해요",
    "purposeTitle": "VUA가 돕는 일",
    "purpose": "VRChat에 필요한 소프트웨어를 준비하고 다음 단계를 안내하며, 보유한 소재로 아바타를 조립해요. 로그인과 SDK의 최종 업로드는 사용자가 직접 진행해요.",
    "changesTitle": "이번 버전의 변경 사항",
    "changes": "목차를 하나로 합치고, 홈에 처리할 일을 보여 주며, 버튼에 다음 단계를 적었어요. VUA에서 다시 설치도 할 수 있어요.",
    "changelog": "변경 기록 보기",
    "sourceTitle": "오픈 소스와 무료 다운로드",
    "source": "VUA는 Apache 2.0으로 공개된 오픈 소스이며 GitHub에서 무료로 다운로드할 수 있어요.",
    "github": "소스 코드와 다운로드",
    "licensesTitle": "타사 라이선스",
    "licenses": "포함된 소프트웨어와 연결 도구에는 각각의 라이선스가 적용돼요.",
    "licensesLink": "타사 라이선스 보기",
    "enter": "VUA 기능 둘러보기 시작",
    "beginner": "VRChat이 처음이에요: 시작 가이드"
  },
  "vrchat": {
    "id": "what-is-vrchat",
    "title": "VRChat이란?",
    "paragraphs": [
      "VRChat은 커뮤니티가 만든 월드와 아바타를 중심으로 교류하는 소셜 플랫폼이에요. 함께 만나고 탐험하며 여러 활동에 참여할 수 있어요.",
      "Windows PC 화면만으로도 플레이할 수 있어 VR 기기가 필수는 아니에요. 먼저 데스크톱 모드로 조작에 익숙해진 뒤 VR 기기 사용을 생각해 보세요."
    ]
  },
  "wizard": {
    "title": "VRChat 시작 가이드",
    "progress": "{total}쪽 중 {current}쪽",
    "want": "{page}(으)로 이동",
    "next": "다음 쪽",
    "finish": "완료",
    "previous": "이전 쪽",
    "article": "전체 설명 보기",
    "unavailable": "이 설명을 불러올 수 없어요. 도움말에서 다른 주제를 선택하세요."
  }
}
};

