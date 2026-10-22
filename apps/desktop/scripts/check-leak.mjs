/**
 * fixture 泄漏扫描(G2-A,check:leak):验证"DEV fixture 生产构建不可达"。
 *
 * 流程:自跑 vite 生产构建(临时输出目录)→ 提取 strings.fixtures.zh-CN.ts
 * 全部导出字符串作为负载指纹 → 扫描产物 js/css/html,命中即失败。
 *
 * 说明:
 * - 指纹直接取字符串表,fixture 新增文案自动纳入,无需维护第二份清单;
 * - 与主字符串表(strings.en.ts 等已交付语言表)相同的值不算负载:
 *   它经主表合法
 *   进入生产包(如诚实空态文案提到"VRChat 本体"),扫它会误报;
 * - 短于 6 字符的值不扫(如"网络"是通用词,可能合法出现在正式文案,
 *   会误报;危险负载——虚构安全结论——都是完整句子,全部在覆盖内);
 * - 同时匹配原文与 \uXXXX 转义形态,覆盖不同 minify 输出;
 * - 产物目录在 node_modules/.cache 下,每次构建前清空,不入库。
 */
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, rmSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const outDir = join(root, "node_modules", ".cache", "vua-leak-check");
const MIN_PAYLOAD_LENGTH = 6;

console.log("check-leak: 执行生产构建(vite build,临时目录)…");
rmSync(outDir, { recursive: true, force: true });
execFileSync(
  process.execPath,
  [
    join(root, "node_modules", "vite", "bin", "vite.js"),
    "build",
    "--outDir",
    outDir,
    "--emptyOutDir",
    "--logLevel",
    "warn",
  ],
  { cwd: root, stdio: "inherit" },
);

const { fixtureStrings } = await import("../src/renderer/i18n/strings.fixtures.zh-CN.ts");
const mainTables = [];
for (const id of ["zh-CN", "zh-TW", "en", "ja", "ko"]) {
  const { strings } = await import(`../src/renderer/i18n/strings.${id}.ts`);
  mainTables.push(strings);
}

function collectStrings(value, acc) {
  if (typeof value === "string") {
    acc.push(value);
  } else if (Array.isArray(value)) {
    for (const item of value) collectStrings(item, acc);
  } else if (value !== null && typeof value === "object") {
    for (const item of Object.values(value)) collectStrings(item, acc);
  }
  return acc;
}

// 主表合法值:fixture 值若本身或作为交付语言表长句的子串出现,其存在已被
// 合法解释,不构成泄漏证据(如"VRChat 本体"见于诚实空态文案长句)。
// 豁免面 = 全部四张交付语言表(zh-CN/en/ja/ko 都随包发布,任一表合法
// 存在的值本就在产物中;只豁免 zh-CN 会误报英文表合法存在的词,如
// "VRChat SDK" 见于英文发布说明)。
// 注意:这意味着标题类短值的泄漏掩码风险由长句负载的覆盖来对冲——
// 危险负载(虚构安全结论)均为完整长句,不受子串豁免影响。
const mainValues = mainTables.flatMap((table) => collectStrings(table, []));

const payloads = [
  ...new Set(collectStrings(fixtureStrings, [])).values(),
].filter(
  (s) =>
    [...s].length >= MIN_PAYLOAD_LENGTH && !mainValues.some((m) => m.includes(s)),
);
// 018 备注:开发模式 per-port 选择(DEV-only)的存储键常量位于主模块
// storage-keys.ts(键唯一来源纪律),其字符串随主包合法存在——故不纳入
// 指纹集(纳入即常量性误报)。本断言覆盖范围:仅 fixture 负载字符串;
// 不覆盖 DEV 分支代码的构建期剔除是否生效(该键被读取的调用点在
// import.meta.env.DEV 分支内,剔除由静态替换＋Rollup 死代码消除机制
// 保证;本脚本无对应检查,如需断言须另行专项检查)。

if (payloads.length === 0) {
  console.error("check-leak: 未提取到任何指纹,扫描无效");
  process.exit(1);
}

const escapeUnicode = (s) =>
  s.replace(/[^\x00-\x7F]/g, (ch) => `\\u${ch.codePointAt(0).toString(16).padStart(4, "0")}`);

function* bundleFiles(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      yield* bundleFiles(path);
    } else if (/\.(js|css|html)$/.test(entry.name)) {
      yield path;
    }
  }
}

const violations = [];
for (const file of bundleFiles(outDir)) {
  const content = readFileSync(file, "utf8");
  for (const payload of payloads) {
    if (content.includes(payload) || content.includes(escapeUnicode(payload))) {
      violations.push(`${relative(root, file)} 命中负载:「${payload}」`);
    }
  }
}

if (violations.length > 0) {
  console.error("check-leak: 生产构建中发现 fixture 负载,DEV 防线失效:");
  for (const v of violations) console.error(`  ${v}`);
  process.exit(1);
}

console.log(`check-leak: 通过(${payloads.length} 条指纹,生产构建零泄漏)`);
