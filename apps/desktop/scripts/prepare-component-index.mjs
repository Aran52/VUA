import fs from "node:fs";
import path from "node:path";
import { isDeepStrictEqual } from "node:util";
import { fileURLToPath } from "node:url";
import { digest, sealEnvelope, validateIndex, validateIndexV03, verifyIndex, verifyIndexV03 } from "../distribution/component-index.mjs";

/** Signed removal-only revision across v0.2 and (when available or emptied) v0.3. */
export function prepareComponentIndex({ component, previous, previousV03, sequence, removeVersions, output, privateKey, publicKey }) {
  const before = verifyIndex(previous, publicKey, component);
  const beforeV03 = previousV03 ? verifyIndexV03(previousV03, publicKey, component) : null;
  if (beforeV03 && (beforeV03.sequence !== before.sequence || !isDeepStrictEqual(before.releases.map(r => r.version), beforeV03.releases.map(r => r.version)))) {
    throw Error("publication_sequence_mismatch");
  }
  if (!Number.isSafeInteger(sequence) || sequence <= before.sequence) throw Error("publication_sequence_mismatch");
  if (!Array.isArray(removeVersions) || !removeVersions.length || removeVersions.some(v => !before.releases.some(item => item.version === v))) throw Error("invalid_removed_versions");
  const index = validateIndex({ ...before, sequence, releases: before.releases.filter(item => !removeVersions.includes(item.version)) });
  const raw = sealEnvelope(index, "0.2", privateKey, publicKey);
  const directory = path.join(output, "updates/beta"); fs.mkdirSync(directory, { recursive: true });
  fs.writeFileSync(path.join(directory, `${component}.json`), raw, { flag: "wx" });
  let v03;
  if (beforeV03 || index.releases.length === 0) {
    const indexV03 = validateIndexV03({
      ...(beforeV03 ?? { ...structuredClone(before), schemaVersion: "0.3" }),
      sequence,
      releases: (beforeV03 ?? before).releases.filter(item => !removeVersions.includes(item.version)),
    });
    const rawV03 = sealEnvelope(indexV03, "0.3", privateKey, publicKey);
    const directoryV03 = path.join(output, "updates/v0.3/beta"); fs.mkdirSync(directoryV03, { recursive: true });
    fs.writeFileSync(path.join(directoryV03, `${component}.json`), rawV03, { flag: "wx" });
    v03 = { previousIndexSha256: previousV03 ? digest(previousV03) : null, indexSha256: digest(rawV03) };
  }
  const record = { schemaVersion: "0.2", kind: "index", component, previousIndexSha256: digest(previous), indexSha256: digest(raw), ...(v03 ? { v03 } : {}) };
  fs.writeFileSync(path.join(output, "publication.json"), JSON.stringify(record, null, 2) + "\n", { flag: "wx" });
  return record;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.filter(arg => !arg.startsWith("--previous-v03="));
  const previousV03Path = process.argv.find(arg => arg.startsWith("--previous-v03="))?.slice(15);
  const [component, previous, sequence, versions, output, keyFile] = args.slice(2);
  if (!keyFile) throw Error("Usage: prepare-component-index.mjs <host|amf> <previous.json> <sequence> <comma-separated-versions> <new-output> <private-key-file> [--previous-v03=<previous-v03.json>]");
  const trust = JSON.parse(fs.readFileSync(new URL("../distribution/trust.json", import.meta.url)));
  console.log(JSON.stringify(prepareComponentIndex({
    component,
    previous: fs.readFileSync(previous),
    previousV03: previousV03Path ? fs.readFileSync(previousV03Path) : undefined,
    sequence: Number(sequence),
    removeVersions: versions.split(","),
    output: path.resolve(output),
    privateKey: fs.readFileSync(keyFile),
    publicKey: trust.publicKey,
  })));
}
