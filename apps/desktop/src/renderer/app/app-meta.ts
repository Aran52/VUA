/**
 * 应用元数据常量(数据,非文案):版本号与仓库地址的唯一代码来源。
 * version 与 apps/desktop/package.json 保持同步,发布流程负责核对。
 * repoUrl points to the canonical repository under VUA-Project.
 */
export const appMeta = {
  name: "vua-desktop",
  version: "0.8.0-qa.1",
  repoUrl: "https://github.com/VUA-Project/VUA",
} as const;
