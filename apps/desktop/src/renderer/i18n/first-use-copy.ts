import type { LocaleId } from "./index.ts";

const en = {
  "notice": {
    "title": "Welcome to VUA",
    "purposeTitle": "What VUA helps with",
    "purpose": "Prepare VRChat software, understand the next step, and assemble Avatars from materials you own. Sign-ins and the final SDK upload stay with you.",
    "changesTitle": "In this version",
    "changes": "A station list, library migration grouping, estimates before assembly, and clearer setup guidance.",
    "changelog": "Read the changelog",
    "sourceTitle": "Open source and free",
    "source": "VUA is open source under Apache 2.0 and free to download on GitHub.",
    "github": "Source and downloads",
    "licensesTitle": "Third-party licenses",
    "licenses": "Bundled software and connected tools have their own licenses.",
    "licensesLink": "Read third-party licenses",
    "enter": "Got it",
    "beginner": "I have no idea what to do"
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
    "title": "Getting started",
    "progress": "Concept {current} of {total}",
    "want": "I want this",
    "know": "Got it",
    "previous": "Previous concept",
    "article": "Read the full explanation",
    "manual": "VUA manual",
    "resume": "Back to getting started",
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
    "changes": "工位列表、资料库迁移分组、组装前预估，以及更清晰的准备引导。",
    "changelog": "查看更新日志",
    "sourceTitle": "开源与免费下载",
    "source": "VUA 以 Apache 2.0 开源，可在 GitHub 免费下载。",
    "github": "源码与下载",
    "licensesTitle": "第三方许可证",
    "licenses": "随附软件和连接的工具各自遵循其许可证。",
    "licensesLink": "查看第三方许可证",
    "enter": "知道了",
    "beginner": "我完全不知道要做什么"
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
    "title": "入门向导",
    "progress": "第 {current} / {total} 个概念",
    "want": "我要这个",
    "know": "我知道了",
    "previous": "上一个概念",
    "article": "查看完整说明",
    "manual": "VUA 使用手册",
    "resume": "返回入门向导",
    "unavailable": "这条说明暂不可用。可在帮助中选择其他主题。"
  }
},
  "zh-TW": {
  "notice": {
    "title": "歡迎使用 VUA",
    "purposeTitle": "VUA 能做什麼",
    "purpose": "準備 VRChat 所需軟體，說明下一步，並以已有素材組裝模型。帳號登入與 SDK 最終上傳由玩家完成。",
    "changesTitle": "此版本更新",
    "changes": "工作站清單、素材庫移轉分組、組裝前預估，以及更清楚的準備引導。",
    "changelog": "查看更新紀錄",
    "sourceTitle": "開放原始碼與免費下載",
    "source": "VUA 以 Apache 2.0 開放原始碼，可在 GitHub 免費下載。",
    "github": "原始碼與下載",
    "licensesTitle": "第三方授權條款",
    "licenses": "隨附軟體與連接的工具各自遵循其授權條款。",
    "licensesLink": "查看第三方授權條款",
    "enter": "知道了",
    "beginner": "我完全不知道要做什麼"
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
    "title": "入門精靈",
    "progress": "第 {current} / {total} 個概念",
    "want": "我要這個",
    "know": "我知道了",
    "previous": "上一個概念",
    "article": "查看完整說明",
    "manual": "VUA 使用手冊",
    "resume": "返回入門精靈",
    "unavailable": "這則說明暫時無法使用。可在說明中選擇其他主題。"
  }
},
  "ja": {
  "notice": {
    "title": "VUA へようこそ",
    "purposeTitle": "VUA でできること",
    "purpose": "VRChat に必要なソフトを準備し、次の手順を案内し、手持ちの素材からアバターを組み立てます。ログインと SDK での最終アップロードはプレイヤーが行います。",
    "changesTitle": "このバージョンの変更",
    "changes": "工程一覧、ライブラリ移行時のグループ分け、組み立て前の見積もり、準備手順の案内を追加しました。",
    "changelog": "変更履歴を読む",
    "sourceTitle": "オープンソース・無料配布",
    "source": "VUA は Apache 2.0 のオープンソースソフトウェアです。GitHub から無料でダウンロードできます。",
    "github": "ソースコードとダウンロード",
    "licensesTitle": "サードパーティーのライセンス",
    "licenses": "同梱ソフトウェアと連携ツールには、それぞれのライセンスが適用されます。",
    "licensesLink": "サードパーティーのライセンスを読む",
    "enter": "わかりました",
    "beginner": "何から始めればよいかわかりません"
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
    "title": "はじめてのガイド",
    "progress": "{total} 項目中 {current} 項目",
    "want": "これを使いたい",
    "know": "わかりました",
    "previous": "前の項目",
    "article": "詳しい説明を読む",
    "manual": "VUA マニュアル",
    "resume": "はじめてのガイドに戻る",
    "unavailable": "この説明は現在利用できません。ヘルプから別の項目を選んでください。"
  }
},
  "ko": {
  "notice": {
    "title": "VUA에 오신 것을 환영해요",
    "purposeTitle": "VUA가 돕는 일",
    "purpose": "VRChat에 필요한 소프트웨어를 준비하고 다음 단계를 안내하며, 보유한 소재로 아바타를 조립해요. 로그인과 SDK의 최종 업로드는 사용자가 직접 진행해요.",
    "changesTitle": "이번 버전의 변경 사항",
    "changes": "제작 단계 목록, 라이브러리 이전 시 그룹 나누기, 조립 전 예상 수치와 준비 안내가 추가됐어요.",
    "changelog": "변경 기록 보기",
    "sourceTitle": "오픈 소스와 무료 다운로드",
    "source": "VUA는 Apache 2.0으로 공개된 오픈 소스이며 GitHub에서 무료로 다운로드할 수 있어요.",
    "github": "소스 코드와 다운로드",
    "licensesTitle": "타사 라이선스",
    "licenses": "포함된 소프트웨어와 연결 도구에는 각각의 라이선스가 적용돼요.",
    "licensesLink": "타사 라이선스 보기",
    "enter": "알겠어요",
    "beginner": "무엇부터 해야 할지 모르겠어요"
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
    "title": "처음 시작하기",
    "progress": "{total}개 개념 중 {current}번째",
    "want": "이걸 사용할래요",
    "know": "알겠어요",
    "previous": "이전 개념",
    "article": "전체 설명 보기",
    "manual": "VUA 사용 설명서",
    "resume": "처음 시작하기로 돌아가기",
    "unavailable": "이 설명을 불러올 수 없어요. 도움말에서 다른 주제를 선택하세요."
  }
}
};
