// Generates the native skins' SVG motifs; drawn in code so repeats (lace scallops, wings) stay exact.
// Usage: node packages/design-system/skins/make-motifs.mjs packages/design-system/skins
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

const root = process.argv[2];
const n = v => Math.round(v * 100) / 100;
const svg = (w, h, body, title) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round">\n  <title>${title}</title>\n${body.trim().split("\n").map(l => "  " + l.trim()).join("\n")}\n</svg>\n`;
const write = (skin, name, w, h, body, title) => {
  const dir = path.join(root, skin, "motifs");
  mkdirSync(dir, { recursive: true });
  writeFileSync(path.join(dir, name), svg(w, h, body, title));
};

// ---- shared shapes ----
const sparkle = (x, y, r) => `<path d="M${n(x)} ${n(y - r)}C${n(x + r * 0.12)} ${n(y - r * 0.25)} ${n(x + r * 0.25)} ${n(y - r * 0.12)} ${n(x + r)} ${n(y)}C${n(x + r * 0.25)} ${n(y + r * 0.12)} ${n(x + r * 0.12)} ${n(y + r * 0.25)} ${n(x)} ${n(y + r)}C${n(x - r * 0.12)} ${n(y + r * 0.25)} ${n(x - r * 0.25)} ${n(y + r * 0.12)} ${n(x - r)} ${n(y)}C${n(x - r * 0.25)} ${n(y - r * 0.12)} ${n(x - r * 0.12)} ${n(y - r * 0.25)} ${n(x)} ${n(y - r)}Z" fill="currentColor" stroke="none"/>`;
const heartPath = (cx, cy, s) => {
  const p = (x, y) => `${n(cx + (x - 12) * s)} ${n(cy + (y - 11.5) * s)}`;
  return `M${p(12, 21)}C${p(5, 15)} ${p(2, 11)} ${p(2, 7.5)}C${p(2, 4.5)} ${p(4.5, 2)} ${p(7.5, 2)}C${p(9.5, 2)} ${p(11, 3)} ${p(12, 4.5)}C${p(13, 3)} ${p(14.5, 2)} ${p(16.5, 2)}C${p(19.5, 2)} ${p(22, 4.5)} ${p(22, 7.5)}C${p(22, 11)} ${p(19, 15)} ${p(12, 21)}Z`;
};
// A right wing in a 100 x 64 box, shoulder at the left; `t` maps a point.
const wing = t => {
  const P = (x, y) => { const [a, b] = t(x, y); return `${n(a)} ${n(b)}`; };
  const outline = `M${P(8, 30)}C${P(26, 8)} ${P(66, 2)} ${P(94, 12)}C${P(99, 14)} ${P(98, 21)} ${P(93, 23)}C${P(84, 26)} ${P(79, 28)} ${P(74, 31)}C${P(79, 37)} ${P(75, 44)} ${P(66, 44)}C${P(63, 51)} ${P(55, 54)} ${P(47, 51)}C${P(45, 58)} ${P(35, 62)} ${P(27, 57)}C${P(18, 52)} ${P(11, 42)} ${P(8, 30)}Z`;
  const feathers = [`M${P(28, 25)}C${P(46, 20)} ${P(64, 18)} ${P(82, 20)}`, `M${P(26, 35)}C${P(40, 32)} ${P(54, 32)} ${P(68, 35)}`, `M${P(24, 45)}C${P(32, 44)} ${P(40, 44)} ${P(48, 45)}`];
  return `<path d="${outline}" fill="currentColor" fill-opacity="0.16" stroke-width="2"/>\n` + feathers.map(d => `<path d="${d}" stroke-width="1.6" stroke-opacity="0.7"/>`).join("\n");
};
const affine = (sx, sy, dx, dy, deg = 0) => (x, y) => {
  const r = deg * Math.PI / 180, X = x * sx, Y = y * sy;
  return [X * Math.cos(r) - Y * Math.sin(r) + dx, X * Math.sin(r) + Y * Math.cos(r) + dy];
};
// Scallops hanging below a polyline sampled from f(t), t in [0, 1].
const scallops = (f, count, depth) => {
  let d = "";
  for (let i = 0; i < count; i += 1) {
    const [x0, y0] = f(i / count), [x1, y1] = f((i + 1) / count);
    const mx = (x0 + x1) / 2, my = (y0 + y1) / 2, len = Math.hypot(x1 - x0, y1 - y0);
    const nx = -(y1 - y0) / len, ny = (x1 - x0) / len;
    d += `${i === 0 ? `M${n(x0)} ${n(y0)}` : ""}Q${n(mx + nx * depth * 2)} ${n(my + ny * depth * 2)} ${n(x1)} ${n(y1)}`;
  }
  return d;
};
const eyelets = (f, count, offset, r) => Array.from({ length: count }, (_, i) => {
  const t = (i + 0.5) / count, [x, y] = f(t), [xa, ya] = f(Math.max(0, t - 0.01)), [xb, yb] = f(Math.min(1, t + 0.01));
  const len = Math.hypot(xb - xa, yb - ya) || 1, nx = -(yb - ya) / len, ny = (xb - xa) / len;
  return `<circle cx="${n(x + nx * offset)}" cy="${n(y + ny * offset)}" r="${r}" fill="currentColor" stroke="none"/>`;
}).join("\n");
const bow = (cx, cy, s) => {
  const p = (x, y) => `${n(cx + (x - 16) * s)} ${n(cy + (y - 16) * s)}`;
  return `<path d="M${p(16, 14)}C${p(10, 5)} ${p(2, 7)} ${p(3, 13)}C${p(4, 20)} ${p(11, 19)} ${p(16, 16)}Z" fill="currentColor" fill-opacity="0.22" stroke-width="${n(1.8 * Math.min(1, s + 0.2))}"/>
<path d="M${p(16, 14)}C${p(22, 5)} ${p(30, 7)} ${p(29, 13)}C${p(28, 20)} ${p(21, 19)} ${p(16, 16)}Z" fill="currentColor" fill-opacity="0.22" stroke-width="${n(1.8 * Math.min(1, s + 0.2))}"/>
<path d="M${p(14.5, 17)}L${p(10, 29)}L${p(13, 27.5)}L${p(14.5, 30)}L${p(16, 18)}" fill="currentColor" fill-opacity="0.22" stroke-width="${n(1.6 * Math.min(1, s + 0.2))}"/>
<path d="M${p(17.5, 17)}L${p(22, 29)}L${p(19, 27.5)}L${p(17.5, 30)}L${p(16, 18)}" fill="currentColor" fill-opacity="0.22" stroke-width="${n(1.6 * Math.min(1, s + 0.2))}"/>
<rect x="${n(cx - 2.6 * s)}" y="${n(cy - 3.4 * s)}" width="${n(5.2 * s)}" height="${n(6.2 * s)}" rx="${n(2 * s)}" fill="currentColor" stroke="none"/>`;
};

