import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(path.join(desktop, "package.json"));
const builder = createRequire(require.resolve("electron-builder"));
const library = createRequire(builder.resolve("app-builder-lib"));
const tool = await library("./toolsets/windows.js").getMakeNsisPath();
const input = JSON.parse(fs.readFileSync(path.join(desktop, "out/distribution/latest-build.json"))).output;
const metadata = JSON.parse(fs.readFileSync(path.join(input, "build.json")));
const preview = process.argv.includes("--preview");
const script = path.join(input, "installer.nsi");
fs.writeFileSync(script, "\ufeff" + fs.readFileSync(path.join(desktop, "distribution/installer.nsi"), "utf8"));
for (const offline of [false, true]) {
  const output = path.join(input, `VUA-${metadata.version}-windows-x64-${preview ? "preview-" : ""}${offline ? "offline" : "setup"}.exe`);
  const result = spawnSync(tool.path, ["/V2", `/DVERSION=${metadata.version}`, `/DINPUT=${input}`, `/DOUTPUT=${output}`, ...(offline ? ["/DOFFLINE"] : []), ...(preview ? ["/DPREVIEW"] : []), script],
    { cwd: desktop, env: { ...process.env, ...tool.env }, windowsHide: true, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) throw Error(`NSIS failed (${result.status})`);
  console.log(JSON.stringify({ artifact: path.basename(output), bytes: fs.statSync(output).size }));
}
