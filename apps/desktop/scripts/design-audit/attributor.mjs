// Root-cause attribution: maps each finding element to the CSS rule that
// produces the offending computed value. Primary path is the Chrome DevTools
// Protocol (CSS.getMatchedStylesForNode, whose per-declaration "overridden"
// flags identify the cascade winner); fallback is an in-page CSSOM index with
// a specificity approximation, and below that the element classes + area.

import { selectorSpecificity, compareSpecificity } from "./tokens.mjs";

// Longhand/shorthand/logical candidates a computed physical property may come from.
const PROPERTY_CANDIDATES = {
  font: ["font-size", "line-height", "font"],
  "font-weight": ["font-weight", "font"],
  color: ["color"],
  "background-color": ["background-color", "background"],
  "border-color": [
    "border-top-color", "border-right-color", "border-bottom-color", "border-left-color",
    "border-color", "border-top", "border-right", "border-bottom", "border-left", "border",
    "outline-color",
  ],
  "box-shadow": ["box-shadow"],
  "backdrop-filter": ["backdrop-filter"],
  radius: [
    "border-top-left-radius", "border-top-right-radius", "border-bottom-right-radius",
    "border-bottom-left-radius", "border-radius",
  ],
};

export function candidatesFor(property) {
  if (PROPERTY_CANDIDATES[property]) return PROPERTY_CANDIDATES[property];
  const spacing = property.match(/^(padding|margin|gap)(?:-(top|right|bottom|left|row|column))?$/);
  if (spacing) {
    const [, kind, side] = spacing;
    if (!side) return [kind];
    const logical = { top: "block-start", bottom: "block-end", left: "inline-start", right: "inline-end", row: "row", column: "column" }[side];
    return [`${kind}-${side}`, kind, `${kind}-block`, `${kind}-inline`, `${kind}-${logical}`];
  }
  return [property];
}

export function normalizeFile(file) {
  if (!file) return "unknown";
  const marker = file.indexOf("apps/desktop/src/");
  if (marker >= 0) return file.slice(marker);
  const pkg = file.indexOf("packages/");
  if (pkg >= 0) return file.slice(pkg);
  return file;
}

function rootCauseKeyOf(selector, file) {
  return `${selector} @ ${normalizeFile(file)}`;
}

export function fallbackRootCause(el, area) {
  const cls = (el.cls ?? []).join(".");
  return `${el.tag}${cls ? "." + cls : ""} @ (no matching rule; area: ${area})`;
}

/** Collects, in the page, a flat CSSOM index and a stylesheet-text map. */
export function cssIndexSource() {
  return `(function buildCssIndex() {
  const specificity = ${selectorSpecificity.toString()};
  const fileOf = (sheet) => {
    const node = sheet.ownerNode;
    if (node && node.getAttribute) {
      const devId = node.getAttribute("data-vite-dev-id");
      if (devId) return devId;
      if (node.tagName === "STYLE") return "inline-html-style";
    }
    return (node && node.href) || "unknown";
  };
  const index = [];
  function walk(rules, file, media) {
    for (const rule of rules) {
      if (rule.cssRules && rule.media) {
        try { if (!window.matchMedia(rule.media.mediaText || rule.media.media || "").matches) continue; } catch { /* keep */ }
        walk(rule.cssRules, file, media);
        continue;
      }
      if (rule.cssRules && !rule.selectorText && !rule.style) { walk(rule.cssRules, file, media); continue; }
      if (rule.selectorText && rule.style) {
        index.push({
          selector: rule.selectorText,
          props: Array.from(rule.style).filter(p => !p.startsWith("--")),
          file,
          order: index.length,
          specificity: specificity(rule.selectorText),
        });
      }
    }
  }
  for (const sheet of document.styleSheets) {
    try { if (sheet.cssRules) walk(sheet.cssRules, fileOf(sheet)); } catch { /* cross-origin */ }
  }
  const styleTags = [];
  for (const tag of document.querySelectorAll("style")) {
    styleTags.push({ id: tag.getAttribute("data-vite-dev-id") || "inline-html-style", text: tag.textContent.replace(/\\s+/g, "") });
  }
  window.__vuaAuditCss = { index, styleTags };
  return index.length;
})()`;
}