// ================= 天使界隈 Tenshi =================
write("tenshi", "page-corner.svg", 160, 160, `
${wing(affine(-1.05, 1.05, 156, 30, -18))}
<ellipse cx="118" cy="22" rx="24" ry="7" stroke-width="3"/>
${sparkle(62, 30, 6)}
${sparkle(146, 100, 5)}
${sparkle(84, 74, 3.5)}`, "Tenshi page corner: wing and halo");

write("tenshi", "empty-state.svg", 120, 96, `
<path d="M28 74C16 74 10 66 10 58C10 49 18 43 27 44C29 33 38 26 50 26C60 26 68 32 71 41C74 39 78 38 82 38C93 38 102 46 102 57C102 67 94 74 84 74Z" fill="currentColor" fill-opacity="0.12" stroke-width="2.4"/>
<ellipse cx="56" cy="16" rx="16" ry="5" stroke-width="2.4"/>
${sparkle(100, 22, 6)}
${sparkle(18, 30, 4)}
${sparkle(108, 82, 3.5)}`, "Tenshi empty state: cloud and halo");

write("tenshi", "dialog-header.svg", 32, 32, `
${wing(affine(0.3, 0.3, 2, 7))}`, "Tenshi dialog header: small wing");

write("tenshi", "panel-corner.svg", 48, 48, `
<path d="M14 6A30 30 0 0 1 42 34" stroke-width="2.2"/>
<path d="M22 6A22 22 0 0 1 42 26" stroke-width="1.4" stroke-opacity="0.6"/>
${sparkle(40, 10, 5)}`, "Tenshi panel corner: halo arc");

write("tenshi", "stage-edge.svg", 64, 16, `
<path d="M0 8H22M42 8H64" stroke-width="1.4" stroke-dasharray="1 5"/>
${sparkle(32, 8, 6)}`, "Tenshi stage edge tile: sparkle line");

write("tenshi", "album-cover.svg", 320, 240, `
<rect x="104" y="70" width="112" height="132" rx="18" stroke-width="2.4"/>
<rect x="114" y="80" width="92" height="112" rx="12" fill="currentColor" fill-opacity="0.08" stroke-width="1.4" stroke-opacity="0.6"/>
<ellipse cx="160" cy="44" rx="34" ry="9" stroke-width="3"/>
${wing(affine(-1.05, 1.05, 106, 96, -10))}
${wing(affine(1.05, 1.05, 214, 96, 10))}
${sparkle(58, 52, 7)}
${sparkle(266, 186, 6)}
${sparkle(246, 40, 4)}`, "Tenshi album cover: frame with wings and halo");

