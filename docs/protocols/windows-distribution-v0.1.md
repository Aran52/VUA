# Windows distribution v0.1

> Document version: 0.1
> Status: Candidate
> Updated: 2026-10-13
> Scope: Release, installed selection, pending update and Desktop distribution contracts

[Architecture](../architecture/windows-distribution.md) owns layout/lifecycle. Schemas and vectors
live in [schemas/windows-distribution/v0.1](../../schemas/windows-distribution/v0.1/).
This new face does not mutate application, Provider, Bridge, BDL or AMF-registration contracts.

`envelope.schema.json` wraps an exact JSON payload string and Ed25519 signature. Verify the
signature over its UTF-8 bytes before decoding `manifest.schema.json`; never reserialize before
verification. The shipped release key is the authority. Test vectors use the explicitly synthetic
seed `[7; 32]`, whose public key differs from the shipped anchor. Schema validity alone is not
signature or file-integrity evidence.

The payload identifies VUA, Windows/x64, release URL/version, runtime, host and AMF components,
their exact builds and complete files. A pack has byte length, SHA-256 and one to four HTTPS
sources for the same object. Component files have relative path, byte length, SHA-256 and
contiguous pack offset. Runtime files describe archive entries rather than concatenation ranges.
Bounds are 4 MiB envelope bytes, 2 GiB object bytes, 512 MiB per file and 20,000 files per inventory.
Credentials/fragments, reserved Windows names, traversal, links, duplicate case-insensitive ASCII
paths, missing required entry/notices, overlapping/gapped ranges, wrong versions or incompatible
ABI/protocol/data reader are semantic rejection cases in addition to schema validation.

`installed.schema.json` selects signed host and AMF envelope digests. `pending.schema.json`
records a signed target digest, `interrupted` or `staged`, and a distinct explicit activation choice.
An unfinished download cannot become active; ordinary launch does not replay a pending mutation.
Registration/enabled/data-layout state remains in the existing AMF registration contract.

`DesktopDistributionApiV01` is exposed by additive `VuaDesktopApiV3`; v1/v2 remain intact.
Renderer accesses it through the typed Gateway. It offers snapshot/check/stage/cancel/apply/restore
and subscriptions. Renderer supplies an opaque checked manifest digest, never an executable,
feed, filesystem destination, signing key or arbitrary module URI. Main validates local senders,
fixes native command/root and controls safe restart admission. Native verification remains
mandatory even after a successful check. Snapshot separates installed versions, target identity,
progress and recovery states; failed checking does not claim an old cached target is current.

## Document changelog

- 0.1 (2026-10-13): introduce signed complete component inventories, installed/pending pointers, first-party UI ABI and typed Desktop v3 update face with producer/consumer vectors.
