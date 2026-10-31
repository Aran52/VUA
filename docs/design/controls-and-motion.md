# Controls and motion

> Updated: 2026-10-28
> Scope: control sizes, the shape language a theme defines, states, elevation and motion
> Status: approved direction (author, 2026-10-25); the Grid values below are the default theme

This document owns how controls are sized, how a theme gives them one visual language, and how
the interface moves. The [design standard](design-standard.md) owns the principles, colour and
type tokens and accessibility; the [skin system](../development/skin-system.md) owns how themes
are packaged. The [design audit](design-audit.md) checks the rules here.

## Two requirements

The author's ruling of 2026-10-25 replaces fixed shape rules with two requirements:

1. **Every control has a size specification** that holds in every theme: height, padding, icon
   size, gap, minimum width and hit area per size step.
2. **Within one theme's declared scope, every control speaks the same language**: one corner style, one stroke,
   one fill logic, one state ladder and one motion feel, applied to all controls.

Which control is square, rounded or a special shape is the theme's decision, not the standard's.
A theme states its language once in tokens; controls read it, so a new theme restyles every
control without touching a component.

## Control catalogue and sizes

Every control is one of these roles and uses its primitive. A feature that needs a variant adds it
to the primitive instead of styling its own button, tab or tag.

| Role | Primitive | Examples today |
| --- | --- | --- |
| Action | `Button` (primary, secondary, subtle, danger), `IconButton` | new line, stop, fix with the SDK, window controls |
| Choice | `Segmented` (one of a few), `Tabs` (views of one page), `Toggle`, `Checkbox`, `Radio` | shell tabs, room tabs, capacity choices, package section tabs, mode switch |
| Field | `TextField`, `Select`, `Search` | library search, version select |
| Indicator | `Tag` (a fixed label), `StatusChip` (a state), `Badge` (a count), `StatusLight`, `Progress` | room tags, line status, notification count, import progress |
| Container | `Card`, `Panel`, `Section`, `ListRow`, `TableRow` | line cards, library cards, settings sections |
| Overlay | `Menu`, `ContextMenu`, `Popover`, `Tooltip`, `Dialog`, `Drawer`, `Toast` | mode picker, line menu, confirmations |
| Sequence | `Stepper` | workshop stations |

Text inputs without a feature-specific appearance use the shared `.vua-text-input` skin in
`packages/design-system/src/base.css`, including legacy package/project forms. It reuses the
folder picker's path-field fill, border, padding, corners and focus outline. The folder picker
uses the same class; feature layouts still own width and path direction. Existing search and
other deliberately styled fields keep their own appearance, and the AMF geometry scope still
owns their rectangular shape.

The Workshop automation selector shows its label above the current mode in a
64 px control with the module accent border and tint. Its header and the rail's
New line button share that height and baseline; new lines expose the selector
before work starts (author follow-up, 2026-10-31).

### Size steps

Controls come in three steps; a page uses one step for its main controls and may use a smaller
step inside dense rows.

| Step | Height | Inline padding | Icon | Icon–text gap | Text | Use |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `sm` | 28 px | 8 px | 16 px | 4 px | caption, 600 for actions | rows, tables, toolbars inside cards |
| `md` | 32 px | 12 px | 16 px | 8 px | body | default for forms and page toolbars |
| `lg` | 40 px | 16 px | 20 px | 8 px | body, 600 | the page's main call to action |

- **Hit area:** at least 32 × 32 px for primary actions and 24 × 24 px as the floor; an `sm`
  icon button pads its hit area to 28 px.
- **Minimum width:** text buttons are at least twice their height wide; icon buttons are square.
- **Fields** use the same heights as actions of the same step, so a field and its button line up.
- **Indicators:** `Tag` and `StatusChip` are 20 px high with 8 px inline padding and caption text;
  `Badge` is 16 px high with a 16 px minimum width; `StatusLight` is 8 px.
- **Containers:** inner padding 12 px (`sm`), 16 px (`md`) or 24 px (`lg`); a container's controls
  use the step below its padding (an `md` card holds `sm` or `md` controls).
- **Overlays:** menus have 32 px rows and 4 px inner padding; dialogs are 400, 560 or 720 px wide
  with 24 px padding; drawers are 360 or 480 px wide; tooltips wrap at 280 px.