write("tenshi", "photo-corner.svg", 40, 40, `
<g transform="rotate(-45 20 20)">
<rect x="2" y="13" width="36" height="14" rx="7" fill="currentColor" fill-opacity="0.14" stroke-width="1.8"/>
<rect x="14" y="15.5" width="12" height="9" rx="2.5" fill="currentColor" fill-opacity="0.22" stroke-width="1.2"/>
<circle cx="7.5" cy="18" r="0.9" fill="currentColor" stroke="none"/><circle cx="7.5" cy="22" r="0.9" fill="currentColor" stroke="none"/>
<circle cx="32.5" cy="18" r="0.9" fill="currentColor" stroke="none"/><circle cx="32.5" cy="22" r="0.9" fill="currentColor" stroke="none"/>
</g>`, "Tenshi photo corner: bandage");

write("tenshi", "canvas-pattern.svg", 96, 96, `
<g opacity="0.08" stroke-width="1.6">
<path d="M20 14V26M14 20H26"/>
<path d="M68 62V74M62 68H74"/>
</g>
<g opacity="0.08">
${sparkle(70, 20, 4)}
${sparkle(22, 70, 3)}
</g>`, "Tenshi canvas pattern tile: crosses and sparkles");

write("tenshi", "room-plate.svg", 220, 56, `
<rect x="34" y="12" width="152" height="32" rx="16" fill="currentColor" fill-opacity="0.1" stroke-width="2"/>
${wing(affine(-0.36, 0.36, 38, 16, -8))}
${wing(affine(0.36, 0.36, 182, 16, 8))}`, "Tenshi room plate with wings");

write("tenshi", "pill.svg", 24, 24, `
<g transform="rotate(-40 12 12)">
<rect x="3" y="8" width="18" height="8" rx="4" stroke-width="1.8"/>
<path d="M12 8V16" stroke-width="1.8"/>
<path d="M12 8H17A4 4 0 0 1 17 16H12Z" fill="currentColor" fill-opacity="0.3" stroke="none"/>
</g>`, "Tenshi pill");

write("tenshi", "heart-wings.svg", 24, 24, `
${wing(affine(-0.19, 0.19, 10, 6, -12))}
${wing(affine(0.19, 0.19, 14, 6, 12))}
<path d="${heartPath(12, 14, 0.5)}" fill="currentColor" fill-opacity="0.35" stroke-width="1.6"/>`, "Tenshi heart with wings");

// ================= 地雷系 Jirai =================
const quarter = (cx, cy, r, a0, a1) => t => { const a = (a0 + (a1 - a0) * t) * Math.PI / 180; return [cx + r * Math.cos(a), cy + r * Math.sin(a)]; };

{
  const outer = quarter(160, 0, 118, 180, 90);
  const inner = quarter(160, 0, 98, 180, 90);
  write("jirai", "page-corner.svg", 160, 160, `
<path d="M42 0A118 118 0 0 0 160 118" stroke-width="1.6" stroke-opacity="0.7"/>
<path d="${scallops(t => outer(t), 9, 4.5)}" stroke-width="1.8"/>
${eyelets(inner, 9, 0, 1.6)}
<path d="M62 0A98 98 0 0 0 160 98" stroke-width="1" stroke-dasharray="2 3" stroke-opacity="0.6"/>
${bow(120, 34, 1.15)}`, "Jirai page corner: lace and bow");
}

write("jirai", "empty-state.svg", 120, 96, `
<path d="${heartPath(60, 54, 3.1)}" fill="currentColor" fill-opacity="0.14" stroke-width="2.4"/>
<path d="M48 40L56 48M56 40L48 48M64 58L72 66M72 58L64 66" stroke-width="1.8"/>
${bow(60, 20, 0.9)}`, "Jirai empty state: stitched heart and bow");

write("jirai", "dialog-header.svg", 32, 32, bow(16, 16, 1), "Jirai dialog header: bow");

{
  const arc = quarter(48, 0, 40, 180, 90);
  write("jirai", "panel-corner.svg", 48, 48, `
<path d="${scallops(arc, 5, 2.6)}" stroke-width="1.6"/>
${eyelets(quarter(48, 0, 33, 180, 90), 5, 0, 1.1)}`, "Jirai panel corner: lace");
}

write("jirai", "stage-edge.svg", 64, 16, `
<path d="M0 3H64" stroke-width="1.4"/>
<path d="${scallops(t => [t * 64, 3], 4, 3)}" stroke-width="1.6"/>
${eyelets(t => [t * 64, 3], 4, 3.4, 1.2)}`, "Jirai stage edge tile: lace border");

