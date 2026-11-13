/** Tools VUA describes ahead of connecting them. When a tool's connection arrives, it moves from here to
 * the live cards; the tool catalog's planned connections list what each one needs. */
import { INSTALLER_TOOL_IDS, type InstallerToolId } from "@vua/contracts";

export interface PlannedTool {
  readonly source: "steam" | "github";
  /** Steam tools: whether the store sells it. */
  readonly paid?: boolean;
  /** GitHub tools: the license its repository states. */
  readonly license?: string;
  readonly url: string;
}

export const PLANNED_TOOLS = {
  xsoverlay: { source: "steam", paid: true, url: "https://store.steampowered.com/app/1173510/" },
  liv: { source: "steam", paid: false, url: "https://store.steampowered.com/app/755540/" },
} as const satisfies Record<string, PlannedTool>;

export type PlannedToolId = keyof typeof PLANNED_TOOLS;
export const isPlannedTool = (id: string): id is PlannedToolId => Object.hasOwn(PLANNED_TOOLS, id);
export const isInstallerTool = (id: string): id is InstallerToolId => (INSTALLER_TOOL_IDS as readonly string[]).includes(id);
