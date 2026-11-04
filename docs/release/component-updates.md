# Independent component publication

> Updated: 2026-10-26
> Scope: build/export/publish tools for the independent Host and AMF Beta channels
> Status: implemented tooling; live deployment and update activation acceptance remain in U3–U6

The [U sequence](../development/windows-distribution-plan.md#9-independent-component-update-sequence)
owns delivery, and [distribution v0.2](../protocols/windows-distribution-v0.2.md) owns channel
semantics. The original [official installer hosting](official-hosting.md) procedure and v0.1
release assets remain available. Independent AMF publication does not rebuild or bump the host.

## Build one component

Run the `Independent component build` workflow with `host` or `amf`. It uses a disposable Windows
checkout, builds only the selected native executable(s) and renderer, generates fresh notices
and assembles `payload/` plus `component-build.json`. Signing keys are not part of this workflow
or its artifacts. A host payload includes its versioned native update helper under
`launcher/VUA.exe`, allowing future host updates to deliver helper changes as verified payload
files instead of overwriting a running root launcher.

Equivalent local commands, from a clean release checkout after the contract/provider build:

```powershell
node apps/desktop/scripts/build-windows-provider.mjs --component=amf
node --experimental-strip-types apps/desktop/scripts/build-components.mjs --component=amf
node apps/desktop/scripts/generate-release-notices.mjs --component=amf --output=apps/desktop/out/amf-notices
node apps/desktop/scripts/assemble-component.mjs amf apps/desktop/out/amf-build apps/desktop/out/amf-notices
```

For `host`, also run `node apps/desktop/scripts/build-electron.mjs`, choose `--component=host`
for native/renderer/notices and assemble with `host`. Destination and notices directories must
be new. The native artifact lookup respects `CARGO_TARGET_DIR`. Do not substitute a build from
another commit or alter a payload after its receipt is generated. A local dirty build is labelled
in its receipt and cannot be signed by the publication CLI.

## Prepare a signed publication privately

Select the component version under the [version policy](versioning.md), commit its source and
verify it. Obtain the exact signed baseline inventory and, when the channel exists, its current
signed index. Keep the existing publishing key on the publishing machine; the website never
receives it. `VUA_RELEASE_KEY` can select the existing private PEM file. Otherwise the tool uses
the established private publishing-key location.

```powershell
node apps/desktop/scripts/prepare-component-release.mjs amf C:/private/baseline/vua-release.json C:/private/amf-build/payload C:/private/amf-publication --previous=C:/private/amf-index.json
```

The CLI requires the clean source revision, matching build receipt and actual native AMF version,
source identity, BDL writer and signed UI metadata. The AMF executable reports these compile-time
facts with `--component-identity` before opening user data.
It verifies the baseline/index signatures against the shipped anchor before preparing anything.
It emits only the chosen component's pack, immutable signed v0.1 inventory, signed v0.2 channel
and deterministic public version page. Host/runtime records stay byte-for-byte equivalent for
an AMF-only update; AMF stays unchanged for a host-only update. A host runtime change requires
a verified baseline with the exact configured runtime inventory.

New component versions must advance from the selected baseline. Seeding a channel from an
already published version requires the same build and byte-identical pack, including its file
order. A new channel does not authorize repacking an existing version. Published index entries
remain immutable; retained history allows old compatible hosts to find their applicable AMF.
BDL writer changes advertise restart activation rather than hot replacement.

## Promote after upload verification

Upload the entire prepared directory into private server staging. Install the scoped locations
in `apps/desktop/distribution/ph-r-locations.conf` through the existing tested Nginx procedure.
They expose only public channel/object/version-page directories. Keep operation locks, staging
and ownership markers outside those aliases. Preserve the existing website and installer routes.
The Beta indexes require cache revalidation; immutable objects use long-lived caching, identity
encoding and ordinary byte-range delivery.

Export the operator tool with `node apps/desktop/scripts/export-component-publisher.mjs <new-directory>`
or use the source-labelled CI artifact. It carries the public trust anchor, exact `semver`
dependency and original licenses; it contains no signing key and needs no server-side package
installation. On the server, run it with Node.js 24 or later from private staging. This operator tool is not
an end-user dependency:

```text
node apps/desktop/scripts/publish-component-release.mjs /private/staging/amf-publication /var/www/vua
```

The publisher checks the prepared signature, inventory binding, pack digest and deterministic
page. It rejects unowned populated update/component trees. A per-channel lease serializes
publication. The exact previous index digest must still match the server, preventing stale
uploads from overwriting a concurrent publication. Complete flushed objects become visible
through atomic immutable links before the latest index is replaced and flushed. The other
component's index and all original product release directories remain untouched.

A successful retry verifies existing objects and can restore missing identical objects. A
conflicting immutable object or stale operation lock requires inspection; do not delete active
locks or overwrite a published version to make a failed run appear successful. A crash before
index promotion leaves clients on the previous authenticated target. Verify the public index,
manifest and an exact 206 range before recording live delivery evidence.

## Verification and remaining gates

Shared vectors exercise producer and native signature/identity/compatibility parsing. Synthetic
publication tests cover host-only/AMF-only preservation, retained versions, stale publication,
corrupt objects/indexes/pages, same-version immutability, build receipts, destination ownership,
links and idempotent retries. These tests publish only into isolated temporary roots.

The new official routes and channel contents are not declared deployed by the tooling tests.
The [AMF runtime handshake](../protocols/amf-ui-runtime-v0.1.md) implements generation-bound
provider/UI replacement and safe refusal for busy or older modules. It does not pass a delivery
gate by itself. [AMF data recovery](../protocols/amf-data-recovery-v0.1.md) snapshots only the
module's databases, migrates offline and fences user work until exact provider/UI health commits.
Writer-changing updates require compatible root, host and module recovery capabilities; older
combinations fail closed before activation and require the transition installer.

Actual signed test publication, blocked-GitHub transfer, process-preserving AMF activation,
host restart/recovery, transition installation and clean-Windows/human acceptance remain the
separate U3–U6 gates. No production version or release is created by this document.
