import type { GuideContentIdV01, GuideContentPayloadV01, GuideLocalizedTextV01 } from "@vua/contracts";

const SHARED_VERIFICATION: GuideLocalizedTextV01 = {
  en: "Confirm the official page loaded over HTTPS in your browser.",
  "zh-CN": "确认浏览器已通过 HTTPS 打开对应的官方页面。",
  "zh-TW": "確認瀏覽器已透過 HTTPS 開啟對應的官方頁面。",
  ja: "ブラウザーで公式ページが HTTPS 接続で開いたことを確認します。",
  ko: "브라우저에서 공식 페이지가 HTTPS로 열렸는지 확인합니다.",
};

const SHARED_ON_FAILURE: GuideLocalizedTextV01 = {
  en: "Run VUA's website reachability check to inspect connectivity to the required site.",
  "zh-CN": "运行 VUA 的网站连通性检查，排查与目标站点的网络连接。",
  "zh-TW": "執行 VUA 的網站連通性檢查，排查與目標網站的網路連線。",
  ja: "VUA のサイト接続チェックを実行し、対象サイトへの通信状態を確認します。",
  ko: "VUA의 웹사이트 연결 확인을 실행해 필요한 사이트와의 연결 상태를 점검합니다.",
};

interface BaselineGuideInput {
  id: GuideContentIdV01;
  title: GuideLocalizedTextV01;
  stepId: string;
  action: GuideLocalizedTextV01;
  expectedResult: GuideLocalizedTextV01;
  site: string;
  sourceLabelEn: string;
  linkLabel: GuideLocalizedTextV01;
  sourceUrl: string;
}

function baselineGuide(input: BaselineGuideInput): GuideContentPayloadV01 {
  return {
    schemaVersion: "0.1",
    id: input.id,
    sequence: 1,
    version: "0.1.0",
    minClient: "1.0.0",
    sop: {
      title: input.title,
      appliesTo: {
        software: ["VUA >=1.0.0"],
        devices: ["Windows PC"],
        region: "outside-japan",
      },
      prerequisites: [],
      steps: [
        {
          id: input.stepId,
          performer: "vua_opens",
          action: input.action,
          expectedResult: input.expectedResult,
          verification: SHARED_VERIFICATION,
          onFailure: SHARED_ON_FAILURE,
          sites: [input.site],
          links: [{ label: input.linkLabel, url: input.sourceUrl }],
        },
      ],
      regionalNotes: [],
      lastVerification: {
        date: "2026-11-07",
        environment: "Bundled v1 baseline reference",
        softwareVersions: ["VUA 1.0.0"],
      },
      sources: [{ label: input.sourceLabelEn, url: input.sourceUrl }],
    },
    assets: {},
  };
}

