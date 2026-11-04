# Stable navigation and Avatar-editing layout

> Updated: 2026-10-26
> Scope: fixed sidebar geometry, reserved scrollbar space and an aligned Creator directory
> Status: implemented; author review in the dev app remains pending

The author reported sidebar size changes on blur/hover, card-width jumps when details caused
scrolling, and an irregular Avatar-editing page. The sidebar now keeps the same text size,
padding and bounds in idle, hover, focus and selected states. Its glass may fade without
resizing. Page scroll containers reserve a stable vertical gutter before overflow appears.

Avatar editing uses one full-width software row per editor or manager, with aligned names,
icons and observed status. Disabled Unity 6 fills the same row as enabled software, including
through its tooltip anchor. Status is part of the button's layout rather than an absolutely
positioned caption. Details expand directly beneath their own row. AMF remains a separate,
compact preference group; its activation and library-opening behavior are unchanged.

The [design standard](../design/design-standard.md) owns these requirements. The 2026-10-26
fixed-sidebar ruling supersedes the older idle-small-type and activation-reflow direction.

## Evidence

[Dark/light Creator comparison](assets/grid-layout-creator-2026-10-26.png) uses unchanged
screenshots of real components, the same synthetic inventory and an asserted actual theme.
The before interface is `1a7ae6ce`; the after interface is this layout revision. Original user
screenshots and machine-specific observations remain local.

`smoke:layout-stability` measures actual Chromium bounds rather than inspecting CSS strings.
It checks sidebar idle/hover/focus geometry, ordinary and extra-editor inventories, and the
disabled Unity 6 row. In Workshop, Play, Tools and Avatar editing, each case starts without
scrolling, opens a real detail that causes overflow, and closes it again. Persistent card
widths and left edges must be identical through that round trip. Narrow Creator cases also
check that software rows and AMF controls fit. Dark and light variants run in TypeScript CI.

Verification: **16 layout/scroll cases**, **1,472 desktop unit tests** (one existing skip),
**57 audit regressions** and the **84 first-run/theme/Play interaction checks** pass. Types,
production build, module isolation, boundary, translation, contrast and leakage checks also pass.
The three browser-shell notification/viewport cases pass after the shared layout changes.

The filtered design audit passes **36/36 states, 4,044 elements, zero findings and no collection
failures**: running Workshop, Theme Settings, Play, ordinary/standard-editor Creator and Tools.
Each has zh-CN dark/light and en/ja/ko/zh-TW dark overflow passes. The complete registry is
now 102 views / 612 states; this partial run does not refresh its full-matrix baseline.

Native dev-app review opens and collapses VCC details against the existing local inventory.
The resulting scrollbar does not change the software-row edges; the sidebar retains its
size while focus is in the content. Automated checks use isolated synthetic ports. Neither
these checks nor the comparison figure constitutes real-material/device or author visual
acceptance.
