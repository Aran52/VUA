// Static checks: hardcoded font sizes, radii and hex colors in renderer CSS
// and TSX inline styles. File+line reporting only — no DOM involved.

import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

async function walkFiles(dir, extensions, out = []) {
  let entries;
  try { entries = await readdir(dir, { withFileTypes: true }); } catch { return out; }
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) await walkFiles(full, extensions, out);
    else if (extensions.some(ext => entry.name.endsWith(ext))) out.push(full);
  }
  return out;
}

function stripComments(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, match => match.replace(/[^\n]/g, " "));
}

/** Parses flat + nested rule blocks: [{selector, atRules, body, line}]. */
export function parseCssBlocks(css) {
  const blocks = [];
  const stack = []; // {selector, atRule, line}
  let mode = "selector"; // "selector" | "body"
  let buf = "";
  let bufLine = 1;
  let line = 1;
  const isAtSelector = s => /^@(?!font-face|property)/i.test(s);

  const emit = (selector, body, atRules, lineNo) => {
    if (selector && !selector.startsWith("@") && body.trim() !== "") {
      blocks.push({ selector, atRules, body, line: lineNo });
    }
  };

  for (let i = 0; i < css.length; i += 1) {
    const ch = css[i];
    if (ch === "\n") line += 1;
    if (ch === "{" ) {
      if (mode === "body") {
        // Nested block: the buf head holds the parent's own declarations.
        const cut = Math.max(buf.lastIndexOf(";"), buf.lastIndexOf("}")) + 1;
        const parent = stack[stack.length - 1];
        if (parent && !parent.atRule) emit(parent.selector, buf.slice(0, cut), stack.slice(0, -1).filter(f => f.atRule).map(f => f.selector), parent.line);
        buf = buf.slice(cut);
      }
      stack.push({ selector: buf.trim(), atRule: isAtSelector(buf.trim()), line: bufLine });
      buf = "";
      bufLine = line;
      mode = "body";
      continue;
    }
    if (ch === "}" ) {
      const frame = stack.pop();
      if (frame) emit(frame.selector, buf, stack.filter(f => f.atRule).map(f => f.selector), frame.line);
      buf = "";
      bufLine = line;
      mode = stack.length && !stack[stack.length - 1].atRule ? "body" : "selector";
      continue;
    }
    if (ch === ";" && mode === "selector") { buf = ""; continue; }
    if (buf === "" ) bufLine = line;
    buf += ch;
  }
  return blocks;
}

const HEX_SOURCE = "#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\\b";
const containsHex = value => new RegExp(HEX_SOURCE).test(value);
const findHexes = value => value.match(new RegExp(HEX_SOURCE, "g")) ?? [];

function isHardcodedRadius(value) {
  const v = value.replace(/\s+/g, " ").trim();
  if (v.includes("var(")) return false;
  if (/^(0|50%|999px|9999px|inherit|unset)$/.test(v)) return false;
  return /\d/.test(v);
}

const COLOR_PROPERTIES = new Set([
  "color", "background-color", "border-color", "background", "border",
  "border-top", "border-bottom", "border-left", "border-right",
  "border-top-color", "border-bottom-color", "border-left-color", "border-right-color",
  "outline-color", "text-decoration-color", "caret-color", "fill", "stroke",
  "box-shadow", "text-shadow",
]);

export async function scanStatic(rootDir) {
  const cssFindings = [];
  const tsxFindings = [];
  for (const file of await walkFiles(rootDir, [".css"])) {
    const css = stripComments(await readFile(file, "utf8"));
    for (const block of parseCssBlocks(css)) {
      for (const decl of block.body.split(";")) {
        const colon = decl.indexOf(":");
        if (colon < 0) continue;
        const name = decl.slice(0, colon).trim().toLowerCase();
        const value = decl.slice(colon + 1).trim();
        // A `font:` shorthand hardcodes the size unless it uses a font token.
        if (name === "font" && /\d+px/.test(value) && !/var\(--vua-font-(display|title|body|caption)\)/.test(value)) {
          cssFindings.push({ file, line: block.line, selector: block.selector, kind: "font-size", value, snippet: `${name}: ${value}` });
        } else if (name === "font-size" && !value.includes("var(--vua-") && /\d/.test(value)) {
          cssFindings.push({ file, line: block.line, selector: block.selector, kind: "font-size", value, snippet: `${name}: ${value}` });
        } else if (name === "border-radius" && isHardcodedRadius(value)) {
          cssFindings.push({ file, line: block.line, selector: block.selector, kind: "border-radius", value, snippet: `${name}: ${value}` });
        } else if (COLOR_PROPERTIES.has(name) && !value.includes("var(")
          && !value.includes("transparent") && !value.includes("currentColor")
          && !value.includes("url(") && containsHex(value)) {
          cssFindings.push({ file, line: block.line, selector: block.selector, kind: "hex-color", value: findHexes(value).join(", "), snippet: `${name}: ${value}` });
        }
      }
    }
  }
  for (const file of await walkFiles(rootDir, [".tsx"])) {
    const lines = (await readFile(file, "utf8")).split("\n");
    lines.forEach((text, idx) => {
      if (/(querySelector|getElementById|closest\(|matches\()/.test(text)) return;
      const code = text.split("//")[0];
      const fontSize = code.match(/fontSize:\s*["']?(\d+)(px)?["']?/);
      if (fontSize) tsxFindings.push({ file, line: idx + 1, kind: "font-size", value: fontSize[1] + (fontSize[2] ?? ""), snippet: code.trim().slice(0, 140) });
      const radius = code.match(/borderRadius:\s*["']?(\d+)(px)?["']?/);
      if (radius) tsxFindings.push({ file, line: idx + 1, kind: "border-radius", value: radius[1] + (radius[2] ?? ""), snippet: code.trim().slice(0, 140) });
      if (!code.includes("url(") && containsHex(code)) {
        tsxFindings.push({ file, line: idx + 1, kind: "hex-color", value: findHexes(code).join(", "), snippet: code.trim().slice(0, 140) });
      }
    });
  }
  return { css: cssFindings, tsx: tsxFindings };
}