{
  const top = t => [70 + t * 180, 40], bottom = t => [250 - t * 180, 210];
  const left = t => [70, 210 - t * 170], right = t => [250, 40 + t * 170];
  write("jirai", "album-cover.svg", 320, 240, `
<rect x="70" y="40" width="180" height="170" rx="10" stroke-width="2"/>
<rect x="88" y="58" width="144" height="134" rx="6" fill="currentColor" fill-opacity="0.08" stroke-width="1.2" stroke-opacity="0.6"/>
<path d="${scallops(top, 12, -3.6)}" stroke-width="1.6"/>
<path d="${scallops(right, 11, -3.6)}" stroke-width="1.6"/>
<path d="${scallops(bottom, 12, -3.6)}" stroke-width="1.6"/>
<path d="${scallops(left, 11, -3.6)}" stroke-width="1.6"/>
${bow(160, 40, 1.9)}`, "Jirai album cover: lace frame and bow");
}

write("jirai", "photo-corner.svg", 40, 40, `
<path d="M24 16L36 4" stroke-width="2"/>
<circle cx="36" cy="4" r="2.6" fill="currentColor" stroke="none"/>
<path d="${heartPath(17, 22, 1.05)}" fill="currentColor" fill-opacity="0.3" stroke-width="1.8"/>`, "Jirai photo corner: heart pin");

write("jirai", "canvas-pattern.svg", 96, 96, `
<g opacity="0.08">
<path d="${heartPath(24, 24, 0.5)}" fill="currentColor" stroke="none"/>
<path d="${heartPath(72, 72, 0.4)}" fill="currentColor" stroke="none"/>
<circle cx="72" cy="22" r="1.6" fill="currentColor" stroke="none"/><circle cx="66" cy="28" r="1.2" fill="currentColor" stroke="none"/><circle cx="78" cy="28" r="1.2" fill="currentColor" stroke="none"/>
<circle cx="22" cy="72" r="1.6" fill="currentColor" stroke="none"/><circle cx="16" cy="78" r="1.2" fill="currentColor" stroke="none"/><circle cx="28" cy="78" r="1.2" fill="currentColor" stroke="none"/>
</g>`, "Jirai canvas pattern tile: hearts and lace dots");

write("jirai", "room-plate.svg", 220, 56, `
<rect x="30" y="10" width="160" height="30" rx="8" fill="currentColor" fill-opacity="0.1" stroke-width="2"/>
<path d="M44 40L36 54L46 49L52 54L54 40M176 40L184 54L174 49L168 54L166 40" fill="currentColor" fill-opacity="0.2" stroke-width="1.6"/>
<path d="${scallops(t => [30 + t * 160, 40], 16, 2)}" stroke-width="1.2" stroke-opacity="0.7"/>
${bow(30, 25, 0.8)}`, "Jirai room plate with ribbon tails");

write("jirai", "energy-can.svg", 24, 24, `
<path d="M8 4.5H16L17 6.5V19.5L16 21H8L7 19.5V6.5Z" fill="currentColor" fill-opacity="0.14" stroke-width="1.6"/>
<path d="M8.5 3H15.5" stroke-width="1.6"/>
<path d="M7 8H17M7 18H17" stroke-width="1.2" stroke-opacity="0.7"/>
<path d="${heartPath(12, 13, 0.28)}" fill="currentColor" stroke="none"/>`, "Jirai plain energy-drink can");

write("jirai", "safety-pin.svg", 24, 24, `
<g transform="rotate(-45 12 12)">
<path d="M5 12H17.5A2.5 2.5 0 0 0 17.5 7H8" stroke-width="1.6"/>
<path d="M5 12H16" stroke-width="1.6"/>
<rect x="2.5" y="9.5" width="4.5" height="5" rx="1.5" fill="currentColor" fill-opacity="0.3" stroke-width="1.4"/>
<circle cx="20" cy="9.5" r="0.01" stroke-width="3"/>
</g>
<path d="${heartPath(18, 18, 0.22)}" fill="currentColor" stroke="none"/>`, "Jirai safety pin with heart");

// ---- Glyph slots: one-color marks drawn as masks in the room or accent color ----
write("jirai", "heart.svg", 24, 24, `
<path d="${heartPath(12, 12.5, 0.82)}" fill="currentColor" fill-opacity="0.3" stroke-width="1.8"/>
<path d="${heartPath(12, 12.5, 0.56)}" stroke-width="1" stroke-dasharray="1.4 1.6"/>`, "Jirai selection mark: stitched heart");

