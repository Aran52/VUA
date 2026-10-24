# VUA design audit — 2026-10-24

> Committed copy of the run's `summary.md`, with a coverage note, a findings analysis and
> standard suggestions added; the tables are verbatim tool output. Machine artifacts
> (screenshots, annotated screenshots, per-view JSON) ride as CI artifacts on this PR and are
> not committed.

Tool: `pnpm audit:design` at commit `c4ea50d5`. Full machine output lives in the run directory (screenshots, annotated screenshots, per-view JSON, `baseline.json`); this file is the human report and is committed under `docs/research/`.

## Scope

- Views: 99 (564 view/locale/theme states; 75,798 elements inspected).
- Default pass: every view x zh-CN x dark + light, all rules.
- Overflow pass: every view x en, ja, ko, zh-TW x dark, overflow rule only.
- Theme-invariant rules (overflow, radius-nesting, radius-token, spacing-grid, surface-nesting, target-size, type-scale, type-weight) are counted once per view (dark state); color rules are counted per theme.
- Uncovered pages and reasons:
  - Play page environment cards beyond the two preview states (network-check covers the network tile only) — no dedicated fixture; partially covered via env-play/env-create.
  - Tools page and Settings home/categories pages — no fixture mounts them; smoke:vrcft drives the packaged app and is out of scope for the audit harness. Settings accounts is covered via first-run-settings.
  - Package manager page (features/packages) outside the AMF control-center variants — no fixture.
  - First-run wizard steps — the first-run-review fixture only walks them inside run() assertions; the wizard is not reachable as a settled state. Home, Play, Creator and Settings are covered.
  - Production review page (smoke-production-review) — its fixture is an assert-and-advance interaction harness with no settled visual state; the covered workshop/control/import views show the same components.
  - Overlay windows (desktop overlay task window is covered; VR overlay, game guide, tutorial companion, reader window) — separate windows not mounted by any fixture.
  - Command palette and Ctrl+P surfaces — no fixture (first-run-settings opens and closes it mid-run).

Four fixture states fail on this machine (2026-10-24): `library-bulk`, `dependency-clues`,
`deployment-check` and `import-dialog`. Each fails inside its fixture's `run()` assertion chain,
and the upstream smoke scripts (`smoke-library-bulk`, `smoke-import-dialog`, `smoke-deployment`)
fail with the same errors when run directly on this machine, so these are environment-sensitive
interaction assertions rather than audit-harness bugs (the same states pass in CI); whether CI
captures them is recorded in the CI results note below. `first-run-settings` flaked in earlier
runs and captures in this one. One view is a genuine capture failure: `release-queued` crashes
with `Uncaught TypeError: Cannot read properties of undefined (reading 'length')` under its
queued-run fixture — `preview-factory` crashes identically — which is a pre-existing view bug
the audit surfaces (fixing it belongs to a release-room slice, not to this review tool).

## Capture failures

These states did not render auditable content (Vite error overlay, or fewer than 6 elements). They are excluded from the counts below and fail the run:

| View | Locale | Theme | Reason |
| --- | --- | --- | --- |
| release-queued | zh-CN | dark | only 2 elements collected (floor 6); last console error: Uncaught TypeError: Cannot read properties of undefined (reading 'length') |
| release-queued | zh-CN | light | only 2 elements collected (floor 6); last console error: Uncaught TypeError: Cannot read properties of undefined (reading 'length') |
| release-queued | en | dark | only 2 elements collected (floor 6); last console error: Uncaught TypeError: Cannot read properties of undefined (reading 'length') |
| release-queued | ja | dark | only 2 elements collected (floor 6); last console error: Uncaught TypeError: Cannot read properties of undefined (reading 'length') |
| release-queued | ko | dark | only 2 elements collected (floor 6); last console error: Uncaught TypeError: Cannot read properties of undefined (reading 'length') |
| release-queued | zh-TW | dark | only 2 elements collected (floor 6); last console error: Uncaught TypeError: Cannot read properties of undefined (reading 'length') |

## Totals

Findings are grouped per view by rule + root cause; counts below are element instances.

By rule:

| Rule | Category | Severity | Instances | Views |
| --- | --- | --- | --- | --- |
| spacing-grid | current | warning | 8769 | 91 |
| type-scale | current | warning | 1171 | 82 |
| type-weight | current | warning | 876 | 79 |
| overflow | current | error | 687 | 7 |
| contrast | current | error | 535 | 37 |
| shadow-panel | current | warning | 495 | 72 |
| color-token | current | warning | 211 | 25 |
| target-size | current | error | 163 | 50 |
| blur-scope | current | warning | 150 | 40 |
| radius-nesting | proposed | hint | 130 | 33 |
| surface-nesting | proposed | hint | 86 | 19 |
| radius-token | current | warning | 67 | 25 |
| one-accent | current | warning | 40 | 25 |
| border-density | proposed | hint | 10 | 10 |

By severity:

| Severity | Instances |
| --- | --- |
| warning | 11779 |
| error | 1385 |
| hint | 226 |

By functional area:

