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
| Poiyomi Toon Shader | `com.poiyomi.toon` | https://poiyomi.github.io/vpm/index.json | [MIT](https://github.com/poiyomi/PoiyomiToonShader/blob/master/LICENSE.txt) | 2026-10-18 | `auto` | 36 shader GUIDs read from `_PoiyomiShaders/Shaders/**.shader.meta` in the official 9.3.64 package |
| Poiyomi Pro | `com.poiyomi.pro` | https://poiyomi.github.io/vpm/index.json | Proprietary/EULA ([official site](https://www.poiyomi.com/)) | 2026-10-18 | `guide` | No material signal yet; reading the Pro shader GUIDs needs a licensed copy |
| UnlitWF Shader | `jp.whiteflare.unlitwf` | https://whiteflare.github.io/vpm-repos/vpm.json | [Zlib](https://github.com/whiteflare/Unlit_WF_ShaderSuite/blob/master/LICENSE) | 2026-10-18 | `auto` | 84 shader GUIDs read from `Runtime/Shaders/**.shader.meta` in the official 2.9.0 package |
| VRChat Toon Standard | `com.vrchat.base` | https://vrchat.github.io/packages/index.json | [VRChat SDK license](https://hello.vrchat.com/legal/sdk) | 2026-10-18 | `bundled` | `ToonStandard.shader` and `ToonStandardOutline.shader` GUIDs read from the official `com.vrchat.base` 3.10.5 package; the shader ships with SDK 3.8.1 and later |
| VRChat SDK - Avatars | `com.vrchat.avatars` | https://vrchat.github.io/packages/index.json | [VRChat SDK license](https://hello.vrchat.com/legal/sdk) | 2026-10-18 | `auto` | Installed during project creation, before any material signal is needed |
| Modular Avatar | `nadena.dev.modular-avatar` | https://vpm.nadena.dev/vpm.json | [MIT](https://github.com/bdunderscore/modular-avatar/blob/main/COPYING.md) | 2026-10-18 | `auto` | Installed during project creation, before any material signal is needed |
| Non-Destructive Modular Framework | `nadena.dev.ndmf` | https://vpm.nadena.dev/vpm.json | [MIT](https://github.com/bdunderscore/ndmf/blob/main/COPYING.md) | 2026-10-18 | `auto` | Arrives as Modular Avatar's dependency, before any material signal is needed |

Marker properties win over the GUID table when a material is classified, so lilToon keeps its
historical rule. GUIDs are compared in lower case.

## Delivery policies

- `auto`: both preparation (`import_sources`) and a line's material change (`replace_sources`)
  install the newest release that resolves from the official feed when a selected material
  needs the package, none of the selected sources provides that shader, the project's recorded
  `packageVersions` does not carry the package and no Recipe pin fixes it. Installation uses
  the existing digest-bound preview/apply path and reports progress under the package's display
  name; conflicts still need explicit handling, and a failed installation fails the step as
  before. Recipe-pinned known packages have their feeds registered during project creation.
- `guide`: preparation installs nothing. After Unity reports `facts.missingShaderMaterials`,
  each recognized `guide` package — and each recognized `auto` package still absent from the
  project's VPM lock — adds one `amf.package.guide` warning with the package name, the number
  of affected materials and the official page.
- `bundled`: the material is classified and nothing is installed; the SDK provides the shader.

## Legacy copies shipped inside materials

A material package often carries an older copy of the shader itself. Preparation skips those
files for `auto` packages with detection signals and lets the official package take their
place. The skip list comes from the feed cache's `legacyFolders`/`legacyFiles`, joined across
the cached versions and read without network access; when the cache has no declaration, the
table's fallback applies (`Assets\lilToon`, `Assets\_PoiyomiShaders`,
`Assets\Unlit_WF_ShaderSuite`). Files of `guide`, `bundled` and signal-less entries import
unchanged.

## Adding an entry

1. Read the upstream license text and record its SPDX identifier (or `Proprietary/EULA`), the
   source URL and the check date. Choose `auto` only when the license permits delivery from the
   official feed; otherwise choose `guide`, or `bundled` for content the VRChat SDK provides.
2. Add detection signals from official files: a material property only this package writes, or
   the lower-case GUIDs of its shader `.meta` files. Download free official packages only into
   a separate temporary directory to read `package.json` and `.meta` files, and keep package
   bodies out of the repository. Note the package version and paths the signals came from.
3. Copy the feed's `legacyFolders`/`legacyFiles` paths into the fallback fields so the skip
   rule also works before the first feed refresh.
4. Extend the table in `known_packages.rs` and cover the entry with synthetic fixture
   materials and fixture feed manifests; tests never access the network.
5. Update this table and the [AMF production protocol](../protocols/amf-production-v0.1.md).
