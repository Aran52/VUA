import type { IconName } from "@vua/design-system";
import { openExternalUrl } from "../../app/open-external.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { currentLocale, format, strings } from "../../i18n/index.ts";
import { plannedRoutesCopy } from "../../i18n/planned-routes-copy.ts";
import { structureCopy } from "../../i18n/structure-copy.ts";
import { EnvironmentCard } from "../home/EnvironmentCard.tsx";
import { PLANNED_TOOLS, type PlannedTool, type PlannedToolId } from "./planned-tools.ts";
import { ToolDetails } from "./ToolDetails.tsx";
import { ToolLogo } from "./ToolLogo.tsx";

/** "Steam · Paid" or "GitHub · MIT": where a planned tool comes from. */
export function plannedToolSource(tool: PlannedTool): string {
  const copy = structureCopy[currentLocale].planned;
  if (tool.source === "steam") return tool.paid ? copy.steamPaid : copy.steamFree;
  return tool.license ? format(copy.githubLicense, { license: tool.license }) : copy.github;
}

/** A tool VUA describes and links to, with its install and launch still to come. */
export function PlannedToolCard({ id, title, hint, icon, selected, onDetails, onClose }: {
  id: PlannedToolId; title: string; hint: string | undefined; icon: IconName; selected: boolean; onDetails: () => void; onClose: () => void;
}) {
  const tool = PLANNED_TOOLS[id];
  const copy = structureCopy[currentLocale].planned;
  return <>
    <EnvironmentCard title={title} hint={hint} icon={icon} logo={<ToolLogo id={id} icon={icon} />} id={`tool-${id}`} action="unknown" status="" developing
      actionHint={strings.journey.developingTip} selected={selected} onDetails={onDetails} />
    <ToolDetails id={id} title={title} selected={selected}>
      <header className="vua-environment-heading"><h2>{title}</h2><Button variant="subtle" data-back onClick={onClose}>{strings.environmentCards.closeDetails}</Button></header>
      <p>{strings.helpUi[id]}</p>
      <p className="vua-tool-source">{plannedToolSource(tool)}</p>
      <div className="vua-tool-reinstall" data-tool-planned={id}>
        <p>{copy.note}</p>
        <Button data-nav-id={`tool-official-${id}`} onClick={() => { void openExternalUrl(tool.url); }}>{plannedRoutesCopy[currentLocale].official}</Button>
      </div>
    </ToolDetails>
  </>;
}
