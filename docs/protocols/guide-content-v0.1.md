# Guide Content Channel Contract (`guide-content` v0.1)

- Version: `0.1.0`
- Status: Candidate
- Owner: Desktop / Integration
- Updated: 2026-11-11

## 1. Purpose and Scope

`guide-content` v0.1 delivers signed, plain-text SOP updates for VUA's licensed-entry guide ([licensed-entry-guide.md](../development/licensed-entry-guide.md)) independently of desktop binary releases.

- Bundled v1 guide documents ship inside the desktop application (`apps/desktop/src/electron/bundled-guides.ts`) so the guide works offline and never waits on this channel.
- Electron Main (`apps/desktop/src/electron/guide-content.ts`) owns network retrieval, Ed25519 signature verification, monotonic sequence persistence, `minClient` gating, and fallback selection (`remote` -> `cached` -> `bundled`). That keeps portable ZIP builds, development checkouts, and NSIS installations on one identical verification boundary without requiring the native installer helper.
- The renderer passes only a closed `GuideContentIdV01` (`pixiv-account`, `booth-prices`, `paying-abroad`, `buying`, `terms-of-use`, `why-licensed`), never a URL.

## 2. Signed Envelope and Verification Order

Remote documents are served over HTTPS at:

- Production: `https://ph-r.com/vua/updates/guides/<id>.json`
- Candidate staging: `https://ph-r.com/vua/updates/guides-candidate/<id>.json`

Both locations set `expires -1`, `gzip off`, `autoindex off`, and `include /etc/nginx/snippets/vua-site-security.conf` (`apps/desktop/distribution/ph-r-locations.conf`).

### Envelope (`schemas/guide-content/v0.1/envelope.schema.json`)

```json
{
  "schemaVersion": "0.1",
  "algorithm": "ed25519",
  "payload": "<serialized UTF-8 JSON payload>",
  "signature": "<128 lowercase hex chars>"
}
```

Verification rules:

1. Enforce total response size `<= 1,048,576` bytes (`1 MiB`) both on `Content-Length` and while reading the response stream.
2. Parse the outer JSON envelope and confirm `schemaVersion === "0.1"`, `algorithm === "ed25519"`, non-empty string `payload`, and 64-byte hex `signature`.
3. Verify `signature` over the exact UTF-8 bytes of `envelope.payload` against the shipped Ed25519 public key in `apps/desktop/distribution/trust.json` **before** calling `JSON.parse(envelope.payload)`.
4. Parse `envelope.payload` and validate `GuideContentPayloadV01` (`schemas/guide-content/v0.1/payload.schema.json`), confirming `payload.id === requestedId`.
5. Unknown fields at every object level are ignored (`additionalProperties: true`). Incompatible structural changes increment `minClient` so older clients fall back cleanly to their cached or bundled document instead of failing closed on unknown keys.

## 3. Payload and SOP Shape

`GuideContentPayloadV01`:

- `schemaVersion`: `"0.1"`
- `id`: closed `GuideContentIdV01`
- `sequence`: monotonically increasing integer (`>= 1`) per `id`
- `version`: SemVer string for the SOP document revision
- `minClient`: minimum VUA desktop SemVer required to render this payload
- `sop`: structured SOP following [north-star.md](../north-star.md):
  - `title`: `GuideLocalizedTextV01` (`{ en, "zh-CN"?, "zh-TW"?, ja?, ko? }`; missing locales fall back to `en`)
  - `appliesTo`: `{ software: string[], devices: string[], region: string }`
  - `prerequisites`: `GuideLocalizedTextV01[]`
  - `steps`: non-empty list of `{ id, performer: "vua" | "vua_opens" | "player", action, expectedResult, verification, onFailure, sites, links }`
  - `regionalNotes`: `{ region, text, verification: { date, network, cardIssuingRegion } }[]`
  - `lastVerification`: `{ date, environment, softwareVersions }`
  - `sources`: non-empty `{ label, url }[]`
- `assets`: phase-2 lazy media map `{ [assetKey]: { sha256, bytes, sources } }` (downloaded only when a section is opened; empty `{}` in phase 1).

All `links[].url`, `sources[].url`, and `assets[*].sources[]` entries must use `https:` without credentials and belong to `GUIDE_ALLOWED_LINK_HOSTS` (`packages/contracts/src/guide-content.ts`). All text fields are rendered strictly as plain text, never HTML.

## 4. Client Lifecycle, High-Water State, and Fallback

- **Zero startup traffic**: instantiating `GuideContentService` performs no network or disk writes. Requests happen only when the player opens a guide topic and invokes `vua:guide-content:read` (`guideContent.read(id)`).
- **8-second timeout**: HTTPS requests use `redirect: "error"` and abort after `8,000 ms`.
- **High-water anti-replay & anti-equivocation**: `<userData>/guide-content/state.json` records `{ sequence, payloadSha256, checkedAt }` per `id`, alongside verified envelopes in `<userData>/guide-content/cache/<id>.json`:
  - `payload.sequence < highWater.sequence` -> rejected (`guide_sequence_replayed`).
  - `payload.sequence === highWater.sequence && payloadSha256 !== highWater.payloadSha256` -> rejected (`guide_sequence_equivocation`).
- **`minClient` gate**: if the running VUA version is lower than `payload.minClient`, the remote payload is not cached and high-water `sequence` is not advanced; the client returns the verified cached copy if compatible, or the bundled copy.
- **Fallback order**: `remote` -> verified `cached` -> `bundled`. `read(id)` returns `{ document, source: "bundled" | "cached" | "remote", checkedAt }` and never blocks guide rendering on network failure.

## 5. Candidate Staging and Source-Change Monitor

- **Signing & candidate verification**: `node apps/desktop/scripts/prepare-guide-content.mjs --in <payload.json|dir> --out-dir <staging-dir> --signing-key-file <key.pem>` seals and self-verifies envelopes against `apps/desktop/distribution/trust.json`; `--verify-only --dir <dir>` validates staged candidate envelopes before atomic directory promotion on `ph-r.com`.
- **Official source hash monitor**: `apps/desktop/scripts/check-guide-sources.mjs` and `docs/release/guide-source-hashes.json` track normalized plain-text SHA-256 hashes and check dates for the 8 starred upstream reference pages (VRChat, BOOTH, Jinxxy, Gumroad, Payhip). Webpage text is never stored in the repository. Upstream HTTP `401`/`403`/`429`/`5xx`, anti-bot challenges, or timeouts report `unreachable` rather than false-positive `changed`.