| Area | Instances |
| --- | --- |
| workshop | 6774 |
| control-center | 1935 |
| factory-map | 1724 |
| release-room | 1394 |
| feedback | 444 |
| library | 346 |
| environment | 146 |
| terms | 127 |
| resource-monitor | 114 |
| settings-licenses | 96 |
| materials-start | 96 |
| network-tiers | 72 |
| ai-preparation | 40 |
| task-window | 37 |
| first-run | 24 |
| settings | 21 |

By category (current standard vs proposed rules):

| Category | Instances |
| --- | --- |
| current | 13164 |
| proposed | 226 |

## Top 20 root causes — current standard

Ranked by how many element instances one style source explains; fixing one line often clears many findings. Suggested fixes are mechanical mappings (nearest token); the author decides the intended design.

| # | Rule | Root cause (selector @ file) | Views | Instances | Typical view | Suggestion |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | spacing-grid | `.vua-line__status @ apps/desktop/src/renderer/features/amf/factory.css` | 38 | 1908 | workshop-running | move to 4px (4 px grid) |
| 2 | spacing-grid | `.vua-workshop__line @ apps/desktop/src/renderer/features/amf/factory.css` | 33 | 1716 | workshop-running | move to 8px (4 px grid) |
| 3 | type-scale | `.vua-line__status @ apps/desktop/src/renderer/features/amf/factory.css` | 34 | 473 | workshop-running | use --vua-font-caption (12/16) |
| 4 | spacing-grid | `user-agent stylesheet: input[type="button" i], input[type="submit" i], input[type="reset" i], input[type="file" i]::-webkit-file-upload-button, button` | 74 | 312 | workshop-running | move to 4px (4 px grid) |
| 5 | spacing-grid | `user-agent stylesheet: p` | 36 | 244 | workshop-running | move to 12px (4 px grid) |
| 6 | spacing-grid | `.vua-workshop__capacity-choices button @ apps/desktop/src/renderer/features/amf/factory.css` | 33 | 198 | workshop-running | move to 8px (4 px grid) |
| 7 | spacing-grid | `.vua-room__bar @ apps/desktop/src/renderer/features/amf/factory.css` | 64 | 192 | workshop-running | move to 12px (4 px grid) |
| 8 | spacing-grid | `.vua-room__name @ apps/desktop/src/renderer/features/amf/factory.css` | 64 | 192 | workshop-running | move to 8px (4 px grid) |
| 9 | type-weight | `user-agent stylesheet: strong, b` | 39 | 191 | workshop-running | use weight 400 or 600 |
| 10 | spacing-grid | `.vua-line__station @ apps/desktop/src/renderer/features/amf/factory.css` | 31 | 186 | workshop-running | move to 8px (4 px grid) |
| 11 | type-scale | `.vua-line__station small @ apps/desktop/src/renderer/features/amf/factory.css` | 31 | 186 | workshop-running | use --vua-font-caption (12/16) |
| 12 | spacing-grid | `.vua-factory__enter @ apps/desktop/src/renderer/features/amf/factory.css` | 6 | 174 | map | move to 4px (4 px grid) |
| 13 | spacing-grid | `.vua-control__part @ apps/desktop/src/renderer/features/amf/control-center.css` | 7 | 168 | control-recipes | move to 8px (4 px grid) |
| 14 | shadow-panel | `.vua-card @ apps/desktop/src/renderer/components/primitives/card.css` | 40 | 150 | workshop-running | remove the shadow (§4: no shadow on ordinary panels/cards) |
| 15 | blur-scope | `.vua-card @ apps/desktop/src/renderer/components/primitives/card.css` | 40 | 150 | workshop-running | limit backdrop-filter to top/side/task/elevated structure |
| 16 | spacing-grid | `.vua-control__switch button @ apps/desktop/src/renderer/features/amf/control-center.css` | 13 | 130 | control-recipes | move to 4px (4 px grid) |
| 17 | spacing-grid | `.vua-room__tabs button @ apps/desktop/src/renderer/features/amf/factory.css` | 16 | 128 | control-recipes | move to 4px (4 px grid) |
| 18 | spacing-grid | `.vua-mode-picker__button @ apps/desktop/src/renderer/features/amf/factory.css` | 31 | 124 | workshop-running | move to 8px (4 px grid) |
| 19 | spacing-grid | `.vua-factory__label @ apps/desktop/src/renderer/features/amf/factory.css` | 6 | 120 | map | move to 16px (4 px grid) |
| 20 | spacing-grid | `.vua-library-list__title > button @ apps/desktop/src/renderer/features/warehouse/warehouse.css` | 2 | 112 | library-wall | move to 4px (4 px grid) |

## Top 20 root causes — proposed rules (not yet VUA standard)

Ranked by how many element instances one style source explains; fixing one line often clears many findings. Suggested fixes are mechanical mappings (nearest token); the author decides the intended design.

