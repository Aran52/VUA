// Subsets each skin's title font to the characters VUA can show in a title, as WOFF2.
// The source fonts stay outside the repository; their download URLs and SHA-256 are listed below,
// and each output folder keeps the font's own license (SIL OFL 1.1).
// Usage: node packages/design-system/skins/make-fonts.mjs <folder with the source files>
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import subsetFont from "subset-font";

const here = path.dirname(fileURLToPath(import.meta.url));
const sources = path.resolve(process.argv[2] ?? "");
const copyRoot = path.resolve(here, "../../../apps/desktop/src/renderer/i18n");

const FONTS = [
  {
    skin: "tenshi",
    output: "ResourceHanRoundedCN-Title.woff2",
    // Extracted from RHR-CN-0.990.7z (SHA-256 e7005f7b4a7a0b8352d32c4a1358ff47564eb73be7fdb2db00d9f792755e9dc7),
    // https://github.com/CyanoHao/Resource-Han-Rounded/releases/tag/v0.990. The TrueType release is used
    // because the subsetter cannot read v1.910's CFF2 variable font. License: LICENSE.txt at the same tag.
    file: "ResourceHanRoundedCN-Medium.ttf",
    sha256: "b2ec1727fbc04b70255db6cb71be0eca3b7a7167ef11655059c5576cc920143f",
    license: "LICENSE-v0.990.txt",
  },
  {
    skin: "jirai",
    output: "ZCOOLKuaiLe-Title.woff2",
    // https://github.com/google/fonts/tree/main/ofl/zcoolkuaile
    file: "ZCOOLKuaiLe-Regular.ttf",
    sha256: "812a6fc1fe54b6d73a419245c32dfeba8aa33104d5be90d1cf6af082007cb71d",
    license: "OFL.txt",
  },
];

// Every character in the UI copy, printable ASCII, and the 3,755 common simplified characters
// (GB2312 level 1), so Chinese Avatar and project names in titles mostly stay in the title font.
function titleCharacters() {
  const chars = new Set();
  for (let code = 0x20; code < 0x7f; code += 1) chars.add(String.fromCharCode(code));
  for (const ch of "，。、：；！？“”‘’（）《》【】…—·～") chars.add(ch);
  const gb2312 = new TextDecoder("gb2312");
  for (let row = 0xb0; row <= 0xd7; row += 1) {
    for (let cell = 0xa1; cell <= 0xfe; cell += 1) {
      const ch = gb2312.decode(new Uint8Array([row, cell]));
      if (ch.length === 1 && ch !== "�") chars.add(ch);
    }
  }
  for (const name of readdirSync(copyRoot)) {
    const file = path.join(copyRoot, name);
    if (!/\.ts$/.test(name) || /\.test\.ts$/.test(name) || !statSync(file).isFile()) continue;
    for (const ch of readFileSync(file, "utf8")) if (ch.codePointAt(0) >= 0x80) chars.add(ch);
  }
  return [...chars].join("");
}

const text = titleCharacters();
for (const font of FONTS) {
  const source = readFileSync(path.join(sources, font.file));
  const digest = createHash("sha256").update(source).digest("hex");
  if (digest !== font.sha256) throw new Error(`${font.file}: SHA-256 ${digest} does not match the recorded source`);
  const woff2 = await subsetFont(source, text, {
    targetFormat: "woff2",
    // Keep the copyright, license and license URL records inside the font.
    preserveNameIds: [0, 13, 14],
  });
  const dir = path.join(here, font.skin, "fonts");
  mkdirSync(dir, { recursive: true });
  writeFileSync(path.join(dir, font.output), woff2);
  copyFileSync(path.join(sources, font.license), path.join(dir, "OFL.txt"));
  console.log(`${font.skin}/fonts/${font.output}: ${(woff2.length / 1024).toFixed(0)} KiB, ${[...text].length} characters requested`);
}
