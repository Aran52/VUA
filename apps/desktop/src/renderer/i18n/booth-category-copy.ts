/** BOOTH's 3D categories in each language. A category outside this table keeps BOOTH's own words. */
import { currentLocale } from "./current-table.ts";
import type { LocaleId } from "./locales.ts";

type Names = Readonly<Record<Exclude<LocaleId, "ja">, string>>;

const CATEGORIES: Readonly<Record<string, Names>> = {
  "3Dモデル（その他）": { en: "3D models (other)", "zh-CN": "3D 模型（其他）", "zh-TW": "3D 模型（其他）", ko: "3D 모델(기타)" },
  "3Dキャラクター": { en: "3D characters", "zh-CN": "3D 角色", "zh-TW": "3D 角色", ko: "3D 캐릭터" },
  "3D衣装": { en: "3D clothing", "zh-CN": "3D 服装", "zh-TW": "3D 服裝", ko: "3D 의상" },
  "3D装飾品": { en: "3D accessories", "zh-CN": "3D 饰品", "zh-TW": "3D 飾品", ko: "3D 장식품" },
  "3D小道具": { en: "3D props", "zh-CN": "3D 道具", "zh-TW": "3D 道具", ko: "3D 소품" },
  "3D髪型": { en: "3D hairstyles", "zh-CN": "3D 发型", "zh-TW": "3D 髮型", ko: "3D 헤어스타일" },
  "3D靴": { en: "3D shoes", "zh-CN": "3D 鞋", "zh-TW": "3D 鞋", ko: "3D 신발" },
  "3Dテクスチャ": { en: "3D textures", "zh-CN": "3D 贴图", "zh-TW": "3D 貼圖", ko: "3D 텍스처" },
  "3Dツール・システム": { en: "3D tools and systems", "zh-CN": "3D 工具与系统", "zh-TW": "3D 工具與系統", ko: "3D 도구·시스템" },
  "3Dモーション・アニメーション": { en: "3D motion and animation", "zh-CN": "3D 动作与动画", "zh-TW": "3D 動作與動畫", ko: "3D 모션·애니메이션" },
  "3D環境・ワールド": { en: "3D environments and worlds", "zh-CN": "3D 环境与世界", "zh-TW": "3D 環境與世界", ko: "3D 환경·월드" },
  "3DCG・3DCG集": { en: "3DCG and collections", "zh-CN": "3DCG 与作品集", "zh-TW": "3DCG 與作品集", ko: "3DCG·3DCG 모음" },
  "イラスト3D素材": { en: "3D material for illustration", "zh-CN": "插画用 3D 素材", "zh-TW": "插畫用 3D 素材", ko: "일러스트용 3D 소재" },
};

export function boothCategoryLabel(category: string, locale: LocaleId = currentLocale): string {
  return locale === "ja" ? category : CATEGORIES[category]?.[locale] ?? category;
}