- **Stepper:** workshop station cards are at least 80 px high. A neutral edge or thin
  underline marks viewing; execution uses the icon, status text and progress connector.
  Neither state gives the card a cast shadow or a large accent fill.

The steps are theme-independent; a theme changes shapes, colours and motion, never these sizes.

## The theme's language

A theme defines the following token groups. Components only consume them.

### Corners

Grid uses rectangles and rounded rectangles only, with small radii, and the same geometry in the
VUA host and in AMF content (author ruling, 2026-10-28; it replaces the 2026-10-25 cut-corner
direction). Only the module accent colour differs between host and AMF pages.
The material card's upper-left status/action marker is a named exception: a 40 px
triangle attached to the corner, restored by the author ruling of 2026-10-31.
Its 72 px decorative tail fades to transparency down-right at 45 degrees and passes
pointer input through to the preview. The marker paints above the card frame in resting,
hover and selected states. Its triangular outline appears only for keyboard focus;
the card itself keeps the container geometry (author follow-up, 2026-10-31).

Corners are set per **role group**, one radius per group:

| Role group | Members | Grid |
| --- | --- | --- |
| Actions | primary and secondary buttons, icon buttons, segmented items, tabs | 4 px |
| Fields | inputs, selects, search | 4 px |
| Indicators | tags, status chips, count badges | 2 px |
| Status lights | `StatusLight` | 4 px rounded square |
| Toggles | `Toggle` track and thumb | 4 px track, 2 px thumb |
| Progress | progress bars | square, 2 px tall |
| Containers | cards, panels, workshop station cards, base-map rooms | 8 px |
| Repeated rows | lists, tables, compact groups, the workshop's line list | square, separated by lines and space; themes may round and outline each row (`--vua-corners-row`) |
| Large overlays | dialogs, drawers, browser frames | 8 px |
| Small overlays | menus, tooltips, compact popovers | 4 px |

- Styles a theme may choose: `round` (a radius per role), `square` (none), `pill` (fully round
  ends, for actions and indicators) and `mask` (a named mask image such as a lace edge, from the
  skin). Cut, bevel, notch and parallelogram corners are not part of any theme.
- **Nesting:** an inner corner is never larger than the corner of the element around it.
- **Special shapes** are allowed for named roles only: the brand mark, avatar circles, activity
  spinners, status dots and the material card corner marker. A feature does not invent its own shape.
- **Futuristic feel without shapes.** Grid's character comes from precision and light: 1 px
  hairlines; 2 px solid accent light bars for state (the selected tab's lower edge, the current
  navigation item, a base-map room's leading edge, a panel's short top bar); cool near-black
  surfaces; accent colour only on actions and states; tabular digits everywhere and
  `--vua-font-mono` for IDs, versions, paths and URLs; label-over-value data layout; HUD corner
  ticks on large containers and a light bar along the top of the running step card (author ruling,
  2026-10-28); and short, exact motion without bounce. Gradient buttons, neon glow and card shadows
  are not used.

Grid's inner card HUD ticks use `--vua-grid-hud-color`, mixed at 65% of
`--vua-border-strong` with transparency (author follow-up, 2026-10-31).
This quiets the decoration while preserving ordinary borders and focus indicators.

Implementation. Each theme sets the role radii as `--vua-corners-<role>` tokens and the host
radii as `--vua-radius-*`; in Grid both resolve to the table above. The shared
`packages/design-system/src/shapes.css` applies the role radii to AMF's own controls, so bespoke
AMF buttons, fields and panels follow the theme too. Non-inherited role metadata lets the
[design audit](design-audit.md) check one radius per role and the absence of cut corners
(`corner-shape` other than round, or a `clip-path` polygon).

Frutiger Aero, Tenshi Kaiwai and Jirai Kei set their own radii through the same tokens.

### Stroke and fill

| Token | Meaning | Grid |
| --- | --- | --- |
| stroke width | edges of fields, secondary actions, containers | 1 px |
| focus width | focus ring | 2 px, accent |
| primary fill | the main action | solid accent, no gradient |
| secondary | other actions | 1 px accent-tinted stroke, transparent fill |
| subtle | low-emphasis actions | text only, tint on hover |
| selected | chosen tab, segment, row | accent tint fill, accent text, 2 px accent bar on the leading or bottom edge |
| container fill | cards and panels | solid panel colour |