| # | Rule | Root cause (selector @ file) | Views | Instances | Typical view | Suggestion |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | surface-nesting | `span.vua-control__shared-dot @ (no matching rule; area: control-center)` | 7 | 48 | control-recipes | flatten nested bordered/filled surfaces (proposed rule) |
| 2 | radius-nesting | `section.vua-album__page @ (no matching rule; area: release-room)` | 12 | 26 | release-album | step radius down per nesting level (proposed rule) |
| 3 | radius-nesting | `button.vua-revisions__row @ (no matching rule; area: workshop)` | 4 | 20 | workshop-versions | step radius down per nesting level (proposed rule) |
| 4 | radius-nesting | `li @ (no matching rule; area: workshop)` | 6 | 13 | workshop-waiting-auto | step radius down per nesting level (proposed rule) |
| 5 | radius-nesting | `div.vua-sdk-review__rank @ (no matching rule; area: release-room)` | 11 | 12 | release-album | step radius down per nesting level (proposed rule) |
| 6 | radius-nesting | `button.vua-album__dogear @ (no matching rule; area: release-room)` | 11 | 12 | release-album | step radius down per nesting level (proposed rule) |
| 7 | border-density | `unattributed` | 10 | 10 | control-recipes | reduce bordered elements; separate with spacing/hierarchy (proposed rule) |
| 8 | surface-nesting | `select.vua-control__version @ (no matching rule; area: control-center)` | 2 | 10 | control-packages-project | flatten nested bordered/filled surfaces (proposed rule) |
| 9 | surface-nesting | `li @ (no matching rule; area: release-room)` | 2 | 10 | release-signin | flatten nested bordered/filled surfaces (proposed rule) |
| 10 | radius-nesting | `button.vua-n3__recipe @ (no matching rule; area: feedback)` | 2 | 6 | feedback-tiers | step radius down per nesting level (proposed rule) |
| 11 | radius-nesting | `label.vua-n3__material @ (no matching rule; area: feedback)` | 2 | 6 | feedback-tiers | step radius down per nesting level (proposed rule) |
| 12 | radius-nesting | `button.vua-network__region-test @ (no matching rule; area: network-tiers)` | 1 | 4 | network-tiers-details | step radius down per nesting level (proposed rule) |
| 13 | radius-nesting | `button.vua-network__region-test @ (no matching rule; area: environment)` | 1 | 4 | network-check | step radius down per nesting level (proposed rule) |
| 14 | radius-nesting | `span.vua-usage-panel__bar-fill @ (no matching rule; area: resource-monitor)` | 1 | 4 | resource-monitor | step radius down per nesting level (proposed rule) |
| 15 | radius-nesting | `label.vua-n3__material @ (no matching rule; area: workshop)` | 1 | 3 | workshop-materials | step radius down per nesting level (proposed rule) |
| 16 | radius-nesting | `li.vua-network__site @ (no matching rule; area: network-tiers)` | 1 | 3 | network-tiers-details | step radius down per nesting level (proposed rule) |
| 17 | radius-nesting | `li.vua-network__site @ (no matching rule; area: environment)` | 1 | 3 | network-check | step radius down per nesting level (proposed rule) |
| 18 | radius-nesting | `button.vua-ft__menu @ (no matching rule; area: workshop)` | 2 | 2 | workshop-ft | step radius down per nesting level (proposed rule) |
| 19 | surface-nesting | `code @ (no matching rule; area: release-room)` | 2 | 2 | release-uploaded | flatten nested bordered/filled surfaces (proposed rule) |
| 20 | surface-nesting | `div.vua-vrchat-check @ (no matching rule; area: release-room)` | 2 | 2 | release-uploaded | flatten nested bordered/filled surfaces (proposed rule) |

## Per-page card styles vs the shared Card

The shared `Card` primitive (`components/primitives/card.css`) is imported by 41 files. The CSS scan found 15 other card-style components:

| Card component | Defined in | Radius | Padding | Border | Background | Shadow | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `.preview-lab` (4 rules) | apps/desktop/src/renderer/dev/preview-lab/preview-lab.css | 10px | — | 1px solid #2e2a45 | #151322 | — | close to Card; align padding/border, then replace |
| `.vua-card` (1 rules) | apps/desktop/src/renderer/dev/showcase.css | — | — | — | — | — | keep as a variant; record the token it should use |
| `.vua-card-entry` (7 rules) | apps/desktop/src/renderer/features/home/home.css | — | — | — | — | — | keep as a variant; record the token it should use |
| `.vua-environment-card` (25 rules) | apps/desktop/src/renderer/features/home/home.css | 12px | — | 1px solid #2e2a45 | color-mix(in srgb, #151322 75%, transparent) | — | keep as a variant; record the token it should use |
| `.vua-exp-card` (4 rules) | apps/desktop/src/renderer/features/settings/settings.css | — | — | — | — | — | keep as a variant; record the token it should use |
| `.vua-flow` (5 rules) | apps/desktop/src/renderer/features/warehouse/warehouse.css | — | — | — | — | — | keep as a variant; record the token it should use |
| `.vua-manager-card` (3 rules) | apps/desktop/src/renderer/features/home/home.css | — | — | — | — | — | keep as a variant; record the token it should use |
| `.vua-mode-picker` (2 rules) | apps/desktop/src/renderer/features/amf/factory.css | 10px | 6px | 1px solid #2e2a45 | #1e1b30 | 0 10px 30px color-mix(in srgb, #0b0a12 55%, transparent) | keep as a variant; record the token it should use |
| `.vua-network` (2 rules) | apps/desktop/src/renderer/features/deployer/website-tests.css | — | 24px | — | — | — | keep as a variant; record the token it should use |
| `.vua-recipe-canvas-card` (1 rules) | apps/desktop/src/renderer/features/recipe/recipe.css | — | — | — | — | — | keep as a variant; record the token it should use |
| `.vua-release-card` (8 rules) | apps/desktop/src/renderer/features/release/release.css | 10px | 12px | 1px solid #2e2a45 | #151322 | none | close to Card; align padding/border, then replace |
| `.vua-tool-card` (8 rules) | apps/desktop/src/renderer/features/tools/tools.css | 10px | 16px | 1px solid #2e2a45 | #151322 | — | close to Card; align padding/border, then replace |
| `.vua-tour` (7 rules) | apps/desktop/src/renderer/features/tour/tour.css | — | var(--vua-space-6, 32px) | 0 | transparent | none | keep as a variant; record the token it should use |
| `.vua-vrchat-shelf` (5 rules) | apps/desktop/src/renderer/features/amf/release/release-album.css | 10px | 8px | 1px solid #2e2a45 | #151322 | — | close to Card; align padding/border, then replace |
| `.vua-warehouse-card` (29 rules) | apps/desktop/src/renderer/features/warehouse/warehouse.css | 10px | — | 1px solid #2e2a45 | color-mix(in srgb, #151322 78%, transparent) | — | close to Card; align padding/border, then replace |

