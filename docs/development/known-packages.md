# Known packages

> Updated: 2026-10-19

AMF production resolves the shaders its selected materials need through one known-package
table. The table lives in `crates/provider-host/src/known_packages.rs`; this document records
the evidence behind each entry. The wire behavior and the `amf.package.guide` diagnostic are
owned by the [AMF production protocol](../protocols/amf-production-v0.1.md).

## Table

| Package | ID | Official VPM feed | License (source) | Checked | Delivery | Detection signals |
| --- | --- | --- | --- | --- | --- | --- |
| lilToon | `jp.lilxyzw.liltoon` | https://lilxyzw.github.io/vpm-repos/vpm.json | [MIT](https://github.com/lilxyzw/lilToon/blob/master/LICENSE) | 2026-10-18 | `auto` | `_lilToonVersion` material property |
| Poiyomi Toon Shader | `com.poiyomi.toon` | https://poiyomi.github.io/vpm/index.json | [MIT](https://github.com/poiyomi/PoiyomiToonShader/blob/master/LICENSE.txt) | 2026-10-18 | `auto` | 35 shader GUIDs grouped by version line; see the table below |
| Poiyomi Pro | `com.poiyomi.pro` | https://poiyomi.github.io/vpm/index.json | Proprietary/EULA ([official site](https://www.poiyomi.com/)) | 2026-10-18 | `guide` | No material signal yet; reading the Pro shader GUIDs needs a licensed copy |
| UnlitWF Shader | `jp.whiteflare.unlitwf` | https://whiteflare.github.io/vpm-repos/vpm.json | [Zlib](https://github.com/whiteflare/Unlit_WF_ShaderSuite/blob/master/LICENSE) | 2026-10-18 | `auto` | 84 shader GUIDs read from `Runtime/Shaders/**.shader.meta` in 2.9.0; the newest 2.14.0 release carries exactly the same 84 GUIDs (checked 2026-10-19) |
| VRChat Toon Standard | `com.vrchat.base` | https://vrchat.github.io/packages/index.json | [VRChat SDK license](https://hello.vrchat.com/legal/sdk) | 2026-10-18 | `bundled` | `ToonStandard.shader` and `ToonStandardOutline.shader` GUIDs read from the official `com.vrchat.base` 3.10.5 package; the shader ships with SDK 3.8.1 and later |
| VRChat SDK - Avatars | `com.vrchat.avatars` | https://vrchat.github.io/packages/index.json | [VRChat SDK license](https://hello.vrchat.com/legal/sdk) | 2026-10-18 | `auto` | Installed during project creation, before any material signal is needed |
| Modular Avatar | `nadena.dev.modular-avatar` | https://vpm.nadena.dev/vpm.json | [MIT](https://github.com/bdunderscore/modular-avatar/blob/main/COPYING.md) | 2026-10-18 | `auto` | Installed during project creation, before any material signal is needed |
| Non-Destructive Modular Framework | `nadena.dev.ndmf` | https://vpm.nadena.dev/vpm.json | [MIT](https://github.com/bdunderscore/ndmf/blob/main/COPYING.md) | 2026-10-18 | `auto` | Arrives as Modular Avatar's dependency, before any material signal is needed |

Marker properties win over the GUID table when a material is classified, so lilToon keeps its
historical rule. GUIDs are compared in lower case.

## Poiyomi version lines

Poiyomi keeps its shaders under `_PoiyomiShaders/Shaders/<line>/`, and every line uses
different GUIDs. Each line's GUIDs were read from that line's newest release package, only
from its own `_PoiyomiShaders/Shaders/<line>/**.shader.meta`. No GUID is shared between two
lines. A GUID shared by lines in a future table belongs to the newest line that carries it.

| Version line | Newest release read | Shader GUIDs |
| --- | --- | --- |
| 8.1 | 8.1.167 | 6 |
| 9.0 | 9.0.61 | 5 |
| 9.1 | 9.1.30 | 5 |
| 9.2 | 9.2.79 | 5 |
| 9.3 | 9.3.64 | 7 |
| 10.0 | 10.0.25 | 7 |

When every material that needs Poiyomi points at one line and neither the selected sources
nor the project provides it, preparation reads the feed cache and installs that line's newest
stable release with an exact `PackageRequestV1.version` pin. Materials spanning several
lines, or a project already carrying the package, get no install; the `amf.package.guide`
warning names the required lines. lilToon and UnlitWF need no lines: lilToon's GUIDs are
version-stable, and UnlitWF 2.14.0 covers every GUID registered from 2.9.0.

## Delivery policies

- `auto`: both preparation (`import_sources`) and a line's material change (`replace_sources`)
  install from the official feed when a selected material needs the package, none of the
  selected sources provides that shader, the project's recorded `packageVersions` does not
  carry the package and no Recipe pin fixes it. Version-lined packages install their line's
  pinned release; the others install the newest resolving release. Installation uses the
  existing digest-bound preview/apply path and reports progress under the package's display
  name; conflicts still need explicit handling, and a failed installation fails the step as
  before. Recipe-pinned known packages have their feeds registered during project creation.
- `guide`: preparation installs nothing. After Unity reports `facts.missingShaderMaterials`,
  each recognized `guide` package — and each recognized `auto` package whose needed version
  lines the project does not carry — adds one `amf.package.guide` warning with the package
  name, the number of affected materials, the needed version lines and the official page.
- `bundled`: the material is classified and nothing is installed; the SDK provides the shader.

## Legacy copies shipped inside materials

A material package often carries an older copy of the shader itself. The candidate locations
come from the feed cache's `legacyFolders`/`legacyFiles`, joined across the cached versions
and read without network access; when the cache has no declaration, the table's fallback
applies (`Assets\lilToon`, `Assets\_PoiyomiShaders`, `Assets\Unlit_WF_ShaderSuite`). Files of
`guide`, `bundled` and signal-less entries import unchanged.

lilToon keeps its historical rule: a bundled copy always leaves for the official package. For
the other `auto` packages, preparation reads every `.shader` GUID in the copy from the staged
`asset.meta` files and replaces the copy only when the exact official release it is about to
install covers all of them, with materials and selected sources on the same version line. The
copy then leaves for that release. In every other case — an unknown or cross-line GUID, a
Poiyomi Pro copy, an unreadable shader, or a project that already carries the package — the
copy imports unchanged and nothing is installed, exactly as before this table existed.

## Adding an entry

1. Read the upstream license text and record its SPDX identifier (or `Proprietary/EULA`), the
   source URL and the check date. Choose `auto` only when the license permits delivery from the
   official feed; otherwise choose `guide`, or `bundled` for content the VRChat SDK provides.
2. Add detection signals from official files: a material property only this package writes, or
   the lower-case GUIDs of its shader `.meta` files. For a package whose shaders differ by
   release line, read each line's newest release package and record the line with its GUIDs.
   Download free official packages only into a separate temporary directory to read
   `package.json` and `.meta` files, and keep package bodies out of the repository. Note the
   package versions and paths the signals came from.
3. Copy the feed's `legacyFolders`/`legacyFiles` paths into the fallback fields so the skip
   rule also works before the first feed refresh.
4. Extend the table in `known_packages.rs` and cover the entry with synthetic fixture
   materials and fixture feed manifests; tests never access the network.
5. Update this table and the [AMF production protocol](../protocols/amf-production-v0.1.md).
