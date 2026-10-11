# Desktop-library v0.1: open the active local material folder

> Document version: 1.0.0
> Status: Candidate
> Updated: 2026-10-11
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

Preload invokes `vua:library:open-folder` with no payload. Main requires the local main window's
own main frame and a ready AMF runtime. It resolves the root from the module registry, including
the preserved legacy layout where applicable; Renderer cannot supply a path or choose another
module. Main returns `opened: true` only when `shell.openPath` accepts the handoff; a handler
failure returns `opened: false`. Sender/module guard failures reject the call. The UI disables
the action when its native face is absent or AMF is unavailable, prevents duplicate pending
requests, and reports a failed handoff without displaying private system error text.

This explicit local action does not enumerate files, download or import materials, change
module ownership, or expose credentials, file paths or an arbitrary process-launch command to
Renderer. Remote pages have no preload and cannot access the face.

## Compatibility and evidence

The optional member preserves older injected aggregates and browser previews. This Candidate
face has no accepted freeze exemption and does not modify frozen Provider schemas.

The [controlled consumer smoke](../../apps/desktop/scripts/smoke-booth-session.mjs) exercises
real Main/preload/Renderer and the AMF provider with an isolated profile and synthetic BOOTH
pages. It verifies login-close refresh, synchronization, four-language/theme/zoom filter layouts,
the module-resolved folder handoff and disabled-module rejection. Its folder handler is stubbed;
this is not evidence of a real account/material acceptance run or a visible Explorer window.
Raw screenshots and session artifacts remain local.

## Document changelog

- 1.0.0 (2026-10-11): define the independent, main-frame-only local material-folder action with active AMF ownership and explicit handoff/failure results.
