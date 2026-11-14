import { useCallback, useEffect, useRef, useState } from "react";
import type { IconName } from "@vua/design-system";
import type { ExternalToolId } from "@vua/contracts";
import { currentLocale, strings } from "../../i18n/index.ts";
import { structureCopy } from "../../i18n/structure-copy.ts";
import { VrcftCard } from "./VrcftCard.tsx";
import { SteamToolCard } from "./SteamToolCard.tsx";
import { InstallerToolCard } from "./InstallerToolCard.tsx";
import { PlannedToolCard } from "./PlannedToolCard.tsx";
import { isInstallerTool, isPlannedTool } from "./planned-tools.ts";
import { SlideDrawer } from "../../components/SlideDrawer.tsx";
import { useEscapeClose } from "../../components/use-escape-close.ts";
import { useGridColumns } from "../../components/use-grid-columns.ts";
import { ToolDetailsContext } from "./ToolDetails.tsx";
import { PoseGraphSection } from "./PoseGraphSection.tsx";
import "./tools.css";

/** Connected Steam entries share the Candidate connection (VRCFT adds device guidance); planned tools describe themselves and link out. */
const hints = structureCopy[currentLocale].toolHints;
const platforms = structureCopy[currentLocale].platforms;
export const TOOL_HUB_GROUPS = [
  { key: "toolTracking", icon: "avatar", tools: [{ id: "vrcft", title: "VRCFaceTracking" }, { id: "space", title: "OpenVR Space Calibrator" }, { id: "slimevr", title: "SlimeVR Server" }] },
  { key: "toolTranslation", icon: "question", tools: [{ id: "vrcs", title: "VRCS" }, { id: "translator", title: "OVR Overlay Translator" }, { id: "vrct", title: "VRCT" }] },
  { key: "toolUtilities", icon: "flask", tools: [{ id: "advanced", title: "OVR Advanced Settings" }, { id: "toolkit", title: "OVR Toolkit" }, { id: "oyasumi", title: "OyasumiVR" }, { id: "xsoverlay", title: "XSOverlay" }, { id: "desktopplus", title: "Desktop+" }, { id: "fpsvr", title: "fpsVR" }] },
  { key: "toolSocial", icon: "avatar", tools: [{ id: "vrcx", title: "VRCX" }] },
  { key: "toolCapture", icon: "anim", tools: [{ id: "liv", title: "LIV" }] },
] as const;
const ALL_TOOL_IDS = new Set<string>(TOOL_HUB_GROUPS.flatMap(group => group.tools.map(tool => tool.id)));

function readRequestedTool(hash = window.location.hash): string | null {
  const [page, query] = hash.replace(/^#\/?/, "").split("?");
  if ((page !== "tools" && page !== "tools-discover") || !query) return null;
  const id = new URLSearchParams(query).get("tool");
  return id !== null && ALL_TOOL_IDS.has(id) ? id : null;
}

interface ToolGroupItem { readonly id: string; readonly title: string }
/** A titled row of tool cards with one shared details drawer. */
export function ToolGroup({ title, group, selected, onSelect, onPrepareSteam }: {
  title: string; group: { readonly icon: IconName; readonly tools: readonly ToolGroupItem[] }; selected: string | null; onSelect(id: string | null): void; onPrepareSteam(): void;
}) {
  const [shown, setShown] = useState<string>(() => group.tools.find(tool => tool.id === selected)?.id ?? group.tools[0]!.id);
  const [host, setHost] = useState<HTMLDivElement | null>(null);
  const grid = useRef<HTMLDivElement>(null);
  const columns = useGridColumns(grid);
  useEffect(() => {
    if (selected !== null && group.tools.some(tool => tool.id === selected)) setShown(selected);
  }, [selected, group.tools]);
  const reveal = (id: string) => { setShown(id); onSelect(id); };
  const toggle = (id: string) => { setShown(id); onSelect(selected === id ? null : id); };
  const shownIndex = Math.max(0, group.tools.findIndex(tool => tool.id === shown));
  const drawerAfter = Math.min(group.tools.length - 1, (Math.floor(shownIndex / columns) + 1) * columns - 1);
  const drawer = <SlideDrawer key="details" open={group.tools.some(tool => tool.id === selected)} className="vua-tools-drawer" label={group.tools[shownIndex]!.title}>
    <div ref={setHost} />
  </SlideDrawer>;
  const cards = group.tools.map(tool => <div className="vua-card-entry" data-card-entry={`tool-${tool.id}`} key={tool.id}>
    {tool.id === "vrcft" ? <VrcftCard hint={hints.vrcft} selected={selected === tool.id} onDetails={() => toggle(tool.id)} onReveal={() => reveal(tool.id)} onPrepareSteam={onPrepareSteam} />
      : isPlannedTool(tool.id) ? <PlannedToolCard id={tool.id} title={tool.title} hint={hints[tool.id as keyof typeof hints]} icon={group.icon}
          selected={selected === tool.id} onDetails={() => toggle(tool.id)} onClose={() => onSelect(null)} />
      : isInstallerTool(tool.id) ? <InstallerToolCard toolId={tool.id} title={tool.title} hint={hints[tool.id as keyof typeof hints]} icon={group.icon} selected={selected === tool.id}
          onDetails={() => toggle(tool.id)} onReveal={() => reveal(tool.id)} />
      : <SteamToolCard toolId={tool.id as ExternalToolId} title={tool.title} hint={hints[tool.id as keyof typeof hints] ?? platforms[tool.id as keyof typeof platforms]} icon={group.icon} selected={selected === tool.id}
        onDetails={() => toggle(tool.id)} onReveal={() => reveal(tool.id)} onPrepareSteam={onPrepareSteam} />}
  </div>);
  return <section className="vua-environment-section">
    <h2>{title}</h2>
    <ToolDetailsContext.Provider value={{ host, shown }}>
      <div className="vua-environment-grid" ref={grid}>{cards.flatMap((card, index) => index === drawerAfter ? [card, drawer] : [card])}</div>
    </ToolDetailsContext.Provider>
  </section>;
}

export function ToolsHub({ onPrepareSteam }: { onPrepareSteam(): void }) {
  const [selected, setSelected] = useState<string | null>(() => readRequestedTool());
  useEffect(() => {
    const applyHash = () => {
      const target = readRequestedTool();
      if (target !== null) {
        setSelected(target);
        const escaped =
          typeof CSS !== "undefined" && typeof CSS.escape === "function"
            ? CSS.escape(target)
            : target.replace(/["\\]/g, "\\$&");
        requestAnimationFrame(() => document.querySelector(`[data-card-entry="tool-${escaped}"]`)?.scrollIntoView?.({ block: "nearest" }));
      }
    };
    applyHash();
    window.addEventListener("hashchange", applyHash);
    return () => window.removeEventListener("hashchange", applyHash);
  }, []);
  const close = useCallback(() => setSelected(null), []);
  useEscapeClose(selected !== null, close);
  return <div className="vua-page vua-route-page vua-tools-hub vua-compact-environments" data-focus-scope>
    <header className="vua-page__hero"><h1 className="vua-title">{strings.journey.tools}</h1></header>
    {TOOL_HUB_GROUPS.map(group => <ToolGroup key={group.key} title={strings.journey[group.key]} group={group} selected={selected} onSelect={setSelected} onPrepareSteam={onPrepareSteam} />)}
    <PoseGraphSection />
  </div>;
}
