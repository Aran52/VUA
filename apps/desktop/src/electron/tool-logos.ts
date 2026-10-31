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
export type ToolLogoId = keyof typeof STEAM_TOOL_ICONS;
export const isToolLogoId = (value: unknown): value is ToolLogoId => typeof value === "string" && Object.hasOwn(STEAM_TOOL_ICONS, value);

/** Two Steam CDN fronts; either may be the reachable one on a given network. */
const HOSTS = ["cdn.cloudflare.steamstatic.com", "cdn.akamai.steamstatic.com"] as const;
const MAX_BYTES = 512 * 1024;
const TIMEOUT_MS = 8_000;

/** A Windows icon file: reserved 0, type 1, at least one image. */
export function isIcoFile(bytes: Uint8Array): boolean {
  return bytes.length > 22 && bytes.length <= MAX_BYTES && bytes[0] === 0 && bytes[1] === 0 && bytes[2] === 1 && bytes[3] === 0 && (bytes[4]! | (bytes[5]! << 8)) > 0;
}

export const toolLogoUrl = (host: string, id: ToolLogoId) => {
  const icon = STEAM_TOOL_ICONS[id];
  return `https://${host}/steamcommunity/public/images/apps/${icon.appId}/${icon.clientIcon}.ico`;
};

export interface ToolLogoOptions {
  readonly cacheRoot: string;
  readonly fetch: (url: string, init: { signal: AbortSignal }) => Promise<Response>;
}

/** Returns a data URL of the tool's icon, or null when no host answers; the caller keeps its glyph. */
export function createToolLogos(options: ToolLogoOptions): (id: ToolLogoId) => Promise<string | null> {
  const inFlight = new Map<ToolLogoId, Promise<string | null>>();
  const dataUrl = (bytes: Uint8Array) => `data:image/x-icon;base64,${Buffer.from(bytes).toString("base64")}`;
  const load = async (id: ToolLogoId): Promise<string | null> => {
    const icon = STEAM_TOOL_ICONS[id];
    const file = path.join(options.cacheRoot, `${icon.appId}-${icon.clientIcon}.ico`);
    try {
      const cached = new Uint8Array(await fs.readFile(file));
      if (isIcoFile(cached)) return dataUrl(cached);
    } catch { /* Not cached yet. */ }
    for (const host of HOSTS) {
      try {
        const response = await options.fetch(toolLogoUrl(host, id), { signal: AbortSignal.timeout(TIMEOUT_MS) });
        if (!response.ok) continue;
        const bytes = new Uint8Array(await response.arrayBuffer());
        if (!isIcoFile(bytes)) continue;
        try {
          await fs.mkdir(options.cacheRoot, { recursive: true });
          const temporary = `${file}.${process.pid}.tmp`;
          await fs.writeFile(temporary, bytes);
          await fs.rename(temporary, file);
        } catch { /* An uncached icon still displays this session. */ }
        return dataUrl(bytes);
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
