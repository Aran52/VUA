# Desktop-library v0.1: local library handoffs

> Document version: 1.2.0
> Status: Candidate
> Updated: 2026-10-12
> Scope: Local Electron preload/Main action; no Provider or task-frame change

For people: Asset library offers an explicit action to open its local material folder in
Windows File Explorer. Search, library source and download state remain the visible filters.

For Agents: `DesktopLibraryApiV01` is an independent optional `library` member of
`VuaDesktopApiV2`. The retained V1 aggregate, Gateway methods and material contracts are unchanged.
The [desktop architecture](../architecture/desktop.md) owns the AMF runtime/data-root resolution;
the [design standard](../design/design-standard.md#8-module-visual-language) owns presentation.

## Member and boundary

| Member | Input/output | Behavior |
| --- | --- | --- |
| `openFolder()` | No arguments → `Promise<{ opened: boolean }>` | Open the `warehouse` directory inside the active AMF data root through the system folder handler |
| `openBoothProduct(productId)` | `booth:<digits>` → `Promise<{ opened: boolean }>` | Open the canonical `https://booth.pm/items/<digits>` page in the system browser |
| `resolveDroppedFiles(files)` | Native `File[]` → `Promise<readonly string[]>` | Resolve explicit OS file/folder drops for the active AMF intake command |

Preload invokes `vua:library:open-folder` with no payload. Main requires the local main window's
own main frame and a ready AMF runtime. It resolves the root from the module registry, including
the preserved legacy layout where applicable; Renderer cannot supply a path or choose another
module. Main returns `opened: true` only when `shell.openPath` accepts the handoff; a handler
failure returns `opened: false`. Sender/module guard failures reject the call. The UI disables
the action when its native face is absent or AMF is unavailable, prevents duplicate pending
requests, and reports a failed handoff without displaying private system error text.

These two handoffs do not enumerate files, download or import materials, change
module ownership, or expose credentials, file paths or an arbitrary process-launch command to
Renderer. Remote pages have no preload and cannot access the face.

The product action invokes `vua:library:open-booth-product`, with the same local main-frame and
ready-AMF guards. Main constructs the fixed official URL; Renderer cannot supply an arbitrary
URL, command or local path. Malformed IDs reject; system-browser failures return `opened: false`.
The handoff does not transfer the embedded account session, download files or infer ownership.

The drop resolver uses Electron `webUtils.getPathForFile` inside preload, never `File.path` or
page-supplied URLs. JavaScript-created files without native backing reject. Preload bounds the
selection to 1–200 files, then Main receives only paths through `vua:library:resolve-dropped-files`.
Main requires its own local main frame and ready AMF, validates ordinary absolute files/folders,
rejects links, deduplicates Windows aliases and removes descendants covered by a selected folder.
The returned local paths go through the typed [library-intake v0.2](library-intake-v0.2.md) command;
this resolver itself neither copies files nor starts a task. The host prompt offers explicit AMF
activation before resolution, and failures remain visible. Remote pages still have no preload.

## Compatibility and evidence

The optional member preserves older injected aggregates and browser previews. This Candidate
face has no accepted freeze exemption and does not modify frozen Provider schemas.

The [controlled consumer smoke](../../apps/desktop/scripts/smoke-booth-session.mjs) exercises
real Main/preload/Renderer and the AMF provider with an isolated profile and synthetic BOOTH
pages. It verifies login-close refresh, synchronization, four-language/theme/zoom filter layouts,
the module-resolved folder handoff and disabled-module rejection. Its folder handler is stubbed;
this is not evidence of a real account/material acceptance run or a visible Explorer window.
Raw screenshots and session artifacts remain local.

The [native drop smoke](../../apps/desktop/scripts/smoke-library-drop.mjs) exercises the real
Chromium native File store, mixed file/folder input, explicit enable/cancel, retained File handles,
source preservation and text/URL exclusion in all four UI languages with a temporary profile.

## Document changelog

- 1.2.0 (2026-10-12): add the main-frame/active-AMF native drop resolver; JavaScript-created files and unselected sibling folders cannot become intake inputs.
- 1.1.0 (2026-10-12): add the bounded canonical BOOTH product-page handoff for menus, details and source candidates.
- 1.0.0 (2026-10-11): define the independent, main-frame-only local material-folder action with active AMF ownership and explicit handoff/failure results.