// ================= Frutiger Aero =================
const bubble = (cx, cy, r) => `<circle cx="${n(cx)}" cy="${n(cy)}" r="${n(r)}" fill="currentColor" fill-opacity="0.18" stroke-width="${n(Math.max(1.2, r * 0.16))}"/>
<path d="M${n(cx - r * 0.62)} ${n(cy - r * 0.12)}A${n(r * 0.64)} ${n(r * 0.64)} 0 0 1 ${n(cx - r * 0.1)} ${n(cy - r * 0.64)}" stroke-width="${n(Math.max(1, r * 0.14))}"/>`;
const leaf = (x, y, s, deg) => `<g transform="translate(${n(x)} ${n(y)}) rotate(${deg}) scale(${s})"><path d="M0 0C4 -8 12 -10 18 -9C17 -2 11 4 0 0Z" fill="currentColor" fill-opacity="0.45" stroke-width="1.4"/><path d="M1 -1C6 -4 10 -6 14 -7" stroke-width="1" stroke-opacity="0.8"/></g>`;

write("frutiger-aero", "bubble.svg", 24, 24, `
${bubble(13, 13, 8)}
${bubble(5, 6, 3)}`, "Frutiger Aero selection mark: bubbles");

write("frutiger-aero", "empty-state.svg", 120, 96, `
${bubble(56, 44, 22)}
${bubble(90, 30, 9)}
${bubble(24, 22, 6)}
${leaf(70, 74, 1.3, -18)}
<path d="M8 84C24 78 40 90 56 84S88 78 112 84" stroke-width="2"/>`, "Frutiger Aero empty state: bubbles, leaf and water");

// ---- Pass 2 (author review 2026-10-28): Vista-era Aero, Jirai's claw can and black lace, Tenshi's stripes and techwear ----
// These carry their own colors, so they render as images rather than one-color masks.
const writeRaw = (skin, name, w, h, body, title) => {
  const dir = path.join(root, skin, "motifs");
  mkdirSync(dir, { recursive: true });
  writeFileSync(path.join(dir, name), `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}">
  <title>${title}</title>
${body.trim().split("\n").map(l => "  " + l.trim()).join("\n")}
</svg>
`);
};

// Tenshi: a zipper tile on light-blue tape, teeth interlocking from both edges.
writeRaw("tenshi", "zipper.svg", 24, 10, `
<rect width="24" height="10" fill="#dcedff"/>
<rect width="24" height="1" fill="#9fc3e8"/><rect y="9" width="24" height="1" fill="#9fc3e8"/>
${[1, 13].map(x => `<rect x="${x}" y="1.5" width="4.5" height="4" rx="1" fill="#c7d5e3" stroke="#7d93aa" stroke-width="0.6"/><rect x="${x + 0.8}" y="2.1" width="2.6" height="0.9" rx="0.4" fill="#ffffff" fill-opacity="0.9"/>`).join("\n")}
${[7, 19].map(x => `<rect x="${x}" y="4.5" width="4.5" height="4" rx="1" fill="#c7d5e3" stroke="#7d93aa" stroke-width="0.6"/><rect x="${x + 0.8}" y="5.1" width="2.6" height="0.9" rx="0.4" fill="#ffffff" fill-opacity="0.9"/>`).join("\n")}`, "Tenshi zipper tile");

// Tenshi: a pixel heart, the style's digital accent.
{
  const rows = [".XX.XX.", "XXXXXXX", "XXXXXXX", ".XXXXX.", "..XXX..", "...X..."];
  const cells = rows.flatMap((row, y) => [...row].map((c, x) => c === "X" ? `<rect x="${1.5 + x * 3}" y="${3 + y * 3}" width="3" height="3"/>` : "")).filter(Boolean);
  writeRaw("tenshi", "pixel-heart.svg", 24, 24, `<g fill="#000000">${cells.join("")}</g>`, "Tenshi selection mark: pixel heart");
}

// Jirai: black lace hanging from a pink ribbon band.
writeRaw("jirai", "lace-black.svg", 32, 12, `
<rect width="32" height="3" fill="#eaa0c2" fill-opacity="0.6"/>
<path d="M0 3Q4 12.5 8 3Q12 12.5 16 3Q20 12.5 24 3Q28 12.5 32 3Z" fill="#0b0508" stroke="#eaa0c2" stroke-opacity="0.8" stroke-width="0.8"/>
${[4, 12, 20, 28].map(x => `<circle cx="${x}" cy="6" r="1.2" fill="#5a2a40"/>`).join("")}`, "Jirai black lace tile");

