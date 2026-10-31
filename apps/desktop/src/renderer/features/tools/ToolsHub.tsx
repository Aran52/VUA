import { useRef, useState } from "react";
import { EnvironmentCard } from "../home/EnvironmentCard.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { strings } from "../../i18n/index.ts";
import { VrcftCard } from "./VrcftCard.tsx";
import { SteamToolCard } from "./SteamToolCard.tsx";
import { ToolLogo } from "./ToolLogo.tsx";
import { SlideDrawer } from "../../components/SlideDrawer.tsx";
import { useGridColumns } from "../../components/use-grid-columns.ts";
import { ToolDetails, ToolDetailsContext } from "./ToolDetails.tsx";
import "./tools.css";

/** The six fixed Steam entries share the Candidate connection (VRCFT adds device guidance); VRCS and LIV retain their roadmap entrances. */
const groups = [
  { key: "toolTracking", icon: "avatar", tools: [{ id: "vrcft", title: "VRCFaceTracking" }, { id: "space", title: "OpenVR Space Calibrator" }] },
  { key: "toolTranslation", icon: "question", tools: [{ id: "vrcs", title: "VRCS" }, { id: "translator", title: "OVR Overlay Translator" }] },
  { key: "toolUtilities", icon: "flask", tools: [{ id: "advanced", title: "OVR Advanced Settings" }, { id: "toolkit", title: "OVR Toolkit" }, { id: "oyasumi", title: "OyasumiVR" }] },
  { key: "toolCapture", icon: "anim", tools: [{ id: "liv", title: "LIV" }] },
] as const;

function ToolGroup({ group, selected, onSelect, onPrepareSteam }: {
  group: (typeof groups)[number]; selected: string | null; onSelect(id: string | null): void; onPrepareSteam(): void;
}) {
  const [shown, setShown] = useState<string>(group.tools[0].id);
  const [host, setHost] = useState<HTMLDivElement | null>(null);
  const grid = useRef<HTMLDivElement>(null);
  const columns = useGridColumns(grid);
  const reveal = (id: string) => { setShown(id); onSelect(id); };
  const toggle = (id: string) => { setShown(id); onSelect(selected === id ? null : id); };
  const shownIndex = Math.max(0, group.tools.findIndex(tool => tool.id === shown));
  const drawerAfter = Math.min(group.tools.length - 1, (Math.floor(shownIndex / columns) + 1) * columns - 1);
  const drawer = <SlideDrawer key="details" open={group.tools.some(tool => tool.id === selected)} className="vua-tools-drawer" label={group.tools[shownIndex]!.title}>
    <div ref={setHost} />
  </SlideDrawer>;
  const cards = group.tools.map(tool => <div className="vua-card-entry" data-card-entry={`tool-${tool.id}`} key={tool.id}>
    {tool.id === "vrcft" ? <VrcftCard selected={selected === tool.id} onDetails={() => toggle(tool.id)} onReveal={() => reveal(tool.id)} onPrepareSteam={onPrepareSteam} />
      : tool.id === "vrcs" || tool.id === "liv" ? <>
        <EnvironmentCard title={tool.title} icon={group.icon} logo={<ToolLogo id={tool.id} icon={group.icon} />}
          id={`tool-${tool.id}`} action="unknown" status="" developing iconOnlyAction actionHint={strings.journey.developingTip}
          selected={selected === tool.id} onDetails={() => toggle(tool.id)} />
        <ToolDetails id={tool.id} title={tool.title} selected={selected === tool.id}>
          <header className="vua-environment-heading"><h2>{tool.title}</h2><Button variant="subtle" data-back onClick={() => onSelect(null)}>{strings.environmentCards.closeDetails}</Button></header>
          <p>{strings.helpUi[tool.id]}</p>
        </ToolDetails>
      </> : <SteamToolCard toolId={tool.id} title={tool.title} icon={group.icon} selected={selected === tool.id}
        onDetails={() => toggle(tool.id)} onReveal={() => reveal(tool.id)} onPrepareSteam={onPrepareSteam} />}
  </div>);
  return <section className="vua-environment-section">
    <h2>{strings.journey[group.key]}</h2>
    <ToolDetailsContext.Provider value={{ host, shown }}>
      <div className="vua-environment-grid" ref={grid}>{cards.flatMap((card, index) => index === drawerAfter ? [card, drawer] : [card])}</div>
    </ToolDetailsContext.Provider>
  </section>;
}

export function ToolsHub({ onPrepareSteam }: { onPrepareSteam(): void }) {
  const [selected, setSelected] = useState<string | null>(null);
  return <div className="vua-page vua-route-page vua-tools-hub vua-compact-environments" data-focus-scope>
    <header className="vua-page__hero"><h1 className="vua-title">{strings.journey.tools}</h1></header>
    {groups.map(group => <ToolGroup key={group.key} group={group} selected={selected} onSelect={setSelected} onPrepareSteam={onPrepareSteam} />)}
  </div>;
}
