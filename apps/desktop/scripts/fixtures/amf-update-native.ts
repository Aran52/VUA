// Small local file inventories for Main projection/controller tests, not release evidence.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createHash } from "node:crypto";
import type { InstalledContext } from "../../src/electron/installed-context.js";
const digest = (value: string) => createHash("sha256").update(value).digest("hex");
export function amfUpdateFixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "vua-amf-hot-test-")); fs.mkdirSync(path.join(root, "manifests"));
  const artifact = (version: string, build: string) => {
    const values = { "ui/amf-entry.js": `export const version=${JSON.stringify(version)};`, "ui/style.css": `:root{--module-version:${JSON.stringify(version)}}`, "provider/vua-amf-provider.exe": `test-only-executable-${build}` };
    let offset = 0;
    const files = Object.entries(values).map(([name, value]) => { const file = { path: name, bytes: Buffer.byteLength(value), sha256: digest(value), offset }; offset += file.bytes; return file; });
    const pack = digest(Object.values(values).join("")); const payloadRoot = path.join(root, "payloads", "amf", pack);
    for (const [name, value] of Object.entries(values)) { const file = path.join(payloadRoot, name); fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, value); }
    const manifest = { schemaVersion: "0.1", product: "VUA", amf: { version, build, pack: { sha256: pack }, files } };
    // Authentication is the injected native preflight's job in these Main-only tests.
    const raw = JSON.stringify({ payload: JSON.stringify(manifest) }); const id = digest(raw);
    fs.writeFileSync(path.join(root, "manifests", `${id}.json`), raw);
    return { id, version, build, root: payloadRoot, files, manifest };
  };
  const old = artifact("0.2.0", "a".repeat(40)); const next = artifact("0.2.1", "b".repeat(40));
  const context = { schemaVersion: "0.1", root, hostRoot: path.join(root, "host"), runtimeRoot: path.join(root, "runtime"), amfRoot: old.root,
    hostManifestId: "c".repeat(64), amfManifestId: old.id, amfAvailable: true,
    host: { host: { version: "0.7.0", build: "c".repeat(40), files: [] } }, amf: { amf: old.manifest.amf } } as unknown as InstalledContext;
  return { root, old, next, context, dispose: () => {
    const base = path.resolve(os.tmpdir()) + path.sep; const resolved = path.resolve(root);
    if (!resolved.startsWith(base) || !path.basename(resolved).startsWith("vua-amf-hot-test-")) throw Error("Unsafe test cleanup");
    fs.rmSync(resolved, { recursive: true, force: true });
  } };
}
