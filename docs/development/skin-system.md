# Skin system

> Updated: 2026-10-28
> Status: architecture approved by the author on 2026-10-17; slice 13 settles the native skins

A skin changes how VUA looks and moves without changing what it does. **Grid** is the default.
The Appearance page offers Grid and **Frutiger Aero** in dark and light, **Tenshi Kaiwai** in light
and **Jirai Kei** in dark. These display names remain English in every UI language.
`data-theme-family` selects the family. The existing `dark | light | system`
preference remains independent for Grid and Aero; single-mode presets lock the appearance choices
and override Windows' mode. Existing profiles resolve to Grid. All four built-in presets use CSS
tokens rather than the separate community package loader.

The [current plan](current-plan.md) schedules further skin work as slice 13. The design standard
keeps the community-skin direction. Built-in presets use the same token boundaries described here;
the `.vuaskin` package loader and community import remain separate work.

## What a skin can change

| Layer | Contents | How the app consumes it |
| --- | --- | --- |
| Color | surfaces, text, borders, the two module accents, the four room colors, status colors, per dark and light | CSS custom properties (`--vua-*`) |
| Type | font families (bundled WOFF2 with its license), the type scale | `--vua-font-*` shorthands, used with `font:`; the default families follow a skin's fonts for any glyph they lack, so all four UI languages render |
| Shape | one corner style and size per role group (actions, fields, indicators, containers, overlays), border weights, named special shapes | shape tokens; clip-path and mask presets chosen by name; see [controls and motion](../design/controls-and-motion.md) |
| Motion | durations, easing, camera flight time, ambient motion strength | `--vua-motion-*` tokens and the scene engine |
| Effects | glow, gradients, surface textures, background pattern | existing `--vua-glow-*`, `--vua-aurora-*`, `--vua-shine` plus a pattern slot |
| Materials | the card material (inner rims; a soft shadow for Jirai only), a stitch line inside containers, a sheen, a decoration overlay on large containers (Grid's corner ticks, Tenshi's stripe band, Jirai's lace hem), list-row edges, fill and selection, secondary-button gloss, press feedback, the dialog frame and footer band, the main area's client surface, heading color, Grid's light bars and Latin room tags | `--vua-card-material`, `--vua-card-stitch`, `--vua-surface-highlight`, `--vua-card-decor`, `--vua-row-*`, `--vua-corners-row`, `--vua-selection-ring`, `--vua-button-*`, `--vua-press-*`, `--vua-dialog-*`, `--vua-client-*`, `--vua-heading-color`, `--vua-light-bar`, `--vua-running-bar`, `--vua-tag-display` |
| Motifs | decorative images in named slots: page corners, room plates, empty states, dialog headers, the base-map frame | illustrations: `data-skin-slot` elements that render a skin's image or nothing; glyphs: one-colour masks in the room or accent colour, drawn by pseudo-elements from `--vua-glyph-mark`, `--vua-glyph-selected` and `--vua-glyph-empty` once `--vua-motif-glyphs` turns them on; tiles: `--vua-header-edge` under the header and `--vua-trim-image` along dialogs |
| Icons | an optional replacement icon set | the `Icon` component looks up the skin's sanitized SVG before the default |
| Factory scenes | the base map's room interiors: backdrops, props, characters, particles | a scene pack: sprites plus a declarative layer description (below) |
| Sound | reserved for later | — |

A skin never changes text, functions, operation order, focus behavior or what a screen shows. Since
the author's ruling of 2026-10-28, a built-in preset may add theme-only shell parts that mirror
functions VUA already has, and may restyle structure: Frutiger Aero adds Vista's breadcrumb address
bar, a search box that opens the command palette, a command bar for help, settings and modules, and
a details pane (`features/shell/ThemeChrome.tsx`); Jirai Kei strings the workshop steps as hearts on
a chain. The structure lives in `packages/design-system/src/themes/*-chrome.css`, scoped to the
preset. High contrast and Windows forced colors override every skin and return to the ordinary
shell, and resource-saving mode stills every skin's motion, so the accessibility gates hold for all
skins.

## Package format

A skin is a folder or a `.vuaskin` zip with a manifest:

- `manifest.json`: `schemaVersion`, `id`, `version`, a fixed English display name, descriptions in
  the supported UI languages, author, SPDX license and a rights statement for every asset, minimum
  VUA version, preview images;
- `tokens.dark.json` and/or `tokens.light.json`: values for the documented token set only, each
  typed (color, length, duration, easing, enum) and range-checked; a skin omits what it keeps
  from the default. A skin may provide one mode only (Tenshi Kaiwai is light only); while it is active,
  Appearance shows the theme switch locked to that mode and Windows' theme does not change it;
- `motifs/`, `icons/`, `fonts/`, `scenes/`: images (PNG, WebP, sanitized SVG), WOFF2 fonts,
  scene packs.

The contract lives in a versioned protocol document (`docs/protocols/skin-v0.1.md`, Candidate) with
a JSON Schema under `schemas/skin/v0.1/`. Native skins use the same format, so the format is proven
by VUA's own skins before anyone else uses it.

## Safety for community skins

A skin is data. It carries no JavaScript and no raw CSS:

- token values are parsed by type and written as custom properties; a value cannot carry `url()`,
  `@import` or a selector; image tokens such as `--vua-glyph-*` are written by the loader from the
  skin's own motif files;
- SVG passes a sanitizer (no scripts, event attributes, `foreignObject` or external references);
- assets load from a local `vua-skin://` protocol under the renderer's content security policy,
  never from the network;
- size limits per file and per skin; image dimensions are capped;
- contrast is computed for every text/background pair the token set defines, in dark and light,
  with the same rules as `check-contrast`; a skin below WCAG AA does not apply and the player
  sees which pairs failed.

Installation starts with local import (Appearance → Import skin, or dropping the file). A shared
catalog, signing and review are a later decision; the format already records author, license and
rights so a catalog can rely on them.

## Factory scene packs

The base map's interiors are drawn by a scene engine in a 400 × 190 space aligned to the room's
bottom, with live room facts (Recipe count, package count, line stations, the latest finished
Avatar). Today each room is drawn in code. The engine gains a declarative path:

- a scene is a list of layers: images or sprite-sheet animations with a position, size, anchor
  and depth;
- motion is one of a fixed set: bob, sway, blink, scroll, walk a path, rotate, frame sequence,
  particles (with count, life and spread);
- bindings connect layers to facts: repeat a prop for each package (up to a cap), place the crate
  at the active station, light a lamp by station state, show the finished Avatar on the stage;
- slots name the places a pack fills, so a pack can replace only the workers or only the backdrop.

The current code-drawn scenes become the default pack's fallback. Resource-saving mode draws one
frame. A pack that fails validation falls back to the default scene for that room.

## Native skins

| Skin | Availability | Color and shape | Motion and effects |
| --- | --- | --- | --- |
| **Grid** (default) | dark and light | neutral flat surfaces, purple/orange jurisdictions, small-radius rectangles with hairlines and accent light bars | crisp; no neon glow or purple/orange gradients |
| **Frutiger Aero** | dark and light | Windows Vista's Aero: a glass header with glare around a white client area, Segoe UI, 3–6 px radii, glossy push buttons, Explorer's selection, task dialogs with a footer band; dark mode is Vista's black glass | glossy highlights and glare; removed by accessibility overrides |
| **Tenshi Kaiwai** | built-in preset; light | sky blue and white, a rounded title font, blue-white stripes, zipper tape, a second hairline inside containers, wings and a pixel heart | light, bouncy |
| **Jirai Kei** | built-in preset; dark | pink and black, a display title font, black lace hems, a dashed pink stitch, a soft card shadow, bows and a pink energy-drink can | slower, softer and dimensional |

The [native skins art brief](../design/native-skins.md) owns the palettes and visual vocabulary.
Grid and all additional presets are implemented in `packages/design-system/src/tokens.css` and
`themes/*.css`. Original local SVG motifs in `packages/design-system/skins/` render through
`SkinMotif` into the sidebar, Appearance previews and the workshop's selected-line header, and as
glyphs before room names, section and dialog titles, after the current navigation item, above
empty states and along the lower edge of dialogs.
Tenshi and Jirai bundle OFL title-font subsets (see the [art brief](../design/native-skins.md#title-fonts)); extended scene packs and `.vuaskin` packages remain separate work.

## What to do before slice 13

Rules for every screen built from now on, including the control center and release work in
progress:

1. Colors, radii, shadows, font sizes and durations come from tokens; new screens add tokens
   instead of literals. Fonts use the shorthand tokens with `font:`.
2. Room colors get their own tokens (`--vua-room-control`, `--vua-room-materials`,
   `--vua-room-workshop`, `--vua-room-release`). Status colors keep their meaning and are not used
   as room colors.
3. Shapes come from role tokens and the shared native corner renderer (`--vua-corner-*` /
   `--vua-corners-*` in `packages/design-system`), rather than feature polygons. The owning
   [control language](../design/controls-and-motion.md#corners) defines Grid's geometry.
4. Decorative extras sit in `data-skin-slot` elements; a screen works and reads the same with
   every slot empty.
5. Canvas and WebGL drawing reads its colors from tokens through one palette helper and keeps its
   motion parameters in one place, so the scene engine and packs can take them over.
6. Motion honors resource-saving mode through one hook; scripted motion does not rely on CSS
   transitions.

Work in order:

1. Room color, shape and motion tokens (in place since 2026-10-17: `--vua-room-*`,
   `--vua-shape-*`, `--vua-motion-*`, `--vua-scene-*`); the factory, workshop, control center and
   release pages move to them; fix the `font-size: var(--vua-font-*)` uses (a shorthand is not a size).
2. A token linter: new literal colors, durations and radii in CSS fail the check, with an
   allowlist for the remaining legacy files that shrinks over time.
3. The skin loader with the default skin expressed as a package, the schema, validation and the
   contrast gate; Appearance shows the default only.
4. Slice 13: the design session, Tenshi and Jirai as packages, one-click switching in Appearance
   and the command palette, the scene engine's declarative path.
5. Community import once the author decides the distribution model.

## Decisions

The author decided on 2026-10-17:

- A skin may recolor the two module accents; each skin still gives the two modules distinct
  accents, checked for contrast pair by pair.
- Community skins start with local import; a shared catalog with review is decided later.
- Community skins may replace the factory scenes through scene packs.

- A skin may bring its own fonts; VUA's default font families stay behind them as fallbacks for
  missing glyphs, and the four-language review checks each skin's text.
- A skin may support one mode only; the theme switch is then locked to that mode.
