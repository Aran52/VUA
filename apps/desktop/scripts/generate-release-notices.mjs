// Per-build license inventory. Only public dependency sources are copied;
// local Cargo/cache paths never enter the distributed JSON or notices.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { createHash } from "node:crypto";
import { deflateRawSync } from "node:zlib";

const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workspace = path.resolve(desktop, "../..");
const outputArgument = process.argv.find(value => value.startsWith("--output="))?.slice(9);
const output = outputArgument ? path.resolve(outputArgument) : path.join(desktop, "out/release-notices");
if (outputArgument && fs.existsSync(output)) throw Error("Component notices require a new output directory");
const component = process.argv.find(value => value.startsWith("--component="))?.slice(12);
if (component && !["host", "amf"].includes(component)) throw Error("Unknown notices component");
const zip = process.argv.includes("--zip");
if (zip && component) throw Error("--zip cannot be combined with --component");

const pkg = JSON.parse(fs.readFileSync(path.join(desktop, "package.json"), "utf8"));
const rootPkg = JSON.parse(fs.readFileSync(path.join(workspace, "package.json"), "utf8"));
const meta = fs.readFileSync(path.join(desktop, "src/renderer/app/app-meta.ts"), "utf8");
if (rootPkg.version !== pkg.version || !meta.includes(`version: "${pkg.version}"`)) throw new Error("Product versions disagree");

const sha256Bytes = buffer => createHash("sha256").update(buffer).digest("hex");
const hash = file => sha256Bytes(fs.readFileSync(file));
const git = args => execFileSync("git", args, { cwd: workspace, encoding: "utf8", windowsHide: true }).trim();
const metadata = JSON.parse(execFileSync("cargo", ["metadata", "--locked", "--format-version", "1", "--filter-platform", "x86_64-pc-windows-msvc"], { cwd: workspace, encoding: "utf8", maxBuffer: 32 * 1024 * 1024, windowsHide: true }));
const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
const sysroot = execFileSync("rustc", ["--print", "sysroot"], { encoding: "utf8", windowsHide: true }).trim();
const compiler = execFileSync("rustc", ["--version"], { encoding: "utf8", windowsHide: true }).trim();
const sourceCommit = git(["rev-parse", "HEAD"]);
const dirty = git(["status", "--porcelain", "--untracked-files=no"]) !== "";
const locks = ["Cargo.lock", "pnpm-lock.yaml"].map(name => ({ name, sha256: hash(path.join(workspace, name)) }));
const require = createRequire(path.join(desktop, "package.json"));

