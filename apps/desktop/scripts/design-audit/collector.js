// In-page collector for the design audit. This function is serialized with
// Function.toString() and executed inside the audited page via
// webContents.executeJavaScript, so it must stay dependency-free and
// ES2020-safe. It walks rendered elements, resolves the --vua-* token palette
// for the active theme/module scope, and returns one serializable snapshot.

export function collectAuditSnapshot(options) {
  const opts = options || { proposed: true, overflowChecks: true };
  const MAX_TEXT = 60;
  const interactiveControlSelector = [
    "button", "a[href]", "input", "select", "textarea", "summary",
    '[role="button"]', '[role="tab"]', '[role="menuitem"]', '[role="option"]',
    '[role="checkbox"]', '[role="radio"]', '[role="switch"]', '[role="link"]',
    '[tabindex]:not([tabindex="-1"])',
  ].join(",");

  function parseColor(text) {
    if (typeof text !== "string") return null;
    const value = text.trim();
    const rgb = value.match(/^rgba?\(([^)]+)\)$/i);
    if (rgb) {
      const parts = rgb[1].split(/[\s,/]+/).filter(Boolean).map(Number);
      if (parts.length >= 3 && parts.every(n => Number.isFinite(n))) {
        return { r: parts[0], g: parts[1], b: parts[2], a: parts.length > 3 ? parts[3] : 1 };
      }
      return null;
    }
    const hex = value.match(/^#([0-9a-f]+)$/i);
    if (hex) {
      const d = hex[1];
      const expand = s => parseInt(s.length === 1 ? s + s : s, 16);
      if (d.length === 3 || d.length === 4) {
        return { r: expand(d[0]), g: expand(d[1]), b: expand(d[2]), a: d.length === 4 ? expand(d[3]) / 255 : 1 };
      }
      if (d.length === 6 || d.length === 8) {
        return {
          r: parseInt(d.slice(0, 2), 16), g: parseInt(d.slice(2, 4), 16), b: parseInt(d.slice(4, 6), 16),
          a: d.length === 8 ? parseInt(d.slice(6, 8), 16) / 255 : 1,
        };
      }
    }
    return null;
  }

  function composite(top, bottom) {
    const a = top.a;
    return { r: top.r * a + bottom.r * (1 - a), g: top.g * a + bottom.g * (1 - a), b: top.b * a + bottom.b * (1 - a), a: 1 };
  }

  function rgbDistance(a, b) {
    return Math.sqrt((a.r - b.r) ** 2 + (a.g - b.g) ** 2 + (a.b - b.b) ** 2);
  }

  // ---- token palette ---------------------------------------------------------
  const customNames = new Set();
  function gatherNames(rules) {
    for (const rule of rules) {
      if (rule.style) {
        for (const prop of rule.style) if (prop.startsWith("--vua-")) customNames.add(prop);
      }
      if (rule.cssRules) gatherNames(rule.cssRules);
    }
  }
  for (const sheet of document.styleSheets) {
    try { if (sheet.cssRules) gatherNames(sheet.cssRules); } catch { /* cross-origin */ }
  }

  const moduleRoot = document.querySelector("body [data-module], body[data-module]") || document.body;
  function resolveTokenColor(name, host) {
    const probe = document.createElement("div");
    probe.style.position = "absolute";
    probe.style.display = "none";
    host.appendChild(probe);
    let resolved = null;
    try {
      probe.style.color = "var(" + name + ")";
      resolved = parseColor(getComputedStyle(probe).color);
    } catch { resolved = null; }
    probe.remove();
    return resolved;
  }
  function resolveTokenRaw(name, host) {
    const probe = document.createElement("div");
    probe.style.position = "absolute";
    probe.style.display = "none";
    host.appendChild(probe);
    let raw = "";
    try { raw = getComputedStyle(probe).getPropertyValue(name).trim(); } catch { raw = ""; }
    probe.remove();
    return raw;
  }

  // Palette: every --vua-* name that resolves to a color in the module scope.
  const palette = [];
  const tokenRaw = {};
  for (const name of customNames) {
    const raw = resolveTokenRaw(name, moduleRoot);
    tokenRaw[name] = raw;
    // Only values that start as a color resolve to one; composite values such
    // as box-shadow lists are not palette members.
    if (!/^\s*(var\(|#|rgb|hsl|color-mix)/i.test(raw)) continue;
    const color = resolveTokenColor(name, moduleRoot);
    if (color) palette.push({ name, color });
  }
  // Accent in the default (non-production) scope for jurisdiction family checks.
  const accentRootScope = resolveTokenColor("--vua-accent", document.documentElement);
  const accentModuleScope = resolveTokenColor("--vua-accent", moduleRoot);

  const familyColors = { purple: [], orange: [] };
  for (const entry of palette) {
    if (/purple|accent/.test(entry.name)) familyColors.purple.push(entry.color);
    if (/orange/.test(entry.name)) familyColors.orange.push(entry.color);
  }
  if (accentRootScope) familyColors.purple.push(accentRootScope);
  if (accentModuleScope) familyColors.orange.push(accentModuleScope);

  // Resolved shadow/glow tokens for shadow-panel classification.
  const shadowTokens = {};
  for (const name of ["--vua-shadow-elevated", "--vua-glow-accent", "--vua-glow-accent-soft"]) {
    const probe = document.createElement("div");
    probe.style.position = "absolute";
    probe.style.display = "none";
    moduleRoot.appendChild(probe);
    try { probe.style.boxShadow = "var(" + name + ")"; shadowTokens[name] = getComputedStyle(probe).boxShadow; } catch { /* ignore */ }
    probe.remove();
  }

  const radiusTokens = {};
  for (const name of ["--vua-radius-control", "--vua-radius-card", "--vua-radius-overlay"]) {
    const raw = (tokenRaw[name] || "").trim();
    const value = parseFloat(raw);
    if (Number.isFinite(value)) radiusTokens[name] = value;
  }

  function familyOf(color) {
    if (!color || color.a < 0.95) return null;
    for (const [family, colors] of Object.entries(familyColors)) {
      for (const candidate of colors) {
        if (rgbDistance(color, candidate) <= 12) return family;
      }
    }
    return null;
  }

  const overlayRe = /(dialog|modal|popover|menu|tooltip|toast|drawer|overlay|dropdown|floating|flyout|combobox)/i;
  const structureRe = /(topbar|sidebar|shell|navbar|header|task|notification|titlebar)/i;

  function isOverlayContext(el) {
    for (let node = el; node && node !== document.documentElement; node = node.parentElement) {
      if (node.getAttribute) {
        const role = node.getAttribute("role");
        if (role && /^(dialog|alertdialog|menu|listbox|tooltip|combobox)$/.test(role)) return true;
        const cls = node.getAttribute("class");
        if (cls && overlayRe.test(cls)) return true;
      }
    }
    return false;
  }
  function isShellStructure(el) {
    for (let node = el; node && node !== document.documentElement; node = node.parentElement) {
      const cls = node.getAttribute && node.getAttribute("class");
      if (cls && structureRe.test(cls)) return true;
    }
    return false;
  }
  function insideScrollContainer(el) {
    for (let node = el.parentElement; node && node !== document.documentElement; node = node.parentElement) {
      const cs = getComputedStyle(node);
      if (/(auto|scroll)/.test(cs.overflowY) && node.scrollHeight > node.clientHeight + 1) return true;
      if (/(auto|scroll)/.test(cs.overflowX) && node.scrollWidth > node.clientWidth + 1) return true;
    }
    return false;
  }

  // Effective opaque background, composited up the real ancestor chain.
  function effectiveBackground(el) {
    let layers = [];
    for (let node = el; node && node !== document.documentElement; node = node.parentElement) {
      const cs = getComputedStyle(node);
      const bg = parseColor(cs.backgroundColor);
      if (bg && bg.a > 0) layers.push(bg);
      if (bg && bg.a >= 1) break;
    }
    if (!layers.length) return { color: null, fromCanvas: true };
    if (layers[layers.length - 1].a < 1) {
      const cs = getComputedStyle(document.body);
      const canvas = parseColor(cs.backgroundColor) || { r: 0, g: 0, b: 0, a: 1 };
      layers = [...layers, canvas];
    }
    let acc = layers[layers.length - 1];
    for (let i = layers.length - 2; i >= 0; i -= 1) acc = composite(layers[i], acc);
    return { color: acc, fromCanvas: false };
  }

  function parseRadius(raw) {
    if (!raw || raw === "0px") return 0;
    if (raw.endsWith("%")) return { pct: parseFloat(raw) };
    const value = parseFloat(raw);
    return Number.isFinite(value) ? value : 0;
  }

  function directText(el) {
    let text = "";
    for (const node of el.childNodes) {
      if (node.nodeType === 3) text += node.nodeValue;
    }
    return text.replace(/\s+/g, " ").trim();
  }

  function elementPath(el) {
    const path = [];
    for (let node = el; node && node !== document.documentElement; node = node.parentElement) {
      const parent = node.parentElement;
      if (!parent) break;
      path.unshift(Array.prototype.indexOf.call(parent.children, node));
    }
    return path;
  }

  const elements = [];
  // The TreeWalker rejects [data-audit-skip] subtrees, so art layers below never
  // reach the loop body.
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, {
    acceptNode(node) {
      if (node.closest && node.closest("[data-audit-skip]")) {
        return NodeFilter.FILTER_REJECT;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });
  for (let el = walker.currentNode; el; el = walker.nextNode()) {
    const tag = el.tagName.toLowerCase();
    if (tag === "script" || tag === "style" || tag === "link" || tag === "template") continue;
    if (el.getClientRects().length === 0) continue;
    const inSvg = el.namespaceURI && el.namespaceURI.includes("svg");
    if (inSvg && tag !== "svg") continue;

    const rect = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    if (cs.visibility === "hidden" || cs.display === "none") continue;

    const text = directText(el);
    const color = parseColor(cs.color);
    const backgroundColor = parseColor(cs.backgroundColor);
    const eff = effectiveBackground(el);
    const interactive = Boolean(el.closest(interactiveControlSelector));
    const isControl = el.matches(interactiveControlSelector);
    const disabled = el.disabled === true || el.getAttribute("aria-disabled") === "true"
      || Boolean(el.closest('[aria-disabled="true"]'));
    const primary = isControl && !disabled && backgroundColor && backgroundColor.a >= 0.95
      && (rgbDistance(backgroundColor, accentModuleScope || { r: -1, g: -1, b: -1 }) <= 12
        || rgbDistance(backgroundColor, accentRootScope || { r: -1, g: -1, b: -1 }) <= 12);

    const borders = ["top", "right", "bottom", "left"].map(side => ({
      width: parseFloat(cs["border" + side.charAt(0).toUpperCase() + side.slice(1) + "Width"]) || 0,
      style: cs["border" + side.charAt(0).toUpperCase() + side.slice(1) + "Style"],
      color: parseColor(cs["border" + side.charAt(0).toUpperCase() + side.slice(1) + "Color"]),
    }));

    // Nearest ancestor radius and surface depth, computed on the real chain.
    // Skipped when the proposed rules are not being evaluated (overflow passes).
    let ancestorRadius = 0;
    let ancestorDecorative = false;
    let surfaceDepth = 0;
    if (opts.proposed) {
      for (let node = el.parentElement; node && node !== document.documentElement; node = node.parentElement) {
        if (node.namespaceURI && node.namespaceURI.includes("svg")) continue;
        const acs = getComputedStyle(node);
        if (!ancestorRadius) {
          const r = parseRadius(acs.borderTopLeftRadius);
          if (r && !(typeof r === "object" && r.pct >= 45) && r < 100) ancestorRadius = typeof r === "object" ? 0 : r;
          if ((typeof r === "object" && r.pct >= 45) || r >= 100) ancestorDecorative = true;
        }
        const abg = parseColor(acs.backgroundColor);
        const aborder = (parseFloat(acs.borderTopWidth) || 0) > 0 && acs.borderTopStyle !== "none";
        if ((abg && abg.a >= 0.05) || aborder) surfaceDepth += 1;
      }
    }
    const ownBg = parseColor(cs.backgroundColor);
    const ownBorder = borders.some(b => b.width >= 1 && b.style !== "none");
    const selfSurface = (ownBg && ownBg.a >= 0.05) || ownBorder ? 1 : 0;

    let hasIcon = false;
    for (const child of el.querySelectorAll("svg, img")) {
      const r = child.getBoundingClientRect();
      if (r.width >= 8 && r.height >= 8) { hasIcon = true; break; }
    }

    // The scroll-container walk is expensive; run it only for overflow candidates.
    let insideScroll = false;
    if (opts.overflowChecks) {
      const tol = 2;
      const beyond = rect.x < -tol || rect.y < -tol
        || rect.x + rect.width > window.innerWidth + tol || rect.y + rect.height > window.innerHeight + tol;
      const clipped = (cs.overflowX === "hidden" && el.scrollWidth > el.clientWidth + 1)
        || (cs.overflowY === "hidden" && el.scrollHeight > el.clientHeight + 1);
      if (beyond || clipped) insideScroll = insideScrollContainer(el);
    }

    elements.push({
      path: elementPath(el),
      tag,
      id: el.id || "",
      cls: (el.getAttribute("class") || "").split(/\s+/).filter(Boolean),
      rect: { x: Math.round(rect.x), y: Math.round(rect.y), w: Math.round(rect.width), h: Math.round(rect.height) },
      text: text.slice(0, MAX_TEXT),
      textLength: text.length,
      fontSize: parseFloat(cs.fontSize) || 0,
      lineHeight: cs.lineHeight === "normal" ? "normal" : parseFloat(cs.lineHeight) || 0,
      fontWeight: parseInt(cs.fontWeight, 10) || 0,
      color,
      backgroundColor,
      backgroundImage: cs.backgroundImage !== "none",
      boxShadow: cs.boxShadow,
      backdropFilter: cs.backdropFilter || cs.webkitBackdropFilter || "none",
      borders,
      radius: ["TopLeft", "TopRight", "BottomRight", "BottomLeft"].map(k => parseRadius(cs["border" + k + "Radius"])),
      paddings: ["Top", "Right", "Bottom", "Left"].map(k => Math.round(parseFloat(cs["padding" + k]) || 0)),
      margins: ["Top", "Right", "Bottom", "Left"].map(k => Math.round(parseFloat(cs["margin" + k]) || 0)),
      rowGap: cs.rowGap === "normal" ? 0 : Math.round(parseFloat(cs.rowGap) || 0),
      columnGap: cs.columnGap === "normal" ? 0 : Math.round(parseFloat(cs.columnGap) || 0),
      overflowX: cs.overflowX, overflowY: cs.overflowY,
      textOverflow: cs.textOverflow,
      lineClamp: cs.webkitLineClamp || "none",
      scrollW: el.scrollWidth, clientW: el.clientWidth, scrollH: el.scrollHeight, clientH: el.clientHeight,
      display: cs.display, position: cs.position, zIndex: parseInt(cs.zIndex, 10) || 0,
      opacity: parseFloat(cs.opacity),
      inlineStyle: Boolean(el.getAttribute("style")),
      interactive, isControl, disabled, primary,
      family: familyOf(backgroundColor) || familyOf(color),
      overlayContext: isOverlayContext(el),
      shellStructure: isShellStructure(el),
      insideScrollContainer: insideScroll,
      effBg: eff.color,
      effBgFromCanvas: eff.fromCanvas,
      hasBackgroundImage: cs.backgroundImage !== "none",
      iconOnly: !text && hasIcon,
      hasText: Boolean(text),
      ancestorRadius, ancestorDecorative,
      surfaceDepth: surfaceDepth + selfSurface,
    });
  }

  return {
    viewport: { w: window.innerWidth, h: window.innerHeight },
    scroll: { x: window.scrollX, y: window.scrollY },
    theme: document.documentElement.getAttribute("data-theme") || "",
    module: moduleRoot.getAttribute("data-module") || "",
    locale: localStorage.getItem("vua-locale") || "",
    accentModuleScope, accentRootScope,
    palette, shadowTokens, radiusTokens, tokenRaw,
    elementCount: elements.length,
    elements,
  };
}