## Static checks (renderer CSS + TSX inline styles)

- Hardcoded font sizes: 63 in CSS, 0 in TSX.
- Hardcoded border radii: 42 in CSS, 0 in TSX.
- Hardcoded hex colors: 9 in CSS, 4 in TSX.

### CSS — hardcoded font-size (63)

| File:line | Value | Selector / context | Snippet |
| --- | --- | --- | --- |
| `src/renderer/app-shell.css:336` | 0.86em | .vua-shell__sidebar:not(:hover):not(:focus-within) .vua-shell__sidebar-item | `font-size: 0.86em` |
| `src/renderer/app-shell.css:342` | 1.14em | .vua-shell__sidebar-item:hover | `font-size: 1.14em` |
| `src/renderer/app-shell.css:350` | 1.14em | .vua-shell__sidebar-item[aria-current="page"],
.vua-shell__sidebar-item[aria-current="page"]:hover | `font-size: 1.14em` |
| `src/renderer/components/primitives/confirm-dialog.css:40` | 1.25rem | .vua-confirm-dialog__close | `font-size: 1.25rem` |
| `src/renderer/components/primitives/content-dialog.css:43` | 1.25rem | .vua-content-dialog__close | `font-size: 1.25rem` |
| `src/renderer/components/primitives/media-slot.css:36` | 13px | .vua-media-slot__note | `font-size: 13px` |
| `src/renderer/features/ai-requests/ai-requests.css:8` | 13px | .vua-ai-review pre | `font-size: 13px` |
| `src/renderer/features/amf/amf-production.css:5` | 13px | .vua-n3__track li > span | `font-size: 13px` |
| `src/renderer/features/amf/amf-production.css:52` | 13px | .vua-n3__preview figcaption | `font-size: 13px` |
| `src/renderer/features/amf/factory.css:53` | 11px | .vua-factory__tag | `font-size: 11px` |
| `src/renderer/features/amf/factory.css:54` | 26px | .vua-factory__name | `font-size: 26px` |
| `src/renderer/features/amf/factory.css:69` | 22px | .vua-factory__panel h2 | `font-size: 22px` |
| `src/renderer/features/amf/factory.css:98` | 11px | .vua-factory__stations > span | `font-size: 11px` |
| `src/renderer/features/amf/factory.css:160` | 12px | .vua-line__status | `font-size: 12px` |
| `src/renderer/features/amf/factory.css:192` | 11px | .vua-line__station small | `font-size: 11px` |
| `src/renderer/features/amf/factory.css:194` | 11px | .vua-line__stop | `font-size: 11px` |
| `src/renderer/features/amf/factory.css:237` | 12px | .vua-start__fits | `font-size: 12px` |
| `src/renderer/features/deployer/website-tests.css:4` | 20px | .vua-network__header h2 | `font-size: 20px` |
| `src/renderer/features/deployer/website-tests.css:8` | 30px | .vua-network__icon-button | `font-size: 30px` |
| `src/renderer/features/deployer/website-tests.css:17` | 20px | .vua-network__site strong | `font-size: 20px` |
| `src/renderer/features/deployer/website-tests.css:20` | 22px | .vua-network__edit | `font-size: 22px` |
| `src/renderer/features/game-guide/game-guide.css:23` | 14px | .vua-game-guide__panel | `font-size: 14px` |
| `src/renderer/features/game-guide/game-guide.css:66` | 0.92em | .vua-game-guide__title | `font-size: 0.92em` |
| `src/renderer/features/game-guide/game-guide.css:121` | 0.85em | .vua-game-guide__follow | `font-size: 0.85em` |
| `src/renderer/features/game-guide/game-guide.css:135` | 0.78em | .vua-game-guide__follow-status | `font-size: 0.78em` |
| `src/renderer/features/game-guide/game-guide.css:161` | 0.82em | .vua-game-guide__counter | `font-size: 0.82em` |
| `src/renderer/features/game-guide/game-guide.css:167` | 1.12em | .vua-game-guide__step-title | `font-size: 1.12em` |
| `src/renderer/features/help/ai-tools.css:4` | 14px | .vua-ai-tools__prompt | `font-size: 14px` |
| `src/renderer/features/home/home.css:101` | 18px | .vua-compact-environments .vua-environment-card strong | `font-size: 18px` |
| `src/renderer/features/home/home.css:102` | 16px | .vua-tools-hub .vua-environment-card strong | `font-size: 16px` |
| `src/renderer/features/import/folder-picker.css:37` | 1.25rem | .vua-folder-picker__close | `font-size: 1.25rem` |
| `src/renderer/features/overlay/overlay.css:49` | 0.86em | .vua-overlay__title | `font-size: 0.86em` |
| `src/renderer/features/overlay/overlay.css:130` | 1.15em | .vua-overlay-guide__title | `font-size: 1.15em` |
| `src/renderer/features/overlay/overlay.css:149` | 0.95em | .vua-overlay-guide__section-title | `font-size: 0.95em` |
| `src/renderer/features/overlay/overlay.css:169` | 0.85em | .vua-overlay-guide__caption | `font-size: 0.85em` |
| `src/renderer/features/overlay/overlay.css:214` | 1.15em | .vua-overlay__status-title | `font-size: 1.15em` |
| `src/renderer/features/overlay/overlay.css:221` | 0.93em | .vua-overlay__status-detail | `font-size: 0.93em` |
| `src/renderer/features/overlay/overlay.css:227` | 0.86em | .vua-overlay__section-label | `font-size: 0.86em` |
| `src/renderer/features/overlay/overlay.css:233` | 1em | .vua-overlay__task-title | `font-size: 1em` |
| `src/renderer/features/overlay/overlay.css:240` | 0.93em | .vua-overlay__task-meta | `font-size: 0.93em` |
| `src/renderer/features/overlay/overlay.css:293` | 0.93em | .vua-overlay__production-meta | `font-size: 0.93em` |
| `src/renderer/features/overlay/overlay.css:312` | 0.86em | .vua-overlay__hint | `font-size: 0.86em` |
| `src/renderer/features/overlay/overlay.css:318` | 0.86em | .vua-overlay__reason | `font-size: 0.86em` |
| `src/renderer/features/overlay/overlay.css:342` | 1em | .vua-overlay--vr .vua-overlay__title | `font-size: 1em` |
| `src/renderer/features/overlay/overlay.css:362` | 1.3em | .vua-overlay--vr .vua-overlay__status-title | `font-size: 1.3em` |
| `src/renderer/features/overlay/overlay.css:379` | 1.1em | .vua-overlay--vr .vua-overlay__task-title | `font-size: 1.1em` |
| `src/renderer/features/overlay/overlay.css:387` | 1em | .vua-overlay--vr .vua-overlay__actions .vua-button | `font-size: 1em` |
| `src/renderer/features/overlay/overlay.css:397` | 0.9em | .vua-overlay--vr .vua-overlay__hint,
.vua-overlay--vr .vua-overlay__reason | `font-size: 0.9em` |
| `src/renderer/features/packages/packages.css:473` | 12px | .vua-packages__legacy-list | `font-size: 12px` |
| `src/renderer/features/reader/reader.css:1` | 15px | .vua-reader | `font-size: 15px` |
| `src/renderer/features/task-center/notification-popover.css:27` | 10px | .vua-shell__notify-badge | `font-size: 10px` |
| `src/renderer/features/tour/tour.css:32` | clamp(28px, 3vw, 42px) | .vua-tour__card--welcome .vua-tour__title | `font-size: clamp(28px, 3vw, 42px)` |
| `src/renderer/features/tour/tour.css:33` | 16px | .vua-tour__card--welcome .vua-tour__body | `font-size: 16px` |
| `src/renderer/features/tour/tour.css:51` | 14px | .vua-tour__card | `font-size: 14px` |
| `src/renderer/features/tour/tour.css:73` | 0.82em | .vua-tour__counter | `font-size: 0.82em` |
| `src/renderer/features/tour/tour.css:79` | 1.1em | .vua-tour__title | `font-size: 1.1em` |
| `src/renderer/features/tour/tour.css:91` | 0.92em | .vua-tour__absent | `font-size: 0.92em` |
| `src/renderer/features/warehouse/warehouse.css:566` | 12px | .vua-warehouse-detail__debug | `font-size: 12px` |
| `src/renderer/features/warehouse/warehouse.css:689` | 28px | .vua-warehouse-detail-album__zone | `font-size: 28px` |
| `src/renderer/features/warehouse/warehouse.css:750` | 18px | .vua-warehouse-lightbox__close | `font-size: 18px` |
| `src/renderer/features/warehouse/warehouse.css:1034` | 1rem | .vua-warehouse__group-title | `font-size: 1rem` |
| `src/renderer/features/warehouse/warehouse.css:1050` | 0.875rem | .vua-warehouse-card__variant | `font-size: 0.875rem` |
| `src/renderer/features/warehouse/warehouse.css:1177` | 0.75rem | .vua-compatible-items__kind | `font-size: 0.75rem` |