const crcTable = new Uint32Array(256).map((_, index) => {
  let c = index;
  for (let k = 0; k < 8; k += 1) c = (c & 1) ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
  return c >>> 0;
});
function crc32(buffer) {
  let c = 0xffffffff;
  for (let i = 0; i < buffer.length; i += 1) c = crcTable[(c ^ buffer[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function buildDeterministicZip(entriesMap) {
  const sorted = [...entriesMap.entries()].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  const localParts = [];
  const centralParts = [];
  let localOffset = 0;
  for (const [entryPath, raw] of sorted) {
    const nameBytes = Buffer.from(entryPath, "utf8");
    const deflated = deflateRawSync(raw, { level: 9 });
    const useDeflate = deflated.length < raw.length;
    const method = useDeflate ? 8 : 0;
    const body = useDeflate ? deflated : raw;
    const checksum = crc32(raw);

    const localHeader = Buffer.alloc(30);
    localHeader.writeUInt32LE(0x04034b50, 0);
    localHeader.writeUInt16LE(20, 4);
    localHeader.writeUInt16LE(0x0800, 6);
    localHeader.writeUInt16LE(method, 8);
    localHeader.writeUInt16LE(0x0000, 10);
    localHeader.writeUInt16LE(0x0021, 12);
    localHeader.writeUInt32LE(checksum, 14);
    localHeader.writeUInt32LE(body.length, 18);
    localHeader.writeUInt32LE(raw.length, 22);
    localHeader.writeUInt16LE(nameBytes.length, 26);
    localHeader.writeUInt16LE(0, 28);
    localParts.push(localHeader, nameBytes, body);

    const centralHeader = Buffer.alloc(46);
    centralHeader.writeUInt32LE(0x02014b50, 0);
    centralHeader.writeUInt16LE(20, 4);
    centralHeader.writeUInt16LE(20, 6);
    centralHeader.writeUInt16LE(0x0800, 8);
    centralHeader.writeUInt16LE(method, 10);
    centralHeader.writeUInt16LE(0x0000, 12);
    centralHeader.writeUInt16LE(0x0021, 14);
    centralHeader.writeUInt32LE(checksum, 16);
    centralHeader.writeUInt32LE(body.length, 20);
    centralHeader.writeUInt32LE(raw.length, 24);
    centralHeader.writeUInt16LE(nameBytes.length, 28);
    centralHeader.writeUInt16LE(0, 30);
    centralHeader.writeUInt16LE(0, 32);
    centralHeader.writeUInt16LE(0, 34);
    centralHeader.writeUInt16LE(0, 36);
    centralHeader.writeUInt32LE(0, 38);
    centralHeader.writeUInt32LE(localOffset, 42);
    centralParts.push(centralHeader, nameBytes);

    localOffset += localHeader.length + nameBytes.length + body.length;
  }
  const centralSize = centralParts.reduce((sum, part) => sum + part.length, 0);
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(0, 4);
  eocd.writeUInt16LE(0, 6);
  eocd.writeUInt16LE(sorted.length, 8);
  eocd.writeUInt16LE(sorted.length, 10);
  eocd.writeUInt32LE(centralSize, 12);
  eocd.writeUInt32LE(localOffset, 16);
  eocd.writeUInt16LE(0, 20);
  return Buffer.concat([...localParts, ...centralParts, eocd]);
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
  "pdf-extract@0.12.1": "LICENSE-pdf-extract.txt",
  "type1-encoding-parser@0.1.1": "LICENSE-type1-encoding-parser.txt",
  "vrc-get-vpm@0.0.16": "LICENSE-vrc-get-vpm.txt",
  "vrc-get-litedb@0.3.0-beta.8": "LICENSE-vrc-get-litedb.txt",
  "selectors@0.38.0": "LICENSE-MPL-2.0.txt",
};
const supplementalSources = new Set([
  "adobe-cmap-parser@0.4.1", "enum-map@2.7.3", "enum-map-derive@0.17.0",
  "pdf-extract@0.12.1", "type1-encoding-parser@0.1.1", "vrc-get-litedb@0.3.0-beta.8",
]);

function generateNotices({ targetComponent, targetOutput, legacyUnarchivedZip }) {
  const seen = new Set();
  function visit(id) {
    if (seen.has(id)) return;
    seen.add(id);
    for (const dependency of nodes.get(id).deps) if (dependency.dep_kinds.some(kind => kind.kind !== "dev")) visit(dependency.pkg);
  }
  const nativeNames = targetComponent === "amf" ? ["vua-amf-provider"] : targetComponent === "host" ? ["vua-provider-host", "vua-distribution"]
    : ["vua-provider-host", "vua-amf-provider", ...(legacyUnarchivedZip ? [] : ["vua-distribution"])];
  const providers = metadata.packages.filter(item => nativeNames.includes(item.name));
  if (providers.length !== nativeNames.length) throw new Error("Provider/distribution metadata missing");
  providers.forEach(item => visit(item.id));
  fs.mkdirSync(targetOutput, { recursive: true });

  const archivedEntries = new Map();
  function recordNoticeFile(source, relative) {
    const normalized = relative.replaceAll("\\", "/");
    const raw = fs.readFileSync(source);
    const digest = sha256Bytes(raw);
    if (legacyUnarchivedZip) {
      const target = path.join(targetOutput, relative);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, raw);
    } else {
      archivedEntries.set(normalized, raw);
    }
    return { path: normalized, sha256: digest };
  }

  const components = [];
  for (const item of metadata.packages.filter(item => seen.has(item.id) && item.source).sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version))) {
    const root = path.dirname(item.manifest_path);
    const files = filesIn(root);
    const licenseFiles = new Set(files.filter(file => /^(LICEN[CS]ES?|COPYING|NOTICE|COPYRIGHT)([-._ ]|$)/i.test(path.basename(file)) || file.includes(`${path.sep}LICENSES${path.sep}`)));
    if (item.license_file) licenseFiles.add(path.resolve(root, item.license_file));
    const key = `${item.name}@${item.version}`;
    const notices = [...licenseFiles].map(file => recordNoticeFile(file, `rust/${key}/${path.relative(root, file).replaceAll("..", "parent")}`));
    if (supplemental[key]) notices.push(recordNoticeFile(path.join(desktop, "packaging/licenses", supplemental[key]), `rust/${key}/${supplemental[key]}`));
    if (!item.license || !notices.length) throw new Error(`Dependency needs license review: ${key}`);
    // Preserve original declarations and unmodified sources for manifest-only notices.
    const sources = item.license.includes("MPL-2.0") || supplementalSources.has(key)
      ? files.filter(file => !path.basename(file).startsWith(".")).map(file => recordNoticeFile(file, `sources/${key}/${path.relative(root, file)}`)) : [];
    components.push({ ecosystem: "cargo", name: item.name, version: item.version, license: item.license,
      source: `https://crates.io/api/v1/crates/${item.name}/${item.version}/download`, repository: item.repository,
      authors: item.authors, notices, ...(sources.length ? { sources } : {}) });
  }

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
    const notices = fs.readdirSync(root).filter(file => /^(LICENSE|COPYING|NOTICE)([-._ ]|$)/i.test(file)).map(file => recordNoticeFile(path.join(root, file), `npm/${key}/${file}`));
    if (!item.license || !notices.length) throw new Error(`JavaScript dependency needs license review: ${key}`);
    components.push({ ecosystem: "npm", name: item.name, version: item.version, license: item.license,
      source: `https://registry.npmjs.org/${item.name}/-/${item.name.split("/").at(-1)}-${item.version}.tgz`, notices });
    const childResolver = createRequire(manifest);
    for (const child of Object.keys(item.dependencies ?? {})) npmDependency(child, childResolver);
  }
  // Main bundles the MCP runtime too; first-party workspace packages use VUA's notices.
  const npmRoots = targetComponent === "amf" ? ["react", "react-dom", "three"]
    : Object.entries(pkg.dependencies).filter(([, specifier]) => !specifier.startsWith("workspace:")).map(([name]) => name);
  for (const name of npmRoots) npmDependency(name);

  let electron;
  if (targetComponent !== "amf") {
    const electronRoot = path.dirname(require.resolve("electron/package.json"));
    const runtime = path.join(electronRoot, "dist");
    const electronVersion = fs.readFileSync(path.join(runtime, "version"), "utf8").trim();
    if (electronVersion !== pkg.devDependencies.electron) throw new Error("Electron runtime differs from pinned build");
    if (legacyUnarchivedZip) {
      for (const file of ["LICENSE", "LICENSES.chromium.html"]) recordNoticeFile(path.join(runtime, file), `electron/${file}`);
      electron = { version: electronVersion, executableSha256: hash(path.join(runtime, "electron.exe")), notices: ["electron/LICENSE", "electron/LICENSES.chromium.html"] };
    } else {
      // Installed host shares the verified runtime/<sha256> directory, which carries
      // LICENSE and LICENSES.chromium.html under signed manifest verification.
      for (const file of ["LICENSE", "LICENSES.chromium.html"]) {
        if (!fs.existsSync(path.join(runtime, file))) throw new Error(`Missing runtime notice: ${file}`);
      }
      electron = {
        version: electronVersion,
        executableSha256: hash(path.join(runtime, "electron.exe")),
        runtimeNotices: ["LICENSE", "LICENSES.chromium.html"],
      };
    }
  }

  const rustNotices = path.join(sysroot, "share/doc/rust");
  recordNoticeFile(path.join(rustNotices, "COPYRIGHT-library.html"), "rust-toolchain/COPYRIGHT-library.html");
  for (const file of filesIn(path.join(rustNotices, "licenses"))) {
    recordNoticeFile(file, `rust-toolchain/licenses/${path.relative(path.join(rustNotices, "licenses"), file)}`);
  }

  let noticeArchive;
  if (!legacyUnarchivedZip) {
    const zipBytes = buildDeterministicZip(archivedEntries);
    fs.writeFileSync(path.join(targetOutput, "notices.zip"), zipBytes);
    noticeArchive = { path: "notices.zip", bytes: zipBytes.length, sha256: sha256Bytes(zipBytes), entries: archivedEntries.size };
  }

  const nativeRoot = path.resolve(workspace, process.env.CARGO_TARGET_DIR ?? "target", "x86_64-pc-windows-msvc/release");
  const executables = nativeNames.map(name => name === "vua-provider-host" ? "vua-orchestrator-provider.exe" : `${name}.exe`).map(name => ({ name, sha256: hash(path.join(nativeRoot, name)) }));
  const inventory = {
    productVersion: pkg.version, sourceCommit, dirty,
    platform: "win32", architecture: "x64", rustTarget: "x86_64-pc-windows-msvc",
    compiler,
    locks,
    ...(electron ? { electron } : { hostProvided: ["Electron runtime and Chromium/Node notices"] }),
    ...(targetComponent ? { component: targetComponent, componentVersion: targetComponent === "host" ? pkg.version : JSON.parse(fs.readFileSync(path.join(desktop, "distribution/amf-version.json"), "utf8")).version } : {}),
    ...(noticeArchive ? { noticeArchive } : {}),
    providers: providers.map(item => ({ name: item.name, crateVersion: item.version })), executables,
    scope: `${targetComponent === "amf" ? "AMF uses the host-provided Electron runtime; no runtime is included in this payload." : "Pinned Electron runtime and its bundled Chromium/Node inventory."} Bundled JavaScript runtime dependencies; resolved Windows provider normal/build Cargo dependencies, excluding dev-only dependencies. Cargo feature resolution may retain more source packages than linked code. External Steam/PICO/Unity/VRCFT software is not redistributed.`,
    components,
  };
  fs.writeFileSync(path.join(targetOutput, "dependency-inventory.json"), JSON.stringify(inventory, null, 2) + "\n");
  const runtimeNote = targetComponent === "amf"
    ? "AMF uses the host's verified Electron runtime; its Chromium/Node notices are supplied by the installed runtime."
    : legacyUnarchivedZip
      ? "Electron's complete Chromium/Node notices are in electron/LICENSES.chromium.html."
      : "Electron's complete Chromium/Node notices are verified in the installed runtime directory (LICENSE and LICENSES.chromium.html).";
  const archiveNote = legacyUnarchivedZip
    ? "Rust and JavaScript notices retain their original texts. Unmodified MPL source\npackages are supplied under sources/ and retain the Mozilla Public License 2.0.\nPackages with supplemental manifest-only notices also retain their unmodified\nsource and original author/license declarations under sources/."
    : "Rust, JavaScript, toolchain notices and unmodified MPL/supplemental source\npackages are archived in notices.zip at the paths listed in dependency-inventory.json.";
  fs.writeFileSync(path.join(targetOutput, "README.txt"), `VUA ${pkg.version} exact-build notices\n\nRead dependency-inventory.json for versions, source URLs and SHA-256 values.\n${runtimeNote}\n${archiveNote}\nRuntime toolchain/System libraries retain their upstream distribution terms.\nExternal applications are obtained separately and are not contained in this payload.\n`);
  return { version: pkg.version, component: targetComponent, cargo: components.filter(item => item.ecosystem === "cargo").length, npm: npmSeen.size, electron: electron?.version, noticeArchive: noticeArchive?.entries, output: path.relative(workspace, targetOutput).replaceAll("\\", "/") };
}

if (zip || component) {
  if (!outputArgument) fs.rmSync(output, { recursive: true, force: true });
  console.log(JSON.stringify(generateNotices({ targetComponent: component, targetOutput: output, legacyUnarchivedZip: zip })));
} else {
  if (!outputArgument) fs.rmSync(output, { recursive: true, force: true });
  const hostSummary = generateNotices({ targetComponent: "host", targetOutput: path.join(output, "host"), legacyUnarchivedZip: false });
  const amfSummary = generateNotices({ targetComponent: "amf", targetOutput: path.join(output, "amf"), legacyUnarchivedZip: false });
  console.log(JSON.stringify({ version: pkg.version, host: hostSummary, amf: amfSummary, output: path.relative(workspace, output).replaceAll("\\", "/") }));
}
