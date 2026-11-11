import { createPrivateKey, createPublicKey, sign, verify } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const MAX_GUIDE_CONTENT_BYTES = 1_048_576;
const GUIDE_CONTENT_IDS = new Set([
  "pixiv-account",
  "booth-prices",
  "paying-abroad",
  "buying",
  "terms-of-use",
  "why-licensed",
]);
const ALLOWED_HOSTS = new Set([
  "ph-r.com",
  "pixiv.net",
  "www.pixiv.net",
  "accounts.pixiv.net",
  "www.help.pixiv.net",
  "booth.pm",
  "accounts.booth.pm",
  "manage.booth.pm",
  "paypal.com",
  "www.paypal.com",
  "vrchat.com",
  "hello.vrchat.com",
  "docs.vrchat.com",
  "creators.vrchat.com",
  "help.vrchat.com",
  "wiki.vrchat.com",
  "jinxxy.com",
  "gumroad.com",
  "payhip.com",
  "help.payhip.com",
  "store.steampowered.com",
  "help.steampowered.com",
  "unity.com",
  "login.unity.com",
]);

function spkiPublicKey(hex) {
  if (typeof hex !== "string" || !/^[a-f0-9]{64}$/u.test(hex)) {
    throw new Error("invalid_guide_trust_key");
  }
  return createPublicKey({
    key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), Buffer.from(hex, "hex")]),
    type: "spki",
    format: "der",
  });
}

function loadPrivateKey(rawInput) {
  if (rawInput && typeof rawInput === "object" && rawInput.type === "private") {
    return rawInput;
  }
  const trimmed = String(rawInput).trim();
  if (/^[a-f0-9]{64}$/iu.test(trimmed)) {
    return createPrivateKey({
      key: Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), Buffer.from(trimmed.toLowerCase(), "hex")]),
      type: "pkcs8",
      format: "der",
    });
  }
  return createPrivateKey(trimmed);
}

function allowedHttpsUrl(url) {
  try {
    if (typeof url !== "string" || /\p{Cs}/u.test(url) || url.length > 2048) return false;
    const parsed = new URL(url);
    return parsed.protocol === "https:"
      && parsed.username === ""
      && parsed.password === ""
      && ALLOWED_HOSTS.has(parsed.hostname.toLowerCase());
  } catch {
    return false;
  }
}

export function assertValidGuidePayload(payload) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    throw new Error("invalid_guide_payload");
  }
  if (payload.schemaVersion !== "0.1" || !GUIDE_CONTENT_IDS.has(payload.id)) {
    throw new Error("invalid_guide_identity");
  }
  if (!Number.isSafeInteger(payload.sequence) || payload.sequence < 1) {
    throw new Error("invalid_guide_sequence");
  }
  const semverRe = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[A-Za-z0-9.-]+)?(?:\+[A-Za-z0-9.-]+)?$/u;
  if (!semverRe.test(payload.version) || !semverRe.test(payload.minClient)) {
    throw new Error("invalid_guide_semver");
  }
  if (!payload.sop || typeof payload.sop !== "object" || !Array.isArray(payload.sop.steps) || payload.sop.steps.length < 1) {
    throw new Error("invalid_guide_sop");
  }
  for (const step of payload.sop.steps) {
    for (const link of step.links ?? []) {
      if (!allowedHttpsUrl(link?.url)) {
        throw new Error(`disallowed_guide_link:${String(link?.url)}`);
      }
    }
  }
  if (!Array.isArray(payload.sop.sources) || payload.sop.sources.length < 1) {
    throw new Error("missing_guide_sources");
  }
  for (const src of payload.sop.sources) {
    if (!allowedHttpsUrl(src?.url)) {
      throw new Error(`disallowed_guide_source:${String(src?.url)}`);
    }
  }
  if (!payload.assets || typeof payload.assets !== "object" || Array.isArray(payload.assets)) {
    throw new Error("invalid_guide_assets");
  }
  for (const asset of Object.values(payload.assets)) {
    for (const source of asset?.sources ?? []) {
      if (!allowedHttpsUrl(source)) {
        throw new Error(`disallowed_guide_asset_source:${String(source)}`);
      }
    }
  }
  return payload;
}

export function sealGuideEnvelope(payload, privateKeyInput, trustedPublicKeyHex) {
  assertValidGuidePayload(payload);
  const key = loadPrivateKey(privateKeyInput);
  const derivedHex = createPublicKey(key).export({ type: "spki", format: "der" }).subarray(-32).toString("hex");
  if (trustedPublicKeyHex && derivedHex !== trustedPublicKeyHex) {
    throw new Error("signing_key_differs_from_trust_anchor");
  }
  const payloadText = JSON.stringify(payload);
  const signature = sign(null, Buffer.from(payloadText, "utf8"), key).toString("hex");
  const envelopeText = `${JSON.stringify({
    schemaVersion: "0.1",
    algorithm: "ed25519",
    payload: payloadText,
    signature,
  })}\n`;
  const bytes = Buffer.from(envelopeText, "utf8");
  if (bytes.byteLength > MAX_GUIDE_CONTENT_BYTES) {
    throw new Error("guide_envelope_too_large");
  }
  return bytes;
}

