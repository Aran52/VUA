const en = {
  sectionTitle: "VRChat quick answers",
  vuaTitle: "VUA guides",
  readMore: "Read the explanation",
  uploadAction: "Check upload requirements",
  eligibilityTitle: "Check your account before upload",
  eligibility: "Uploading a custom Avatar with Unity and the official SDK needs a full VRChat account and the New User trust rank or higher. You can still build and test locally below that rank. VUA cannot raise your rank or skip the official check.",
  trust: {
    title: "Trust rank",
    body: "A full VRChat account with New User or higher can upload custom Avatars. Below that rank, local testing is still available.",
    what: "What is a trust rank?",
    improve: "How do I raise it?",
  },
  shield: {
    title: "Shield Levels",
    body: "Controls which voices, Avatars, particles, shaders and other features you see. This is not upload eligibility or an Avatar's Performance Rank.",
  },
  personalSpace: {
    title: "Personal Space",
    body: "Hides or fades Avatars that come too close. It affects only your view; it does not block the other player or change what they see.",
  },
  untrustedUrls: {
    title: "Unknown links and media",
    body: "Allow Untrusted URLs is off by default. Enabling it lets world media load from external addresses; a link from someone you know is not automatically safe.",
  },
};

export const helpFaqCopy: Record<"en" | "zh-CN" | "zh-TW" | "ja" | "ko", typeof en> = {
  en,
  "zh-CN": {
    sectionTitle: "VRChat 常见问题",
    vuaTitle: "VUA 使用帮助",
    readMore: "查看说明",
    uploadAction: "查看上传条件",
    eligibilityTitle: "上传前确认账号条件",
    eligibility: "用 Unity 和官方 SDK 上传自定义 Avatar，需要完整 VRChat 账号，并达到 New User 或更高信任等级。等级不足时仍可本地测试。VUA 不能替你提升等级或跳过官方检查。",
    trust: {
      title: "用户等级",
      body: "完整 VRChat 账号且达到 New User 或更高，才能上传自定义 Avatar。等级不足仍可本地测试。",
      what: "用户等级是什么？",
      improve: "如何提升？",
    },
    shield: {
      title: "安全显示等级（Shield）",
      body: "决定其他玩家的语音、Avatar、粒子、Shader 等内容是否向你显示。它和上传资格、Avatar 性能等级不是一回事。",
    },
    personalSpace: {
      title: "个人空间",
      body: "开启后，靠得太近的 Avatar 会被隐藏或淡出。只影响你看到的内容，不会屏蔽对方，也不会改变对方看到的内容。",
    },
    untrustedUrls: {
      title: "未知链接与媒体播放",
      body: "Allow Untrusted URLs 默认关闭。开启后，世界中的外部图片、视频等媒体才可能加载；链接来自熟人也不等于安全。",
    },
  },
  "zh-TW": {
    sectionTitle: "VRChat 常見問題",
    vuaTitle: "VUA 使用幫助",
    readMore: "查看說明",
    uploadAction: "查看上傳條件",
    eligibilityTitle: "上傳前確認帳號條件",
    eligibility: "用 Unity 和官方 SDK 上傳自訂 Avatar，需要完整 VRChat 帳號，並達到 New User 或更高信任等級。等級不足時仍可本機測試。VUA 不能替你提升等級或跳過官方檢查。",
    trust: {
      title: "使用者等級",
      body: "完整 VRChat 帳號且達到 New User 或更高，才能上傳自訂 Avatar。等級不足仍可本機測試。",
      what: "使用者等級是什麼？",
      improve: "如何提升？",
    },
    shield: {
      title: "安全顯示等級（Shield）",
      body: "決定其他玩家的語音、Avatar、粒子、Shader 等內容是否向你顯示。它和上傳資格、Avatar 效能等級不是一回事。",
    },
    personalSpace: {
      title: "個人空間",
      body: "開啟後，靠得太近的 Avatar 會被隱藏或淡出。只影響你看到的內容，不會封鎖對方，也不會改變對方看到的內容。",
    },
    untrustedUrls: {
      title: "未知連結與媒體播放",
      body: "Allow Untrusted URLs 預設關閉。開啟後，世界中的外部圖片、影片等媒體才可能載入；連結來自熟人也不等於安全。",
    },
  },
  ja: {
    sectionTitle: "VRChat のよくある質問",
    vuaTitle: "VUA ガイド",
    readMore: "説明を見る",
    uploadAction: "アップロード条件を確認",
    eligibilityTitle: "アップロード前にアカウントを確認",
    eligibility: "Unity と公式 SDK でカスタム Avatar をアップロードするには、VRChat アカウントと New User 以上のトラストランクが必要です。ランクが足りなくてもローカルでテストできます。VUA がランクを上げたり、公式チェックを省略したりすることはできません。",
    trust: {
      title: "トラストランク",
      body: "VRChat アカウントで New User 以上になると、カスタム Avatar をアップロードできます。それ未満でもローカルテストは可能です。",
      what: "トラストランクとは？",
      improve: "ランクを上げるには？",
    },
    shield: {
      title: "Shield Level（安全表示）",
      body: "ほかのプレイヤーの音声、Avatar、パーティクル、Shader などを表示するかを決めます。アップロード資格や Avatar の Performance Rank とは別の設定です。",
    },
    personalSpace: {
      title: "Personal Space",
      body: "近づきすぎた Avatar を非表示またはフェードアウトします。自分の表示だけに影響し、相手をブロックしたり、相手の表示を変えたりはしません。",
    },
    untrustedUrls: {
      title: "不明なリンクとメディア",
      body: "Allow Untrusted URLs は初期値でオフです。オンにするとワールド内の外部メディアを読み込めるようになりますが、知人からのリンクでも自動的に安全になるわけではありません。",
    },
  },
  ko: {
    sectionTitle: "VRChat 자주 묻는 질문",
    vuaTitle: "VUA 사용 도움말",
    readMore: "설명 보기",
    uploadAction: "업로드 조건 보기",
    eligibilityTitle: "업로드 전 계정 확인",
    eligibility: "Unity와 공식 SDK로 커스텀 Avatar를 업로드하려면 VRChat 계정과 New User 이상의 신뢰 등급이 필요해요. 등급이 부족해도 로컬 테스트는 할 수 있어요. VUA가 등급을 올리거나 공식 확인을 우회하지는 않아요.",
    trust: {
      title: "사용자 등급",
      body: "VRChat 계정에서 New User 이상이어야 커스텀 Avatar를 업로드할 수 있어요. 그 미만이어도 로컬 테스트는 가능해요.",
      what: "사용자 등급이란?",
      improve: "어떻게 올리나요?",
    },
    shield: {
      title: "Shield Level(안전 표시)",
      body: "다른 플레이어의 음성, Avatar, 파티클, Shader 등을 표시할지 결정해요. 업로드 자격이나 Avatar Performance Rank와는 다른 설정이에요.",
    },
    personalSpace: {
      title: "Personal Space",
      body: "너무 가까이 온 Avatar를 숨기거나 페이드 아웃해요. 내 화면에만 영향을 주며 상대를 차단하거나 상대의 화면을 바꾸지는 않아요.",
    },
    untrustedUrls: {
      title: "알 수 없는 링크와 미디어",
      body: "Allow Untrusted URLs는 기본으로 꺼져 있어요. 켜면 월드의 외부 미디어를 불러올 수 있지만, 아는 사람의 링크라도 자동으로 안전한 것은 아니에요.",
    },
  },
};
