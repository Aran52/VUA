/** The directory's own words: its group headings, the Unity page's name, and what Home asks next. */
const en = {
  unitySetup: "Unity setup",
  home: {
    prepareRoute: "Prepare",
    confirmLine: "Review",
    fixLine: "Fix",
    waitingLine: "Awaiting review",
    attentionLine: "Needs attention",
    moreLines: "{count} more lines in the workshop",
    importMaterials: "Import materials",
    startHint: "Build an Avatar from materials",
    amfOff: "AMF builds Avatars from materials.",
  },
  badge: { waiting: "{count} awaiting review", attention: "{count} need attention", running: "{count} running" },
  workshop: {
    fromMaterials: "From materials",
    fromRecipe: "From a Recipe",
    start: "Start the line",
    startHint: "VUA works out which package is the Avatar and which are outfits, and asks when it cannot tell.",
    noMaterials: "Import an Avatar and an outfit first.",
    importMaterials: "Import materials",
    needUnity: "Unity not ready",
    openUnitySetup: "Open Unity setup",
    toFinished: "View finished Avatar",
  },
  library: {
    selected: "{count} selected",
    openDetails: "Details",
    howToStart: "Select an Avatar to start a line.",
  },
  help: {
    wizardHint: "One concept per page, starting from what VRChat is.",
    tourHint: "Points out the main controls one at a time.",
    gameHint: "A floating guide that follows the VRChat window.",
    encyclopediaHint: "Search VRChat rules, devices, Avatars and uploads.",
  },
};
type Copy = typeof en;

const zhCN: Copy = {
  unitySetup: "Unity 环境",
  home: {
    prepareRoute: "去准备",
    confirmLine: "去确认",
    fixLine: "去处理",
    waitingLine: "待确认",
    attentionLine: "需要处理",
    moreLines: "车间里还有 {count} 条",
    importMaterials: "导入素材",
    startHint: "用素材组装 Avatar",
    amfOff: "AMF 用素材组装 Avatar。",
  },
  badge: { waiting: "{count} 条待确认", attention: "{count} 条需要处理", running: "{count} 条运行中" },
  workshop: {
    fromMaterials: "从素材开始",
    fromRecipe: "从配方开始",
    start: "开工",
    startHint: "VUA 自动判断哪个包是 Avatar、哪些是衣服，无法判断时询问。",
    noMaterials: "先导入一个 Avatar 和一套衣服。",
    importMaterials: "导入素材",
    needUnity: "Unity 未就绪",
    openUnitySetup: "打开 Unity 环境",
    toFinished: "查看成品",
  },
  library: {
    selected: "已选 {count} 项",
    openDetails: "详情",
    howToStart: "选中 Avatar 后可开一条产线。",
  },
  help: {
    wizardHint: "从「什么是 VRChat」开始，一页讲一个概念。",
    tourHint: "依次指出常用入口。",
    gameHint: "跟随 VRChat 窗口的悬浮说明。",
    encyclopediaHint: "查 VRChat 规则、设备、Avatar 和上传。",
  },
};

const zhTW: Copy = {
  unitySetup: "Unity 環境",
  home: {
    prepareRoute: "去準備",
    confirmLine: "去確認",
    fixLine: "去處理",
    waitingLine: "待確認",
    attentionLine: "需要處理",
    moreLines: "車間裡還有 {count} 條",
    importMaterials: "匯入素材",
    startHint: "用素材組裝 Avatar",
    amfOff: "AMF 用素材組裝 Avatar。",
  },
  badge: { waiting: "{count} 條待確認", attention: "{count} 條需要處理", running: "{count} 條執行中" },
  workshop: {
    fromMaterials: "從素材開始",
    fromRecipe: "從配方開始",
    start: "開工",
    startHint: "VUA 自動判斷哪個包是 Avatar、哪些是衣服，無法判斷時詢問。",
    noMaterials: "先匯入一個 Avatar 和一套衣服。",
    importMaterials: "匯入素材",
    needUnity: "Unity 未就緒",
    openUnitySetup: "開啟 Unity 環境",
    toFinished: "查看成品",
  },
  library: {
    selected: "已選 {count} 項",
    openDetails: "詳情",
    howToStart: "選中 Avatar 後可開一條產線。",
  },
  help: {
    wizardHint: "從「什麼是 VRChat」開始，一頁講一個概念。",
    tourHint: "依序指出常用入口。",
    gameHint: "跟隨 VRChat 視窗的懸浮說明。",
    encyclopediaHint: "查 VRChat 規則、裝置、Avatar 和上傳。",
  },
};

