# Design audit process

> Scope: the automated UI design review tool, its rules, outputs and how fix
> work consumes its reports
> Updated: 2026-10-24

The design audit renders every registered view through the existing preview and
smoke fixtures, walks the rendered computed styles, and reports where the
implementation deviates from the [design standard](design-standard.md). It is a
read-only review instrument: it never edits component styles or tokens. The
only permitted UI-code change is the `data-audit-skip` attribute that excludes
art layers (below).

## Running

```powershell
# full audit: default pass (all views x zh-CN x dark+light, all rules)
# plus the overflow pass (all views x en/ja/ko/zh-TW x dark)
pnpm --filter @vua/desktop audit:design

# common filters
pnpm --filter @vua/desktop audit:design -- --only=workshop        # view-id prefix (comma-separated list works)
pnpm --filter @vua/desktop audit:design -- --locale=zh-CN --theme=dark
pnpm --filter @vua/desktop audit:design -- --no-screens           # findings only, faster

# compare against the committed baseline (CI gate mode; exit 1 on new findings)
pnpm --filter @vua/desktop audit:design -- --check-baseline

# refresh the committed baseline (only after a fix lowers the violation count)
pnpm --filter @vua/desktop audit:design -- --update-baseline

# tool unit + end-to-end tests
pnpm --filter @vua/desktop test:design-audit
```

Environment: any machine that can run the smoke tests (Electron, no network).
The runner uses the same isolated temporary profile as the smokes and binds a
local Vite dev server on 127.0.0.1; no external requests are made. A full run
takes roughly 30–60 minutes and about 100–200 MB of disk under
`apps/desktop/_local_design_audit/<timestamp>/` (git-ignored local evidence).

Output directory contents:

| Artifact | What it is |
| --- | --- |
| `shots/<view>--<theme>.png` | Clean screenshot of each view (default locale) |
| `shots/<view>--<theme>.annot.png` | Numbered red-box annotations over the same render |
| `json/<view>--<locale>--<theme>.json` | Findings grouped by rule + root cause, with element examples |
| `index.html` | Filterable overview (by rule, area, severity; click a thumbnail for the annotated image and finding list) |
| `summary.md` | The human report: scope, totals, top root causes, card inventory, static checks |
| `baseline.json` | Fresh baseline of this run (also written without `--update-baseline`) |
| `run-summary.json` | Machine record of states run and failures |

## View registry

Views live in `apps/desktop/scripts/design-audit/views.mjs`. Adding a view to
the audit is one entry in `VIEWS`:

```js
{ id: "my-view", fixture: "factory", area: "workshop", steps: ["await window.myReview.run()"], settleMs: 500 },
```

- `fixture` names a harness in the same file: its HTML shell (locale/theme
  handling), readiness global, and viewport. Reuse an existing fixture when the
  page renders through one of them; add a fixture block only for a new mount
  path, mirroring the corresponding smoke script's shell exactly.
- `steps` drive the fixture to the state to audit. Most fixtures mount their UI
  inside `run()`, so the usual step is a call to that review function; the
  state it leaves on screen is what gets audited.
- `area` is the functional area used by the report grouping.
- `exemptRules` disables named rules for a view; use it only where the standard
  itself exempts the view (currently the base map's four room colors for
  `one-accent`, per design-standard §8).

## Rules

Severities: **error** = violates a hard gate (WCAG contrast/targets, broken
layout), **warning** = violates the current standard's token discipline,
**hint** = advisory (proposed rules and view-level metrics).

### Current standard (design-standard §4)

