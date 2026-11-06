/** Tools VUA describes ahead of connecting them. When a tool's connection arrives, it moves from here to
 * the live cards; the tool catalog's planned connections list what each one needs. */
export interface PlannedTool {
  readonly source: "steam" | "github";
  /** Steam tools: whether the store sells it. */
  readonly paid?: boolean;
  /** GitHub tools: the license its repository states. */
  readonly license?: string;
  readonly url: string;
}

export const PLANNED_TOOLS = {
  slimevr: { source: "github", license: "MIT / Apache-2.0", url: "https://github.com/SlimeVR/SlimeVR-Installer/releases/latest" },
  vrcs: { source: "github", url: "https://github.com/Dreaminko/VRCS" },
  vrct: { source: "github", license: "MIT", url: "https://github.com/misyaguziya/VRCT/releases/latest" },
  xsoverlay: { source: "steam", paid: true, url: "https://store.steampowered.com/app/1173510/" },
  desktopplus: { source: "steam", paid: false, url: "https://store.steampowered.com/app/1494460/" },
  fpsvr: { source: "steam", paid: true, url: "https://store.steampowered.com/app/908520/" },
  vrcx: { source: "github", license: "MIT", url: "https://github.com/vrcx-team/VRCX/releases/latest" },
  liv: { source: "steam", paid: false, url: "https://store.steampowered.com/app/755540/" },
} as const satisfies Record<string, PlannedTool>;

export type PlannedToolId = keyof typeof PLANNED_TOOLS;
export const isPlannedTool = (id: string): id is PlannedToolId => Object.hasOwn(PLANNED_TOOLS, id);
