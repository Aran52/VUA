import type { GuidePageContent } from "../features/guide/guide-content.ts";
import type { LocaleId } from "./index.ts";

const en: GuidePageContent = {
  title: "Avatars & outfits",
  intro: "Switching avatars, understanding why others see or hide your model, and checking outfits before upload.",
  sections: [
    { id: "change-avatar", title: "Switch avatars and understand Public vs. Private", paragraphs: [
      "Open the Avatars menu in VRChat to switch to any avatar uploaded to your account. Public avatars can be cloned by other players when cloning is enabled, whereas Private avatars can only be worn by you. Cloning lets someone wear a public avatar in-game, not download its Unity project files.",
      "For others to see your avatar properly, it also needs a build for their platform (PC or Android/Quest). Adding an outfit package does not create cross-platform builds automatically.",
    ] },
    { id: "visibility", title: "Why can't other players see my avatar?", paragraphs: [
      "A local test build (Build & Test) only exists on your PC, so nobody else can see it. For other players to see your avatar, you must upload it to your account with a build matching their platform.",
      "If others could see it before, check whether they are on Quest/mobile or if their Safety Shield or Performance Rank filter is hiding it. Seeing a fallback avatar, muted shaders, or a blocked model usually comes from viewer-side filters rather than a failed upload.",
    ] },
    { id: "performance", title: "Performance Rank vs. Trust Rank and FPS", paragraphs: [
      "Performance Rank estimates how heavy an avatar is to render based on triangles, material slots, bones, and textures. Trust Rank belongs to your VRChat account and determines whether you can upload content. The two systems are completely separate.",
      "PC and mobile platforms apply different default display limits. Improving Performance Rank helps prevent your avatar from being auto-hidden, though actual FPS still depends on everyone else in the instance.",
    ] },
    { id: "outfit-assembly", title: "Base avatars, outfits, UnityPackage, FBX, Prefab, and VPM", paragraphs: [
      "Keep the base avatar separate from the outfits and accessories you add to it. Before combining them, check the creator's product notes for supported base avatars and required packages rather than guessing from file names.",
      "In Unity workflows, a UnityPackage bundles assets for import, an FBX stores the 3D mesh and rig, a Prefab saves a configured object hierarchy, and VPM manages tool and shader packages. Modular Avatar merges outfits non-destructively at build time, though outfits made for a different base still need manual fitting.",
    ] },
    { id: "outfit-diagnostics", title: "When an outfit won't follow, sits crooked, or clips", paragraphs: [
      "If an outfit stays still while the body moves, check that it matches your base avatar, armature naming, and attachment bone. If it floats or sits at the wrong size, inspect its parent object, scale, and rest pose; if skin pokes through fabric, check blendshapes (shrink shapes) and weight painting.",
      "When toggles fail to respond, check Animator parameters and Expressions menu wiring; when hair or skirts behave strangely, check PhysBone settings and colliders. Change one thing at a time when testing—not every glitch comes from a missing package.",
    ] },
    { id: "pink-materials", title: "Why did my avatar or outfit turn bright pink?", paragraphs: [
      "Bright magenta (pink) means Unity cannot render the material's shader. First verify that both the outfit's texture/material files and every shader package listed by the creator (such as lilToon or Poiyomi) are installed in the project.",
      "If the shader package is already installed, check whether the material uses a broken shader reference or a shader unsupported on the current target platform (such as Quest/Android). Reassign the shader recommended by the creator rather than picking a random replacement.",
    ] },
    { id: "build-test", title: "Build & Test vs. Build & Publish", paragraphs: [
      "Build & Test compiles a local test avatar on your PC without requiring New User rank—ideal for checking proportions, toggles, and shading in VRChat first. Build & Publish uploads the avatar to VRChat's servers and requires a full VRChat account at New User rank or above.",
      "Before publishing, review the avatar name, release visibility (Private vs. Public), content warnings, and SDK validation list. VUA prepares the project and keeps build records, while the final upload always happens with your confirmation in the official SDK.",
    ] },
    { id: "platforms", title: "Building for both PC and Android / Quest", paragraphs: [
      "PC VR and standalone Android/Quest headsets use separate asset limits, shader rules, and default visibility thresholds. Uploading only a Windows PC build means Quest and mobile players will see a fallback instead of your avatar.",
      "Textures, polygon count, material slots, and PhysBones all count toward platform limits. Test directly on the target platform before deciding which assets to trim or whether to upload a dedicated Quest version or custom fallback.",
    ] },
  ],
};