| ID | Meaning | Thresholds and exceptions |
| --- | --- | --- |
| `type-scale` | Font size/line-height pair is not one of the token combinations | Tokens: 12/16, 14/20, 20/28, 28/36 (`--vua-font-*`); zero-size text exempt |
| `type-weight` | Font weight is not 400 or 600 | — |
| `color-token` | Solid text/background/border color matches no resolved `--vua-*` color | RGB Euclidean distance > 12 to every token counts as "no match" (token pairs differ by ≥ 14, amber #ffd34d vs warning #fde047). Translucent fills (alpha < 0.95) are exempt: the glass panel system is token-composed by construction |
| `contrast` | Text or icon foreground vs its effective background | Body 4.5:1; large text (≥ 24px, or ≥ 18.66px at weight ≥ 600) and icons 3:1. The background walks ancestors to the first opaque fill and alpha-composites translucent layers in between. Disabled controls exempt |
| `spacing-grid` | `padding`/`margin`/`gap` off the 4 px grid | Values with `v mod 4 ∈ {0, 1, 3}` pass: multiples of 4 plus ±1 px border compensation. Symmetric horizontal margins (auto-centering) exempt |
| `radius-token` | Border radius is not 0, a `--vua-radius-*` value, a pill (≥ 100px), or a circle (50%) | Tokens: 6 (control), 10 (card), 12 (overlay) |
| `shadow-panel` | Box shadow on an ordinary panel/card | Floating structure exempt (dialog/menu/popover/toast classes or roles, shell top/sidebar/task/notification classes, `position: fixed`, absolutely positioned z ≥ 600). Accent glow tokens exempt; the elevated-surface shadow token is reported as a finding on non-elevated panels |
| `blur-scope` | `backdrop-filter` outside top/side/task/elevated structure | Same floating/structure exemptions as `shadow-panel` |
| `one-accent` | Two jurisdiction accent families (purple + orange) in one view, or more than one filled accent control | Family = opaque fill or text within RGB distance 12 of the purple or orange token family. The base map composes four room colors by design and is exempt in the registry |
| `target-size` | Interactive target below 24×24 (error); primary action below 32×32 (reported as an error with `floor: 32`) | "Primary" = interactive element filled with an opaque accent color. Inline text links and disabled controls exempt |
| `overflow` | Text clipped without an intentional ellipsis, or an element extending horizontally beyond the viewport | Ellipsis (`text-overflow: ellipsis`) and line-clamp exempt; children of a scrolling container exempt; vertical growth beyond the fold is normal page scrolling and not flagged |
| `border-density` (hint) | Bordered elements per 10k px² above the all-view P90 | Cross-view metric evaluated over the default-locale dark states |

### Proposed rules (ZCode-inspired, not yet VUA standard)

Advisory only (`hint` severity); adopting them is a design-standard change for
the author to rule on. They are reported separately so the current standard and
the suggestions never blur together.

| ID | Meaning | Exceptions |
| --- | --- | --- |
| `radius-nesting` | Element radius ≥ nearest radiused ancestor (should step down per level) | Pills (≥ 100px), circles (50%), and zero-radius elements exempt |
| `surface-nesting` | An element that is itself a bordered or independently-filled surface sits 3+ surface levels deep | Only self-surfaces are reported — text and icons inside a surface never repeat the finding; SVG frames and canvas-colored containers (the backdrop, including body) do not count |

### Counting and themes

Layout rules cannot differ between themes: `spacing-grid`, `type-scale`,
`type-weight`, `radius-token`, `radius-nesting`, `surface-nesting`,
`target-size` and `overflow` are **counted once per view** (the dark state).
Color rules (`color-token`, `contrast`, `shadow-panel`, `one-accent`) are
counted per theme. The per-state JSON files keep every raw finding; the
deduplication applies to the aggregate tables, the top-20 rankings (one table
for current-standard rules, one for proposed rules) and the baseline.

## Capture failures and when the run fails

A state is a **capture failure** — recorded and listed at the top of
`summary.md`, excluded from all counts — when:

- the page shows Vite's error overlay (the view did not render, e.g. an
  unbuilt workspace package), or
- fewer than 20 elements were collected (`--per-state-elements` to override).

A full run (no `--only`/`--max`/locale/theme filters) **fails** — exit code 1,
so the CI job shows red — when there is any capture failure or when the total
inspected elements are below 30,000 (`--min-total-elements` to override; the
2026-10-24 reference run inspected 74,058). Filtered runs skip the total floor
unless it is passed explicitly. CI builds `@vua/contracts` and
`@vua/orchestrator-provider` before running the audit; the run also fails on
`ERR_ABORTED` loads that survive one retry.

## Reading the outputs

Start from `index.html`: sort by findings, filter to one rule or area, and open
the top thumbnails — the annotated image's numbered boxes correspond to the
legend under the same numbers. `summary.md` ranks **root causes**: each finding
is attributed to the CSS rule that produced the offending value (via Chrome
DevTools Protocol `getMatchedStylesForNode`, falling back to an in-page CSSOM
cascade match, then to class + area), so "fix one selector, clear N findings"
is the intended reading order. The per-element values and suggested tokens sit
in the view JSON files.

## Art layers: `data-audit-skip`

Art content — the base-map scene canvas, WebGL scenes, turntable frames,
product photos, avatar covers — is excluded from the audit. Wrap the art
element itself (not its interactive container) with `data-audit-skip`:

```tsx
<canvas ref={canvasRef} className="vua-factory__canvas" data-audit-skip="factory-scene-art" ... />
```

Add the attribute when the element is pure presentation (canvas/scene/image
pixels) whose computed styles would only produce noise. Do not add it to
interactive or text-bearing containers — status corners on product cards, for
example, stay audited even though the photo behind them is skipped. Current
markers (2026-10-24): `RoomCanvas` (factory scene art), `SceneCanvas` (WebGL
scenes), `TurntablePlayer` (turntable frames), `AmfPreview` (bridge preview
images), `MediaSlot` (product photos), `VrchatAvatarShelf` (avatar covers).

## Fix workflow

1. Pick the top root cause from `summary.md` that belongs to the functional
   area your slice owns.
2. Land the style fix in a normal `slice/<slug>` PR. Attach before/after
   overview screenshots for the affected area, and the finding count must go
   down for the rules touched.
3. Refresh the baseline in the same PR: `pnpm --filter @vua/desktop audit:design -- --update-baseline`.
   Baselines may only move when the violation count goes down; the committed
   `apps/desktop/scripts/design-audit/baseline.json` diff in the PR is the
   record.
4. CI runs the audit on every PR (`.github/workflows/design-audit.yml`, not a
   required check — it never blocks merge, but a broken run shows red instead
   of pretending success)
   and uploads the screenshots/JSON as artifacts. `--check-baseline` mode is
   available for a hard gate once the first fix wave lands; it is deliberately
   not wired into the required check chain yet.

## New views

When a fixture gains a state worth auditing, add one `VIEWS` entry (above). If
the fixture does not exist yet, write it like the existing smoke fixtures
(synthetic Gateway, readiness global, self-imported CSS), then register it. The
audit must never require network access or paid assets: fixtures are synthetic
and local.
