import { useSyncExternalStore } from "react";

export type SkinMotifFamily = "grid" | "frutiger-aero" | "tenshi" | "jirai";
export type SkinMotifSlot = "page-corner" | "control-panel-corner" | "preview";

const motifUrls: Record<Exclude<SkinMotifFamily, "grid">, string> = {
  "frutiger-aero": new URL("../../skins/frutiger-aero/motifs/bubbles-and-sprout.svg", import.meta.url).href,
  tenshi: new URL("../../skins/tenshi/motifs/wing-halo-bandage.svg", import.meta.url).href,
  jirai: new URL("../../skins/jirai/motifs/ribbon-lace-heart.svg", import.meta.url).href,
};

function currentSkinFamily(): SkinMotifFamily {
  const value = document.documentElement.dataset.themeFamily;
  return value === "frutiger-aero" || value === "tenshi" || value === "jirai" ? value : "grid";
}

function subscribeSkinFamily(callback: () => void): () => void {
  const observer = new MutationObserver(callback);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme-family"] });
  return () => observer.disconnect();
}

export function useSkinFamily(): SkinMotifFamily {
  return useSyncExternalStore(subscribeSkinFamily, currentSkinFamily);
}

/** Built-in, original decoration. No workflow or status depends on these images. */
export function SkinMotif({ family, slot, className = "" }: {
  family?: SkinMotifFamily;
  slot: SkinMotifSlot;
  className?: string;
}) {
  const current = useSkinFamily();
  const selected = family ?? current;
  if (selected === "grid") return null;
  return <img
    className={`vua-skin-motif ${className}`.trim()}
    src={motifUrls[selected]}
    alt=""
    aria-hidden="true"
    draggable={false}
    decoding="async"
    data-skin-slot={slot}
    data-motif-family={selected}
    data-audit-skip
  />;
}
