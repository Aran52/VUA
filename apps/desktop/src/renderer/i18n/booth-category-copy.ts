/** BOOTH's 3D categories in each language. BOOTH's own pages say the Japanese name in Japanese and the English name
 * in every other language, so a recorded category is one of those two. Any other category keeps BOOTH's words. */
import { currentLocale } from "./current-table.ts";
import type { LocaleId } from "./locales.ts";

const CATEGORIES: readonly Readonly<Record<LocaleId, string>>[] = [
  { ja: "3D衣装", en: "3D Clothing", "zh-CN": "3D 服装", "zh-TW": "3D 服裝", ko: "3D 의상" },
  { ja: "3Dモデル（その他）", en: "3D Models (Other)", "zh-CN": "3D 模型（其他）", "zh-TW": "3D 模型（其他）", ko: "3D 모델(기타)" },
  { ja: "3D小道具", en: "3D Props", "zh-CN": "3D 道具", "zh-TW": "3D 道具", ko: "3D 소품" },
  { ja: "3Dテクスチャ", en: "3D Textures", "zh-CN": "3D 贴图", "zh-TW": "3D 貼圖", ko: "3D 텍스처" },
  { ja: "3D装飾品", en: "3D Accessories", "zh-CN": "3D 饰品", "zh-TW": "3D 飾品", ko: "3D 장식품" },
  { ja: "3Dキャラクター", en: "3D Characters", "zh-CN": "3D 角色", "zh-TW": "3D 角色", ko: "3D 캐릭터" },
  { ja: "3D環境・ワールド", en: "3D Environments / World", "zh-CN": "3D 环境与世界", "zh-TW": "3D 環境與世界", ko: "3D 환경·월드" },
  { ja: "3Dツール・システム", en: "3D Tools & Systems", "zh-CN": "3D 工具与系统", "zh-TW": "3D 工具與系統", ko: "3D 도구·시스템" },
  { ja: "3Dモーション・アニメーション", en: "3D Motion & Animation", "zh-CN": "3D 动作与动画", "zh-TW": "3D 動作與動畫", ko: "3D 모션·애니메이션" },
  { ja: "イラスト3D素材", en: "Graphics (3D)", "zh-CN": "插画用 3D 素材", "zh-TW": "插畫用 3D 素材", ko: "일러스트용 3D 소재" },
  { ja: "3D髪型", en: "3D Hair", "zh-CN": "3D 发型", "zh-TW": "3D 髮型", ko: "3D 헤어스타일" },
  { ja: "3DCG・3DCG集", en: "3DCG", "zh-CN": "3DCG 与作品集", "zh-TW": "3DCG 與作品集", ko: "3DCG·3DCG 모음" },
  { ja: "3D靴", en: "3D Shoes", "zh-CN": "3D 鞋", "zh-TW": "3D 鞋", ko: "3D 신발" },
];
const BY_BOOTH_NAME = new Map(CATEGORIES.flatMap(names => [[names.ja, names], [names.en, names]] as const));

export function boothCategoryLabel(category: string, locale: LocaleId = currentLocale): string {
  return BY_BOOTH_NAME.get(category)?.[locale] ?? category;
}