const ja: Copy = {
  unitySetup: "Unity 環境",
  home: {
    prepareRoute: "準備する",
    confirmLine: "確認する",
    fixLine: "対応する",
    waitingLine: "確認待ち",
    attentionLine: "対応が必要",
    moreLines: "工房にあと {count} 本",
    importMaterials: "素材を取り込む",
    startHint: "素材でアバターを組み立てる",
    amfOff: "AMF は素材からアバターを組み立てます。",
  },
  badge: { waiting: "{count} 本が確認待ち", attention: "{count} 本に対応が必要", running: "{count} 本が稼働中" },
  workshop: {
    fromMaterials: "素材から始める",
    fromRecipe: "レシピから始める",
    start: "着手する",
    startHint: "どれがアバターでどれが衣装かは VUA が判断し、判断できないときだけ確認します。",
    noMaterials: "先にアバターと衣装を取り込んでください。",
    importMaterials: "素材を取り込む",
    needUnity: "Unity の準備が未完了",
    openUnitySetup: "Unity 環境を開く",
    toFinished: "完成品を見る",
  },
  library: {
    selected: "{count} 件を選択中",
    openDetails: "詳細",
    howToStart: "アバターを選ぶと生産ラインを始められます。",
  },
  help: {
    wizardHint: "「VRChat とは」から、1 ページに 1 つずつ説明します。",
    tourHint: "主な操作場所を順に案内します。",
    gameHint: "VRChat のウィンドウに追従する案内です。",
    encyclopediaHint: "VRChat のルール、デバイス、アバター、アップロードを調べます。",
  },
};

const ko: Copy = {
  unitySetup: "Unity 환경",
  home: {
    prepareRoute: "준비하기",
    confirmLine: "확인하기",
    fixLine: "처리하기",
    waitingLine: "확인 대기",
    attentionLine: "처리 필요",
    moreLines: "작업장에 {count}개 더",
    importMaterials: "소재 가져오기",
    startHint: "소재로 아바타 조립",
    amfOff: "AMF는 소재로 아바타를 조립해요.",
  },
  badge: { waiting: "{count}개 확인 대기", attention: "{count}개 처리 필요", running: "{count}개 실행 중" },
  workshop: {
    fromMaterials: "소재에서 시작",
    fromRecipe: "레시피에서 시작",
    start: "작업 시작",
    startHint: "어떤 패키지가 아바타이고 어떤 것이 의상인지는 VUA가 판단하고, 판단할 수 없을 때만 물어봐요.",
    noMaterials: "먼저 아바타와 의상을 가져오세요.",
    importMaterials: "소재 가져오기",
    needUnity: "Unity 준비 안 됨",
    openUnitySetup: "Unity 환경 열기",
    toFinished: "완성품 보기",
  },
  library: {
    selected: "{count}개 선택됨",
    openDetails: "세부 정보",
    howToStart: "아바타를 선택하면 생산 라인을 시작할 수 있어요.",
  },
  help: {
    wizardHint: "‘VRChat이란’부터 한 페이지에 한 가지씩 설명해요.",
    tourHint: "주요 조작 위치를 하나씩 보여줘요.",
    gameHint: "VRChat 창을 따라다니는 안내예요.",
    encyclopediaHint: "VRChat 규칙, 기기, 아바타, 업로드를 찾아봐요.",
  },
};

export const structureCopy = { en, "zh-CN": zhCN, "zh-TW": zhTW, ja, ko };
