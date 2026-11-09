# Windows component updates v0.2

> Document version: 0.3
> Status: Candidate
> Updated: 2026-11-09
> Scope: independent first-party update indexes and the additive Desktop v5 update face

The [distribution architecture](../architecture/windows-distribution.md) owns program/data
separation. The [U sequence](../development/windows-distribution-plan.md#9-independent-component-update-sequence)
owns implementation and acceptance. This contract adds independent discovery without changing
the existing [v0.1 signed inventories and installed selection](windows-distribution-v0.1.md).

## Signed channels

The native client fixes two HTTPS endpoints:

| Component | Beta index |
| --- | --- |
| Host | `https://ph-r.com/vua/updates/beta/host.json` |
| AMF | `https://ph-r.com/vua/updates/beta/amf.json` |

An index uses the Ed25519 envelope with `schemaVersion: "0.2"`. Authenticate the exact UTF-8
`payload` bytes before parsing. The immutable trust anchor remains in the native helper;
publishing keys stay outside public hosting. The limit is 4 MiB, including the envelope.
Schemas and positive/negative vectors live in `schemas/windows-distribution/v0.2/`.

The payload binds VUA, Windows/x64, Beta, component identity and a positive monotonically
increasing `sequence`, bounded to the JavaScript safe-integer range. Each component keeps its
own last authenticated sequence in `updates-v02/<component>-index.json`. Existing cache records
remain readable. An equal sequence must retain the same payload; otherwise reject it as
`component_feed_equivocation`. Reject lower sequences and wrong component/channel/product
identities. Failed checks do not substitute cached metadata or claim the installed version is
current. GitHub may mirror signed immutable objects; it does not independently choose a newer
channel target outside the official index.

An index contains zero to 64 release entries. Each binds component SemVer/build, release page,
activation mode, compatibility declarations and a v0.1 signed inventory object's byte length,
SHA-256 and one to four HTTPS sources. Host entries additionally bind the runtime version and
require `restart`. AMF entries cannot select a runtime. Unsafe source credentials/fragments,
duplicate SemVer precedence, malformed versions/constraints, empty compatibility labels and
invalid data readership are rejected. Nonempty unsupported UI ABI or application labels make
that entry inapplicable, without rejecting the other entries. Unknown fields still reject the
index; the wire version remains `0.2`. Equal-precedence build metadata cannot introduce replacement bytes.

The v0.1 inventory is an immutable complete build record, not the channel. An AMF-only release
may carry an unchanged host/runtime baseline in that record while publishing a new AMF
component and index sequence. It does not rebuild, activate or increment the baseline host.
Legacy clients continue receiving the existing joint v0.1 endpoint during transition.

## Request identity

All requests through the native transfer client, including index, inventory, pack and GitHub
mirror requests, carry `VUA-distribution/0.2 (host=<version>; amf=<version>)`. Versions come from
the separately selected active Host and AMF inventories. The AMF value is its installed version,
including when the module is disabled. Each value must be a valid SemVer of at most 96 ASCII
characters. If either inventory cannot be read or authenticated, either version is invalid, or
there is no installation yet, use `VUA-distribution/0.2`. No installation or random identifier is
added, and no additional statistics request is sent.

## Selection and binding

Compare SemVer precedence, not strings. Keep the latest published version separate from the
latest compatible improvement. A newer incompatible AMF release must not hide an older
compatible update. AMF selection checks the active host range, UI ABI, exact React version,
application/Bridge protocols and ability to read the current BDL writer. Host selection checks
that the installed AMF remains compatible; a combined upgrade requires a separately presented
compatible transaction rather than silently changing the other component.

Unsupported UI ABI or application entries may supply `latest`, but cannot supply a single
component target or either side of a combined update. A matching pair of unsupported entries is
still inapplicable. The publication tools retain their stricter ABI/application checks while
installed 1.0.x clients still reject whole indexes containing such entries. A future ABI migration
must first deliver a client that can skip unsupported entries; this client change does not
authorize publishing them to the existing endpoints.

Before staging, fetch the referenced inventory within its length limit, match exact length and
SHA-256, verify its own v0.1 signature/semantics and bind the component version, build,
compatibility declarations and host runtime back to the index. Index authentication never
replaces inventory or installed-file validation. All mirror sources provide identical bytes.

AMF may declare `reload`. A BDL writer change forces `restart` even when a publisher requested
reload; migration and rollback admission still apply before activation. No update replaces live
production work, performs a Unity-project migration or changes the saved module enable choice.

## Withdrawal after staging

After a component check authenticates and accepts an index, compare a pending update's inventory
digest with every retained release. If it is absent, clear that component's pending intent and
return `state: "idle"` with `error: "update_withdrawn"`. That check does not fetch a replacement
target; a subsequent check may offer another compatible release. Retain downloaded
immutable files and inventories for reuse. A retained target leaves pending intent unchanged,
subject to the existing active-selection and recovered-activation checks.

Do not change pending intent while an activation journal is `prepared` or `verifying`. Reconcile
under the same operation and component-stage locks used by activation/staging, and reject a
changed active selection. Withdrawal never downgrades an already activated component; publishing
a higher compatible version remains the repair path.

## Desktop API

`DesktopComponentUpdatesApiV02` is an optional additive `VuaDesktopApiV5.componentUpdates` face;
Desktop v1–v4 and the existing `distribution` face remain available. The renderer supplies only
`host`/`amf` and, for staging, the checked opaque inventory digest. It cannot choose a feed URL,
program path, executable, signing key or module URL.

Each component has a separate snapshot, checking/downloading/cancellation state, installed
version/build, check time, latest published version, compatible target, actual transfer progress,
rollback availability and error. Targets name the effective restart/reload mode and separate
changed, reused and runtime byte counts. States are `idle`, `checking`, `downloading`, `staged`,
`waiting-for-idle`, `activating`, `verifying`, `interrupted` and `failed`. Activation and recovery
remain explicit; an interrupted operation is not replayed on ordinary startup.

## Document changelog

- 0.3 (2026-11-09): specify installed-version User-Agent fallback, staged-update withdrawal and
  skipping unsupported ABI/application entries, including combined updates; preserve strict
  publication policy for older clients and existing authenticated cache records.
- 0.2 (2026-10-26): add separate authenticated component channels, compatibility-aware selection,
  inventory binding and the typed independent Desktop update face; retain v0.1 inventories.