// Jirai: a pink energy-drink can with three wavy lines, a bow on top and black lace at its foot.
writeRaw("jirai", "claw-can.svg", 192, 128, `
<defs>
<linearGradient id="can" x1="0" y1="0" x2="1" y2="0">
<stop offset="0" stop-color="#b8487c"/><stop offset="0.22" stop-color="#f6b3d1"/><stop offset="0.36" stop-color="#ffe3ef"/>
<stop offset="0.52" stop-color="#ee8fbb"/><stop offset="1" stop-color="#9c3a69"/>
</linearGradient>
<linearGradient id="rim" x1="0" y1="0" x2="1" y2="0">
<stop offset="0" stop-color="#6b6170"/><stop offset="0.4" stop-color="#e9e3ec"/><stop offset="1" stop-color="#5a5060"/>
</linearGradient>
</defs>
<ellipse cx="74" cy="118" rx="34" ry="5" fill="#000000" fill-opacity="0.35"/>
<path d="M50 30Q50 24 56 22H92Q98 24 98 30V108Q98 114 92 116H56Q50 114 50 108Z" fill="url(#can)" stroke="#2a0f1c" stroke-width="1.5"/>
<path d="M53 22Q53 16 60 15H88Q95 16 95 22Z" fill="url(#rim)" stroke="#2a0f1c" stroke-width="1.2"/>
<ellipse cx="74" cy="15" rx="14" ry="2.6" fill="#d9d2dc" stroke="#2a0f1c" stroke-width="1"/>
<path d="M53 112Q53 118 60 119H88Q95 118 95 112Z" fill="url(#rim)" stroke="#2a0f1c" stroke-width="1.2"/>
${[62, 74, 86].map((x, i) => `<path d="M${x} 40C${x - 5} 50 ${x + 5} 58 ${x} 68C${x - 5} 78 ${x + 5} 86 ${x} 96" fill="none" stroke="#1a0710" stroke-width="${i === 1 ? 4.2 : 3.6}" stroke-linecap="round"/>`).join("\n")}
<path d="M50 100H98" stroke="#2a0f1c" stroke-width="1" stroke-opacity="0.5"/>
<g transform="translate(118 74)">
<path d="M0 0C-14 -16 -30 -10 -26 2C-22 14 -8 10 0 4Z" fill="#f2a6c9" stroke="#2a0f1c" stroke-width="1.4"/>
<path d="M0 0C14 -16 30 -10 26 2C22 14 8 10 0 4Z" fill="#f2a6c9" stroke="#2a0f1c" stroke-width="1.4"/>
<path d="M-3 5L-12 30L-6 27L-3 32L1 7Z" fill="#e58ab4" stroke="#2a0f1c" stroke-width="1.2"/>
<path d="M3 5L12 30L6 27L3 32L-1 7Z" fill="#e58ab4" stroke="#2a0f1c" stroke-width="1.2"/>
<rect x="-5" y="-4" width="10" height="11" rx="3" fill="#1a0710"/>
</g>
<path d="${heartPath(160, 40, 1.1)}" fill="#1a0710" stroke="#eaa0c2" stroke-width="1.4"/>
<path d="M153 33L167 47M167 33L153 47" stroke="#eaa0c2" stroke-width="1" stroke-opacity="0.7"/>
<rect x="0" y="118" width="192" height="3" fill="#eaa0c2" fill-opacity="0.6"/>
<path d="${Array.from({ length: 12 }, (_, i) => `${i === 0 ? "M0 121" : ""}Q${i * 16 + 8} 131 ${(i + 1) * 16} 121`).join("")}Z" fill="#0b0508" stroke="#eaa0c2" stroke-opacity="0.8" stroke-width="0.8"/>`, "Jirai page corner: pink energy-drink can with wavy lines, bow and black lace");

// Frutiger Aero (Vista era): a glass orb for the mark slot.
write("frutiger-aero", "orb.svg", 24, 24, `
<circle cx="12" cy="12" r="10" fill="currentColor" fill-opacity="0.45" stroke-width="1.4"/>
<path d="M5 10A7.5 6 0 0 1 19 10C16 8.6 8 8.6 5 10Z" fill="currentColor" stroke="none"/>
<path d="M8 17A6 3 0 0 0 16 17" stroke-width="1.2" stroke-opacity="0.8"/>`, "Frutiger Aero mark: glass orb");

// ---- Pass 3 (from-scratch mockups approved 2026-10-28): Jirai's ribbons and charms, Tenshi's techwear ----
const pink = "#f2a6c9", pinkDeep = "#d9779f", plum = "#120810";
const heartD = "M12 21C5 15 2 11 2 7.5 2 4.5 4.5 2 7.5 2c2 0 3.5 1 4.5 2.5C13 3 14.5 2 16.5 2 19.5 2 22 4.5 22 7.5 22 11 19 15 12 21z";

