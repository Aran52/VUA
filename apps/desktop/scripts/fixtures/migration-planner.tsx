// Synthetic library-migration planner states for visual review; no real files are read.
import { createRoot } from "react-dom/client";
import { GatewayProvider } from "../../src/renderer/gateway/GatewayProvider.tsx";
import { emptyGateway } from "../../src/renderer/gateway/empty-gateway.ts";
import { LibraryMigrationPreview } from "../../src/renderer/features/import/LibraryMigrationPreview.tsx";
import "../../src/renderer/features/import/import-menu.css";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";

const T = 1_760_000_000_000;
const root = "D:\\SyntheticCollection";
const items = (directory: string, entries: readonly { name: string; kind: "file" | "directory" | "link"; bytes?: number }[]) =>
  ({ path: directory, state: "ready" as const, truncated: false,
    items: entries.map(entry => ({ name: entry.name, path: `${directory}\\${entry.name}`, kind: entry.kind,
      sizeBytes: entry.kind === "file" ? entry.bytes ?? 1024 * 1024 : null,
      modifiedMs: entry.kind === "file" ? T : null })) });
const previews: Record<string, unknown> = {
  [root]: items(root, [
    { name: "Meiyun Coat", kind: "directory" },
    { name: "Meiyun Hat", kind: "directory" },
    { name: "README.txt", kind: "file", bytes: 2048 },
    { name: "LICENSE.txt", kind: "file", bytes: 1024 },
    { name: "extra-poses.unitypackage", kind: "file", bytes: 8 * 1024 * 1024 },
    { name: "shortcut", kind: "link" },
  ]),
  [`${root}\\Meiyun Coat`]: items(`${root}\\Meiyun Coat`, [
    { name: "Coat.unitypackage", kind: "file", bytes: 42 * 1024 * 1024 },
    { name: "Textures", kind: "directory" },
  ]),
  [`${root}\\Meiyun Coat\\Textures`]: items(`${root}\\Meiyun Coat\\Textures`, [
    { name: "coat_body.png", kind: "file", bytes: 6 * 1024 * 1024 },
  ]),
  [`${root}\\Meiyun Hat`]: items(`${root}\\Meiyun Hat`, [
    { name: "Hat.unitypackage", kind: "file", bytes: 11 * 1024 * 1024 },
  ]),
};
(window as unknown as { vua: unknown }).vua = {
  library: { previewDirectory: async (path: string) => previews[path] ?? { path, state: "unavailable", truncated: false, items: [] } },
  dialog: { pickWarehouseFolders: async () => [root] },
};

createRoot(document.getElementById("root")!).render(
  <GatewayProvider gateway={emptyGateway()}>
    <LibraryMigrationPreview onClose={() => {}} initialRoots={[root]} />
  </GatewayProvider>,
);