export const guideAvatarCopy: Record<LocaleId, GuidePageContent> = {
  en,
  "zh-CN": {
    title: "Avatar 与换装",
    intro: "切换模型、排查别人看不见的原因，以及上传前必看的穿衣与材质检查要点。",
    sections: [
      { id: "change-avatar", title: "切换 Avatar 与公开 / 私有权限", paragraphs: [
        "在 VRChat 的 Avatars 菜单中可以切换账号下已上传的模型。设为 Public（公开）的模型在开启允许克隆时可被其他玩家现场换上；设为 Private（私有）则只有你自己能穿。注意，游戏内克隆只是穿上公开模型，拿不到原始工程文件。",
        "其他玩家能否正常看到你的模型，还取决于你是否上传了对应平台（PC 或 Android/Quest）的构建版本，单靠导入衣服包并不会自动生成跨平台版本。",
      ] },
      { id: "visibility", title: "为什么别人看不到我的模型？", paragraphs: [
        "通过本地测试（Build & Test）生成的模型只保存在你自己的电脑上，其他人看不到。想让房间里的玩家看到，必须正式上传账号，并为你想覆盖的平台完成构建。",
        "如果以前别人能看到、突然看不到了，先确认对方是不是在用一体机/手机端，或者对方的 Shield 安全等级、性能过滤（Performance Rank）是否把你的模型拦住了。对方看到替代模型（Fallback）、特效被关或模型被隐藏，通常是对方本地过滤设置所致，不代表上传失败。",
      ] },
      { id: "performance", title: "性能等级、信任等级与帧率不是一回事", paragraphs: [
        "Performance Rank（性能等级）根据面数、材质球、骨骼和贴图估算单个模型的渲染开销；Trust Rank（信任等级）则是你的账号资历，决定能不能上传模型。两者互不相干。",
        "PC 端与移动端的默认性能过滤档位不同。留意性能等级有助于找出拖慢渲染的高消耗部件、避免被系统默认隐藏，但它本身并不能保证实际游戏帧率，也不决定别人是否手动屏蔽模型。",
      ] },
      { id: "outfit-assembly", title: "分清素体、衣服与 UnityPackage / FBX / Prefab / VPM", paragraphs: [
        "改模前先分清“模型本体（素体）”和要往身上穿的“衣服 / 配饰”。判断一套衣服能不能穿，要看商品说明里写明的适配素体和前置依赖插件，不能只看文件名猜。",
        "常见格式各有分工：UnityPackage 用来把整套素材导入 Unity；FBX 是三维网格与骨骼模型文件；Prefab 是组装好层级与组件的预制体；VPM 则负责安装和管理插件包。Modular Avatar 能在构建时非破坏性地把衣服合到素体上，但非对应素体的衣服依然需要手动改型适配。",
      ] },
      { id: "outfit-diagnostics", title: "衣服不跟骨骼动、位置歪斜或破皮穿模", paragraphs: [
        "角色走动时衣服留在原地，先查是不是买错了适配素体、骨骼命名不匹配或挂错了父节点；衣服悬空、大小不对时检查父级位置、缩放比例和初始姿态；皮肤露出来穿模时，优先查看素体自带的缩体形变键（BlendShapes）和蒙皮权重。",
        "换装开关失灵时检查 Animator 参数与菜单绑定；裙摆、头发乱飞时检查 PhysBone 动骨和碰撞体设置。每次只改动一处再进测试，别急着把所有异常都当成少装了插件。",
      ] },
      { id: "pink-materials", title: "模型或衣服为什么变成亮粉色？", paragraphs: [
        "画面出现刺眼的紫粉色（洋红色），说明 Unity 无法正常渲染材质所用的着色器（Shader）。先核对衣服本身的贴图、材质文件是否齐全，以及作者要求的前置着色器包（如 lilToon、Poiyomi）是否已经装进工程。",
        "如果插件包已经装好仍然发粉，再检查材质球是否丢了 Shader 引用，或者当前切换到了不支持该 Shader 的平台（如 Android/Quest）。请按作者说明重新指定对应 Shader，不要随便换个默认材质球应付。",
      ] },
      { id: "build-test", title: "本地测试（Build & Test）与正式上传（Build & Publish）", paragraphs: [
        "Build & Test 只在本机生成测试模型，不要求 New User 等级，适合先穿进游戏检查比例、开关和光影；Build & Publish 才会把模型正式传到你的账号下，要求绑定完整 VRChat 账号且达到 New User 及以上等级。",
        "正式发布前请核对模型名称、公开或私有范围、内容分级标签以及 SDK 检查项。VUA 会帮你准备工程并留存构建记录，最终发布始终由你在官方 SDK 中亲自确认。",
      ] },
      { id: "platforms", title: "PC 端与移动端（Quest）需要分别构建", paragraphs: [
        "PC 端和 Android/Quest 端对贴图、面数、材质和着色器的限制完全不同，默认显示门槛也差很多。只上传 Windows PC 版本时，一体机和移动端玩家只能看到默认替代形象。",
        "贴图显存、网格面数、材质槽位和动骨数量都会计入平台限制。建议先在目标平台实测表现，再决定压缩哪些资源，或者单独制作移动端版本与自定义 Fallback。",
      ] },
    ],
  },
  "zh-TW": {
    title: "Avatar 與換裝",
    intro: "切換模型、釐清別人看不到的原因，以及上傳前必看的穿搭與材質檢查重點。",
    sections: [
      { id: "change-avatar", title: "切換 Avatar 與公開／私人權限", paragraphs: [
        "在 VRChat 的 Avatars 選單中可切換帳號內已上傳的模型。設為 Public（公開）的模型在允許複製（Clone）時，其他玩家可在遊戲內直接換上；設為 Private（私人）則僅限自己使用。請注意，遊戲內複製只是穿上公開模型，並不會取得原始 Unity 專案檔。",
        "其他玩家能否看到你的模型，還取決於是否上傳了對應平台（PC 或 Android/Quest）的建置版本，單純匯入衣服套件不會自動產生跨平台版本。",
      ] },
      { id: "visibility", title: "為什麼別人看不到我的模型？", paragraphs: [
        "透過本機測試（Build & Test）產生的模型只存在你的電腦裡，只有自己看得到。想讓其他玩家看見，必須上傳至帳號，並提供支援對方平台的建置版本。",
        "如果之前看得到、後來卻看不到，請先確認對方是否使用獨立型頭顯／行動版，或對方的 Shield 安全等級、效能過濾（Performance Rank）是否將模型擋下了。看到 Fallback 替代模型、部分特效關閉或模型被隱藏，通常是對方畫面的顯示過濾設定所致，不等於上傳失敗。",
      ] },
      { id: "performance", title: "效能等級、信任等級與 FPS 的差別", paragraphs: [
        "Performance Rank（效能等級）會依面數、材質球、骨骼與貼圖估算模型的渲染負擔；Trust Rank（信任等級）則是帳號資歷，決定是否具備上傳資格。兩者是完全獨立的機制。",
        "PC 與行動平台的預設顯示過濾門檻不同。參考效能等級有助於找出吃資源的部件、避免被預設隱藏，但等級本身無法保證實際遊玩影格率（FPS），也不保證對方一定會開啟顯示。",
      ] },
      { id: "outfit-assembly", title: "分清素體、衣服與 UnityPackage／FBX／Prefab／VPM", paragraphs: [
        "改模前請先區分「模型本體（素體）」與要加裝的「衣服／配件」。判斷衣服是否適用，應以作者說明頁標示的對應素體與必要套件為準，不能只看檔名猜測。",
        "常見格式各有用途：UnityPackage 用於將素材匯入 Unity；FBX 是 3D 網格與骨架模型檔；Prefab 是已在 Unity 組裝好的預製物件；VPM 則負責管理與安裝工具套件。Modular Avatar 可在建置時以非破壞方式整合衣服，但非對應素體的服裝仍需手動調整。",
      ] },
      { id: "outfit-diagnostics", title: "衣服不跟隨動作、位置偏移或破皮穿模", paragraphs: [
        "角色移動時衣服卻停在原地，請先確認對應素體、骨架命名與掛載節點是否正確；衣服位置偏移或大小異常時檢查父物件、縮放比例與初始姿勢；身體穿出衣服（穿模）時，優先檢查素體的縮體 BlendShapes 與蒙皮權重。",
        "換裝開關沒反應時檢查 Animator 參數與選單設定；頭髮或裙擺亂飄時檢查 PhysBone 與碰撞體。每次只調整一個項目再測試，不必把所有問題都歸咎於缺少外掛套件。",
      ] },
      { id: "pink-materials", title: "模型或衣服為什麼變成亮粉紅色？", paragraphs: [
        "畫面呈現刺眼的洋紅色（粉紅色），代表 Unity 無法正常載入材質使用的 Shader。請先確認衣服的貼圖、材質檔是否完整匯入，以及作者指定的 Shader 套件（如 lilToon、Poiyomi）是否已安裝至專案。",
        "若套件已安裝仍顯示粉紅，請再檢查材質球是否遺失 Shader 參照，或目前切換到了不支援該 Shader 的目標平台（例如 Android/Quest）。請依作者說明套用正確的 Shader，不要隨意換成其他無關的 Shader。",
      ] },
      { id: "build-test", title: "本機測試（Build & Test）與正式發佈（Build & Publish）", paragraphs: [
        "Build & Test 僅在本機產生測試用模型，不需要 New User 等級，適合先進遊戲確認比例、開關與光影表現；Build & Publish 則會將模型正式上傳至帳號，必須具備完整 VRChat 帳號且達 New User 以上等級。",
        "正式發佈前請確認模型名稱、公開／私人範圍、內容標籤與 SDK 檢查結果。VUA 可協助準備專案並保留建置紀錄，最終上傳仍由你在官方 SDK 中親自確認。",
      ] },
      { id: "platforms", title: "PC 與行動平台（Quest）需要分別建置", paragraphs: [
        "PC 與 Android/Quest 的資源上限、Shader 規範與預設顯示門檻截然不同。若只上傳 Windows PC 版本，Quest 與行動裝置玩家只會看到預設的替代模型。",
        "貼圖記憶體、網格面數、材質數量與骨架都會影響平台限制。建議先在目標平台上實際測試，再決定要精簡哪些項目，或另行準備行動版與專屬 Fallback。",
      ] },
    ],
  },
  ja: {
    title: "アバターと衣装",
    intro: "アバターの切り替え、周囲からの見え方の仕組み、アップロード前に押さえておきたい衣装チェックの基本。",
    sections: [
      { id: "change-avatar", title: "アバターの切り替えと Public / Private の違い", paragraphs: [
        "VRChat 内の Avatars メニューから、アカウントにアップロード済みのアバターへ着替えられます。Public（公開）アバターはクローン許可時に他のプレイヤーもその場で着用でき、Private（非公開）は自分専用になります。なお、クローンはゲーム内で同じアバターを着る機能であり、元の Unity プロジェクトやデータが渡るわけではありません。",
        "他のプレイヤーに表示されるには、相手の環境（PC または Android/Quest）に対応したビルドがアップロードされている必要があります。衣装のパッケージを入れただけでは他プラットフォーム向けビルドにはなりません。",
      ] },
      { id: "visibility", title: "ほかの人に自分のアバターが見えないときは", paragraphs: [
        "ローカルテスト（Build & Test）で出力したアバターは自分の PC 内にしか存在しないため、他のプレイヤーには見えません。周囲に見せるにはアカウントへアップロードし、相手のプラットフォームに対応したビルドを用意する必要があります。",
        "以前は見えていたのに見えなくなった場合は、相手が Quest / モバイル版を使っていないか、セーフティ（Shield Level）やパフォーマンスランク制限で非表示になっていないかを確認してください。フォールバック表示や一部ギミックの無効化は相手側の表示設定によるものが多く、アップロード自体の失敗とは限りません。",
      ] },
      { id: "performance", title: "Performance Rank と Trust Rank・FPS の違い", paragraphs: [
        "Performance Rank（パフォーマンスランク）はポリゴン数やマテリアル数、ボーン、テクスチャなどから描画負荷の目安を示す指標です。一方、Trust Rank（トラストランク）はアカウントの信頼度であり、アップロード権限に関わります。両者はまったく別の仕組みです。",
        "PC とモバイルでは初期状態の表示制限ラインが異なります。ランクは重い箇所を見つける目安になりますが、実際の FPS を保証するものでも、相手が必ず表示してくれる保証でもありません。",
      ] },
      { id: "outfit-assembly", title: "素体・衣装と UnityPackage / FBX / Prefab / VPM の役割", paragraphs: [
        "改変ではまずベースとなる「アバター素体」と、そこへ着せる「衣装・アクセサリー」を分けて考えます。対応状況はファイル名だけで判断せず、ショップページや同梱説明書に記載された対応アバターと前提パッケージを確認してください。",
        "UnityPackage は Unity へ素材一式を取り込む配布形式、FBX は 3D モデル本体、Prefab は設定済みのオブジェクト構成、VPM はツールやシェーダーを管理・導入する仕組みです。Modular Avatar を使えばビルド時に非破壊で衣装を着せられますが、非対応素体向けの衣装が自動で体型に合うわけではありません。",
      ] },
      { id: "outfit-diagnostics", title: "衣装が追従しない・位置がずれる・素体が貫通する", paragraphs: [
        "動いたときに衣装だけその場に残る場合は、対応アバターの間違いやアーマチュア構造・追従先ボーンの不一致を最初に確認します。位置ずれやサイズ違いは親オブジェクト・スケール・初期ポーズを、肌の貫通は素体側のシュリンク用シェイプキー（BlendShape）やウェイトを確認してください。",
        "出し入れスイッチが効かないときは Animator パラメーターとメニュー設定を、髪やスカートの挙動がおかしいときは PhysBone とコライダーを見直します。すべてをツール不足と決めつけず、一つずつ変更して動作確認するのが近道です。",
      ] },
      { id: "pink-materials", title: "アバターや衣装がピンク色（マゼンタ）になる理由", paragraphs: [
        "鮮やかなピンク色になるのは、マテリアルに設定されたシェーダーを Unity が描画できていないサインです。まずは衣装のテクスチャやマテリアルが欠けていないか、作者指定のシェーダーパッケージ（lilToon や Poiyomi など）が導入済みかを確認します。",
        "パッケージ導入済みでもピンクになる場合は、シェーダー参照が外れているか、Quest / Android など対象プラットフォーム非対応のシェーダーが使われています。適当なシェーダーで間に合わせず、作者の案内に沿って指定シェーダーを設定し直してください。",
      ] },
      { id: "build-test", title: "ローカルテスト（Build & Test）と公開（Build & Publish）", paragraphs: [
        "Build & Test は自分だけが見える手元用のテスト出力で、New User 未満でも利用できます。着替えギミックや見た目の確認に便利です。Build & Publish はアカウントへの本アップロードで、VRChat 本アカウントと New User 以上のトラストランクが必要です。",
        "アップロード前にはアバター名、公開範囲（Private / Public）、コンテンツタグ、SDK のチェック項目を確認します。VUA はプロジェクト準備と制作履歴の記録を担い、最終的なアップロード実行は公式 SDK 上でプレイヤー自身が行います。",
      ] },
      { id: "platforms", title: "PC 向けとモバイル（Quest）向けのビルドは別々に必要", paragraphs: [
        "PC と Android / Quest では使用できるシェーダーや容量制限、既定の表示フィルターが大きく異なります。Windows PC 向けだけをアップロードしても、Quest 単体やモバイル版のプレイヤーには同じアバターは表示されません。",
        "テクスチャ容量、ポリゴン数、マテリアル数、PhysBone 数はすべて負荷に関わります。やみくもに削る前にまず対象環境で実機確認し、どこを軽量化するか、専用フォールバックを用意するかを決めるのが確実です。",
      ] },
    ],
  },
  ko: {
    title: "아바타와 의상",
    intro: "아바타 바꾸기, 다른 사람 화면에서 보이는 원리, 업로드 전 의상·머티리얼 점검 핵심.",
    sections: [
      { id: "change-avatar", title: "아바타 변경과 Public / Private 공개 범위", paragraphs: [
        "VRChat의 Avatars 메뉴에서 내 계정에 업로드된 아바타로 갈아입을 수 있어요. Public(공개) 아바타는 클론이 허용된 경우 다른 플레이어도 현장에서 따라 입을 수 있고, Private(비공개)은 본인만 착용할 수 있어요. 클론은 게임 안에서 아바타를 입어 보는 기능일 뿐 원본 Unity 프로젝트를 내려받는 것이 아니에요.",
        "다른 사람에게 아바타가 제대로 보이려면 상대방 플랫폼(PC 또는 Android/Quest)에 맞는 빌드도 업로드되어 있어야 해요. 의상 패키지를 추가한다고 해서 다른 플랫폼용 빌드가 자동으로 생기지는 않아요.",
      ] },
      { id: "visibility", title: "다른 사람에게 내 아바타가 보이지 않을 때", paragraphs: [
        "로컬 테스트(Build & Test)로 만든 아바타는 내 PC에만 존재해서 본인에게만 보여요. 다른 플레이어에게 보이게 하려면 계정에 정식으로 업로드하고 상대방 플랫폼에 맞는 빌드를 준비해야 해요.",
        "예전에는 보였는데 갑자기 안 보인다면 상대방이 Quest·모바일 환경인지, 또는 세이프티(Shield Level)나 퍼포먼스 등급 필터로 가려졌는지 먼저 확인해 보세요. 폴백(Fallback) 아바타로 대체되거나 일부 기능이 꺼진 모습은 대개 상대방의 표시 설정 때문이며 업로드 실패를 뜻하지는 않아요.",
      ] },
      { id: "performance", title: "Performance Rank와 Trust Rank, FPS의 차이", paragraphs: [
        "Performance Rank(퍼포먼스 등급)는 폴리곤 수, 머티리얼 슬롯, 본, 텍스처 등을 바탕으로 렌더링 부담을 가늠하는 지표예요. 반면 Trust Rank(신뢰 등급)는 계정에 부여되는 등급으로 아바타 업로드 자격과 연결돼요. 두 등급은 전혀 다른 시스템이에요.",
        "PC와 모바일은 기본 표시 필터 기준이 서로 달라요. 퍼포먼스 등급은 무거운 요소를 찾고 자동 숨김을 피하는 데 도움이 되지만, 실제 FPS나 상대방의 표시 여부를 무조건 보장해 주지는 않아요.",
      ] },
      { id: "outfit-assembly", title: "베이스 아바타·의상과 UnityPackage / FBX / Prefab / VPM", paragraphs: [
        "개변을 시작할 때는 기본 몸체가 되는 ‘베이스 아바타(소체)’와 추가로 입힐 ‘의상·액세서리’를 먼저 구분해야 해요. 파일 이름만 보고 짐작하지 말고, 판매 페이지나 동봉 안내문에서 대응 아바타와 필수 패키지를 꼭 확인하세요.",
        "UnityPackage는 에셋 묶음을 Unity로 가져오는 파일, FBX는 3D 모델 파일, Prefab은 조립이 끝난 Unity 오브젝트, VPM은 툴과 셰이더 패키지를 설치·관리하는 방식이에요. Modular Avatar를 쓰면 빌드할 때 원본을 훼손하지 않고 의상을 합칠 수 있지만, 다른 체형용 의상까지 자동으로 맞춰 주지는 않아요.",
      ] },
      { id: "outfit-diagnostics", title: "의상이 몸을 안 따라오거나 어긋나고 살이 뚫릴 때", paragraphs: [
        "움직일 때 의상만 제자리에 남는다면 대응 아바타가 맞는지, 아마추어(본 구조)와 장착 위치가 올바른지부터 확인하세요. 위치나 크기가 어긋나면 부모 오브젝트·스케일·기본 포즈를, 피부가 옷 밖으로 뚫고 나오면 소체의 체형 축소용 블렌드셰이프(BlendShape)와 스킨 웨이트를 살펴보세요.",
        "온오프 토글이 작동하지 않으면 Animator 파라미터와 메뉴 연결을, 머리카락이나 치마 움직임이 이상하면 PhysBone과 콜라이더를 점검해 보세요. 모든 증상을 플러그인 누락 탓으로 돌리지 말고 한 번에 한 가지씩 수정하며 확인하는 것이 좋아요.",
      ] },
      { id: "pink-materials", title: "아바타나 의상이 분홍색(마젠타)으로 보일 때", paragraphs: [
        "선명한 분홍색(마젠타)은 머티리얼에 지정된 셰이더를 Unity가 렌더링하지 못할 때 나타나요. 우선 의상의 텍스처·머티리얼 파일이 빠짐없이 들어 있는지, 제작자가 안내한 필수 셰이더 패키지(lilToon, Poiyomi 등)가 설치되어 있는지 확인하세요.",
        "패키지를 설치했는데도 분홍색이라면 셰이더 참조가 끊어졌거나 Android/Quest처럼 현재 빌드 플랫폼이 지원하지 않는 셰이더가 쓰인 경우예요. 임의로 아무 셰이더나 넣지 말고 제작자 안내에 맞는 셰이더로 다시 지정해 주세요.",
      ] },
      { id: "build-test", title: "로컬 테스트(Build & Test)와 정식 업로드(Build & Publish)", paragraphs: [
        "Build & Test는 내 화면에서만 확인하는 로컬 테스트 빌드라 New User 등급이 없어도 바로 써 볼 수 있어요. Build & Publish는 계정에 아바타를 정식 등록하는 단계로, 정식 VRChat 계정과 New User 이상의 신뢰 등급이 필요해요.",
        "게시하기 전에 아바타 이름, 공개 범위(Private / Public), 콘텐츠 태그, SDK 점검 항목을 확인하세요. VUA는 프로젝트 준비와 제작 기록 보관을 돕고, 최종 업로드는 공식 SDK에서 본인이 직접 확인해 진행해요.",
      ] },
      { id: "platforms", title: "PC용과 모바일(Quest)용 빌드는 각각 준비하기", paragraphs: [
        "PC와 Android/Quest는 허용되는 에셋 용량, 셰이더 규칙, 기본 표시 필터가 크게 달라요. Windows PC 버전만 업로드하면 Quest 단독 기기나 모바일 플레이어에게는 같은 아바타가 보이지 않아요.",
        "텍스처 메모리, 메시 폴리곤, 머티리얼 수, 본 개수가 모두 플랫폼 제한에 영향을 줘요. 대상 기기에서 먼저 확인해 본 뒤 어떤 요소를 줄일지, 별도 모바일 빌드나 전용 Fallback을 둘지 결정해 보세요.",
      ] },
    ],
  },
};
