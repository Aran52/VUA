import { useEffect, useState } from "react";
import type { IconName } from "@vua/design-system";
import { RouteGlyph } from "../../components/RouteTile.tsx";

const logos = new Map<string, Promise<string | null>>();
function loadLogo(id: string): Promise<string | null> {
  let request = logos.get(id);
  if (!request) {
    const api = window.vua?.toolLogos;
    request = api ? api.logo(id).then(value => typeof value === "string" && value.startsWith("data:image/") ? value : null, () => null) : Promise.resolve(null);
    // A miss is retried on the next visit; a fetched icon is kept for the session.
    void request.then(value => { if (value === null) logos.delete(id); });
    logos.set(id, request);
  }
  return request;
}

/** The tool's own icon from its configured upstream; the group glyph while it is unavailable. */
export function ToolLogo({ id, icon }: { id: string; icon: IconName }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void loadLogo(id).then(value => { if (alive) setSrc(value); });
    return () => { alive = false; };
  }, [id]);
  return src ? <img className="vua-tool-logo" src={src} alt="" draggable={false} /> : <RouteGlyph icon={icon} />;
}
