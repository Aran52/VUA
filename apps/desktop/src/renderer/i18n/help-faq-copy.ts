const en = {
  sectionTitle: "Common VRChat questions",
  vuaTitle: "Guides & reference",
  readMore: "Learn more",
  uploadAction: "View upload requirements",
  eligibilityTitle: "Account requirements for uploading",
  eligibility: "Uploading a custom avatar through Unity and the official SDK requires a full VRChat account at New User trust rank or higher. Below New User, you can still build and test avatars locally on your PC.",
  trust: {
    title: "Trust Rank",
    body: "You need a full VRChat account at New User rank or higher to upload custom avatars. Until then, you can still test builds locally.",
    what: "What is Trust Rank?",
    improve: "How to reach New User",
  },
  shield: {
    title: "Safety Shield",
    body: "Controls whose voices, avatars, particles, and custom shaders appear on your screen. Separate from upload eligibility and Avatar Performance Rank.",
  },
  personalSpace: {
    title: "Personal Space",
    body: "Automatically fades out avatars that step too close to your view. Only changes what you see—it never blocks or notifies the other player.",
  },
  untrustedUrls: {
    title: "External links & world media",
    body: "Allow Untrusted URLs is off by default. Turn it on when worlds need to play external video streams or images, and only open links from sources you trust.",
  },
};

