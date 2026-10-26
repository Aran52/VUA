// Per-build license inventory. Only public dependency sources are copied;
// local Cargo/cache paths never enter the distributed JSON or notices.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { createHash } from "node:crypto";

const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workspace = path.resolve(desktop, "../..");
const outputArgument = process.argv.find(value => value.startsWith("--output="))?.slice(9);
const output = outputArgument ? path.resolve(outputArgument) : path.join(desktop, "out/release-notices");
if (outputArgument && fs.existsSync(output)) throw Error("Component notices require a new output directory");
const component = process.argv.find(value => value.startsWith("--component="))?.slice(12);
if (component && !["host", "amf"].includes(component)) throw Error("Unknown notices component");
const pkg = JSON.parse(fs.readFileSync(path.join(desktop, "package.json"), "utf8"));
const rootPkg = JSON.parse(fs.readFileSync(path.join(workspace, "package.json"), "utf8"));
const meta = fs.readFileSync(path.join(desktop, "src/renderer/app/app-meta.ts"), "utf8");
if (rootPkg.version !== pkg.version || !meta.includes(`version: "${pkg.version}"`)) throw new Error("Product versions disagree");
const hash = file => createHash("sha256").update(fs.readFileSync(file)).digest("hex");
const git = args => execFileSync("git", args, { cwd: workspace, encoding: "utf8", windowsHide: true }).trim();
const metadata = JSON.parse(execFileSync("cargo", ["metadata", "--locked", "--format-version", "1", "--filter-platform", "x86_64-pc-windows-msvc"], { cwd: workspace, encoding: "utf8", maxBuffer: 32 * 1024 * 1024, windowsHide: true }));
const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
const seen = new Set();
function visit(id) {
  if (seen.has(id)) return;
  seen.add(id);
  for (const dependency of nodes.get(id).deps) if (dependency.dep_kinds.some(kind => kind.kind !== "dev")) visit(dependency.pkg);
}
const zip = process.argv.includes("--zip");
const nativeNames = component === "amf" ? ["vua-amf-provider"] : component === "host" ? ["vua-provider-host", "vua-distribution"]
  : ["vua-provider-host", "vua-amf-provider", ...(zip ? [] : ["vua-distribution"])];
