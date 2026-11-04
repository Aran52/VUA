import type { GuidePageContent } from "../features/guide/guide-content.ts";
import type { LocaleId } from "./index.ts";

const en: GuidePageContent = {
  title: "Avatars & outfits",
  intro: "Change an Avatar, understand what others see, and check an outfit before publishing it.",
  sections: [
    { id: "change-avatar", title: "Change an Avatar and choose who can use it", paragraphs: [
      "Open the Avatars menu in VRChat to choose an uploaded Avatar. Public Avatars can be cloned where the game allows it; private ones are for you. Cloning is not the same as downloading the source project.",
      "A visible Avatar still needs a compatible build for the viewer's platform. An outfit's package does not change that requirement.",
    ] },
    { id: "visibility", title: "Why can't others see my Avatar?", paragraphs: [
      "A local test Avatar is visible only to you. Other players need an uploaded build that supports their platform.",
      "If it worked before, check their platform and whether they changed Shield or performance filters. A fallback, removed components or a hidden Avatar does not by itself prove an upload failure.",
    ] },
    { id: "performance", title: "Performance Rank is not Trust Rank or an FPS promise", paragraphs: [
      "Performance Rank estimates an Avatar's rendering cost from its structure. Trust Rank belongs to the account and affects upload eligibility. They answer different questions.",
      "PC and mobile filters have different defaults. A rank can help you find expensive parts, but it cannot guarantee frame rate or whether another player will show your Avatar.",
    ] },
    { id: "outfit-assembly", title: "Avatar, outfit, UnityPackage, FBX, Prefab and VPM", paragraphs: [
      "Separate the base Avatar from what you want to add. Check the outfit author's instructions, intended Avatar and dependencies; a filename alone is not compatibility evidence.",
      "A UnityPackage imports assets into Unity, FBX is a model file, a Prefab is a composed Unity object, and VPM distributes packages. Modular Avatar can combine outfits without destructively editing the original during its build processing; it does not make every outfit automatically compatible.",
    ] },
    { id: "outfit-diagnostics", title: "The outfit does not follow, is misplaced or clips", paragraphs: [
      "If it does not follow, check the intended Avatar, armature and mounting first. For a misplaced outfit, inspect its parent, scale and pose. Clipping calls for checking shape and skin weights.",
      "For a switch, inspect the Animator parameters and menu. For physical movement, inspect PhysBones and collisions. Change one thing at a time and test again; do not blame every symptom on a missing plugin.",
    ] },
    { id: "pink-materials", title: "Why is the Avatar or outfit pink?", paragraphs: [
      "Check that the materials, textures and packages required by the author are present. Missing files and missing dependencies are different problems; pink alone cannot distinguish them.",
      "Then check shader support for the target platform and rendering path. Follow the author's requirements rather than installing an arbitrary shader and calling it fixed.",
    ] },
    { id: "build-test", title: "Build & Test is not Build & Publish", paragraphs: [
      "Build & Test lets you see a local test Avatar without the New User rank. Build & Publish uploads to your account and requires a full VRChat account and New User or higher.",
      "Before publishing, review the name, visibility, content tags and SDK validation. VUA can retain production records; the final publish remains your explicit action in the official SDK.",
    ] },
    { id: "platforms", title: "PC and mobile need compatible builds", paragraphs: [
      "PC and Android/Quest have different asset limits and display defaults. Build the version for the intended platform; a PC upload alone does not guarantee that mobile players see the same Avatar.",
      "Textures, meshes, materials and bones all affect cost, but a rank is not an FPS guarantee. Test on the target platform before deciding what to optimize or whether to provide a fallback.",
    ] },
  ],
};

