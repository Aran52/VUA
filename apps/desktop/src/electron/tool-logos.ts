import fs from "node:fs/promises";
import path from "node:path";

/** Each tool's own Steam client icon, identified by Steam's public app info. VUA ships no copy: the
 * publisher's artwork comes from Steam's CDN and is cached in the profile, like a browser cache. */
export const STEAM_TOOL_ICONS = {
  vrcft: { appId: 3329480, clientIcon: "ce68c0535a6b1af4d19e14346e0d9efe06d3011c" },
  space: { appId: 3368750, clientIcon: "fa032cf4b0e261ea470865cd80a9ef5e7c228923" },
  advanced: { appId: 1009850, clientIcon: "611f9ab0246895682f5f4561a71b1256104dfb3d" },
  toolkit: { appId: 1068820, clientIcon: "62e7f4e644a270586aba4ada85cab8b3ae62f743" },
  oyasumi: { appId: 2538150, clientIcon: "ffc47e0451d5eed3b73c361fec2eabddbc020654" },
  translator: { appId: 4304620, clientIcon: "84deb04dfef67e831bdfcc381a9ea16bbd61d324" },
  liv: { appId: 755540, clientIcon: "1f014d873d7c47ee077c9be46ebbe3d7c30cfe7e" },
} as const;
// The author's 2026-10-31 follow-up selects VRCS's own GitHub-hosted application icon.
const VRCS_COMMIT = "a7fa889dfa94c11398850eeb42765ca82b8cea82";
export type ToolLogoId = keyof typeof STEAM_TOOL_ICONS | "vrcs";
export const isToolLogoId = (value: unknown): value is ToolLogoId => typeof value === "string" && (value === "vrcs" || Object.hasOwn(STEAM_TOOL_ICONS, value));

/** Two Steam CDN fronts; either may be the reachable one on a given network. */
const HOSTS = ["cdn.cloudflare.steamstatic.com", "cdn.akamai.steamstatic.com"] as const;
const MAX_BYTES = 512 * 1024;
const TIMEOUT_MS = 8_000;

/** A Windows icon file: reserved 0, type 1, at least one image. */
export function isIcoFile(bytes: Uint8Array): boolean {
  return bytes.length > 22 && bytes.length <= MAX_BYTES && bytes[0] === 0 && bytes[1] === 0 && bytes[2] === 1 && bytes[3] === 0 && (bytes[4]! | (bytes[5]! << 8)) > 0;
}
export function isPngFile(bytes: Uint8Array): boolean {
  return bytes.length > 24 && bytes.length <= MAX_BYTES && [137, 80, 78, 71, 13, 10, 26, 10].every((value, index) => bytes[index] === value);
}

export const toolLogoUrl = (host: string, id: ToolLogoId) => {
  if (id === "vrcs") return `https://raw.githubusercontent.com/Dreaminko/VRCS/${VRCS_COMMIT}/apps/desktop/src-tauri/icons/128x128.png`;
  const icon = STEAM_TOOL_ICONS[id];
  return `https://${host}/steamcommunity/public/images/apps/${icon.appId}/${icon.clientIcon}.ico`;
};

export interface ToolLogoOptions {
  readonly cacheRoot: string;
  readonly fetch: (url: string, init: { signal: AbortSignal; headers?: Record<string, string> }) => Promise<Response>;
}

/** Returns a data URL of the tool's icon, or null when no host answers; the caller keeps its glyph. */
export function createToolLogos(options: ToolLogoOptions): (id: ToolLogoId) => Promise<string | null> {
  const inFlight = new Map<ToolLogoId, Promise<string | null>>();
  const dataUrl = (bytes: Uint8Array, id: ToolLogoId) => `data:image/${id === "vrcs" ? "png" : "x-icon"};base64,${Buffer.from(bytes).toString("base64")}`;
  const load = async (id: ToolLogoId): Promise<string | null> => {
    const icon = id === "vrcs" ? null : STEAM_TOOL_ICONS[id];
    const file = path.join(options.cacheRoot, icon ? `${icon.appId}-${icon.clientIcon}.ico` : `vrcs-${VRCS_COMMIT}.png`);
    const valid = id === "vrcs" ? isPngFile : isIcoFile;
    try {
      const cached = new Uint8Array(await fs.readFile(file));
      if (valid(cached)) return dataUrl(cached, id);
    } catch { /* Not cached yet. */ }
    const urls = id === "vrcs" ? [toolLogoUrl("", id),
      `https://api.github.com/repos/Dreaminko/VRCS/contents/apps/desktop/src-tauri/icons/128x128.png?ref=${VRCS_COMMIT}`] : HOSTS.map(host => toolLogoUrl(host, id));
    for (const url of urls) {
      try {
        const response = await options.fetch(url, { signal: AbortSignal.timeout(TIMEOUT_MS),
          ...(id === "vrcs" ? { headers: { Accept: "application/vnd.github.raw+json" } } : {}) });
        if (!response.ok) continue;
        const bytes = new Uint8Array(await response.arrayBuffer());
        if (!valid(bytes)) continue;
        try {
          await fs.mkdir(options.cacheRoot, { recursive: true });
          const temporary = `${file}.${process.pid}.tmp`;
          await fs.writeFile(temporary, bytes);
          await fs.rename(temporary, file);
        } catch { /* An uncached icon still displays this session. */ }
        return dataUrl(bytes, id);
      } catch { /* Try the next CDN front. */ }
    }
    return null;
  };
  return id => {
    const pending = inFlight.get(id);
    if (pending) return pending;
    const request = load(id).finally(() => inFlight.delete(id));
    inFlight.set(id, request);
    return request;
  };
}
