/**
 * locale 注册表与初始语言解析(C-I18N)。
 *
 * 纪律:
 * - en 是源语言;其余语言表交付后才把 available 置真,
 *   未交付语言不出现在可选列表(零死按钮纪律);
 * - 语言自名(endonym)是注册表数据,不进入翻译流程——
 *   任何语言界面下都以该语言自身书写(如 English / 日本語);
 * - 本模块不含界面文案,故豁免字符串表纪律(数据而非文案)。
 */
export type LocaleId = "zh-CN" | "zh-TW" | "en" | "ja" | "ko";

export interface LocaleMeta {
  readonly id: LocaleId;
  readonly endonym: string;
  readonly available: boolean;
}

export const localeRegistry: readonly LocaleMeta[] = [
  { id: "zh-CN", endonym: "简体中文", available: true },
  { id: "zh-TW", endonym: "繁體中文", available: true },
  { id: "en", endonym: "English", available: true },
  { id: "ja", endonym: "日本語", available: true },
  { id: "ko", endonym: "한국어", available: true },
];

export const defaultLocale: LocaleId = "en";

/** BCP 47 语言标签 → 注册 locale 前缀映射;不认识的标签返回 null */
export function matchLocale(tag: string): LocaleId | null {
  const lower = tag.trim().toLowerCase();
  // Traditional script: an explicit Hant tag, or Taiwan, Hong Kong and Macau without a script.
  if (/^zh-(hant|tw|hk|mo)(-|$)/.test(lower)) return "zh-TW";
  if (lower.startsWith("zh")) return "zh-CN";
  if (lower.startsWith("en")) return "en";
  if (lower.startsWith("ja")) return "ja";
  if (lower.startsWith("ko")) return "ko";
  return null;
}

/**
 * fallback 链(纯函数,可测):
 * 1. 存储的显式选择(仅当其仍 available——语言包可能被移除);
 * 2. 系统语言列表按序前缀匹配(仅 available);
 * 3. 默认源语言 en。
 */
export function resolveInitialLocale(
  stored: string | null,
  systemLanguages: readonly string[],
): LocaleId {
  const available = new Set(
    localeRegistry.filter((entry) => entry.available).map((entry) => entry.id),
  );
  if (stored !== null && available.has(stored as LocaleId)) return stored as LocaleId;
  for (const tag of systemLanguages) {
    const matched = matchLocale(tag);
    if (matched !== null && available.has(matched)) return matched;
  }
  return defaultLocale;
}
