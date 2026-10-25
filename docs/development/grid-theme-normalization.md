# Grid theme normalization

> Updated: 2026-10-25
> Status: calibrated repairs delivered; Grid geometry revision awaiting author review in the dev app
> Scope: reliable design auditing, the default Grid theme and verified interface defects

The author requested audit reliability and false-positive calibration first, this plan after
reading [Aran52/VUA#67](https://github.com/Aran52/VUA/pull/67), then correction of the real
problems. This is plan 53's implementation route. The [design standard](../design/design-standard.md),
[controls and motion](../design/controls-and-motion.md) and [skin system](skin-system.md) own
the rules; the [audit process](../design/design-audit.md) owns measurement.

## Starting point

PR #67 supplies four type steps, two weights, solid ordinary cards, radius aliases and the
approved Grid control direction. It does not finish the control catalogue or migrate every
feature. At that starting revision the implementation combined pill actions, rounded
fields/cards, skewed shell tabs and factory cut corners. Several motion and size tokens
retained earlier values.

The first post-merge CI report, run `40130221511`, collected 568 of 594 planned states. Its
summary omitted 26 execution failures; per-state JSON contained the run plan instead of the
findings. Most viewport-overflow findings came from the clipped factory camera, and checkbox
targets omitted associated labels. Those counts are historical diagnostic evidence, not an
accepted baseline for deciding which UI changes worked.

## 1. Make measurement trustworthy

- Reconcile each planned view/locale/theme state with exactly one outcome. Any execution,
  navigation, annotation or capture failure makes the run fail and appears in both reports.
- Save actual per-state findings, element geometry and root-cause groups. Verify that an
  annotated screenshot actually contains its annotation layer.
- Measure painted geometry through clipping ancestors; keep checks for ordinary clipped text
  and real horizontal leakage. Include visible associated label fragments in checkbox/radio
  hit areas without joining separate fragments.
- Exclude invisible elements, unused gaps, actual auto margins, separator edges and controls
  from the measurements where they do not apply. Follow the approved inner-not-larger rule.
  Preserve synthetic regressions for real small targets, contrast, spacing and clipped text.
- Repair fixture selection, current navigation stubs, language-dependent assertions and waits
  for actual dialog completion. Keep assertions that expose product defects.
  The retired name-based dependency dialog is replaced in the audit by the current inline
  description-link filter required by the [product boundary](../product-boundary.md).
- Baselines retain the worst count/severity across locales and reject count growth, severity
  growth and new groups. Only a complete full run can update the committed baseline.

**Acceptance:** partial, failed and empty runs return failure; reports and state accounting
agree; the calibrated full run has no execution or capture failures. Calibration and visual
corrections have separate commits and evidence, so their effects are distinguishable.

## 2. Normalize Grid from shared foundations

The complete theme migration follows these paths. Each path includes dark/light, keyboard
and reduced-motion review before the next feature area moves.

| Path | Implementation | Acceptance |
| --- | --- | --- |
| Tokens and shape renderer | Role-group corner tokens, size steps, stroke/fill and motion tokens in `packages/design-system`; legacy names alias replacements during migration | One shared cut-corner implementation draws the diagonal stroke and visible focus; forced colours preserve both |
| Actions and choices | Button/icon actions, tabs/segmented choices, checkbox/radio and toggle states | One rest/hover/pressed/selected/focus/disabled/busy ladder; labels and keyboard behaviour remain intact |
| Fields and indicators | Text/search/select fields, tags/status chips, badges and lights | Fields align with same-step actions; fixed-size indicators are distinct from their accessible hit regions |
| Containers and composition | Card/panel/list row, plain section header, key-value row, toolbar and stat tile where repeated layouts need them | At most two real container surfaces, inner corners no larger, solid ordinary panel fills |
| Overlays | Menus, tooltips, dialogs, drawers and toast hosts | Shared corners/strokes, appropriate elevation, visible close/focus and correct modal ownership |
| Shell and feature migration | Shell, workshop, control centre, release, library, environment/help/settings | Features consume shared roles; each area has a before/after sheet and retains its functional routes |
| Motion | CSS and scripted motion consume the owning tokens; loops represent real activity | Immediate response; reduced motion and effects-off modes flatten motion without hiding state |

Native skin installation, community CSS execution and new backend behaviour stay with their
own scheduled work. Gateway permissions, task state, material ownership and confirmation
behaviour remain authoritative throughout presentation changes.

### Delivery order and exit checks

1. **Measurement and verified repairs (this PR).** Complete the calibrated comparison, repair
   confirmed defects and refresh the gate baseline from a complete run. Keep audit-policy changes
   separate from presentation changes in both commits and the comparison report.
2. **Foundation and shape implementation.** Add the role-group corner tokens and one shared
   shape renderer. Exercise diagonal strokes, inset focus, forced colours, nested cuts and
   overlay elevation in a primitive fixture before changing feature markup. Legacy token names
   alias the new roles while their consumers migrate.
3. **Primitive catalogue.** Implement the actions/choices first, then fields/indicators,
   containers and overlays. Each primitive exposes the three size steps where applicable and
   the complete state ladder. Verify accessible names, keyboard ownership, disabled/busy width
   and native-select behaviour; expand the audit's size/corner/motion checks with the catalogue.
4. **Shell, then feature areas.** Migrate the shell first, followed by workshop, control centre,
   release, library, environment/help and settings in separate reviewable batches. Each batch
   replaces bespoke recipes with the shared role and supplies a dark/light before/after sheet.
   Re-run its existing interaction fixture and the shared primitive regressions.
5. **Coverage and completion review.** Add settled fixtures for the audit's documented gaps,
   including settings categories, tools, wizard steps and separate windows. A catalogue entry
   is complete only when its consumers and states are represented; a green registered-view run alone
   cannot stand for these uncovered surfaces. Retire compatibility aliases after their final
   consumer moves.

The migration is complete when every default-theme control uses the shared role/size/state
recipe, literal corner and duration recipes have no unrecorded consumers, the registered audit
states complete without execution/capture failures or confirmed current-rule defects, and the
author has reviewed the affected keyboard/focus and visuals in the latest dev app. Screenshots
support that review and do not replace it. Device, real-material and
N-stage acceptance still follow their owning evidence rows.

## 3. Correct the verified findings

Use a frozen revision of the calibrated auditor for the before/after comparison. Repair
shared rules first, then inspect the remaining per-page findings:

1. Restore the clicked product's adaptation action in a multi-selection menu; its existing
   fixture must pass. Preserve the selection and existing permission/read behaviour.
2. Fix actual cropped content, undersized hit regions and insufficient text/icon contrast,
   starting with light-theme warning/terms text and shared status indicators.
3. Normalize workshop/control-centre spacing roots and inherited browser type defaults.
   Apply shared corrections to release, library and environment surfaces.
4. Remove ordinary-panel shadow/glass and excess container surfaces where the calibrated
   audit confirms them. Use named overlay and scene-art exceptions from the owning standard.
5. Review residual colour, accent and corner findings in screenshots. An exception requires
   an existing owning rule; a selector is never excluded merely to lower the count.

The current repair slice implements the confirmed defects and shared corrections exposed by
these runs. The table above also schedules the wider control-catalogue migration, including
surfaces that lack settled audit fixtures; its completion is tracked separately.

## Verification and evidence

- Run audit Node regressions, synthetic Electron tests, desktop types, relevant interaction
  fixtures and normal desktop checks.
- Run all registered states with the same viewport, locale/theme plan and calibrated rules.
  Record planned/audited counts, failures, rule totals and source revision.
- Keep raw reports and screenshots in ignored local directories and CI artifacts. Commit
  sanitized comparisons and a contact sheet for affected areas. Separate reductions caused
  by calibration from reductions caused by interface changes.
- Refresh the gate baseline only from a complete final run after reviewing its differences.
  A remaining confirmed defect stays explicit and prevents claiming that defect fixed.
- Human review covers understandable labels, discoverable actions, focus/keyboard behaviour
  and the before/after sheets. Automated screenshots are component evidence and do not close
  N-stage, physical-device or real-material acceptance rows.

The implementation PR records revisions, complete-run coverage, remaining findings and the
location of its before/after evidence. This plan reserves no product version and changes no
published release record.

The [calibration and repair comparison](../research/design-audit-calibration-2026-10-25.md)
records both complete 594-state runs: current-rule instances fall from 8,523 to zero, with
eight unchanged advisory hints. Those results apply to the revisions named in that report,
before the geometry revision below. They do not establish acceptance of the latest appearance.

## Author geometry review, 2026-10-25

The author rejected the earlier visual completion: industrial cuts were too weak, capsule
controls remained, station cards lacked mass, and absent refresh actions shifted repository
columns. The ruling now owned by [controls and motion](../design/controls-and-motion.md)
requires the following concrete revision:

- Every former capsule control consumes the shared role bevel. The cut is upper-left with
  a 120-degree interior angle matching the VUA mark; vertical cut depth is √3 × horizontal.
  Shell tabs lose their skew transform so it cannot change that angle.
- Workshop stations consume the larger container bevel. The selected station has a solid
  lower edge and `--vua-shadow-workshop-active`; resting stations stay flat. This is a named
  author exception to ordinary flat cards.
- Repository refresh stays visible and disabled when unavailable. Fixed action slots and a
  removal placeholder preserve the right edge of the middle columns.
- Acceptance happens in the latest native dev app, with interactive synthetic state review
  available for states the current local data cannot produce. No screenshot closes that review.

The native shared renderer uses the bundled Chromium's `corner-shape: bevel`, preserving
diagonal strokes, focus and shadow rather than clipping them. Regression tests check real
native pixels and hit testing, role isolation, incorrect corner/angle/capsule detection, and
the selected-station shadow exception. Rendered fixture assertions check station shape and
refresh-slot alignment, including unavailable lifecycle and missing-identifier rows.

A local targeted run on the working revision completed 16/16 zh-CN dark/light states with
no execution/capture failure, no current-rule finding and one existing advisory density hint.
The new unavailable-lifecycle fixture expands the full registry to 100 views / 600 states.
The complete new matrix and the author's interactive visual/keyboard acceptance are separate
from the historical 594-state comparison; both must be reported with their actual outcomes.
