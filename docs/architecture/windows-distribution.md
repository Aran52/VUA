# Component-based Windows distribution

> Document version: 1.0.0
> Status: Implementation baseline
> Updated: 2026-10-13
> Scope: First-party Windows x64 installation, launch and AMF updates

The [product boundary](../product-boundary.md#windows-delivery-and-amf-updates-user-ruling-2026-10-11)
owns scope. [N7](../development-outline.md#n7-beta-installer-regression-and-illustrated-user-guide)
owns product acceptance. This implementation is a first-party distribution boundary; it does
not introduce an arbitrary plugin loader or change AMF/BDL data ownership.

## Delivery and identity

NSIS defaults to a per-user installation in `%LOCALAPPDATA%\Programs\VUA`. The small installer
contains the host and a compatible AMF baseline. The offline installer additionally contains
the exact Electron archive. Both use the same signed inventory and native Rust helper, with
the Microsoft C runtime statically linked. Node.js, pnpm, a source checkout and administrator
rights are not required for this installation. No automatic reboot is performed.

| Owned program path | Contents |
| --- | --- |
| `VUA.exe` | Native installer/update/launch helper |
| `runtime/<archive-sha256>/` | Complete pinned upstream Electron distribution and notices |
| `payloads/host/<pack-sha256>/` | Main/preload, host renderer, host Provider, bootstrap and notices |
| `payloads/amf/<pack-sha256>/` | AMF renderer entry/assets, AMF Provider and notices |
| `manifests/<envelope-sha256>.json` | Verified immutable signed release metadata |
| `active.json`, `previous.json`, `pending.json` | Versioned selection and explicit update recovery state |
| `cache/` | Verified objects and inspectable incomplete acquisition |

User data remains in the existing [module-owned profile layout](modules.md#data-and-migration),
outside program payloads. Installation does not modify the AMF enable flag, material files,
projects, Recipes or BOOTH profile. Uninstall removes owned program files and retains profiles.
Unknown populated target directories and reparse/link destinations are rejected.

Electron starts without an external application argument, using a small packaged
`resources/app.asar` bootstrap. `VUA.Runtime.exe` is a byte-identical copy of the verified
upstream `electron.exe`; the original remains in the runtime inventory. Its product filename
allows Electron's real `app.isPackaged` classification. The bootstrap verifies signed host/module
inventories and establishes a frozen Main-only installation context. It never selects code from
an environment variable. The launcher clears Electron/Node/development overrides. Existing
production guards, sandboxed preload, Renderer isolation and navigation policies remain active.
The bootstrap and launch alias are derived owned files, not unreported changes to upstream bytes.

## Mirrors and trust

The inventory includes npmMirror, Huawei Cloud and the official Electron GitHub release. The two
China mirrors and official fallback must provide the same exact-version archive. Bounded parallel
range probes test that object, with a five-second per-source budget and at most four candidates;
usable sources are ordered by observed response. There is no region inference or accelerator
recommendation. An unavailable source falls back to another source. Partial responses must prove
the expected range/total; full SHA-256 verification is required before extraction or reuse.

The expected archive checksum comes from the pinned official Electron npm package, not from the
mirror. Extraction accepts only the signed complete inventory, safe relative names and expected
lengths; every installed file is checked. A matching version directory alone is insufficient.
When a complete runtime is retained, host/AMF installation downloads no runtime object. An
installed Chrome browser or another application's private Electron is never used as VUA's runtime.

Ed25519 authenticates the exact UTF-8 manifest payload; SHA-256 identifies complete inventories
and file objects. The public trust anchor ships in the helper/bootstrap, while the publishing key
remains private and outside the repository. Manifest authentication is distinct from Windows
Authenticode: the current VUA helper/NSIS artifacts remain unsigned Windows executables.
[Windows distribution v0.1](../protocols/windows-distribution-v0.1.md) owns metadata and limits.

## Independent AMF payload and updates

The AMF UI is a separate ESM build with its own version, assets and native executable. It binds
the host's exact React/ReactDOM instance, Gateway context, task identities, modal ownership and
browser shell state through `amf-ui/0.1`. Local `vua-module://amf/ui/` requests are restricted to
the active signed inventory and verified again when served. Remote views cannot supply entries.
Legacy ZIP and development builds retain their source-based lazy entry.

Settings → Modules and About expose version/check/download/restart/recovery through the typed
Desktop v3 distribution face. The installed build checks the signed public release asset directly;
it does not require a GitHub API request. Host upgrades use NSIS; an AMF update does not select a
host or runtime upgrade. Host range, UI ABI, React, application/Bridge and declared data readership
must remain compatible. The initial AMF version is 0.1.0 and writes the existing BDL 0.10 format.

Component packs concatenate the complete file set with signed offsets, lengths and per-file
digests. The updater verifies and copies unchanged files, requesting only changed/missing byte
ranges. A server that ignores ranges triggers an explicitly reported full-pack fallback, followed
by full-pack and per-file verification. This is file-level incremental delivery, not binary patching
inside changed executables. Local verified full-pack input is also available to the native recovery
command; it does not bypass signature or complete-target checks.

Staging records `interrupted` before acquisition and becomes `staged` only after the complete
target is verified. Ordinary startup never resumes it or activates it automatically. The user can
explicitly continue downloading or restart to apply. Main uses task/mutation/browser/download
admission and the normal safe shutdown boundary; an update never forces unfinished work to stop.
Native operation/running file locks protect installation, activation and removal. The previous
immutable host/module pair is retained; rollback checks compatibility with the current declared
data write format. Initial updates do not migrate BDL formats. A future format change still requires
the owning migration/backup evidence; an incompatible old reader is not opened as a repair shortcut.
Only signed, unreferenced owned payload directories are eligible for cleanup; active/previous/pending
versions remain. A cleanup failure retains files and does not undo a successful activation.

## Sources, verification and remaining evidence

Publication currently targets GitHub under the author's ruling. PH-R.COM waits for deployment
details. Runtime acquisition has two tested China alternatives; AMF feed/object availability when
GitHub is blocked remains pending until an alternative feed/object host is deployed. Online and
offline installers do not need that feed to complete initial installation.

Controlled local Windows evidence covers both NSIS installers, startup with real providers and
the separate AMF page, unchanged-runtime reuse, uninstall/profile retention, two named compatible
AMF builds on one unchanged host and compatible rollback. Synthetic loopback tests measure
changed-only file range requests, full-package fallback, interruption, digest/signature/path rejection
and busy admission. These are not clean-machine, physical PICO/VRChat, real-material or four-language
human acceptance. The [delivery plan](../development/windows-distribution-plan.md) retains those gaps.

## Document changelog

- 1.0.0 (2026-10-13): describe NSIS, complete mirrored runtime identity, independent AMF UI/native payloads, signed file updates, activation/rollback and controlled evidence limits.
