// Isolated CI preview only. Never uses a production publishing key or uploads
// a production-trusted feed. Both verifier and producer get an ephemeral anchor
// in this disposable checkout; ordinary release preparation remains unchanged.
import fs from "node:fs";
import path from "node:path";
import { createPublicKey, generateKeyPairSync } from "node:crypto";
import { execFileSync } from "node:child_process";
if (process.env.GITHUB_ACTIONS !== "true" || !process.env.RUNNER_TEMP) throw Error("CI preview requires a disposable Actions checkout");
const workspace = process.cwd();
const desktop = path.join(workspace, "apps/desktop");
const key = generateKeyPairSync("ed25519").privateKey;
const pub = createPublicKey(key).export({ type: "spki", format: "der" }).subarray(-32);
const privateFile = path.join(process.env.RUNNER_TEMP, "vua-ci-nsis-key.pem");
fs.writeFileSync(privateFile, key.export({ type: "pkcs8", format: "pem" }), { flag: "wx", mode: 0o600 });
fs.writeFileSync(path.join(desktop, "distribution/trust.json"), JSON.stringify({ keyId: "ephemeral-ci-preview", algorithm: "ed25519", publicKey: pub.toString("hex") }));
const native = path.join(workspace, "crates/distribution/src/layout.rs");
fs.writeFileSync(native, fs.readFileSync(native, "utf8").replace(/pub const PUBLIC_KEY: \[u8; 32\] = \[[\s\S]*?\];/, `pub const PUBLIC_KEY: [u8; 32] = [${[...pub].join(",")}];`));
const env = { ...process.env, VUA_RELEASE_KEY: privateFile };
const run = (file, args) => execFileSync(file, args, { cwd: workspace, env, windowsHide: true, stdio: "inherit" });
run(process.execPath, [path.join(desktop, "scripts/build-windows-provider.mjs")]);
run(process.execPath, [path.join(desktop, "scripts/build-electron.mjs")]);
run(process.execPath, ["--experimental-strip-types", path.join(desktop, "scripts/build-components.mjs")]);
run(process.execPath, [path.join(desktop, "scripts/generate-release-notices.mjs")]);
run(process.execPath, [path.join(desktop, "scripts/prepare-distribution.mjs"), "--preview"]);
run(process.execPath, [path.join(desktop, "scripts/package-nsis.mjs"), "--preview"]);
fs.unlinkSync(privateFile);
