import { protocol } from "electron";
import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import { installedContext } from "./installed-context.js";

export function registerAmfUiScheme(): void {
  protocol.registerSchemesAsPrivileged([{ scheme: "vua-module", privileges: { standard: true, secure: true, supportFetchAPI: true, corsEnabled: true } }]);
}
export function installAmfUiProtocol(): void {
  protocol.handle("vua-module", request => {
    const context = installedContext();
    const url = new URL(request.url);
    const name = decodeURIComponent(url.pathname).slice(1);
    const file = context?.amf.amf.files.find(file => file.path === name);
    if (!context?.amfAvailable || url.hostname !== "amf" || url.username || url.password || url.search || url.hash
        || request.method !== "GET" || !file || !name.startsWith("ui/") || name.includes("\\") || name.split("/").some(part => part === "..")) return new Response(null, { status: 404 });
    try {
      let target = context.amfRoot;
      for (const part of name.split("/")) { target = path.join(target, part); if (fs.lstatSync(target).isSymbolicLink()) return new Response(null, { status: 403 }); }
      const bytes = fs.readFileSync(target);
      if (bytes.length !== file.bytes || createHash("sha256").update(bytes).digest("hex") !== file.sha256) return new Response(null, { status: 409 });
      const type = name.endsWith(".js") ? "text/javascript" : name.endsWith(".css") ? "text/css" : name.endsWith(".svg") ? "image/svg+xml" : name.endsWith(".png") ? "image/png" : "application/octet-stream";
      return new Response(bytes, { headers: { "Content-Type": type, "Access-Control-Allow-Origin": "*", "Cache-Control": "no-store" } });
    } catch { return new Response(null, { status: 404 }); }
  });
}
