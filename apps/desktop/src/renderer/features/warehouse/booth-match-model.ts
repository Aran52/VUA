import type { BoothSearchItemV01, BoothSearchResultV01 } from "@vua/contracts";

/** Only the file/folder label leaves the machine, never its parent directory. */
export function boothSearchKeyword(displayName: string): string {
  const keyword = (displayName.split(/[\\/]/).at(-1) ?? displayName).replace(/\.(zip|unitypackage|fbx|psd|png|jpe?g)$/i, "")
    .replace(/(?:[_ -]+v(?:er(?:sion)?)?\.?\s*\d+(?:\.\d+)*|[_ -]+\d{4}-?\d{2}-?\d{2})$/i, "")
    .replace(/[_]+/g, " ").replace(/[\x00-\x1f\x7f-\x9f]/g, " ").trim();
  return Array.from(keyword).slice(0, 200).join("");
}
const exactTitle = (name: string): string => name.normalize("NFKC").trim().replace(/\s+/g, " ").toLocaleLowerCase("en");
export function exactBoothMatch(result: BoothSearchResultV01, keyword: string): BoothSearchItemV01 | null {
  if (result.truncated || Array.from(keyword).length < 4) return null;
  const matches = result.items.filter(item => exactTitle(item.title) === exactTitle(keyword));
  return matches.length === 1 ? matches[0]! : null;
}