export const helpFaqCopy: Record<"en" | "zh-CN" | "zh-TW" | "ja" | "ko", typeof en> = {
  en,
  "zh-CN": {
    sectionTitle: "VRChat 常见问题",
    vuaTitle: "引导与资料",
    readMore: "查看详细说明",
    uploadAction: "查看上传条件",
    eligibilityTitle: "上传前请确认账号条件",
    eligibility: "使用 Unity 和官方 SDK 上传自定义 Avatar，需要绑定完整 VRChat 账号并达到 New User 及以上信任等级。等级暂未达到时，仍可先在本机生成测试模型自己穿戴查看。",
    trust: {
      title: "信任等级（Trust Rank）",
      body: "拥有完整 VRChat 账号且达到 New User 及以上等级，才能正式上传自定义 Avatar；未达标前仍可在本地构建测试。",
      what: "什么是信任等级？",
      improve: "怎么升到 New User？",
    },
    shield: {
      title: "安全屏蔽等级（Shield）",
      body: "控制不同信任等级玩家的语音、模型、粒子特效和自定义 Shader 是否向你显示，与账号上传资格或模型性能等级互不相关。",
    },
    personalSpace: {
      title: "个人空间（Personal Space）",
      body: "开启后会自动淡出贴得太近的其他玩家模型，仅作用于你自己的画面，不会拉黑对方，对方也不会收到提示。",
    },
    untrustedUrls: {
      title: "外部链接与世界媒体播放",
      body: "“Allow Untrusted URLs”默认关闭。开启后世界里的视频播放器、外部图片和音乐才能正常加载；即使链接来自熟人，也请确认来源可靠再打开。",
    },
  },
  "zh-TW": {
    sectionTitle: "VRChat 常見問題",
    vuaTitle: "引導與資料",
    readMore: "查看詳細說明",
    uploadAction: "查看上傳條件",
    eligibilityTitle: "上傳前請確認帳號條件",
    eligibility: "使用 Unity 與官方 SDK 上傳自訂 Avatar，必須具備完整 VRChat 帳號並達到 New User 以上的信任等級。等級未達標前，仍可先在本機建置測試模型。",
    trust: {
      title: "信任等級（Trust Rank）",
      body: "具備完整 VRChat 帳號且達 New User 以上等級，才能正式上傳自訂 Avatar；等級不足時仍可進行本機測試。",
      what: "什麼是信任等級？",
      improve: "如何升上 New User？",
    },
    shield: {
      title: "安全防護等級（Shield）",
      body: "決定其他玩家的語音、Avatar、粒子特效與自訂 Shader 是否顯示在你的畫面上，與上傳資格或 Avatar 效能等級無關。",
    },
    personalSpace: {
      title: "個人空間（Personal Space）",
      body: "開啟後會自動淡出靠得太近的其他玩家模型，只影響你自己的畫面，不會封鎖對方，也不會改變對方看到的內容。",
    },
    untrustedUrls: {
      title: "外部連結與世界媒體播放",
      body: "「Allow Untrusted URLs」預設為關閉。開啟後世界內的外部影片、圖片與音樂才能載入；就算連結由熟人提供，仍請留意來源是否可信。",
    },
  },
  ja: {
    sectionTitle: "VRChat のよくある質問",
    vuaTitle: "ガイドと資料",
    readMore: "詳しく見る",
    uploadAction: "アップロード条件を確認",
    eligibilityTitle: "アップロード前のアカウント条件",
    eligibility: "Unity と公式 SDK でカスタムアバターをアップロードするには、VRChat 本アカウントと New User 以上のトラストランクが必要です。ランク到達前でも、手元でのローカルテスト（Build & Test）は利用できます。",
    trust: {
      title: "トラストランク（Trust Rank）",
      body: "VRChat 本アカウントで New User 以上になるとカスタムアバターをアップロードできます。未到達の間もローカルテストで動作確認が可能です。",
      what: "トラストランクとは？",
      improve: "New User に上がるには？",
    },
    shield: {
      title: "セーフティ（Shield Level）",
      body: "他のプレイヤーの声・アバター・パーティクル・シェーダーなどをどこまで表示するかを決める設定です。アップロード権限やパフォーマンスランクとは別物です。",
    },
    personalSpace: {
      title: "パーソナルスペース（Personal Space）",
      body: "近づきすぎた相手のアバターを自動で透明化・非表示にします。自分の画面だけに反映され、相手をブロックしたり相手に通知されたりすることはありません。",
    },
    untrustedUrls: {
      title: "外部 URL とワールド内の動画再生",
      body: "「Allow Untrusted URLs」は初期状態でオフです。オンにするとワールド内の動画プレイヤーや外部画像を読み込めますが、知人から共有された URL であっても接続先の安全性には注意してください。",
    },
  },
  ko: {
    sectionTitle: "VRChat 자주 묻는 질문",
    vuaTitle: "가이드 및 참고 자료",
    readMore: "자세히 보기",
    uploadAction: "업로드 조건 확인",
    eligibilityTitle: "업로드 전 계정 조건 확인",
    eligibility: "Unity와 공식 SDK로 커스텀 아바타를 업로드하려면 정식 VRChat 계정과 New User 이상의 신뢰 등급(Trust Rank)이 필요해요. 아직 등급이 낮더라도 내 PC에서 확인하는 로컬 테스트는 바로 이용할 수 있어요.",
    trust: {
      title: "신뢰 등급(Trust Rank)",
      body: "정식 VRChat 계정에서 New User 이상이 되면 커스텀 아바타를 업로드할 수 있어요. 그전에도 로컬 테스트 빌드는 사용할 수 있어요.",
      what: "신뢰 등급이란?",
      improve: "New User가 되려면?",
    },
    shield: {
      title: "세이프티 등급(Shield Level)",
      body: "다른 플레이어의 음성, 아바타, 파티클, 셰이더 등을 내 화면에 어디까지 표시할지 정하는 설정이에요. 업로드 자격이나 아바타 퍼포먼스 등급과는 별개예요.",
    },
    personalSpace: {
      title: "개인 공간(Personal Space)",
      body: "너무 가까이 다가온 아바타를 내 화면에서만 흐리게 하거나 숨겨 줘요. 상대를 차단하거나 상대 화면에 영향을 주지는 않아요.",
    },
    untrustedUrls: {
      title: "외부 링크와 월드 미디어 재생",
      body: "‘Allow Untrusted URLs’는 기본으로 꺼져 있어요. 켜 두면 월드 안의 비디오 플레이어나 외부 이미지를 불러올 수 있지만, 지인이 보낸 링크라도 출처가 안전한지는 직접 살피는 것이 좋아요.",
    },
  },
};