export const guideAvatarCopy: Record<LocaleId, GuidePageContent> = {
  en,
  "zh-CN": {
    title: "Avatar 与换装",
    intro: "更换模型、弄清别人看到的内容，并在上传前检查衣服。",
    sections: [
      { id: "change-avatar", title: "更换 Avatar，选择谁能使用", paragraphs: [
        "在 VRChat 的 Avatars 菜单中选择已经上传的模型。公开模型可以按游戏规则被他人克隆，私有模型只供自己使用；克隆不等于拿到源工程。",
        "别人能否看到，还取决于有没有适用于其平台的构建。衣服包不会改变这个要求。",
      ] },
      { id: "visibility", title: "为什么别人看不到我的模型？", paragraphs: [
        "本地测试的 Avatar 只有自己能看到。想让别人看到，需要上传，并提供适用于对方平台的构建。",
        "如果之前能看到，先确认对方用的平台，以及是否调整了 Shield 或性能过滤。看到 Fallback、部分组件被关掉或模型被隐藏，不等于上传失败。",
      ] },
      { id: "performance", title: "性能等级不是信任等级，也不是帧率保证", paragraphs: [
        "Performance Rank 根据模型结构估算渲染开销；Trust Rank 属于账号，会影响上传资格。两者回答的是不同问题。",
        "PC 和移动端的默认过滤级别不同。等级可以帮助找出高开销部件，但不能保证帧率，也不能保证对方一定显示你的模型。",
      ] },
      { id: "outfit-assembly", title: "模型、衣服、UnityPackage、FBX、Prefab 和 VPM", paragraphs: [
        "先分清模型本体和要添加的衣服。是否合适，要看作者说明、目标模型和依赖，不能只看文件名。",
        "UnityPackage 用来把资源带入 Unity；FBX 是模型文件；Prefab 是 Unity 中组合好的对象；VPM 是包管理分发方式。Modular Avatar 能在构建时以非破坏方式组合衣服，但不代表所有衣服都能自动适配。",
      ] },
      { id: "outfit-diagnostics", title: "衣服不跟随、错位或穿模", paragraphs: [
        "衣服不动，先检查目标模型、骨架和挂载位置。错位时检查父级、缩放和姿态；穿模则检查形状与蒙皮权重。",
        "开关不灵时检查动画参数和菜单，物理效果异常时检查 PhysBone 和碰撞。一次只改一项再测试，不要把所有问题都归结为缺插件。",
      ] },
      { id: "pink-materials", title: "模型或衣服为什么变粉？", paragraphs: [
        "先确认材质、贴图和作者要求的包是否都已导入。缺文件和没装依赖是两种问题，不能只凭粉色判断原因。",
        "再检查 Shader 是否支持目标平台和渲染路径。按作者说明逐项核对，不要随手装一个 Shader 就当作修复完成。",
      ] },
      { id: "build-test", title: "Build & Test 不等于 Build & Publish", paragraphs: [
        "Build & Test 只给自己看，不要求 New User；Build & Publish 会上传到账号，要求完整 VRChat 账号和 New User 或更高等级。",
        "发布前检查名称、可见范围、内容标签和 SDK 验证结果。VUA 可以保留制作记录，但最后发布仍由你在官方 SDK 中确认。",
      ] },
      { id: "platforms", title: "PC 和移动端需要各自的兼容构建", paragraphs: [
        "PC 和 Android/Quest 的素材限制与默认过滤不同，需要分别构建对应版本。只上传 PC 版本，不能保证移动端玩家看到同一个 Avatar。",
        "贴图、网格、材质和骨骼都会影响开销，但性能等级不是帧率保证。先在目标平台实测，再决定优化什么，以及是否需要 Fallback。",
      ] },
    ],
  },
  "zh-TW": {
    title: "Avatar 與換裝",
    intro: "更換模型、弄清別人看到的內容，並在上傳前檢查衣服。",
    sections: [
      { id: "change-avatar", title: "更換 Avatar，選擇誰能使用", paragraphs: [
        "在 VRChat 的 Avatars 選單中選擇已上傳的模型。公開模型可依遊戲規則被他人複製，私有模型只供自己使用；複製不等於取得原始專案。",
        "別人能否看到，也取決於是否有適用其平台的構建。衣服包不會改變這個條件。",
      ] },
      { id: "visibility", title: "為什麼別人看不到我的模型？", paragraphs: [
        "本地測試的 Avatar 只有自己能看到。要讓別人看到，需要上傳並提供適用對方平台的構建。",
        "如果之前能看到，先確認對方的平台，以及是否調整了 Shield 或效能過濾。看到 Fallback、部分元件被關閉或模型被隱藏，不等於上傳失敗。",
      ] },
      { id: "performance", title: "效能等級不是信任等級，也不是幀率保證", paragraphs: [
        "Performance Rank 根據模型結構估算渲染成本；Trust Rank 屬於帳號，會影響上傳資格。兩者回答的是不同問題。",
        "PC 和行動裝置的預設過濾級別不同。等級可協助找出高成本元件，但不能保證幀率，也不能保證對方一定顯示你的模型。",
      ] },
      { id: "outfit-assembly", title: "模型、衣服、UnityPackage、FBX、Prefab 和 VPM", paragraphs: [
        "先分清模型本體和要新增的衣服。是否合適，要看作者說明、目標模型和依賴，不能只看檔名。",
        "UnityPackage 用來把資源帶入 Unity；FBX 是模型檔；Prefab 是 Unity 中組合好的物件；VPM 是套件管理分發方式。Modular Avatar 能在構建時以非破壞方式組合衣服，但不代表所有衣服都能自動相容。",
      ] },
      { id: "outfit-diagnostics", title: "衣服不跟隨、錯位或穿模", paragraphs: [
        "衣服不動，先檢查目標模型、骨架和掛載位置。錯位時檢查父級、縮放和姿態；穿模則檢查形狀與蒙皮權重。",
        "開關不靈時檢查動畫參數和選單，物理效果異常時檢查 PhysBone 和碰撞。一次只改一項再測試，不要把所有問題都歸結為缺外掛。",
      ] },
      { id: "pink-materials", title: "模型或衣服為什麼變粉？", paragraphs: [
        "先確認材質、貼圖和作者要求的套件是否都已匯入。缺檔案和缺少依賴是兩種問題，不能只憑粉色判斷原因。",
        "再檢查 Shader 是否支援目標平台和渲染路徑。按作者說明逐項核對，不要隨手裝一個 Shader 就當作修復完成。",
      ] },
      { id: "build-test", title: "Build & Test 不等於 Build & Publish", paragraphs: [
        "Build & Test 只給自己看，不要求 New User；Build & Publish 會上傳到帳號，要求完整 VRChat 帳號和 New User 或更高等級。",
        "發佈前檢查名稱、可見範圍、內容標籤和 SDK 驗證結果。VUA 可以保留製作記錄，但最後發佈仍由你在官方 SDK 中確認。",
      ] },
      { id: "platforms", title: "PC 和行動裝置需要各自的相容構建", paragraphs: [
        "PC 和 Android/Quest 的素材限制與預設過濾不同，需要分別構建對應版本。只上傳 PC 版本，不能保證行動裝置玩家看到同一個 Avatar。",
        "貼圖、網格、材質和骨骼都會影響成本，但效能等級不是幀率保證。先在目標平台實測，再決定最佳化什麼，以及是否需要 Fallback。",
      ] },
    ],
  },
  ja: {
    title: "Avatar と衣装",
    intro: "Avatar の変更、ほかの人からの見え方、衣装の公開前チェック。",
    sections: [
      { id: "change-avatar", title: "Avatar を変更し、公開範囲を選ぶ", paragraphs: [
        "VRChat の Avatars メニューでアップロード済みの Avatar を選びます。Public はゲームのルールで Clone できますが、Private は自分用です。Clone とソースプロジェクトの取得は別物です。",
        "表示には、見る人のプラットフォームに対応したビルドも必要です。衣装パッケージがこの条件を変えることはありません。",
      ] },
      { id: "visibility", title: "ほかの人に Avatar が見えない理由", paragraphs: [
        "ローカルテストの Avatar は自分にだけ見えます。ほかの人に見せるには、アップロードと、その人のプラットフォーム向けビルドが必要です。",
        "以前は見えたなら、相手のプラットフォームと、Shield やパフォーマンスフィルターの変更を確認します。Fallback、一部コンポーネントの無効化、Avatar の非表示だけでアップロード失敗とは判断できません。",
      ] },
      { id: "performance", title: "Performance Rank と Trust Rank、FPS の違い", paragraphs: [
        "Performance Rank は Avatar の構造から描画コストを推定します。Trust Rank はアカウントのもので、アップロード資格に関係します。別の仕組みです。",
        "PC とモバイルでは既定のフィルターが異なります。ランクは高負荷な部分を探す手掛かりですが、FPS や表示の可否を保証しません。",
      ] },
      { id: "outfit-assembly", title: "Avatar、衣装、UnityPackage、FBX、Prefab、VPM", paragraphs: [
        "Avatar 本体と追加する衣装を分けて考えます。作者の説明、対象 Avatar、依存関係を確認し、ファイル名だけで互換性を判断しません。",
        "UnityPackage は Unity にアセットを取り込む形式、FBX はモデルファイル、Prefab は Unity のまとまったオブジェクト、VPM はパッケージの配布仕組みです。Modular Avatar はビルド時に非破壊で衣装を統合できますが、すべてが自動で合うわけではありません。",
      ] },
      { id: "outfit-diagnostics", title: "衣装が追従しない、ずれる、貫通する", paragraphs: [
        "追従しない場合は対象 Avatar、アーマチュア、取り付け位置を確認します。ずれる場合は親、スケール、姿勢、貫通する場合は形状とスキンウェイトを調べます。",
        "スイッチは Animator パラメーターとメニュー、物理挙動は PhysBone と衝突を確認します。一度に一つずつ変更して試し、すべてをプラグイン不足のせいにしません。",
      ] },
      { id: "pink-materials", title: "Avatar や衣装がピンクになる理由", paragraphs: [
        "マテリアル、テクスチャ、作者が指定するパッケージがそろっているか確認します。ファイル不足と依存関係不足は別の問題で、ピンクだけでは区別できません。",
        "次に Shader が対象プラットフォームと描画パスに対応するか確認します。作者の説明に沿って確認し、適当な Shader を入れただけで直ったと判断しません。",
      ] },
      { id: "build-test", title: "Build & Test と Build & Publish の違い", paragraphs: [
        "Build & Test は自分だけが見るローカルテストで、New User は不要です。Build & Publish はアカウントへのアップロードで、VRChat アカウントと New User 以上が必要です。",
        "名前、公開範囲、コンテンツタグ、SDK の検証結果を確認します。VUA は制作記録を残せますが、公開は公式 SDK で本人が行います。",
      ] },
      { id: "platforms", title: "PC とモバイルに対応したビルド", paragraphs: [
        "PC と Android/Quest ではアセット制限と既定の表示フィルターが異なります。対象プラットフォーム向けにビルドしないと、PC 版だけではモバイルで同じ Avatar を見られません。",
        "テクスチャ、メッシュ、マテリアル、ボーンはいずれもコストに関係しますが、ランクは FPS の保証ではありません。対象環境で試してから、最適化や Fallback の要否を決めます。",
      ] },
    ],
  },
  ko: {
    title: "Avatar와 의상",
    intro: "Avatar 변경, 다른 사람에게 보이는 모습, 업로드 전 의상 점검.",
    sections: [
      { id: "change-avatar", title: "Avatar 변경과 사용 범위 선택", paragraphs: [
        "VRChat의 Avatars 메뉴에서 업로드한 Avatar를 고르세요. Public은 게임 규칙에 따라 Clone할 수 있고 Private은 본인 전용이에요. Clone과 원본 프로젝트 다운로드는 다릅니다.",
        "보는 사람의 플랫폼에 맞는 빌드도 필요해요. 의상 패키지가 이 조건을 바꾸지는 않아요.",
      ] },
      { id: "visibility", title: "다른 사람에게 내 Avatar가 보이지 않을 때", paragraphs: [
        "로컬 테스트 Avatar는 본인에게만 보여요. 다른 사람에게 보이려면 업로드와 상대 플랫폼용 빌드가 필요해요.",
        "이전에 보였다면 상대의 플랫폼과 Shield 또는 성능 필터 변경을 확인하세요. Fallback, 일부 컴포넌트 비활성화, Avatar 숨김만으로 업로드 실패라고 단정할 수 없어요.",
      ] },
      { id: "performance", title: "Performance Rank와 Trust Rank, FPS의 차이", paragraphs: [
        "Performance Rank는 Avatar 구조로 렌더링 비용을 추정해요. Trust Rank는 계정의 등급으로 업로드 자격과 관련돼요. 서로 다른 개념이에요.",
        "PC와 모바일의 기본 필터는 달라요. 등급은 비용이 큰 부분을 찾는 단서이지 FPS나 표시 여부를 보장하지 않아요.",
      ] },
      { id: "outfit-assembly", title: "Avatar, 의상, UnityPackage, FBX, Prefab, VPM", paragraphs: [
        "Avatar 본체와 추가할 의상을 구분하세요. 제작자의 설명, 대상 Avatar, 의존성을 확인하고 파일명만으로 호환성을 판단하지 마세요.",
        "UnityPackage는 Unity에 에셋을 가져오는 형식, FBX는 모델 파일, Prefab은 Unity의 조합된 오브젝트, VPM은 패키지 배포 방식이에요. Modular Avatar는 빌드할 때 비파괴적으로 의상을 결합할 수 있지만 모든 의상이 자동으로 맞는 것은 아니에요.",
      ] },
      { id: "outfit-diagnostics", title: "의상이 따라오지 않거나 어긋나고 뚫릴 때", paragraphs: [
        "따라오지 않으면 대상 Avatar, 아마추어, 부착 위치를 확인하세요. 어긋나면 부모, 스케일, 포즈를, 뚫리면 형태와 스킨 가중치를 살펴요.",
        "스위치는 Animator 파라미터와 메뉴, 물리 움직임은 PhysBone과 충돌을 확인하세요. 한 번에 하나씩 바꾸고 시험하며 모든 원인을 플러그인 부족으로 돌리지 마세요.",
      ] },
      { id: "pink-materials", title: "Avatar나 의상이 분홍색으로 보일 때", paragraphs: [
        "머티리얼, 텍스처, 제작자가 요구한 패키지가 모두 있는지 확인하세요. 파일 부족과 의존성 부족은 다른 문제이며 분홍색만으로 구분할 수 없어요.",
        "Shader가 대상 플랫폼과 렌더링 경로를 지원하는지도 확인하세요. 제작자의 안내에 따라 확인하고, 아무 Shader나 넣었다고 해결됐다고 판단하지 마세요.",
      ] },
      { id: "build-test", title: "Build & Test와 Build & Publish의 차이", paragraphs: [
        "Build & Test는 본인만 보는 로컬 테스트라 New User가 필요 없어요. Build & Publish는 계정에 업로드하며 정식 VRChat 계정과 New User 이상이 필요해요.",
        "이름, 공개 범위, 콘텐츠 태그, SDK 검증 결과를 확인하세요. VUA는 제작 기록을 남길 수 있지만 게시는 공식 SDK에서 본인이 진행해요.",
      ] },
      { id: "platforms", title: "PC와 모바일에 맞는 빌드", paragraphs: [
        "PC와 Android/Quest는 에셋 제한과 기본 표시 필터가 달라요. 대상 플랫폼용으로 빌드해야 하며 PC 업로드만으로 모바일에서 같은 Avatar를 볼 수 있는 것은 아니에요.",
        "텍스처, 메시, 머티리얼, 본 모두 비용에 영향을 주지만 등급이 FPS를 보장하지는 않아요. 대상 환경에서 확인한 뒤 최적화와 Fallback 필요 여부를 정하세요.",
      ] },
    ],
  },
};