export function verifyGuideEnvelopeBytes(rawBytes, trustedPublicKeyHex, expectedId) {
  if (!Buffer.isBuffer(rawBytes) || rawBytes.byteLength === 0 || rawBytes.byteLength > MAX_GUIDE_CONTENT_BYTES) {
    throw new Error("guide_envelope_too_large");
  }
  const envelope = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(rawBytes));
  if (
    !envelope || typeof envelope !== "object" || Array.isArray(envelope)
    || envelope.schemaVersion !== "0.1"
    || envelope.algorithm !== "ed25519"
    || typeof envelope.payload !== "string"
    || typeof envelope.signature !== "string"
    || !/^[a-f0-9]{128}$/u.test(envelope.signature)
  ) {
    throw new Error("invalid_guide_envelope");
  }
  const pub = spkiPublicKey(trustedPublicKeyHex);
  if (!verify(null, Buffer.from(envelope.payload, "utf8"), pub, Buffer.from(envelope.signature, "hex"))) {
    throw new Error("invalid_guide_signature");
  }
  const payload = assertValidGuidePayload(JSON.parse(envelope.payload));
  if (expectedId && payload.id !== expectedId) {
    throw new Error("wrong_guide_id");
  }
  return payload;
}

function writeAtomic(targetPath, bytes) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  const pending = `${targetPath}.${process.pid}.${Date.now()}.pending`;
  fs.writeFileSync(pending, bytes);
  fs.renameSync(pending, targetPath);
}

function parseCliArgs(argv) {
  const args = new Map();
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    if (!token.startsWith("--")) continue;
    const next = argv[i + 1];
    if (next && !next.startsWith("--")) {
      args.set(token.slice(2), next);
      i += 1;
    } else {
      args.set(token.slice(2), "true");
    }
  }
  return args;
}

async function runCli() {
  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const defaultTrustPath = path.resolve(scriptDir, "../distribution/trust.json");
  const args = parseCliArgs(process.argv.slice(2));
  const trustPath = args.get("trust-file") ? path.resolve(args.get("trust-file")) : defaultTrustPath;
  const trust = JSON.parse(fs.readFileSync(trustPath, "utf8"));

  if (args.get("verify-only") === "true") {
    const targetDir = path.resolve(args.get("dir") ?? ".");
    const entries = fs.readdirSync(targetDir).filter((name) => name.endsWith(".json"));
    if (entries.length === 0) throw new Error("no_guide_envelopes_found");
    for (const file of entries) {
      const id = file.slice(0, -".json".length);
      verifyGuideEnvelopeBytes(fs.readFileSync(path.join(targetDir, file)), trust.publicKey, id);
    }
    process.stdout.write(`${JSON.stringify({ verified: entries.length, dir: targetDir })}\n`);
    return;
  }

  const inputPath = args.get("in");
  const outDir = args.get("out-dir");
  if (!inputPath || !outDir) {
    throw new Error("usage: prepare-guide-content.mjs --in <payload.json|dir> --out-dir <dir> [--signing-key-file <pem>|--verify-only --dir <dir>]");
  }
  const keyRaw = args.get("signing-key-file")
    ? fs.readFileSync(path.resolve(args.get("signing-key-file")), "utf8")
    : process.env.VUA_RELEASE_PRIVATE_KEY;
  if (!keyRaw) {
    throw new Error("missing_signing_key");
  }
  const resolvedIn = path.resolve(inputPath);
  const resolvedOut = path.resolve(outDir);
  const inputs = fs.statSync(resolvedIn).isDirectory()
    ? fs.readdirSync(resolvedIn).filter((name) => name.endsWith(".json")).map((name) => path.join(resolvedIn, name))
    : [resolvedIn];

  const sealed = [];
  for (const file of inputs) {
    const payload = JSON.parse(fs.readFileSync(file, "utf8"));
    const bytes = sealGuideEnvelope(payload, keyRaw, trust.publicKey);
    verifyGuideEnvelopeBytes(bytes, trust.publicKey, payload.id);
    writeAtomic(path.join(resolvedOut, `${payload.id}.json`), bytes);
    sealed.push({ id: payload.id, sequence: payload.sequence, version: payload.version, bytes: bytes.byteLength });
  }
  process.stdout.write(`${JSON.stringify({ sealed, outDir: resolvedOut })}\n`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
  runCli().catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exit(1);
  });
}
