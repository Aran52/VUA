import fs from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";
import {digest, sealEnvelope, verifyIndex} from "../distribution/component-index.mjs";

/** Signed removal-only revision. The wire contract remains the frozen v0.2. */
export function prepareComponentIndex({component, previous, sequence, removeVersions, output, privateKey, publicKey}) {
  const before = verifyIndex(previous, publicKey, component);
  if (!Number.isSafeInteger(sequence) || sequence <= before.sequence) throw Error("publication_sequence_mismatch");
  if (!Array.isArray(removeVersions) || !removeVersions.length || removeVersions.some(v => !before.releases.some(item => item.version === v))) throw Error("invalid_removed_versions");
  const index = {...before, sequence, releases: before.releases.filter(item => !removeVersions.includes(item.version))};
  const raw = sealEnvelope(index, "0.2", privateKey, publicKey);
  const directory = path.join(output, "updates/beta"); fs.mkdirSync(directory, {recursive: true});
  fs.writeFileSync(path.join(directory, `${component}.json`), raw, {flag: "wx"});
  const record = {schemaVersion: "0.2", kind: "index", component, previousIndexSha256: digest(previous), indexSha256: digest(raw)};
  fs.writeFileSync(path.join(output, "publication.json"), JSON.stringify(record, null, 2)+"\n", {flag: "wx"});
  return record;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [component, previous, sequence, versions, output, keyFile] = process.argv.slice(2);
  if (!keyFile) throw Error("Usage: prepare-component-index.mjs <host|amf> <previous.json> <sequence> <comma-separated-versions> <new-output> <private-key-file>");
  const trust = JSON.parse(fs.readFileSync(new URL("../distribution/trust.json", import.meta.url)));
  console.log(JSON.stringify(prepareComponentIndex({component, previous: fs.readFileSync(previous), sequence: Number(sequence), removeVersions: versions.split(","), output: path.resolve(output), privateKey: fs.readFileSync(keyFile), publicKey: trust.publicKey})));
}
