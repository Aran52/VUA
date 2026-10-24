/**
 * 对比度核对(美术方案验收 §12-2/§12-3、§3.2.2):直接解析
 * src/design-system/tokens/tokens.css 的实际 Token 值,在 5 个上下文
 * (深色/浅色 × 紫辖区/橙辖区 + 手动高对比度 [data-hc="on"])下计算
 * WCAG 2.x 对比度,文字对低于 4.5:1、非文字对低于 3:1 即非零退出;
 * forced-colors 系统色映射不可算对比度,另做结构守卫(§2.6)。
 *
 * 辖区必须使用真实模块标识(§3.2.2):紫辖区 = env /
 * settings(:root 默认),橙辖区 = production([data-module="production"]);
 * 不得用已经废弃的模块名生成虚假的通过结果。防回退守卫:橙辖区未解析到
 * --vua-orange、紫辖区未解析到 --vua-purple(例如选择器改名导致桶为空)
 * 时响亮失败,不静默通过。
 *
 * 解析器面向 tokens.css 的约定写法(简单选择器块 + 一层 var() 别名),
 * 不是通用 CSS 解析器;改动 tokens.css 结构时请同步检查本脚本。
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const tokensPath = fileURLToPath(
  new URL("../../../packages/design-system/src/tokens.css", import.meta.url),
);
const rawSource = readFileSync(tokensPath, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

/* ---- 0. @media 块(C-I18N 高对比度):内层选择器不参与普通桶解析,
 *   forced-colors 块另做系统色关键字结构守卫(系统色运行时决定,不可算对比度) ---- */
function extractAtBlocks(css, atName) {
  const blocks = [];
  let rest = "";
  let i = 0;
  while (i < css.length) {
    if (css.startsWith(atName, i)) {
      let depth = 0;
      const start = i;
      while (i < css.length) {
        if (css[i] === "{") depth += 1;
        else if (css[i] === "}") {
          depth -= 1;
          if (depth === 0) {
            i += 1;
            break;
          }
        }
        i += 1;
      }
      blocks.push(css.slice(start, i));
    } else {
      rest += css[i];
      i += 1;
    }
  }
  return { stripped: rest, blocks };
}

const { stripped: source, blocks: forcedColorsBlocks } = extractAtBlocks(rawSource, "@media");

/* ---- 1. 解析选择器块,按用途归类 ---- */
const buckets = {
  base: {}, // :root(深色主题基准;紫辖区为默认,无需逐模块列出)
  darkProduction: {}, // [data-module="production"](无主题限定)
  lightBase: {}, // [data-theme="light"]
  lightProduction: {}, // [data-theme="light"] + [data-module="production"]
  hc: {}, // [data-hc="on"](C-I18N 手动高对比度)
};

