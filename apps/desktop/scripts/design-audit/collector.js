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
  const styles = new WeakMap();
  const styleOf = element => {
    if (!styles.has(element)) styles.set(element, getComputedStyle(element));
    return styles.get(element);
  };

  // CSS clipping confines a camera scene's paint without breaking the page.
  // Do not clip to the window here: genuinely escaping content must stay visible
  // to the overflow rule, and content below the fold still needs its style audit.
  function paintedRect(element, rectangle) {
    let left = rectangle.left, right = rectangle.right, top = rectangle.top, bottom = rectangle.bottom;
    let escapingPosition = null;
    for (let node = element; node; node = node.parentElement) {
      const style = styleOf(node);
      if (style.display === "none" || (node === element && style.visibility !== "visible") || Number(style.opacity) === 0) return null;
      if (node !== element && style.contentVisibility === "hidden") return null;
      if (node !== element && node.tagName === "DETAILS" && !node.open) {
        const summary = [...node.children].find(child => child.tagName === "SUMMARY");
        if (!summary?.contains(element)) return null;
      }
      // Assistive labels clipped to no painted area are still available to
      // accessibility APIs, but they are not visibly truncated text.
      const inset = style.clipPath.match(/^inset\(([^)]*)\)$/)?.[1].split(/\s+round\s+/)[0].trim().split(/\s+/);
      if (inset && inset.every(value => /^\d+(?:\.\d+)?%$/.test(value))) {
        const [top, right = top, bottom = top, left = right] = inset.map(parseFloat);
        if (top + bottom >= 100 || left + right >= 100) return null;
      }
      const legacyClip = style.clip.match(/^rect\(([^)]*)\)$/)?.[1].split(/[\s,]+/).map(parseFloat);
      if (legacyClip && /^(absolute|fixed)$/.test(style.position) && (legacyClip[1] <= legacyClip[3] || legacyClip[2] <= legacyClip[0])) return null;
      const establishesFixedBlock = style.transform !== "none" || style.perspective !== "none"
        || style.filter !== "none" || style.backdropFilter !== "none"
        || /(?:layout|paint|strict|content)/.test(style.contain)
        || style.contentVisibility === "auto" || /(?:transform|perspective|filter)/.test(style.willChange);
      if (node !== element && escapingPosition && (establishesFixedBlock || (escapingPosition === "absolute" && style.position !== "static"))) escapingPosition = null;
      const clipX = /^(hidden|clip)$/.test(style.overflowX);
      const clipY = /^(hidden|clip)$/.test(style.overflowY);
      if (node !== element && !escapingPosition && (clipX || clipY)) {
        const clip = node.getBoundingClientRect();
        if (clipX) { left = Math.max(left, clip.left); right = Math.min(right, clip.right); }
        if (clipY) { top = Math.max(top, clip.top); bottom = Math.min(bottom, clip.bottom); }
      }
      // Overflow between a positioned descendant and its containing block does
      // not clip it. Visibility and zero-opacity ancestors still apply above.
      if (style.position === "fixed" || style.position === "absolute") escapingPosition = style.position;
    }
    return right > left && bottom > top ? { x: left, y: top, w: right - left, h: bottom - top } : null;
  }

  function hitRegion(element, rectangle) {
    let region = { ...rectangle, source: "control" };
    if (!element.matches('input[type="checkbox"], input[type="radio"]')) return region;
    for (const label of element.labels || []) {
      if (label.control !== element) continue;
      // A fragment is a real clickable label area; a bounding box spanning
      // separate lines or separate labels would invent a target between them.
      for (const fragment of label.getClientRects()) {
        const candidate = paintedRect(label, fragment);
        if (candidate && Math.min(candidate.w, candidate.h) > Math.min(region.w, region.h)) region = { ...candidate, source: "label" };
      }
    }
    return region;
  }

  function isSurface(element, style, canvas) {
    if (!element.matches("div,article,section,aside,main,nav,header,footer,ul,ol,li,dl,table,fieldset,details,form")) return false;
    if (element.matches(interactiveControlSelector) || style.display === "inline" || style.display === "contents") return false;
    const background = parseColor(style.backgroundColor);
    const enclosingEdges = ["Top", "Right", "Bottom", "Left"].filter(side => parseFloat(style[`border${side}Width`]) > 0 && style[`border${side}Style`] !== "none").length;
    return enclosingEdges >= 3 || Boolean(background && background.a >= 0.05 && (!canvas || rgbDistance(background, canvas) > 2));
  }

  function parseColor(text) {
    if (typeof text !== "string") return null;
    const value = text.trim();
    const modern = value.match(/^color\(srgb(-linear)?\s+([^)]*)\)$/i);
    if (modern) {
      const values = modern[2].split(/[\s/]+/).filter(Boolean).map(part => parseFloat(part) / (part.endsWith("%") ? 100 : 1));
      if (values.length < 3 || !values.every(Number.isFinite)) return null;
      const channel = value => {
        const clamped = Math.max(0, Math.min(1, value));
        return 255 * (modern[1] ? clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * clamped ** (1 / 2.4) - 0.055 : clamped);
      };
      return { r: channel(values[0]), g: channel(values[1]), b: channel(values[2]), a: values[3] ?? 1 };
    }
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
  const scopedNames = new Map();
  function gatherNames(rules) {
    for (const rule of rules) {
      if (rule.style) {
        for (const prop of rule.style) if (prop.startsWith("--vua-")) {
          customNames.add(prop);
          if (rule.selectorText) {
            if (!scopedNames.has(rule.selectorText)) scopedNames.set(rule.selectorText, new Set());
            scopedNames.get(rule.selectorText).add(prop);
          }
        }
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
    if (!/^\s*(var\(|#|rgb|hsl|color(?:-mix)?\()/i.test(raw)) continue;
    const color = resolveTokenColor(name, moduleRoot);
    if (color) palette.push({ name, color });
  }
  // Feature tokens (graph and room surfaces) resolve in their declaring scope.
  for (const [selector, names] of scopedNames) {
    let hosts;
    try { hosts = document.querySelectorAll(selector); } catch { continue; }
    for (const host of hosts) {
      if (host === moduleRoot || host === document.documentElement) continue;
      for (const name of names) {
        const raw = styleOf(host).getPropertyValue(name).trim();
        if (!/^\s*(var\(|#|rgb|hsl|color(?:-mix)?\()/i.test(raw)) continue;
        const color = resolveTokenColor(name, host);
        if (color && !palette.some(entry => entry.name === name && rgbDistance(entry.color, color) < 0.01)) palette.push({ name, color });
      }
    }
  }
  // Accent in the default (non-production) scope for jurisdiction family checks.
  const accentRootScope = resolveTokenColor("--vua-accent", document.documentElement);
  const accentModuleScope = resolveTokenColor("--vua-accent", moduleRoot);

  const familyColors = { purple: [], orange: [] };
  for (const entry of palette) {
    if (/^--vua-purple(?:-|$)/.test(entry.name)) familyColors.purple.push(entry.color);
    if (/^--vua-orange(?:-|$)/.test(entry.name)) familyColors.orange.push(entry.color);
  }

  // Resolved shadow/glow tokens for shadow-panel classification.
  const shadowTokens = {};
  for (const name of ["--vua-shadow-elevated", "--vua-action-shadow", "--vua-glow-accent", "--vua-glow-accent-soft"]) {
    const probe = document.createElement("div");
    probe.style.position = "absolute";
    probe.style.display = "none";
    moduleRoot.appendChild(probe);
    try { probe.style.boxShadow = "var(" + name + ")"; shadowTokens[name] = getComputedStyle(probe).boxShadow; } catch { /* ignore */ }
    probe.remove();
  }

  const radiusTokens = {};
  for (const name of ["--vua-radius-sm", "--vua-radius-control", "--vua-radius-card", "--vua-radius-overlay",
    ...["action", "field", "indicator", "container", "overlay", "menu", "progress"].map(role => "--vua-corner-" + role)]) {
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
  function insideScrollContainer(el, axis) {
    for (let node = el.parentElement; node && node !== document.documentElement; node = node.parentElement) {
      const cs = getComputedStyle(node);
      if (axis === "y" && /(auto|scroll)/.test(cs.overflowY) && node.scrollHeight > node.clientHeight + 1) return true;
      if (axis === "x" && /(auto|scroll)/.test(cs.overflowX) && node.scrollWidth > node.clientWidth + 1) return true;
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
    const paint = paintedRect(el, rect);
    if (!paint) continue;
    let truncation = false;
    for (let node = el; node; node = node.parentElement) {
      const style = styleOf(node);
      if (style.textOverflow === "ellipsis" || (style.webkitLineClamp && style.webkitLineClamp !== "none")) { truncation = true; break; }
    }

    const text = cs.contentVisibility === "hidden" ? "" : directText(el);
    const color = parseColor(cs.color);
    const backgroundColor = parseColor(cs.backgroundColor);
    const eff = effectiveBackground(el);
    const interactive = Boolean(el.closest(interactiveControlSelector));
    const isControl = el.matches(interactiveControlSelector);
    const isChoice = el.matches('select, input[type="checkbox"], input[type="radio"], [aria-pressed], [aria-selected], [role="tab"], [role="switch"], [role="radio"], [role="checkbox"], [role="option"]');
    const isTextField = el.matches('textarea, select, input:not([type="checkbox"]):not([type="radio"]):not([type="range"]):not([type="color"]):not([type="file"])');
    const indicatorRole = el.getAttribute("data-vua-control-role") === "indicator";
    const primaryGroup = el.closest("[data-vua-primary-group]");
    const disabled = el.disabled === true || el.getAttribute("aria-disabled") === "true"
      || Boolean(el.closest('[aria-disabled="true"], [inert]'));
    const actionOwner = el.closest('[role="dialog"], [role="alertdialog"]');
    const primary = isControl && !indicatorRole && !disabled && backgroundColor && backgroundColor.a >= 0.95
      && (rgbDistance(backgroundColor, accentModuleScope || { r: -1, g: -1, b: -1 }) <= 12
        || rgbDistance(backgroundColor, accentRootScope || { r: -1, g: -1, b: -1 }) <= 12);

    const borders = ["top", "right", "bottom", "left"].map(side => ({
      width: parseFloat(cs["border" + side.charAt(0).toUpperCase() + side.slice(1) + "Width"]) || 0,
      style: cs["border" + side.charAt(0).toUpperCase() + side.slice(1) + "Style"],
      color: parseColor(cs["border" + side.charAt(0).toUpperCase() + side.slice(1) + "Color"]),
    }));

    // Nearest ancestor radius and surface depth, computed on the real chain.
    // The walk stops at body: body (and containers painted in the canvas
    // color) are the backdrop, not surfaces in the nesting sense.
    // Skipped when the proposed rules are not being evaluated (overflow passes).
    let ancestorRadius = 0;
    let ancestorDecorative = false;
    let surfaceDepth = 0;
    if (opts.proposed) {
      const canvasBg = parseColor(getComputedStyle(document.body).backgroundColor);
      for (let node = el.parentElement; node && node !== document.body && node !== document.documentElement; node = node.parentElement) {
        if (node.namespaceURI && node.namespaceURI.includes("svg")) continue;
        const acs = getComputedStyle(node);
        if (!ancestorRadius) {
          const r = parseRadius(acs.borderTopLeftRadius);
          if (r && !(typeof r === "object" && r.pct >= 45) && r < 100) ancestorRadius = typeof r === "object" ? 0 : r;
          if ((typeof r === "object" && r.pct >= 45) || r >= 100) ancestorDecorative = true;
        }
        const independent = isSurface(node, acs, canvasBg);
        if (independent) surfaceDepth += 1;
      }
    }
    const selfSurface = isSurface(el, cs, parseColor(getComputedStyle(document.body).backgroundColor)) ? 1 : 0;

    let hasIcon = false;
    for (const child of el.querySelectorAll("svg, img")) {
      const r = child.getBoundingClientRect();
      if (r.width >= 8 && r.height >= 8) { hasIcon = true; break; }
    }

    // The scroll-container walk is expensive; run it only for overflow candidates.
    let insideScrollX = false;
    let insideScrollY = false;
    if (opts.overflowChecks) {
      const tol = 2;
      const beyond = rect.x < -tol || rect.y < -tol
        || rect.x + rect.width > window.innerWidth + tol || rect.y + rect.height > window.innerHeight + tol;
      const clipped = (cs.overflowX === "hidden" && el.scrollWidth > el.clientWidth + 1)
        || (cs.overflowY === "hidden" && el.scrollHeight > el.clientHeight + 1);
      if (beyond || clipped) {
        insideScrollX = insideScrollContainer(el, "x");
        insideScrollY = insideScrollContainer(el, "y");
      }
    }

    let typedStyle;
    try { typedStyle = el.computedStyleMap?.(); } catch { /* optional Typed OM */ }
    const gapApplies = /flex|grid/.test(cs.display) && [...el.childNodes].filter(node => node.nodeType === 3
      ? node.textContent.trim() : node.nodeType === 1 && node.getClientRects().length > 0 && styleOf(node).position !== "absolute").length > 1;

    elements.push({
      path: elementPath(el),
      tag,
      id: el.id || "",
      cls: (el.getAttribute("class") || "").split(/\s+/).filter(Boolean),
      rect: { x: Math.round(rect.x), y: Math.round(rect.y), w: Math.round(rect.width), h: Math.round(rect.height) },
      paintedRect: paint,
      hitRect: hitRegion(el, paint),
      clippedByAncestor: paint.w < rect.width - 2 || paint.h < rect.height - 2,
      intentionalTruncation: truncation,
      text: text.slice(0, MAX_TEXT),
      textLength: text.length,
      fontSize: parseFloat(cs.fontSize) || 0,
      lineHeight: cs.lineHeight === "normal" ? "normal" : parseFloat(cs.lineHeight) || 0,
      nativeLineHeight: tag === "select" && cs.appearance !== "none" && cs.lineHeight === "normal",
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
      autoMargins: ["top", "right", "bottom", "left"].map(side => typedStyle?.get(`margin-${side}`)?.toString() === "auto"),
      rowGap: !gapApplies || cs.rowGap === "normal" ? 0 : Math.round(parseFloat(cs.rowGap) || 0),
      columnGap: (!gapApplies && cs.columnCount === "auto") || cs.columnGap === "normal" ? 0 : Math.round(parseFloat(cs.columnGap) || 0),
      overflowX: cs.overflowX, overflowY: cs.overflowY,
      textOverflow: cs.textOverflow,
      lineClamp: cs.webkitLineClamp || "none",
      scrollW: el.scrollWidth, clientW: el.clientWidth, scrollH: el.scrollHeight, clientH: el.clientHeight,
      display: cs.display, position: cs.position, zIndex: parseInt(cs.zIndex, 10) || 0,
      opacity: parseFloat(cs.opacity),
      inlineStyle: Boolean(el.getAttribute("style")),
      interactive, isControl, isChoice, isTextField, indicatorRole, disabled, primary,
      shapeRole: cs.getPropertyValue("--vua-shape-role").trim(),
      shapeTreatment: cs.getPropertyValue("--vua-shape-treatment").trim(),
      shapeTheme: cs.getPropertyValue("--vua-corner-style").trim(),
      shapeExpectedX: parseFloat(cs.getPropertyValue("--vua-corner-" + cs.getPropertyValue("--vua-shape-role").trim())) || 0,
      cornerShapes: ["top-left", "top-right", "bottom-right", "bottom-left"].map(corner => cs.getPropertyValue("corner-" + corner + "-shape")),
      cornerPairs: [cs.borderTopLeftRadius, cs.borderTopRightRadius, cs.borderBottomRightRadius, cs.borderBottomLeftRadius].map(raw => {
        const values = raw.trim().split(/\s+/).map(parseFloat); return [values[0], values[1] ?? values[0]];
      }),
      primaryGroup: primaryGroup ? elementPath(primaryGroup).join(".") : null,
      role: el.getAttribute("role"),
      actionScope: actionOwner ? elementPath(actionOwner).join(".") : "page",
      family: familyOf(backgroundColor) || familyOf(color),
      overlayContext: isOverlayContext(el),
      shellStructure: isShellStructure(el),
      insideScrollContainer: insideScrollX || insideScrollY,
      insideScrollX, insideScrollY,
      effBg: eff.color,
      effBgFromCanvas: eff.fromCanvas,
      hasBackgroundImage: cs.backgroundImage !== "none",
      iconOnly: !text && hasIcon,
      hasText: Boolean(text),
      ancestorRadius, ancestorDecorative,
      selfSurface: selfSurface === 1,
      surfaceDepth: surfaceDepth + selfSurface,
    });
  }

  // Vite's error overlay means the page did not render; the runner records a
  // collection failure instead of auditing the overlay page.
  const errorOverlay = Boolean(document.querySelector("vite-error-overlay"));

  return {
    viewport: { w: window.innerWidth, h: window.innerHeight },
    scroll: { x: window.scrollX, y: window.scrollY },
    theme: document.documentElement.getAttribute("data-theme") || "",
    module: moduleRoot.getAttribute("data-module") || "",
    locale: localStorage.getItem("vua-locale") || "",
    errorOverlay,
    accentModuleScope, accentRootScope,
    palette, shadowTokens, radiusTokens, tokenRaw,
    elementCount: elements.length,
    elements,
  };
}
