import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { ownedPath, fileDigest } from "./prepare-component-release.mjs";

const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workspace = path.resolve(desktop, "../..");
const require = createRequire(path.join(desktop, "package.json"));
export function exportComponentPublisher(destination) {
  const output = path.resolve(destination); ownedPath(output, "tool-build.json"); fs.mkdirSync(path.dirname(output), { recursive: true }); fs.mkdirSync(output, { recursive: false });
  const copy = (source, relative) => {
    const target = ownedPath(output, relative); fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.cpSync(source, target, { recursive: true, dereference: false, errorOnExist: true, force: false });
  };
  for (const name of ["distribution/component-index.mjs", "distribution/trust.json", "scripts/prepare-component-release.mjs", "scripts/publish-component-release.mjs"]) copy(path.join(desktop, name), `apps/desktop/${name}`);
  const semverRoot = path.dirname(require.resolve("semver/package.json"));
  if (JSON.parse(fs.readFileSync(path.join(semverRoot, "package.json"))).version !== "7.8.5") throw Error("Publication dependency differs from lockfile");
  copy(semverRoot, "node_modules/semver");
  for (const license of ["LICENSE", "NOTICE"]) copy(path.join(workspace, license), license);
  fs.writeFileSync(ownedPath(output, "package.json"), JSON.stringify({ name: "vua-component-publisher", private: true, type: "module", version: "0.2.0" }), { flag: "wx" });
  fs.writeFileSync(ownedPath(output, "README.txt"), "VUA component publication operator tool\n\nRequires Node.js 24 or later. No package installation or signing key is needed on the server.\nRun: node apps/desktop/scripts/publish-component-release.mjs <private-prepared-directory> <owned-vua-web-root>\nThe public trust anchor is distribution/trust.json. Keep the tool and prepared input in private staging.\n", { flag: "wx" });
  const files = [];
  function walk(relative = "") {
    for (const entry of fs.readdirSync(relative ? ownedPath(output, relative) : output, { withFileTypes: true })) {
      const name = relative ? `${relative}/${entry.name}` : entry.name; const file = ownedPath(output, name);
      if (entry.isDirectory()) walk(name);
      else if (entry.isFile()) files.push({ path: name, ...fileDigest(file) });
      else throw Error("Publication tool contains a link or special file");
    }
  }
  walk(); files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  const sourceCommit = execFileSync("git", ["rev-parse", "HEAD"], { cwd: workspace, windowsHide: true, encoding: "utf8" }).trim();
  const dirty = execFileSync("git", ["status", "--porcelain"], { cwd: workspace, windowsHide: true, encoding: "utf8" }).trim() !== "";
  const record = { schemaVersion: "0.2", sourceCommit, dirty, files };
  fs.writeFileSync(ownedPath(output, "tool-build.json"), JSON.stringify(record, null, 2) + "\n", { flag: "wx" });
  return { sourceCommit, dirty, files: files.length, output };
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!process.argv[2]) throw Error("Usage: export-component-publisher.mjs <new-private-tool-directory>");
  console.log(JSON.stringify(exportComponentPublisher(process.argv[2])));
}
