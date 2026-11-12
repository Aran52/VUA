import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import {createHash} from "node:crypto";
import {fileURLToPath} from "node:url";
import {verifyInventory} from "../distribution/component-index.mjs";
import {prepareComponentRelease, fileDigest, ownedPath} from "./prepare-component-release.mjs";

/** Extract authenticated bytes, never run a published payload or forge a build receipt. */
export function seedComponentRelease({component, baseline, packFile, previous, previousV03, sequence, retainVersions = [], output, privateKey, publicKey}) {
  const original = verifyInventory(baseline, publicKey);
  if (!["host", "amf"].includes(component)) throw Error("invalid_component_identity");
  const inventory = original[component], actual = fileDigest(packFile);
  if (actual.sha256 !== inventory.pack.sha256 || actual.bytes !== inventory.pack.bytes) throw Error("seed_pack_corrupt");
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "vua-authenticated-seed-"));
  const fd = fs.openSync(packFile, "r");
  try {
    const buffer = Buffer.alloc(1024*1024);
    for (const file of inventory.files) {
      const destination = ownedPath(temporary, file.path); fs.mkdirSync(path.dirname(destination), {recursive: true});
      const target = fs.openSync(destination, "wx"), hash = createHash("sha256"); let position = 0;
      try {
        while (position < file.bytes) {
          const count = fs.readSync(fd, buffer, 0, Math.min(buffer.length, file.bytes-position), file.offset+position);
          if (!count) throw Error("seed_pack_truncated");
          const bytes = buffer.subarray(0, count); hash.update(bytes);
          let written = 0; while (written < count) {const n = fs.writeSync(target, bytes, written, count-written); if (!n) throw Error("seed_write_failed"); written += n;}
          position += count;
        }
      } finally {fs.closeSync(target);}
      if (hash.digest("hex") !== file.sha256) throw Error("seed_file_corrupt");
    }
    return prepareComponentRelease({component, version: inventory.version, build: inventory.build, componentRoot: temporary, baseline, previous, previousV03, sequence, retainVersions, output, privateKey, publicKey});
  } finally {
    fs.closeSync(fd);
    if (path.dirname(path.resolve(temporary)) !== path.resolve(os.tmpdir()) || !path.basename(temporary).startsWith("vua-authenticated-seed-")) throw Error("unsafe_seed_cleanup");
    fs.rmSync(temporary, {recursive: true, force: true});
  }
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const flags = process.argv.slice(2).filter(arg => arg.startsWith("--"));
  const positional = process.argv.slice(2).filter(arg => !arg.startsWith("--"));
  const [component, manifest, packFile, previous, sequence, output, keyFile] = positional;
  if (!keyFile) throw Error("Usage: seed-component-release.mjs <host|amf> <official-vua-release.json> <official-pack.bin> <previous-index.json|none> <sequence> <new-output> <private-key-file> [--previous-v03=<path>] [--retain=<v1,v2>]");
  const previousV03Arg = flags.find(arg => arg.startsWith("--previous-v03="))?.slice("--previous-v03=".length);
  const retainArg = flags.find(arg => arg.startsWith("--retain="))?.slice("--retain=".length);
  const retainVersions = retainArg ? retainArg.split(",").map(v => v.trim()).filter(Boolean) : [];
  const trust = JSON.parse(fs.readFileSync(new URL("../distribution/trust.json", import.meta.url)));
  console.log(JSON.stringify(seedComponentRelease({
    component,
    baseline: fs.readFileSync(manifest),
    packFile: path.resolve(packFile),
    previous: previous === "none" ? undefined : fs.readFileSync(previous),
    previousV03: previousV03Arg ? fs.readFileSync(previousV03Arg) : undefined,
    sequence: Number(sequence),
    retainVersions,
    output: path.resolve(output),
    privateKey: fs.readFileSync(keyFile),
    publicKey: trust.publicKey,
  })));
}
