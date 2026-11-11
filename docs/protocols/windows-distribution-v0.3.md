# Windows component updates v0.3

> Document version: 0.1
> Status: Candidate
> Updated: 2026-11-11
> Scope: compressed per-file component update delivery with dual-channel v0.2 compatibility

The [distribution architecture](../architecture/windows-distribution.md) owns program/data
separation. [Windows component updates v0.2](windows-distribution-v0.2.md) owns independent
Host/AMF channel selection, compatibility rules, staged-update withdrawal and the Desktop v5
update face. This contract adds signed per-file compressed transfer manifests (`v0.3`) without
modifying the frozen `v0.2` wire format served to v1.0.x installations or the `v0.1` installed
inventory layout.

## Dual channels and anti-downgrade

To keep published `v0.2` endpoints free of unknown fields while migrating clients to compressed
per-file delivery, the native helper adds two `v0.3` HTTPS channel endpoints alongside `v0.2`:

| Component | v0.3 Beta index | v0.2 fallback Beta index |
| --- | --- | --- |
| Host | `https://ph-r.com/vua/updates/v0.3/beta/host.json` | `https://ph-r.com/vua/updates/beta/host.json` |
| AMF | `https://ph-r.com/vua/updates/v0.3/beta/amf.json` | `https://ph-r.com/vua/updates/beta/amf.json` |

A `v0.3` index uses the Ed25519 envelope with `schemaVersion: "0.3"` and payload `schemaVersion: "0.3"`.
Each component stores its last authenticated `v0.3` index in `updates-v03/<component>-index.json`.
Monotonic `sequence` enforcement spans both `updates-v02/<component>-index.json` and
`updates-v03/<component>-index.json` (`sequence >= max(cached_v02, cached_v03)`).

- Before `updates-v03/<component>-index.json` exists locally, an HTTP unavailability response on
  the `v0.3` endpoint (`feed_unavailable`, such as closed-by-default `403` before the first `v0.3`
  publication) falls back to the `v0.2` endpoint.
- Once `updates-v03/<component>-index.json` has been authenticated and persisted for a component,
  the client never falls back to `v0.2` or accepts a `0.2` index (`component_feed_downgraded`).
- Equal `sequence` numbers within the same schema version must carry identical payloads; otherwise
  reject as `component_feed_equivocation`.

Requests through the `v0.3`-capable transfer client carry
`VUA-distribution/0.3 (host=<version>; amf=<version>)` (or `VUA-distribution/0.3` when active
inventories are unavailable), under the same SemVer and privacy rules as `v0.2`.

## Compressed transfer manifest

Each release entry in a `v0.3` index retains all `v0.2` fields (`manifest` still references the
immutable `v0.1` inventory used for installed layout and rollback) and may declare an optional
`compressedTransfer` object (`{ sha256, bytes, sources }`, 1–4 HTTPS URLs, up to 4 MiB). Historical
entries carried forward from `v0.2` may omit `compressedTransfer` and continue using uncompressed
`v0.1` pack ranges.

`compressedTransfer` references a separately signed Ed25519 envelope (`schemaVersion: "0.3"`)
cached in `updates-v03/transfers/<sha256>.json`. Its payload binds:

- `schemaVersion: "0.3"`, `product: "VUA"`, `platform: "windows"`, `arch: "x64"`
- `component`: `"host"` or `"amf"`, matching the channel
- `manifestSha256`: exact 64-hex SHA-256 of the entry's signed `v0.1` inventory envelope
- `encoding`: `"zstd"` or `"gzip"`
- `pack`: `{ bytes, sha256, sources }` (1–4 HTTPS URLs for the concatenated compressed `.pack` object)
- `files`: ordered array matching the `v0.1` component inventory 1:1 in `path`, uncompressed
  `bytes` and uncompressed `sha256`, plus contiguous `compressedOffset`, `compressedBytes` and
  `compressedSha256`

Validation rejects any mismatch between `files` and the `v0.1` inventory, any gap or overlap in
`compressedOffset + compressedBytes`, non-empty compressed bytes for zero-byte files, zero
compressed bytes for non-empty files, or final offset differing from `pack.bytes`.

## Verification and bounded decompression

1. Reuse unchanged verified local files by uncompressed `bytes` and `sha256` first. Target
   `downloadBytes` sums `compressedBytes` across only un-reused files (plus runtime archive bytes
   when a Host update requires a missing runtime).
2. For each changed or missing non-empty file, request `Range: bytes=<compressedOffset>-<end>` with
   `Accept-Encoding: identity` from `pack.sources`. Verify exact `206 Partial Content` range bounds,
   `compressedBytes` and `compressedSha256` before decompressing.
3. Decompress in streaming chunks bounded to `bytes + 1`, aborting immediately if output exceeds the
   signed uncompressed `bytes` (`unexpected_decompressed_length`). Verify final uncompressed length
   and `sha256` before accepting the staged file.
4. If range requests are unavailable, fall back to a verified whole `.pack` download (extracting and
   verifying each file's compressed slice and decompressed output), and finally to the `v0.1`
   uncompressed `.bin` pack if `.pack` mirrors are unreachable. Temporary `.pack`/`.bin` cache files
   are removed once all destination files pass verification.

## Publication compatibility

`prepare-component-release.mjs` produces both `updates/beta/<component>.json` (`v0.2`) and
`updates/v0.3/beta/<component>.json` (`v0.3`) at the same `sequence`, together with `.bin`, `.pack`
(`zstd` level 19 by default) and both signed metadata objects under `updates/objects/`. Removal-only
index revisions (`prepare-component-index.mjs`) advance both channels at the same `sequence`.

## Document changelog

- 0.1 (2026-11-11): initial specification of `v0.3` dual-channel indexes, anti-downgrade caching and
  signed per-file compressed transfer manifests (`zstd`/`gzip`).
