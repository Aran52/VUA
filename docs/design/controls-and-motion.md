# Controls and motion

> Updated: 2026-10-25
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
2. **Within one theme, every control speaks the same language**: one corner style, one stroke,
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
- **Stepper:** workshop station cards are at least 80 px high, with room below for the
  selected card's cast shadow; the connector belongs to the theme's shape language.

The steps are theme-independent; a theme changes shapes, colours and motion, never these sizes.

## The theme's language

A theme defines the following token groups. Components only consume them.

### Corners

Corners are set per **role group**, each with a style and a size:

| Role group | Members | Grid |
| --- | --- | --- |
| Actions | buttons, icon buttons, segmented items, tabs | cut, 6 px |
| Fields | text fields, selects, search | cut, 4 px |
| Indicators | tags, status chips, badges | cut, 4 px |
| Containers | cards, panels, list and table groups | cut, 12 px |
| Overlays | menus, popovers, dialogs, drawers, toasts | cut, 16 px (menus 8 px) |

- Styles a theme may choose: `cut` (a chamfer on the upper-left corner), `cut-2` (top-right and
  bottom-left), `round` (a radius of that size), `square` (none), `pill` (fully round ends, for
  actions and indicators only) and `mask` (a named mask image such as a lace edge, from the skin).
- **Grid** uses `cut` throughout: one chamfer on the upper-left corner with a **120-degree
  interior angle at the top edge**, matching the VUA mark (author review, 2026-10-25). The
  table's size is the horizontal cut; its vertical depth is that size × √3. The other three
  corners stay square. All former capsule actions, choices, tags, badges and progress tracks
  follow this language. The diagonal carries the same thin stroke as the other edges.
- **Nesting:** an inner corner is never larger than the corner of the element around it.
- **Special shapes** are allowed for named roles only and declared by the theme: the brand
  mark, library status corner, avatar circles, activity spinners and status dots. Workshop
  station cards use the container bevel, rather than an arrow notch. A feature does not invent
  its own shape.

Implementation. The bundled Chromium supports native `corner-shape: bevel`. The shared
`packages/design-system/src/shapes.css` maps role tokens to an elliptical upper-left
`border-radius` (x / x × √3), with zero radius elsewhere. Native geometry paints the diagonal
border, focus outline and cast shadow without clipping them away. The custom role property
does not inherit into text or icon children. Themes own these values; features do not write
their own polygons. The [CSS borders specification](https://drafts.csswg.org/css-borders-4/)
defines the native corner geometry.

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

### State ladder

Every interactive control follows the same ladder, so hover, press and selection look alike
across the app:

| State | Change |
| --- | --- |
| rest | the role's base |
| hover | stroke to `--vua-border-strong`, or fill one step lighter (dark) or darker (light) |
| pressed | fill one further step; the theme's press feedback (Grid: a 1 px inset shift, no scaling) |
| focus-visible | the focus ring, in addition to the current state |
| selected | the selected recipe above; hover still applies on top |
| disabled | text and stroke at the disabled colour, no hover, a reason in the tooltip |
| busy | a spinner replaces the leading icon; the label stays; the control keeps its width |

### Elevation and glass

| Level | Surfaces | Shadow | Glass |
| --- | --- | --- | --- |
| 0 | page canvas | none | no |
| 1 | cards, panels, rows | none | no |
| workshop selection | selected station card only | solid lower edge plus heavier cast shadow | no |
| 2 | menus, popovers, tooltips | small | allowed |
| 3 | dialogs, drawers | medium | allowed |
| 4 | toasts | medium | allowed |

The shell (top bar, sidebar, task bar) and labels floating over scene art may use glass. Grid
keeps glass only on the shell and on level 2–4 overlays.

The selected workshop station is the author's explicit exception to flat ordinary cards:
`--vua-shadow-workshop-active` gives it visibly more mass than other controls. It follows the
selected (`aria-pressed`) station; resting stations stay flat. High-contrast and forced-colour
modes retain the accent edge and focus while removing this decorative shadow.

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
| stepper | the active station's stroke pulses only while its work runs |

A theme may set a **motion feel**: a duration scale (0.75–1.5 applied to fast, normal and slow),
its own easing pair and its press feedback (Grid: inset shift; Tenshi: a slight bounce; Jirai:
slower, softer). Reduced motion, `data-effects="off"` and resource-saving mode override every
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
spacing, colours, contrast, painted overflow, surface/corner nesting and the shared Grid
role bevel. Role-specific size
steps and complete motion/bespoke-control checks remain scheduled
in the [Grid normalization plan](../development/grid-theme-normalization.md). A complete run
with no current-rule findings does not establish those remaining catalogue requirements.

## Grid migration

Before this migration the default theme mixed pill buttons and chips, skewed shell tabs,
factory cut corners and rounded fields/cards. The shared upper-left bevel now replaces those
recipes; the complete role/size/state primitive catalogue continues in this order:

1. Shape mixin, corner tokens per role group and motion tokens in `packages/design-system`.
2. Primitives: `Button`, `IconButton`, `Segmented`, `Tabs`, `Tag`, `StatusChip`, `Badge`, `Card`,
   `TextField`, `Select`, `Menu`, `Dialog`, `Drawer`, `Toast`, `Tooltip`, `Stepper` in the three
   steps with the state ladder.
3. Shell: the skewed tabs and title-bar buttons become `Tabs` and `IconButton` with cut corners.
4. Feature areas move their bespoke controls onto the primitives one area per pull request, each
   with the audit's before-and-after sheet.
