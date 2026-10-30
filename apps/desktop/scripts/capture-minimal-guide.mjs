// F8: curated real-renderer captures; raw audit output stays in an ignored local root.
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workspace = path.resolve(desktop, "../..");
const git = (...args) => execFileSync("git", args, { cwd: workspace, encoding: "utf8" }).trim();
const pkg = JSON.parse(readFileSync(path.join(desktop, "package.json")));
if (pkg.version !== "1.0.0") throw Error("F8 captures require the actual 1.0.0 source version");
if (git("status", "--porcelain", "--untracked-files=no")) throw Error("Commit the source before capturing");
const sourceCommit = git("rev-parse", "HEAD");
const build = JSON.parse(readFileSync(path.join(desktop, "dist/renderer/build-info.json")));
if (build.version !== pkg.version || build.dirty || build.commit !== git("rev-parse", "--short", "HEAD")) throw Error("Build the clean current 1.0.0 renderer before capturing");
const supplied = process.argv.find(arg => arg.startsWith("--out="))?.slice(6);
if (!supplied) throw Error("Pass --out=<new ignored _local_* audit directory>");
const raw = path.resolve(supplied);
if (!raw.split(path.sep).some(part => part.startsWith("_local_")) || existsSync(raw)) throw Error("Raw output must be a new ignored _local_* directory");
const views = ["first-use-notice", "beginner-wizard", "env-create", "import-dialog", "materials-start", "workshop-inspect", "workshop-prediction", "workshop-finished", "release-sdk"];
const locales = ["en", "zh-CN", "zh-TW", "ja", "ko"];
const require = createRequire(import.meta.url);
const result = spawnSync(require("electron"), [path.join(desktop, "scripts/design-audit.mjs"), "--only=" + views.join(","), "--theme=dark", "--out=" + raw], { cwd: desktop, env: process.env, stdio: "inherit", windowsHide: true });
if (result.error) throw result.error;
if (result.status !== 0) throw Error("Guide capture run did not complete");
const run = JSON.parse(readFileSync(path.join(raw, "run-summary.json")));
const output = path.join(workspace, "docs/user-guide/v1.0.0/images");
const captures = [];
for (const locale of locales) for (const view of views) {
  const suffix = locale === "zh-CN" ? "" : "--" + locale;
  const input = path.join(raw, "shots", view + suffix + "--dark.png");
  const data = readFileSync(input);
  if (data.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a") throw Error("Invalid PNG: " + view);
  const directory = path.join(output, locale); mkdirSync(directory, { recursive: true });
  copyFileSync(input, path.join(directory, view + ".png"));
  captures.push({ locale, view, theme: "dark", themeFamily: "grid", file: "images/" + locale + "/" + view + ".png", bytes: data.length, sha256: createHash("sha256").update(data).digest("hex"), width: data.readUInt32BE(16), height: data.readUInt32BE(20) });
}
const manifest = { productVersion: pkg.version, amfVersion: JSON.parse(readFileSync(path.join(desktop, "distribution/amf-version.json"))).version, sourceCommit, rendererBuild: build, capturedAt: new Date().toISOString(), method: "Actual React renderer through the existing design-audit fixtures; synthetic Gateway facts and stand-in imagery", scope: "Illustrates the 1.0.0 UI; not a real-material production, installer, clean-VM, SDK execution or human acceptance", privacy: "No real accounts, orders, private material files or device inventory; fixture data only", nativeReview: "All five languages pending, including Traditional Chinese", installerCaptures: "Pending author capture on the recorded clean Windows VM, online and offline", captures };
writeFileSync(path.join(output, "../capture-manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(JSON.stringify({ version: pkg.version, sourceCommit, captures: captures.length, auditCompleted: run.coverage?.complete ?? run.complete ?? null }));