for (const match of source.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
  const selectors = match[1].split(",").map((s) => s.trim());
  const declarations = Object.fromEntries(
    [...match[2].matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map((d) => [d[1], d[2].trim()]),
  );
  for (const selector of selectors) {
    const light = selector.includes('data-theme="light"');
    const production = selector.includes('data-module="production"');
    const hc = selector.includes('data-hc="on"');
    const bucket =
      hc
        ? "hc"
        : selector === ":root"
          ? "base"
          : light && production
            ? "lightProduction"
            : light
              ? "lightBase"
              : production
                ? "darkProduction"
                : null;
    if (bucket) Object.assign(buckets[bucket], declarations);
  }
}

/* ---- 2. 合成 4 个上下文并解析 var() 别名 ---- */
function makeContext(...layers) {
  const merged = Object.assign({}, ...layers);
  return (name, seen = new Set()) => {
    if (seen.has(name)) throw new Error(`var() 循环引用:${name}`);
    seen.add(name);
    const raw = merged[name];
    if (raw === undefined) throw new Error(`未知 Token:${name}`);
    const alias = /^var\((--[\w-]+)\)$/.exec(raw);
    return alias ? makeContext(merged)(alias[1], seen) : raw;
  };
}

const contexts = {
  "深色·紫辖区(env/settings)": makeContext(buckets.base),
  "深色·橙辖区(production)": makeContext(buckets.base, buckets.darkProduction),
  "浅色·紫辖区(env/settings)": makeContext(buckets.base, buckets.lightBase),
  "浅色·橙辖区(production)": makeContext(
    buckets.base,
    buckets.darkProduction,
    buckets.lightBase,
    buckets.lightProduction,
  ),
  // C-I18N 手动高对比度:只覆盖 base 桶,不叠辖区([data-hc] 块已把
  // purple/orange 与 accent 抹平为同色,辖区守卫天然成立)
  "高对比度·手动(data-hc=on)": makeContext(buckets.base, buckets.hc),
};

/* ---- 2.5 防回退守卫(§3.2.2):辖区必须解析到各自品牌色,否则响亮失败 ---- */
let guardFailed = 0;
for (const [contextName, resolve] of Object.entries(contexts)) {
  const isProduction = contextName.includes("production");
  const expected = resolve(isProduction ? "--vua-orange" : "--vua-purple");
  const actual = resolve("--vua-accent");
  if (actual.toLowerCase() !== expected.toLowerCase()) {
    guardFailed += 1;
    console.error(
      `  ✗ 守卫 [${contextName}]: --vua-accent = ${actual},应为 ${expected}` +
        "(辖区选择器可能已改名,桶为空导致回退)",
    );
  }
}
if (guardFailed > 0) {
  console.error(`\ncheck-contrast: ${guardFailed} 个上下文未解析到正确的辖区品牌色`);
  process.exit(1);
}

/* ---- 2.6 forced-colors 系统色结构守卫(C-I18N):
 * 系统色关键字由用户的 Windows HC 调色板决定,对比度不可算;
 * 改为结构守卫——块必须存在,且背景/正文/禁用/强调四类语义都有系统色映射 ---- */
const forcedBlock = forcedColorsBlocks.find((block) => block.includes("forced-colors"));
const forcedExpectations = ["Canvas", "CanvasText", "GrayText", "Highlight"];
const forcedMissing = forcedBlock
  ? forcedExpectations.filter((keyword) => !forcedBlock.includes(keyword))
  : forcedExpectations;
if (forcedMissing.length > 0) {
  console.error(
    `\ncheck-contrast: forced-colors 高对比度映射缺失关键字:${forcedMissing.join(", ")}`,
  );
  process.exit(1);
}

/* ---- 3. WCAG 对比度 ---- */
function luminance(hex) {
  const channels = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function ratio(fgHex, bgHex) {
  const [hi, lo] = [luminance(fgHex), luminance(bgHex)].sort((a, b) => b - a);
  return (hi + 0.05) / (lo + 0.05);
}

/* ---- 4. 核对清单:[名称, 前景, 背景, 阈值] ---- */
const pairs = [
  ["主按钮 文字/底色", "--vua-accent-contrast", "--vua-accent", 4.5],
  ["主按钮 文字/hover", "--vua-accent-contrast", "--vua-accent-hover", 4.5],
  ["主按钮 文字/pressed", "--vua-accent-contrast", "--vua-accent-pressed", 4.5],
  ["选中 pill/侧栏 文字/tint", "--vua-accent-text", "--vua-accent-tint", 4.5],
  ["正文/画布", "--vua-text-strong", "--vua-bg-canvas", 4.5],
  ["正文/面板", "--vua-text-strong", "--vua-bg-panel", 4.5],
  ["次要文字/面板", "--vua-text-secondary", "--vua-bg-panel", 4.5],
  ["琥珀结论文字/面板", "--vua-amber-text", "--vua-bg-panel", 4.5],
  ["错误文字/面板", "--vua-error", "--vua-bg-panel", 4.5],
  ["Warning text / panel", "--vua-warning", "--vua-bg-panel", 4.5],
  ["Destructive confirmation text / fill", "--vua-on-error", "--vua-error", 4.5],
  ["Graph badge text / amber fill", "--vua-bg-canvas", "--vua-amber", 4.5],
  ["就绪文字/面板", "--vua-success", "--vua-bg-panel", 4.5],
  ["琥珀节点图标/面板(非文字)", "--vua-amber", "--vua-bg-panel", 3],
  ["状态灯图标/就绪底(非文字)", "--vua-on-status", "--vua-success", 3],
  ["状态灯图标/需注意底(非文字)", "--vua-on-status", "--vua-warning", 3],
  ["状态灯图标/错误底(非文字)", "--vua-on-status", "--vua-error", 3],
];

let failed = 0;
for (const [contextName, resolve] of Object.entries(contexts)) {
  console.log(`\n[${contextName}]`);
  for (const [name, fg, bg, min] of pairs) {
    const value = ratio(resolve(fg), resolve(bg));
    const pass = value >= min;
    if (!pass) failed += 1;
    console.log(
      `  ${pass ? "✓" : "✗"} ${name}: ${value.toFixed(2)}:1(≥${min}:1) ${resolve(fg)} on ${resolve(bg)}`,
    );
  }
}

if (failed > 0) {
  console.error(`\ncheck-contrast: ${failed} 对不达标`);
  process.exit(1);
}
console.log("\ncheck-contrast: 全部达标");
