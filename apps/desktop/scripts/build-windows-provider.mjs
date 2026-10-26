import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

if (process.platform !== "win32") throw new Error("The Windows ZIP Provider must be built on Windows");
const workspace = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const component = process.argv.find(value => value.startsWith("--component="))?.slice(12);
if (component && !["host", "amf"].includes(component)) throw Error("Unknown native component build");
const packages = component === "amf" ? ["vua-amf-provider"] : component === "host" ? ["vua-provider-host", "vua-distribution"]
  : ["vua-provider-host", "vua-amf-provider", "vua-distribution"];
// Link the Microsoft C runtime into this release executable so a new player's
// machine does not need a separate Visual C++ runtime before VUA can even start.
// An explicit target keeps this artifact separate from normal development builds.
const result = spawnSync("cargo", [
  "build", "--release", "--locked", "--target", "x86_64-pc-windows-msvc",
  "--config", 'target.x86_64-pc-windows-msvc.rustflags=["-C","target-feature=+crt-static"]',
  ...packages.flatMap(name => ["-p", name]), "--bins",
], { cwd: workspace, stdio: "inherit", shell: false, windowsHide: true });
if (result.error) throw result.error;
if (result.status !== 0) throw new Error(`Windows Provider build failed (${result.status ?? result.signal})`);