Aero may use a restrained gradient on primary actions. Every gradient stop must meet the same text
contrast requirement as its solid accent tokens. Ordinary cards stay readable and solid in both
themes; atmospheric fills belong to the canvas and allowed overlay structure. A theme can give
primary buttons a declared material through `--vua-action-shadow`, and `--vua-header-sheen` supplies
the shell's glass finish. Level-1 containers wear the theme's declared **card material**: inner rims
and, for Jirai only, a soft cast shadow in `--vua-card-material`; a line drawn just inside the edge
in `--vua-card-stitch` (Tenshi's second hairline, Jirai's dashed stitch); and a sheen in
`--vua-surface-highlight` (Aero's gloss band). Grid sets none of them. These are not permission for
arbitrary card shadows. Accessibility overrides remove all of them; resource-saving mode removes
the material and sheen and keeps the stitch, which costs nothing to draw.

### State ladder

Every interactive control follows the same ladder, so hover, press and selection look alike
across the app:

| State | Change |
| --- | --- |
| rest | the role's base |
| hover | stroke to `--vua-border-strong`, or fill one step lighter (dark) or darker (light) |
| pressed | fill one further step; the theme's press feedback in `--vua-press-transform` and `--vua-press-filter` (Grid: a 1 px inset shift, no scaling; Tenshi a small bounce; Jirai a 2 px sink; Aero a brighter highlight) |
| focus-visible | the focus ring, in addition to the current state |
| selected | the selected recipe above; hover still applies on top |
| disabled | text and stroke at the disabled colour, no hover, a reason in the tooltip |
| busy | a spinner replaces the leading icon; the label stays; the control keeps its width |

### Elevation and glass

| Level | Surfaces | Shadow | Glass |
| --- | --- | --- | --- |
| 0 | page canvas | none | no |
| 1 | cards, panels, rows | none; a theme may declare a card material (Jirai: a soft shadow, author ruling 2026-10-28) | no |
| 2 | menus, popovers, tooltips | small | allowed |
| 3 | dialogs, drawers | medium | allowed |
| 4 | toasts | medium | allowed |

The shell (top bar, sidebar, task bar) and labels floating over scene art may use glass. Grid
keeps glass only on the shell and on level 2–4 overlays.

The author's adopted third-party review (2026-10-25) supersedes the earlier selected-station
shadow exception. Viewing (`aria-pressed`) and execution (`aria-current="step"`) are independent.
Only a step awaiting user input uses amber; future steps stay neutral. The detail area names
both the viewed step and the running step, or explicitly says no step is running.

### Iconography

One icon set with one stroke width per theme (Grid: 1.5 px line icons), at 16, 20 or 24 px,
coloured by the text colour of their control. Icons never carry meaning alone where a label fits.

## Motion

Motion explains a change; it never decorates or delays. These tokens replace the duration table
of the design standard's earlier §7:

| Token | Duration | Use |
| --- | ---: | --- |
| `--vua-motion-instant` | 0 ms | reduced motion, immediate replacement |
| `--vua-motion-fast` | 120 ms | hover, press, focus, colour changes |
| `--vua-motion-normal` | 180 ms | menus, popovers, tooltips, local reveal, page content swap |
| `--vua-motion-slow` | 360 ms | dialogs, drawers, cards entering, staged entrances |
| `--vua-motion-camera` | 650 ms | base-map camera flight, album page turns |
| `--vua-motion-station` | 240 ms | viewing indicator travel between workshop stations |

- **Easing:** entrances use `--vua-motion-ease-enter` (`cubic-bezier(0.2, 0.9, 0.25, 1)`); exits use
  `--vua-motion-ease-exit` (`cubic-bezier(0.4, 0, 1, 1)`) and take two thirds of the entrance time.
- **Properties:** opacity and transform only; never layout, size or colour of large areas.
- **Response first:** feedback to a click starts within one frame; a result never waits for an
  animation, and no motion over 180 ms sits between a click and its result.
- **One entrance per view:** a page or dialog animates in once; content updates inside it fade
  (normal). Lists stagger at most three items, 40 ms apart.
- **Loops** only show real activity (progress, a running station, loading) and pause when off
  screen, unfocused or finished.

### Motion vocabulary

| Pattern | Motion |
| --- | --- |
| hover, press, focus | colour and stroke over fast; Grid press: 1 px inset shift over fast |
| menu, popover, tooltip | fade with a 4 px slide from its anchor, normal; exit fade only |
| dialog | fade with an 8 px rise, slow; backdrop fade, normal |
| drawer | slide from its edge, slow |
| toast | slide 8 px from the stack edge with fade, normal; leaves by fading |
| tab or segment change | the selected bar moves over normal; content cross-fades over normal |
| page change | content fades over normal; no sliding pages |
| row added or removed | fade over normal; neighbours move over normal |
| progress | determinate bars move over normal; indeterminate shimmer only while active |
| stepper | one shared viewing indicator slides to the selected station over 240 ms with `--vua-motion-ease-station` (`cubic-bezier(0.55, 0, 1, 0.45)`), an ease-in/accelerating curve; execution state does not move |

A theme may set a **motion feel**: a duration scale (0.75–1.5 applied to fast, normal and slow),
its own easing pair and its press feedback (Grid: inset shift; Tenshi Kaiwai: a slight bounce;
Jirai Kei: slower, softer). Reduced motion, `data-effects="off"` and resource-saving mode override every
theme: they keep at most a fast opacity change and still every loop. Scripted motion (the
factory scenes, album, camera) reads the same tokens through the motion hook.

## Checking

The completed control catalogue requires these checks per theme:

- control heights on the three steps, and indicators at their fixed heights;
- one corner style and size per role group and size step (a "language" check: every button of a
  step shares its corner);
- inner corners not larger than outer ones;
- durations and easings from the motion tokens (static scan for literal `ms` and `cubic-bezier`);
- bespoke controls: a `<button>` outside the primitives that draws its own border or fill.

The current [audit implementation](design-audit.md) measures hit-area floors, typography,
spacing, colours, contrast, painted overflow, surface/corner nesting and one radius per role
with no cut corners. Role-specific size
steps and complete motion/bespoke-control checks remain scheduled
in the [Grid normalization plan](../development/grid-theme-normalization.md). A complete run
with no current-rule findings does not establish those remaining catalogue requirements.

## Current theme presets

Control sizes and the state ladder stay shared. Shape, color, texture and motion values are
selected by `data-theme-family`. Grid and Aero keep `dark`, `light` and `system` independent;
Tenshi Kaiwai locks Light and Jirai Kei locks Dark, so those choices cannot contradict the preset.
Missing or invalid stored families resolve to Grid. All four display names stay English in every
UI language.

- **Grid** supports dark and light modes. Neutral surfaces, low-contrast structural lines and
  orange/purple jurisdictions carry the futuristic composition: small-radius rectangles, hairlines
  and accent light bars, flat fills, and no gradients, neon glow or synthwave treatment.
- **Frutiger Aero** supports dark and light modes and makes VUA a Windows Vista window (author
  approval of the from-scratch mockup, 2026-10-28): a glass frame with glare over the Aurora
  wallpaper, an orb, Vista's caption buttons, a breadcrumb address bar and search box, a dark command
  bar, Explorer's navigation pane and selection, Control Panel style links with glossy icons, group
  boxes, Vista's glossy push buttons and green progress bar, a details pane, and task dialogs with a
  glass frame and a gray footer band. Dark mode is Vista's black glass. High contrast returns to the
  ordinary shell.
- **Tenshi Kaiwai** locks light mode: sky blue and white with light techwear. A striped header with
  a zipper and its pull, a woven brand label with a halo and wings, a white sidebar with a webbing
  strap, nylon-pouch cards with a zipper and a woven tag, velcro-patch steps with a halo on the
  running one, striped selection, sky-blue stitched pills, a rounded title font and a pastel
  PC-window dialog with pixel buttons and wings.
- **Jirai Kei** locks dark mode: black satin for most of the surface with pink as the accent. A
  satin header with black lace hanging from a pink rim, a black-and-pink gingham sidebar with ribbon
  banners and a ribbon tag on the current item, satin cards with a dashed stitch, a lace hem and a
  bow, hearts strung on a chain for the workshop steps, pink satin pills, a candy-stripe progress bar
  with a heart, a pink energy-drink can marked with three wavy lines, a safety pin and a bandage, a
  display title font and a letter-style dialog with a pearl frame and a big bow.

The complete role/size/state primitive catalogue still migrates feature controls onto shared
primitives. New themes consume that catalogue through tokens rather than adding feature-level
color or shape recipes.