### CSS — hardcoded border-radius (42)

| File:line | Value | Selector / context | Snippet |
| --- | --- | --- | --- |
| `src/renderer/app/login-browser.css:17` | 8px | .vua-login-browser__toast | `border-radius: 8px` |
| `src/renderer/app/login-browser.css:42` | 6px | .vua-login-browser__toast-cancel | `border-radius: 6px` |
| `src/renderer/components/browser-frame.css:7` | 8px 8px 0 0 | .vua-browser-frame__tab | `border-radius: 8px 8px 0 0` |
| `src/renderer/components/browser-frame.css:14` | 6px | .vua-browser-frame__new-tab | `border-radius: 6px` |
| `src/renderer/components/browser-frame.css:17` | 6px | .vua-browser-frame__shortcuts button | `border-radius: 6px` |
| `src/renderer/components/primitives/toggle.css:1` | 12px | .vua-toggle | `border-radius: 12px` |
| `src/renderer/features/amf/amf-production.css:11` | 12px | .vua-n3__material | `border-radius: 12px` |
| `src/renderer/features/amf/amf-production.css:23` | 9px | .vua-n3 input:not([type=checkbox]), .vua-n3 select | `border-radius: 9px` |
| `src/renderer/features/amf/amf-production.css:37` | 12px | .vua-n3__recipe | `border-radius: 12px` |
| `src/renderer/features/amf/amf-production.css:46` | 12px | .vua-n3__gallery button | `border-radius: 12px` |
| `src/renderer/features/amf/amf-production.css:51` | 9px | .vua-n3__preview img, .vua-n3__preview-frame | `border-radius: 9px` |
| `src/renderer/features/amf/amf-production.css:63` | 9px | .vua-recipe-transfer input:not([type=checkbox]), .vua-recipe-transfer select, .vua-recipe-transfer textarea | `border-radius: 9px` |
| `src/renderer/features/amf/amf-production.css:68` | 10px | .vua-recipe-transfer__materials li, .vua-recipe-transfer__products li | `border-radius: 10px` |
| `src/renderer/features/amf/amf-production.css:72` | 12px | .vua-recipe-transfer__picker, .vua-recipe-transfer__preview | `border-radius: 12px` |
| `src/renderer/features/amf/amf-production.css:85` | 10px | .vua-recipe-apply__choices label | `border-radius: 10px` |
| `src/renderer/features/amf/amf-production.css:90` | 12px | .vua-ft__menu | `border-radius: 12px` |
| `src/renderer/features/amf/amf-production.css:102` | 12px | .vua-revisions__row | `border-radius: 12px` |
| `src/renderer/features/amf/amf-production.css:126` | 12px | .vua-plan-images li | `border-radius: 12px` |
| `src/renderer/features/amf/amf-production.css:127` | 8px | .vua-plan-images img, .vua-plan-images__blank | `border-radius: 8px` |
| `src/renderer/features/amf/release/release-album.css:227` | 4px | .vua-release-guide__field | `border-radius: 4px` |
| `src/renderer/features/amf/release/release-album.css:229` | 4px | .vua-release-guide__button | `border-radius: 4px` |
| `src/renderer/features/deployer/website-tests.css:5` | 10px | .vua-network__heading-icon | `border-radius: 10px` |
| `src/renderer/features/deployer/website-tests.css:12` | 14px | .vua-network__site | `border-radius: 14px` |
| `src/renderer/features/deployer/website-tests.css:20` | 5px | .vua-network__edit | `border-radius: 5px` |
| `src/renderer/features/help/ai-tools.css:4` | 12px | .vua-ai-tools__prompt | `border-radius: 12px` |
| `src/renderer/features/help/help.css:2` | 12px | .vua-knowledge__body | `border-radius: 12px` |
| `src/renderer/features/home/home.css:6` | 12px | .vua-route-tile | `border-radius: 12px` |
| `src/renderer/features/home/home.css:13` | 5px | .vua-route-tile__tag | `border-radius: 5px` |
| `src/renderer/features/home/home.css:26` | 8px | .vua-shell__logo-home | `border-radius: 8px` |
| `src/renderer/features/home/home.css:60` | 12px | .vua-environment-card | `border-radius: 12px` |
| `src/renderer/features/home/home.css:77` | 12px | .vua-environment-detail | `border-radius: 12px` |
| `src/renderer/features/home/home.css:81` | 8px | .vua-environment-software li | `border-radius: 8px` |
| `src/renderer/features/resource-monitor/resource-monitor.css:105` | 3px | .vua-usage-panel__bar | `border-radius: 3px` |
| `src/renderer/features/task-center/notification-popover.css:27` | 7px | .vua-shell__notify-badge | `border-radius: 7px` |
| `src/renderer/features/task-center/taskbar.css:204` | 3px | .vua-taskbar__download-progress | `border-radius: 3px` |
| `src/renderer/features/task-center/taskbar.css:209` | 3px | .vua-taskbar__download-progress::-webkit-progress-bar | `border-radius: 3px` |
| `src/renderer/features/task-center/taskbar.css:210` | 3px | .vua-taskbar__download-progress::-webkit-progress-value | `border-radius: 3px` |
| `src/renderer/features/warehouse/edit-source-dialog.css:3` | 6px | .vua-edit-source input, .vua-edit-source select | `border-radius: 6px` |
| `src/renderer/features/warehouse/edit-source-dialog.css:5` | 8px | .vua-edit-source__image | `border-radius: 8px` |
| `src/renderer/features/warehouse/warehouse.css:1106` | 6px | .vua-add-to-recipe__row | `border-radius: 6px` |
| `src/renderer/features/warehouse/warehouse.css:1157` | 6px | .vua-compatible-items__row | `border-radius: 6px` |
| `src/renderer/features/warehouse/warehouse.css:1235` | 6px | .vua-download-checklist__row | `border-radius: 6px` |