/** In-page batch attribution over the CSSOM index (fallback path). */
export function cssAttributeSource(jobs) {
  return `(function attribute() {
  const compare = ${compareSpecificity.toString()};
  const store = window.__vuaAuditCss;
  const resolve = (path) => { let el = document.documentElement; for (const i of path) { el = el.children[i]; if (!el) return null; } return el; };
  return JSON.stringify((${JSON.stringify(jobs)}).map((job) => {
    const el = resolve(job.path);
    if (!el) return { key: null, reason: "element gone" };
    let best = null;
    for (const rule of store.index) {
      if (!job.candidates.some(c => rule.props.includes(c))) continue;
      let match = false;
      try { match = el.matches(rule.selector); } catch { match = false; }
      if (!match) continue;
      const order = compare(rule.specificity, best ? best.specificity : [0, 0, 0]);
      if (!best || order > 0 || (order === 0 && rule.order >= best.order)) best = rule;
    }
    if (!best) return { key: null, reason: "no rule" };
    return { key: best.selector + " @ " + best.file, selector: best.selector, file: best.file };
  }));
})()`;
}

/**
 * Attributes findings via CDP; falls back to the in-page CSSOM index.
 * `targets`: [{ findingIndex, path, property, el }] — el is the snapshot element.
 * Returns Map<findingIndex, rootCauseKey>.
 */
export class Attributor {
  constructor(webContents, areaFor) {
    this.webContents = webContents;
    this.areaFor = areaFor ?? (() => "");
    this.attached = false;
    this.fileCache = new Map();
    this.cdpOk = false;
  }

  async attach() {
    try {
      // The runner owns one debugger session per window; reuse it when present.
      this.iAttached = !this.webContents.debugger.isAttached();
      if (this.iAttached) this.webContents.debugger.attach("1.3");
      // CSS depends on the DOM domain; enable order matters.
      await this.webContents.debugger.sendCommand("DOM.enable");
      await this.webContents.debugger.sendCommand("CSS.enable");
      await this.webContents.debugger.sendCommand("Runtime.enable");
      // Pull the document once so requestNode/getMatchedStylesForNode resolve.
      await this.webContents.debugger.sendCommand("DOM.getDocument", { depth: 0 });
      this.attached = true;
      this.cdpOk = true;
    } catch {
      this.cdpOk = false;
    }
    return this.cdpOk;
  }

  async detach() {
    if (this.attached && this.iAttached) {
      try { this.webContents.debugger.detach(); } catch { /* already gone */ }
      this.iAttached = false;
    }
    this.attached = false;
  }

  async resolveNodeId(path) {
    const expression = `(() => { let el = document.documentElement; for (const i of ${JSON.stringify(path)}) { el = el.children[i]; if (!el) return null; } return el; })()`;
    const result = await this.webContents.debugger.sendCommand("Runtime.evaluate", {
      expression, returnByValue: false,
    });
    if (!result?.result?.objectId) return null;
    const node = await this.webContents.debugger.sendCommand("DOM.requestNode", { objectId: result.result.objectId });
    return node.nodeId ?? null;
  }

  async fileForStyleSheet(styleSheetId) {
    if (this.fileCache.has(styleSheetId)) return this.fileCache.get(styleSheetId);
    let file = "unknown";
    try {
      const text = await this.webContents.debugger.sendCommand("CSS.getStyleSheetText", { styleSheetId });
      const stripped = (text?.text ?? "").replace(/\s+/g, "");
      const tags = this.styleTags ?? [];
      const match = tags.find(tag => tag.text && (tag.text === stripped || stripped.startsWith(tag.text.slice(0, 400))));
      if (match) file = match.id;
      else {
        // Match by first rule text as a weaker signal.
        const head = stripped.slice(0, 120);
        const weak = tags.find(tag => tag.text.includes(head) && head.length > 20);
        if (weak) file = weak.id;
      }
    } catch { file = "unknown"; }
    this.fileCache.set(styleSheetId, file);
    return file;
  }

