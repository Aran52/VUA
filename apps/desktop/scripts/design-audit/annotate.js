// In-page annotation overlay: draws numbered red boxes on offending elements
// plus a legend panel, so the annotated screenshot can be captured directly.
// Serialized into the page via Function.toString(); dependency-free.

export function applyAnnotations(items, meta) {
  for (const existing of document.querySelectorAll("[data-vua-audit-overlay]")) existing.remove();
  const max = 40;
  const drawn = items.slice(0, max);
  const svgNs = "http://www.w3.org/2000/svg";
  const wrapper = document.createElement("div");
  wrapper.setAttribute("data-vua-audit-overlay", "1");
  wrapper.style.cssText = "position:fixed;inset:0;z-index:2147483647;pointer-events:none;";
  const svg = document.createElementNS(svgNs, "svg");
  const docWidth = Math.max(document.documentElement.scrollWidth, window.innerWidth);
  const docHeight = Math.max(document.documentElement.scrollHeight, window.innerHeight);
  svg.setAttribute("width", docWidth);
  svg.setAttribute("height", docHeight);
  svg.style.cssText = "position:absolute;left:0;top:0;overflow:visible;";
  drawn.forEach((item, i) => {
    if (!item.rect) return;
    const box = document.createElementNS(svgNs, "rect");
    box.setAttribute("x", item.rect.x);
    box.setAttribute("y", item.rect.y);
    box.setAttribute("width", Math.max(item.rect.w, 4));
    box.setAttribute("height", Math.max(item.rect.h, 4));
    box.setAttribute("fill", "none");
    box.setAttribute("stroke", item.severity === "error" ? "#ff2d2d" : item.severity === "warning" ? "#ff9d2d" : "#2d9dff");
    box.setAttribute("stroke-width", "2");
    svg.appendChild(box);
    const badge = document.createElementNS(svgNs, "g");
    const bw = 18, bh = 15;
    const bx = Math.max(0, Math.min(item.rect.x, docWidth - bw));
    const by = Math.max(0, item.rect.y - bh);
    const plate = document.createElementNS(svgNs, "rect");
    plate.setAttribute("x", bx); plate.setAttribute("y", by);
    plate.setAttribute("width", bw); plate.setAttribute("height", bh);
    plate.setAttribute("fill", item.severity === "error" ? "#ff2d2d" : item.severity === "warning" ? "#ff9d2d" : "#2d9dff");
    badge.appendChild(plate);
    const label = document.createElementNS(svgNs, "text");
    label.setAttribute("x", bx + bw / 2); label.setAttribute("y", by + bh - 3);
    label.setAttribute("text-anchor", "middle");
    label.setAttribute("font-size", "10");
    label.setAttribute("font-family", "Consolas, monospace");
    label.setAttribute("fill", "#fff");
    label.textContent = String(i + 1);
    badge.appendChild(label);
    svg.appendChild(badge);
  });
  wrapper.appendChild(svg);
  const legend = document.createElement("div");
  legend.style.cssText = [
    "position:fixed", "right:8px", "bottom:8px", "z-index:2147483647",
    "max-width:420px", "max-height:60vh", "overflow:auto",
    "background:rgba(10,10,16,0.92)", "color:#fff", "border:1px solid #ff2d2d",
    "border-radius:6px", "padding:8px 10px", "font:12px/16px Consolas, monospace",
    "pointer-events:none",
  ].join(";");
  const title = document.createElement("div");
  title.style.cssText = "font-weight:600;margin-bottom:6px";
  title.textContent = `VUA design audit — ${meta.view} (${meta.theme}${meta.locale ? ", " + meta.locale : ""}), ${items.length} findings`;
  legend.appendChild(title);
  drawn.forEach((item, i) => {
    const row = document.createElement("div");
    row.style.cssText = "margin:2px 0";
    const n = `<b style="color:#ff9d2d">${String(i + 1).padStart(2, "0")}</b>`;
    const where = item.cls?.length ? `.${item.cls.slice(0, 3).join(".")}` : item.tag;
    row.innerHTML = `${n} <span style="color:#ffd7a0">${item.rule}</span> ${where} — ${item.message}`;
    legend.appendChild(row);
  });
  if (items.length > max) {
    const more = document.createElement("div");
    more.style.cssText = "margin-top:6px;color:#aaa";
    more.textContent = `+ ${items.length - max} more findings not drawn (see JSON)`;
    legend.appendChild(more);
  }
  wrapper.appendChild(legend);
  document.body.appendChild(wrapper);
  return drawn.length;
}

export function removeAnnotations() {
  for (const existing of document.querySelectorAll("[data-vua-audit-overlay]")) existing.remove();
}