writeRaw("jirai", "bow.svg", 64, 44, `
<defs><linearGradient id="s" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ffd9ea"/><stop offset="0.45" stop-color="${pink}"/><stop offset="1" stop-color="${pinkDeep}"/></linearGradient></defs>
<path d="M32 18C22 2 4 4 6 16C8 28 22 26 32 22Z" fill="url(#s)" stroke="${plum}" stroke-width="1.6"/>
<path d="M32 18C42 2 60 4 58 16C56 28 42 26 32 22Z" fill="url(#s)" stroke="${plum}" stroke-width="1.6"/>
<path d="M29 22L20 42L26 39L29 44L32 24Z" fill="url(#s)" stroke="${plum}" stroke-width="1.4"/>
<path d="M35 22L44 42L38 39L35 44L32 24Z" fill="url(#s)" stroke="${plum}" stroke-width="1.4"/>
<rect x="27" y="14" width="10" height="11" rx="3.5" fill="${plum}"/>
<path d="M10 12C14 8 20 9 24 13M54 12C50 8 44 9 40 13" stroke="#ffffff" stroke-opacity="0.6" stroke-width="1.4" fill="none"/>`, "Jirai satin bow");

writeRaw("jirai", "heart-filled.svg", 24, 24, `<path d="${heartD}" fill="${pink}" stroke="${plum}" stroke-width="1.6"/>`, "Jirai filled heart");
writeRaw("jirai", "heart-glow.svg", 24, 24, `<path d="${heartD}" fill="${pink}" stroke="#ffffff" stroke-width="1.6"/>`, "Jirai current heart");
writeRaw("jirai", "heart-outline.svg", 24, 24, `<path d="${heartD}" fill="#1a0d14" stroke="${pink}" stroke-width="1.6"/>`, "Jirai outline heart");

writeRaw("jirai", "chain.svg", 22, 10, `<g fill="none" stroke="${pink}" stroke-width="1.6"><rect x="1" y="2" width="12" height="6" rx="3" stroke-opacity="0.75"/><rect x="11" y="2" width="10" height="6" rx="3" stroke-opacity="0.45"/></g>`, "Jirai chain tile");

writeRaw("jirai", "safety-pin-pink.svg", 56, 20, `
<g fill="none" stroke="#d8d0dc" stroke-width="2" stroke-linecap="round"><path d="M10 10H50a4 4 0 0 0 0-8H14"/><path d="M10 10H46"/></g>
<rect x="2" y="5" width="10" height="10" rx="3" fill="${pink}" stroke="${plum}"/>`, "Jirai safety pin");

writeRaw("jirai", "bandage.svg", 64, 24, `
<rect x="1" y="2" width="62" height="20" rx="10" fill="#f9c3da" stroke="${plum}" stroke-width="1.4"/>
<rect x="22" y="5" width="20" height="14" rx="3" fill="#fde3ee" stroke="${pinkDeep}" stroke-width="1"/>
<g fill="${pinkDeep}"><circle cx="9" cy="9" r="1"/><circle cx="9" cy="15" r="1"/><circle cx="14" cy="12" r="1"/><circle cx="55" cy="9" r="1"/><circle cx="55" cy="15" r="1"/><circle cx="50" cy="12" r="1"/></g>`, "Jirai pink bandage");

writeRaw("jirai", "hearts-tile.svg", 64, 64, `
<g fill="${pink}" fill-opacity="0.07"><path transform="translate(6 6) scale(0.6)" d="${heartD}"/><path transform="translate(38 38) scale(0.45)" d="${heartD}"/></g>
<path d="M48 8v10M43 13h10" stroke="${pink}" stroke-opacity="0.08" stroke-width="1.5"/>`, "Jirai canvas tile: hearts and crosses");