  async attributeOne(path, property) {
    const candidates = candidatesFor(property);
    try {
      const nodeId = await this.resolveNodeId(path);
      if (!nodeId) return { key: null, reason: "element gone" };
      const styles = await this.webContents.debugger.sendCommand("CSS.getMatchedStylesForNode", { nodeId });
      if (!styles) return { key: null, reason: "cdp returned nothing" };

      const inlineProps = styles.inlineStyle?.cssProperties ?? [];
      if (inlineProps.some(p => candidates.includes(p.name) && !p.overridden)) {
        return { key: "inline style attribute", selector: "inline", file: "element style" };
      }
      const rules = styles.matchedCSSRules ?? [];
      let winner = null;
      let uaWinner = null;
      for (const entry of rules) {
        const declarations = entry.rule?.style?.cssProperties ?? [];
        if (!declarations.some(p => candidates.includes(p.name) && !p.overridden)) continue;
        if (entry.rule.origin === "user-agent") uaWinner = entry.rule;
        else winner = entry.rule; // CDP lists rules weakest-first; the last author rule wins the cascade.
      }
      const chosen = winner ?? uaWinner;
      if (!chosen) return { key: null, reason: "no rule" };
      const selectorOf = rule => rule.selectorList?.text ?? rule.selectorText ?? "(unknown selector)";
      if (!winner && uaWinner) {
        return { key: `user-agent stylesheet: ${selectorOf(uaWinner)}`, selector: selectorOf(uaWinner), file: "user-agent" };
      }
      const file = chosen.styleSheetId ? await this.fileForStyleSheet(chosen.styleSheetId) : "unknown";
      return { key: rootCauseKeyOf(selectorOf(chosen), file), selector: selectorOf(chosen), file };
    } catch (error) {
      return { key: null, reason: `cdp: ${error.message}` };
    }
  }

  async ensurePageContext() {
    if (this.styleTags) return;
    const count = await this.webContents.executeJavaScript(cssIndexSource(), true);
    if (!count) this.styleTags = [];
    const tags = await this.webContents.executeJavaScript("window.__vuaAuditCss.styleTags", true);
    this.styleTags = tags ?? [];
  }

  /**
   * Attributes a batch of targets. Tries CDP per distinct (element, property)
   * pair; on any CDP trouble falls back to the in-page CSSOM index, then to
   * class+area descriptions. Findings whose rule has no attributable property
   * go straight to the fallback. Returns Map<findingIndex, rootCauseKey>.
   */
  async attribute(targets, area) {
    await this.ensurePageContext();
    const results = new Map();
    const groups = new Map();
    for (const target of targets) {
      if (!target.property || !target.path) {
        results.set(target.findingIndex, fallbackRootCause(target.el, area));
        continue;
      }
      const groupKey = JSON.stringify([target.path, target.property]);
      if (!groups.has(groupKey)) groups.set(groupKey, { path: target.path, property: target.property, members: [] });
      groups.get(groupKey).members.push(target);
    }
    const pending = [];
    for (const group of groups.values()) {
      const el = group.members[0].el;
      if (this.cdpOk) {
        const outcome = await this.attributeOne(group.path, group.property);
        if (outcome.key) {
          for (const member of group.members) results.set(member.findingIndex, outcome.key);
          continue;
        }
        if (outcome.reason === "element gone") {
          for (const member of group.members) results.set(member.findingIndex, fallbackRootCause(el, area));
          continue;
        }
      }
      pending.push(group);
    }
    if (pending.length) {
      const jobs = pending.map(g => ({ path: g.path, candidates: candidatesFor(g.property) }));
      try {
        const raw = await this.webContents.executeJavaScript(cssAttributeSource(jobs), true);
        const parsed = JSON.parse(raw);
        pending.forEach((group, i) => {
          const outcome = parsed[i];
          for (const member of group.members) {
            results.set(member.findingIndex, outcome?.key ? outcome.key : fallbackRootCause(group.members[0].el, area));
          }
        });
      } catch {
        for (const group of pending) {
          for (const member of group.members) results.set(member.findingIndex, fallbackRootCause(group.members[0].el, area));
        }
      }
    }
    return results;
  }
}