### CSS — hardcoded hex-color (9)

| File:line | Value(s) | Selector / context | Snippet |
| --- | --- | --- | --- |
| `src/renderer/components/primitives/toggle.css:31` | #ffffff | .vua-toggle[data-on]::after | `background: #ffffff` |
| `src/renderer/components/splash/boot-splash.css:13` | #fff | .vua-boot-splash__surface | `background: #fff` |
| `src/renderer/components/splash/boot-splash.css:25` | #000 | .vua-boot-splash__ray | `background: #000` |
| `src/renderer/components/splash/boot-splash.css:43` | #fff | .vua-boot-splash__loader | `border-top-color: #fff` |
| `src/renderer/components/splash/boot-splash.css:59` | #000 | to | `background: #000` |
| `src/renderer/components/splash/boot-splash.css:62` | #000 | .vua-boot-splash__surface[data-static] | `background: #000` |
| `src/renderer/components/splash/boot-splash.css:65` | #fff | html[data-vua-surface="splash"], html[data-vua-surface="splash"] body, html[data-vua-surface="splash"] #root | `background: #fff` |
| `src/renderer/components/splash/boot-splash.css:66` | #fff | html[data-vua-surface="splash"] .vua-boot-splash | `background: #fff` |
| `src/renderer/components/splash/boot-splash.css:69` | #000 | .vua-boot-splash__surface | `background: #000` |