const providers = metadata.packages.filter(item => nativeNames.includes(item.name));
if (providers.length !== nativeNames.length) throw new Error("Provider/distribution metadata missing");
providers.forEach(item => visit(item.id));
fs.mkdirSync(output, { recursive: true });
function copy(source, relative) {
  const target = path.join(output, relative);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(source, target);
  return { path: relative.replaceAll("\\", "/"), sha256: hash(target) };
}
// Crate packages may carry native/subcomponent notices in nested directories.
function filesIn(root) {
  return fs.readdirSync(root, { withFileTypes: true }).flatMap(entry => {
    const file = path.join(root, entry.name);
    if (entry.isSymbolicLink() || [".git", "target"].includes(entry.name)) return [];
    return entry.isDirectory() ? filesIn(file) : [file];
  });
}
const supplemental = {
  "adobe-cmap-parser@0.4.1": "LICENSE-adobe-cmap-parser.txt",
  "enum-map@2.7.3": "LICENSE-enum-map.txt",
  "enum-map-derive@0.17.0": "LICENSE-enum-map-derive.txt",
  "pdf-extract@0.10.0": "LICENSE-pdf-extract.txt",
  "type1-encoding-parser@0.1.1": "LICENSE-type1-encoding-parser.txt",
  "vrc-get-vpm@0.0.16": "LICENSE-vrc-get-vpm.txt",
  "vrc-get-litedb@0.3.0-beta.8": "LICENSE-vrc-get-litedb.txt",
  "selectors@0.38.0": "LICENSE-MPL-2.0.txt",
};
const supplementalSources = new Set([
  "adobe-cmap-parser@0.4.1", "enum-map@2.7.3", "enum-map-derive@0.17.0",
  "pdf-extract@0.10.0", "type1-encoding-parser@0.1.1", "vrc-get-litedb@0.3.0-beta.8",
]);
const components = [];
for (const item of metadata.packages.filter(item => seen.has(item.id) && item.source).sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version))) {
  const root = path.dirname(item.manifest_path);
  const files = filesIn(root);
  const licenseFiles = new Set(files.filter(file => /^(LICEN[CS]ES?|COPYING|NOTICE|COPYRIGHT)([-._ ]|$)/i.test(path.basename(file)) || file.includes(`${path.sep}LICENSES${path.sep}`)));
  if (item.license_file) licenseFiles.add(path.resolve(root, item.license_file));
  const key = `${item.name}@${item.version}`;
  const notices = [...licenseFiles].map(file => copy(file, `rust/${key}/${path.relative(root, file).replaceAll("..", "parent")}`));
  if (supplemental[key]) notices.push(copy(path.join(desktop, "packaging/licenses", supplemental[key]), `rust/${key}/${supplemental[key]}`));
  if (!item.license || !notices.length) throw new Error(`Dependency needs license review: ${key}`);
  // Preserve original declarations and unmodified sources for manifest-only notices.
  const sources = item.license.includes("MPL-2.0") || supplementalSources.has(key)
    ? files.filter(file => !path.basename(file).startsWith(".")).map(file => copy(file, `sources/${key}/${path.relative(root, file)}`)) : [];
  components.push({ ecosystem: "cargo", name: item.name, version: item.version, license: item.license,
    source: `https://crates.io/api/v1/crates/${item.name}/${item.version}/download`, repository: item.repository,
    authors: item.authors, notices, ...(sources.length ? { sources } : {}) });
}
const require = createRequire(path.join(desktop, "package.json"));
const npmSeen = new Set();
function npmDependency(name, resolver = require) {
  let manifest;
  try { manifest = resolver.resolve(`${name}/package.json`); }
  catch {
    let directory = path.dirname(resolver.resolve(name));
    for (;;) {
      const candidate = path.join(directory, "package.json");
      if (fs.existsSync(candidate) && JSON.parse(fs.readFileSync(candidate, "utf8")).name === name) { manifest = candidate; break; }
      const parent = path.dirname(directory);
      if (parent === directory) throw new Error(`Cannot find dependency manifest: ${name}`);
      directory = parent;
    }
  }
  const item = JSON.parse(fs.readFileSync(manifest, "utf8"));
  const key = `${item.name}@${item.version}`;
  if (npmSeen.has(key)) return;
  npmSeen.add(key);
  const root = path.dirname(manifest);
  const notices = fs.readdirSync(root).filter(file => /^(LICENSE|COPYING|NOTICE)([-._ ]|$)/i.test(file)).map(file => copy(path.join(root, file), `npm/${key}/${file}`));
  if (!item.license || !notices.length) throw new Error(`JavaScript dependency needs license review: ${key}`);
  components.push({ ecosystem: "npm", name: item.name, version: item.version, license: item.license,
    source: `https://registry.npmjs.org/${item.name}/-/${item.name.split("/").at(-1)}-${item.version}.tgz`, notices });
  const childResolver = createRequire(manifest);
  for (const child of Object.keys(item.dependencies ?? {})) npmDependency(child, childResolver);
}
for (const name of ["react", "react-dom", "three"]) npmDependency(name);
let electron;
if (component !== "amf") {
  const electronRoot = path.dirname(require.resolve("electron/package.json"));
  const runtime = path.join(electronRoot, "dist");
  const electronVersion = fs.readFileSync(path.join(runtime, "version"), "utf8").trim();
  if (electronVersion !== pkg.devDependencies.electron) throw new Error("Electron runtime differs from pinned build");
  for (const file of ["LICENSE", "LICENSES.chromium.html"]) copy(path.join(runtime, file), `electron/${file}`);
  electron = { version: electronVersion, executableSha256: hash(path.join(runtime, "electron.exe")), notices: ["electron/LICENSE", "electron/LICENSES.chromium.html"] };
}
const sysroot = execFileSync("rustc", ["--print", "sysroot"], { encoding: "utf8", windowsHide: true }).trim();
const rustNotices = path.join(sysroot, "share/doc/rust");
copy(path.join(rustNotices, "COPYRIGHT-library.html"), "rust-toolchain/COPYRIGHT-library.html");
for (const file of filesIn(path.join(rustNotices, "licenses"))) copy(file, `rust-toolchain/licenses/${path.relative(path.join(rustNotices, "licenses"), file)}`);
const nativeRoot = path.resolve(workspace, process.env.CARGO_TARGET_DIR ?? "target", "x86_64-pc-windows-msvc/release");
const executables = nativeNames.map(name => name === "vua-provider-host" ? "vua-orchestrator-provider.exe" : `${name}.exe`).map(name => ({ name, sha256: hash(path.join(nativeRoot, name)) }));
const inventory = {
  productVersion: pkg.version, sourceCommit: git(["rev-parse", "HEAD"]), dirty: git(["status", "--porcelain", "--untracked-files=no"]) !== "",
  platform: "win32", architecture: "x64", rustTarget: "x86_64-pc-windows-msvc",
  compiler: execFileSync("rustc", ["--version"], { encoding: "utf8", windowsHide: true }).trim(),
  locks: ["Cargo.lock", "pnpm-lock.yaml"].map(name => ({ name, sha256: hash(path.join(workspace, name)) })),
  ...(electron ? { electron } : { hostProvided: ["Electron runtime and Chromium/Node notices"] }),
  ...(component ? { component, componentVersion: component === "host" ? pkg.version : JSON.parse(fs.readFileSync(path.join(desktop, "distribution/amf-version.json"))).version } : {}),
  providers: providers.map(item => ({ name: item.name, crateVersion: item.version })), executables,
  scope: `${component === "amf" ? "AMF uses the host-provided Electron runtime; no runtime is included in this payload." : "Pinned Electron runtime and its bundled Chromium/Node inventory."} Bundled JavaScript runtime dependencies; resolved Windows provider normal/build Cargo dependencies, excluding dev-only dependencies. Cargo feature resolution may retain more source packages than linked code. External Steam/PICO/Unity/VRCFT software is not redistributed.`,
  components,
};
fs.writeFileSync(path.join(output, "dependency-inventory.json"), JSON.stringify(inventory, null, 2) + "\n");
fs.writeFileSync(path.join(output, "README.txt"), `VUA ${pkg.version} exact-build notices\n\nRead dependency-inventory.json for versions, source URLs and SHA-256 values.\n${component === "amf" ? "AMF uses the host's verified Electron runtime; its Chromium/Node notices are supplied by the host." : "Electron's complete Chromium/Node notices are in electron/LICENSES.chromium.html."}\nRust and JavaScript notices retain their original texts. Unmodified MPL source\npackages are supplied under sources/ and retain the Mozilla Public License 2.0.\nPackages with supplemental manifest-only notices also retain their unmodified\nsource and original author/license declarations under sources/.\nRuntime toolchain/System libraries retain their upstream distribution terms.\nExternal applications are obtained separately and are not contained in this payload.\n`);
console.log(JSON.stringify({ version: pkg.version, component, cargo: components.filter(item => item.ecosystem === "cargo").length, npm: npmSeen.size, electron: electron?.version, output: path.relative(workspace, output).replaceAll("\\", "/") }));