export const BUNDLED_GUIDE_CONTENT: Readonly<Record<GuideContentIdV01, GuideContentPayloadV01>> = Object.freeze({
  "pixiv-account": baselineGuide({
    id: "pixiv-account",
    title: {
      en: "pixiv account setup for BOOTH",
      "zh-CN": "注册 BOOTH 所需的 pixiv 账号",
      "zh-TW": "註冊 BOOTH 所需的 pixiv 帳號",
      ja: "BOOTH 利用に必要な pixiv アカウントの作成",
      ko: "BOOTH 이용을 위한 pixiv 계정 만들기",
    },
    stepId: "open-pixiv-signup",
    action: {
      en: "Open the official pixiv registration page and enable two-step verification.",
      "zh-CN": "打开 pixiv 官方注册页面完成注册，并开启两步验证。",
      "zh-TW": "開啟 pixiv 官方註冊頁面完成註冊，並啟用兩步驟驗證。",
      ja: "pixiv 公式の登録ページを開いてアカウントを作成し、二段階認証を有効にします。",
      ko: "pixiv 공식 가입 페이지를 열어 계정을 만들고 2단계 인증을 켭니다.",
    },
    expectedResult: {
      en: "Your pixiv account can sign in to BOOTH.",
      "zh-CN": "可使用该 pixiv 账号登录 BOOTH。",
      "zh-TW": "可使用該 pixiv 帳號登入 BOOTH。",
      ja: "作成した pixiv アカウントで BOOTH にログインできるようになります。",
      ko: "만든 pixiv 계정으로 BOOTH에 로그인할 수 있습니다.",
    },
    site: "accounts.pixiv.net",
    sourceLabelEn: "pixiv registration",
    linkLabel: {
      en: "pixiv registration",
      "zh-CN": "pixiv 注册页面",
      "zh-TW": "pixiv 註冊頁面",
      ja: "pixiv アカウント登録",
      ko: "pixiv 가입 페이지",
    },
    sourceUrl: "https://accounts.pixiv.net/signup",
  }),
  "booth-prices": baselineGuide({
    id: "booth-prices",
    title: {
      en: "Understanding BOOTH prices and variants",
      "zh-CN": "了解 BOOTH 价格与商品规格",
      "zh-TW": "了解 BOOTH 價格與商品規格",
      ja: "BOOTH の価格表記と商品バリエーションの見方",
      ko: "BOOTH 가격 표기와 상품 옵션 살펴보기",
    },
    stepId: "review-booth-item",
    action: {
      en: "Check Japanese yen pricing, variants, and supported base Avatars on the product page.",
      "zh-CN": "在商品页核对日元标价、可选规格，以及明确标注支持的素体型号。",
      "zh-TW": "在商品頁核對日圓標價、可選規格，以及明確標示支援的素體型號。",
      ja: "商品ページで日本円の価格、バリエーション（Full Pack／単体など）、対応アバター表記を確認します。",
      ko: "상품 페이지에서 엔화 가격, 세부 옵션, 지원 베이스 아바타 표기를 확인합니다.",
    },
    expectedResult: {
      en: "You identify the exact base Avatar and outfit variant before buying.",
      "zh-CN": "在下单前确认自己需要的素体与服装规格。",
      "zh-TW": "在下單前確認自己需要的素體與服裝規格。",
      ja: "購入前に必要な素体と衣装バリエーションを正確に把握できます。",
      ko: "구매 전에 필요한 베이스 아바타와 의상 옵션을 정확히 구분할 수 있습니다.",
    },
    site: "booth.pm",
    sourceLabelEn: "BOOTH",
    linkLabel: {
      en: "BOOTH",
      "zh-CN": "BOOTH 官网",
      "zh-TW": "BOOTH 官網",
      ja: "BOOTH",
      ko: "BOOTH",
    },
    sourceUrl: "https://booth.pm/",
  }),
  "paying-abroad": baselineGuide({
    id: "paying-abroad",
    title: {
      en: "Paying abroad on BOOTH",
      "zh-CN": "在 BOOTH 跨境支付",
      "zh-TW": "在 BOOTH 跨國付款",
      ja: "日本国外からの BOOTH 決済方法",
      ko: "BOOTH 해외 결제 수단 확인하기",
    },
    stepId: "review-payment-methods",
    action: {
      en: "Review BOOTH's official payment methods page for accepted international cards and PayPal.",
      "zh-CN": "查看 BOOTH 官方支付说明页，确认当前支持的国际信用卡品牌与 PayPal 结算规则。",
      "zh-TW": "查看 BOOTH 官方付款說明頁，確認目前支援的國際信用卡品牌與 PayPal 結帳規則。",
      ja: "BOOTH 公式のお支払い方法ページで、利用可能な国際ブランドカードや PayPal の条件を確認します。",
      ko: "BOOTH 공식 결제 수단 안내 페이지에서 지원되는 해외 카드 브랜드와 PayPal 이용 조건을 확인합니다.",
    },
    expectedResult: {
      en: "You confirm an accepted payment method for your card-issuing region.",
      "zh-CN": "确认适合你发卡地区的可行支付方式。",
      "zh-TW": "確認適合你發卡地區的可行付款方式。",
      ja: "お住まいの地域・カード発行国で利用できる決済手段を確認できます。",
      ko: "카드 발급 국가에서 사용할 수 있는 결제 수단을 확인할 수 있습니다.",
    },
    site: "booth.pm",
    sourceLabelEn: "BOOTH payment methods",
    linkLabel: {
      en: "BOOTH payment methods",
      "zh-CN": "BOOTH 支付方式说明",
      "zh-TW": "BOOTH 付款方式說明",
      ja: "BOOTH のお支払い方法",
      ko: "BOOTH 결제 수단 안내",
    },
    sourceUrl: "https://booth.pm/payment",
  }),
  buying: baselineGuide({
    id: "buying",
    title: {
      en: "Completing a BOOTH purchase and library sync",
      "zh-CN": "完成购买并同步到素材库",
      "zh-TW": "完成購買並同步至素材庫",
      ja: "BOOTH での購入完了とライブラリ同期",
      ko: "BOOTH 구매 완료 후 소재함 동기화하기",
    },
    stepId: "complete-checkout",
    action: {
      en: "Complete checkout on BOOTH's official page, then sync your library in VUA.",
      "zh-CN": "在 BOOTH 官方页面完成结账后，回到 VUA 同步你的已购商品列表。",
      "zh-TW": "在 BOOTH 官方頁面完成結帳後，回到 VUA 同步你的已購商品清單。",
      ja: "BOOTH 公式ページで決済を完了してから、VUA で購入済みライブラリを同期します。",
      ko: "BOOTH 공식 페이지에서 결제를 마친 뒤 VUA에서 구매 목록을 동기화합니다.",
    },
    expectedResult: {
      en: "The purchased item appears in your VUA library ready to download.",
      "zh-CN": "刚买好的商品出现在 VUA 素材库中，可直接选择下载。",
      "zh-TW": "剛買好的商品出現在 VUA 素材庫中，可直接選擇下載。",
      ja: "購入した商品が VUA の素材ライブラリに表示され、ダウンロードできるようになります。",
      ko: "구매한 상품이 VUA 소재함에 나타나며 바로 다운로드할 수 있습니다.",
    },
    site: "accounts.booth.pm",
    sourceLabelEn: "BOOTH sign-in",
    linkLabel: {
      en: "BOOTH sign-in",
      "zh-CN": "登录 BOOTH",
      "zh-TW": "登入 BOOTH",
      ja: "BOOTH ログイン",
      ko: "BOOTH 로그인",
    },
    sourceUrl: "https://accounts.booth.pm/users/sign_in",
  }),
  "terms-of-use": baselineGuide({
    id: "terms-of-use",
    title: {
      en: "Reading VN3 and creator terms of use",
      "zh-CN": "阅读 VN3 与作者利用规约",
      "zh-TW": "閱讀 VN3 與作者使用條款",
      ja: "VN3 ライセンスと作者の利用規約の確認",
      ko: "VN3 라이선스와 제작자 이용 약관 확인하기",
    },
    stepId: "check-creator-terms",
    action: {
      en: "Review the product's VN3 license or creator terms linked from your library entry before editing or uploading.",
      "zh-CN": "改模或上传前，从素材库条目打开商品附带的 VN3 授权书或作者利用规约仔细阅读。",
      "zh-TW": "改模或上傳前，從素材庫項目開啟商品附帶的 VN3 授權書或作者使用條款仔細閱讀。",
      ja: "改変やアップロードの前に、ライブラリ項目からリンクされた VN3 ライセンスまたは作者の利用規約を確認します。",
      ko: "개변이나 업로드 전에 소재함 항목에 연결된 VN3 라이선스 또는 제작자 이용 약관을 확인합니다.",
    },
    expectedResult: {
      en: "You confirm allowed modifications, uploads, and credit conditions.",
      "zh-CN": "明确允许的改模范围、上传权限（公开/仅自己）与署名要求。",
      "zh-TW": "明確允許的改模範圍、上傳權限（公開／私人）與署名要求。",
      ja: "許可されている改変範囲、アップロード条件、クレジット表記の要否を把握できます。",
      ko: "허용되는 수정 범위, 업로드 조건, 출처 표기 의무를 확인할 수 있습니다.",
    },
    site: "booth.pm",
    sourceLabelEn: "BOOTH",
    linkLabel: {
      en: "BOOTH",
      "zh-CN": "BOOTH 官网",
      "zh-TW": "BOOTH 官網",
      ja: "BOOTH",
      ko: "BOOTH",
    },
    sourceUrl: "https://booth.pm/",
  }),
  "why-licensed": baselineGuide({
    id: "why-licensed",
    title: {
      en: "Supporting creators through licensed works",
      "zh-CN": "支持创作者与正版生态",
      "zh-TW": "支持創作者與正版生態",
      ja: "正規購入でクリエイターを支援する",
      ko: "정품 구매로 창작자 응원하기",
    },
    stepId: "discover-creators",
    action: {
      en: "Explore official creator shops, updates, and Avatar communities.",
      "zh-CN": "通过创作者官方店铺购买正版素材，获取后续修复更新与新素体适配。",
      "zh-TW": "透過創作者官方店鋪購買正版素材，取得後續修復更新與新素體適配。",
      ja: "クリエイターの公式ショップから正規版を入手し、アップデート配布や対応追加を受け取れるようにします。",
      ko: "창작자의 공식 샵에서 정품을 구매해 업데이트 파일과 추가 호환 지원을 받습니다.",
    },
    expectedResult: {
      en: "Every completed purchase supports the creator and keeps updates accessible in your library.",
      "zh-CN": "每一笔购买都会直接支持原作者创作，且后续更新始终留在你的素材库中随时可取。",
      "zh-TW": "每一筆購買都會直接支持原作者創作，且後續更新始終留在你的素材庫中隨時可取。",
      ja: "購入が作者の制作継続につながり、更新ファイルも自身のライブラリからいつでも再取得できます。",
      ko: "구매할 때마다 창작자의 활동을 직접 응원하게 되며 업데이트 파일도 내 라이브러리에서 언제든 다시 받을 수 있습니다.",
    },
    site: "booth.pm",
    sourceLabelEn: "BOOTH",
    linkLabel: {
      en: "BOOTH",
      "zh-CN": "BOOTH 官网",
      "zh-TW": "BOOTH 官網",
      ja: "BOOTH",
      ko: "BOOTH",
    },
    sourceUrl: "https://booth.pm/",
  }),
});