### TSX inline styles — hardcoded hex-color (4)

| File:line | Value(s) | Selector / context | Snippet |
| --- | --- | --- | --- |
| `src/renderer/features/amf/RoomCanvas.tsx:28` | #ff7a45, #16141f, #1f1c2b |  | `room: v("--room", "#ff7a45"), panel: v("--vua-bg-panel", "#16141f"), elevated: v("--vua-bg` |
| `src/renderer/features/amf/RoomCanvas.tsx:29` | #3a3550, #f3f2f8, #a9a6b8 |  | `border: v("--vua-border-strong", "#3a3550"), strong: v("--vua-text-strong", "#f3f2f8"), te` |
| `src/renderer/features/amf/RoomCanvas.tsx:30` | #2a2638, #4ade80, #facc15 |  | `muted: v("--vua-border", "#2a2638"), success: v("--vua-success", "#4ade80"), warning: v("-` |
| `src/renderer/features/amf/RoomCanvas.tsx:31` | #f87171, #ff7a45, #f4cdb0, #f3f2f8 |  | `error: v("--vua-error", "#f87171"), accent: v("--vua-accent", "#ff7a45"), skin: v("--vua-s` |

## Proposed rules (ZCode-inspired, not yet VUA standard)

- `radius-nesting`: 130 instances across 33 views.
- `surface-nesting`: 86 instances across 19 views.
- `border-density`: 10 instances across 10 views.
Adopting them is a standard change and needs the author's ruling; the counts above estimate the blast radius.

## What the numbers say

Reading order for fixes: the current-standard table is dominated by two sources. `factory.css`
alone explains the eight largest spacing/type root causes (`.vua-line__status`,
`.vua-workshop__line`, `.vua-room__bar`, `.vua-room__name`, `.vua-line__station`, … — about
5,200 instances combined), and Chromium's user-agent defaults (`button` padding, `p` margins,
`strong` 700) explain another ~750. The shared `Card` primitive is the single largest
shadow/blur source (`.vua-card`: 150 shadow + 150 blur instances — a card wall using
backdrop-filter and hover glow, which §4 disallows on card walls). After factory.css,
`.vua-button--subtle` (light-theme contrast) remains the most impactful shared component.

- **spacing-grid (8,769 across 91 views)** — the dominant drift. Mostly 6/10/14/18/22 px steps
  in `factory.css`, plus elements relying on UA default margins. Every value is within 2 px of
  a grid step, so a mechanical pass mapping each to `--vua-space-*` is low-risk.