{
  const sky = "#4a9be8", skyDeep = "#2a72c8", skyLine = "#8ec5f5", skyPale = "#dcedff";
  writeRaw("tenshi", "pull-tab.svg", 22, 40, `
<defs><linearGradient id="m" x1="0" x2="1"><stop offset="0" stop-color="#9fb0c2"/><stop offset="0.45" stop-color="#ffffff"/><stop offset="1" stop-color="#8496aa"/></linearGradient></defs>
<rect x="5" y="0" width="12" height="10" rx="3" fill="url(#m)" stroke="#5d7088"/>
<rect x="4" y="9" width="14" height="26" rx="7" fill="url(#m)" stroke="#5d7088"/>
<rect x="8" y="15" width="6" height="14" rx="3" fill="#cfe6ff" stroke="#5d7088"/>
<path d="M11 33l1 3 3 .2-2.4 2 .8 3-2.4-1.7-2.4 1.7.8-3-2.4-2 3-.2z" fill="${sky}"/>`, "Tenshi zipper pull with a star");

  writeRaw("tenshi", "buckle.svg", 26, 44, `
<rect x="1" y="1" width="24" height="18" rx="4" fill="#ffffff" stroke="${skyDeep}" stroke-width="1.5"/>
<rect x="6" y="6" width="14" height="8" rx="2" fill="${skyPale}" stroke="${skyDeep}"/>
<path d="M3 19h20l-3 22H6z" fill="#ffffff" stroke="${skyDeep}" stroke-width="1.5"/>
<path d="M9 24h8M9 29h8M9 34h8" stroke="${skyLine}" stroke-width="1.5"/>`, "Tenshi side-release buckle");

  writeRaw("tenshi", "wing.svg", 60, 36, `
<path d="M58 22C46 6 22 0 4 6C1 7 2 11 5 11C10 11 13 12 16 14C12 16 13 20 18 20C19 24 24 26 28 24C30 28 37 30 42 28C49 26 55 26 58 22Z" fill="#ffffff" stroke="${sky}" stroke-width="1.6"/>
<path d="M14 9C24 9 34 12 44 17M20 15C28 15 36 18 44 22" stroke="${skyLine}" stroke-width="1.2" fill="none"/>`, "Tenshi wing, pointing left");

  writeRaw("tenshi", "wing-right.svg", 60, 36, `
<g transform="translate(60 0) scale(-1 1)"><path d="M58 22C46 6 22 0 4 6C1 7 2 11 5 11C10 11 13 12 16 14C12 16 13 20 18 20C19 24 24 26 28 24C30 28 37 30 42 28C49 26 55 26 58 22Z" fill="#ffffff" stroke="${sky}" stroke-width="1.6"/>
<path d="M14 9C24 9 34 12 44 17M20 15C28 15 36 18 44 22" stroke="${skyLine}" stroke-width="1.2" fill="none"/></g>`, "Tenshi wing, pointing right");

  writeRaw("tenshi", "cloud.svg", 220, 120, `
<g fill="#ffffff" fill-opacity="0.9"><ellipse cx="60" cy="80" rx="50" ry="22"/><ellipse cx="95" cy="62" rx="34" ry="28"/><ellipse cx="130" cy="78" rx="44" ry="22"/></g>
<path d="M180 24v12M174 30h12" stroke="${skyLine}" stroke-width="2"/><path d="M30 26v8M26 30h8" stroke="${skyLine}" stroke-width="1.6"/>`, "Tenshi cloud with sparkles");

  writeRaw("tenshi", "zipper-tape.svg", 16, 12, `
<rect width="16" height="12" fill="#cfe6ff"/><rect width="16" height="1.5" fill="${skyLine}"/><rect y="10.5" width="16" height="1.5" fill="${skyLine}"/>
<rect x="1" y="2" width="5" height="4.4" rx="1.2" fill="#e8eef5" stroke="#7f93a8" stroke-width="0.7"/>
<rect x="9" y="5.6" width="5" height="4.4" rx="1.2" fill="#e8eef5" stroke="#7f93a8" stroke-width="0.7"/>`, "Tenshi zipper tape tile");
}

// ---- Home: outside Grid, the top-left button is a house in the theme's style (author ruling 2026-10-28) ----
writeRaw("frutiger-aero", "house.svg", 24, 24, `
<path fill-rule="evenodd" d="M12 2.5L1.5 11.5H4.5V21.5H19.5V11.5H22.5ZM10 21.5V15.5H14V21.5Z" fill="#000000"/>`, "Frutiger Aero home: house glyph for the orb");

writeRaw("jirai", "house.svg", 40, 36, `
<defs><linearGradient id="h" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ffd9ea"/><stop offset="0.5" stop-color="${pink}"/><stop offset="1" stop-color="${pinkDeep}"/></linearGradient></defs>
<path d="M8 16V33H32V16L20 6Z" fill="url(#h)" stroke="${plum}" stroke-width="1.6" stroke-linejoin="round"/>
<path d="M3 18L20 3.5L37 18" fill="none" stroke="${plum}" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M3 18L20 3.5L37 18" fill="none" stroke="${pink}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
<path transform="translate(14.6 12.4) scale(0.45)" d="${heartD}" fill="${plum}"/>
<path d="M17 33V27a3 3 0 0 1 6 0V33Z" fill="${plum}"/>`, "Jirai home: pink house with a heart window");

writeRaw("tenshi", "house.svg", 28, 28, `
<path d="M5 13V25H23V13L14 5.5Z" fill="#ffffff" stroke="#2a72c8" stroke-width="1.6" stroke-linejoin="round"/>
<path d="M2.5 14L14 4L25.5 14" fill="none" stroke="#4a9be8" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M14 11.5l1 2.2 2.4.2-1.8 1.6.6 2.4-2.2-1.3-2.2 1.3.6-2.4-1.8-1.6 2.4-.2z" fill="#4a9be8"/>
<path d="M11.5 25V21a2.5 2.5 0 0 1 5 0V25" fill="#dcedff" stroke="#2a72c8" stroke-width="1.4"/>`, "Tenshi home: white house with a star window");

console.log("ok");