- **type-scale / type-weight (1,171 + 876)** — factory.css uses an 11 px tag size and 650/700
  weights; the overlay/game-guide/tour CSS uses `em`-relative sizes (0.78–1.3 em) that have no
  token mapping at all. Either these are intended styles that need token definitions of their
  own, or they drift to 12 px/400/600 — the author decides.
- **contrast (535 across 37 views)** — dark theme is nearly clean; the real failures are light
  theme: `.vua-button--subtle` below 4.5:1, the factory room muted/hint/plate text family, and
  `.vua-control__shared-dot`. These are the WCAG-gate items a fix wave should start from.
- **shadow-panel (495) / blur-scope (150)** — mostly the shared `.vua-card` glass treatment and
  per-page cards carrying decorative shadows; one decision (whether card walls stay glass at
  all) resolves the largest share.
- **target-size (163 across 50 views)** — sidebar idle labels (152×20), small close/expand
  buttons, and status chips below the 24 px floor.
- **overflow (687 across 7 views)** — concentrated in the base map (map-control/map-release
  scene elements poking beyond the window edge) and the control-center packages table. Both
  deserve a look at their annotated screenshots; the map may be by-design scene clipping (its
  canvas layer is audit-skipped, the DOM room overlays are not).
- **one-accent (40 across 25 views)** — no view mixes purple and orange jurisdictions (the
  standard's hard rule holds everywhere); all findings are "N filled accent controls in one
  view", which is the heuristic's selected-state caveat and should be eyeballed before acting.
- **Proposed rules** — with double-counting removed, the blast radius is small: 226 instances
  total. `surface-nesting` (86) now points at concrete over-deep surfaces (status dots,
  selects, list items); `radius-nesting` (130) mostly wants album/revisions buttons stepped
  down one level. Do not adopt them without a ruling; the counts show the change is affordable.

## Suggestions to the standard (data-driven)

Where the current tokens ran out, in rough priority order:

1. **A fifth type step or an explicit small-caption rule.** 11 px tags and 13 px meta text
   appear throughout `factory.css`, `amf-production.css` and `media-slot.css`. If 11/13 px are
   intended, define them (e.g. a `--vua-font-micro`); otherwise map to 12/14. The `em`-based
   overlay/tour/guide styles need a stated relative-size rule — they are invisible to a
   px-token scale.
2. **Weight 650/700.** `factory.css` uses 650 repeatedly and `strong` falls through at 700. If
   a heavier step is wanted, add it to the token table; otherwise the fix is `font-weight: 600`.
3. **A glass-surface token.** Cards are built as `color-mix(in srgb, var(--vua-bg-panel) 75–78%,
   transparent)` in at least three files with slightly different ratios, and the shared Card
   adds backdrop-filter + hover glow. One `--vua-surface-glass` token (plus a stated blur
   allowance in §4) would collapse the shadow/blur findings into a single decision.
4. **UA-default normalization.** `p` margins and `button` padding from the user-agent
   stylesheet account for ~750 spacing findings. A reset (or explicit token-based defaults)
   in `base.css` removes a whole finding class from every future page.
5. **Machine-readable room palette for canvas code.** `RoomCanvas.tsx` re-declares token values
   as string literals (`#4ade80`, `#facc15`, `#f87171`, …) as fallbacks. Exporting the palette
   from `@vua/design-system` would keep canvas art from drifting when tokens change.
6. **Light-theme muted text.** The light variants of several muted/secondary colors sit below
   4.5:1 on white surfaces (factory muted/hint, subtle button). Either darken those light
   tokens or restrict them to large-text use.

## Baseline


- Refreshed the committed gate baseline with 2690 entries (one per rule + view + root cause).
- Future runs use `--check-baseline` to fail on findings outside this baseline; update it only when a fix lowers the violation count (`pnpm audit:design --update-baseline`).

## Limitations

- Aesthetic judgment, motion quality and information architecture are human review topics; the tool only checks the rules listed in docs/design/design-audit.md.
- Computed-style analysis sees one state per view. Hover, focus, pressed and disabled interactions, and states that require live Gateway traffic, are not exercised unless a fixture mounts them.
- Contrast is computed against alpha-composited backgrounds; gradient and image backgrounds fall back to their background-color layer, so text over busy art may be mis-graded.
- Attribution prefers the CDP cascade winner; hover-only rules (:hover) cannot be matched from the static state, so those findings fall back to class+area attribution.
- Overflow checks see the zh-CN layout for all views plus four extra locales in dark theme; light-theme text growth in other locales is not measured. The viewport branch reports horizontal overflow only — vertical growth beyond the fold is normal document scrolling.
- Primary-action detection is heuristic (opaque accent-filled controls); custom-selected states can inflate the one-accent count.
- data-audit-skip regions (base-map scene canvas, turntable frames, WebGL scenes, product imagery) are excluded by design and not audited.

## Machine-readable artifacts

- `shots/<view>--<theme>.png` and `<view>--<theme>.annot.png`: clean and annotated screenshots (default locale).
- `json/<view>--<locale>--<theme>.json`: grouped findings per state.
- `index.html`: filterable overview (by rule, area, severity) linking screenshots and finding lists.
- `baseline.json`: committed gate input; keys are `rule|view|root cause`.
