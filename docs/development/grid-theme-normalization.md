# Grid theme normalization

> Updated: 2026-10-25
> Status: execution plan
> Scope: reliable design auditing, the default Grid theme and verified interface defects

The author requested audit reliability and false-positive calibration first, this plan after
reading [Aran52/VUA#67](https://github.com/Aran52/VUA/pull/67), then correction of the real
problems. This is plan 53's implementation route. The [design standard](../design/design-standard.md),
[controls and motion](../design/controls-and-motion.md) and [skin system](skin-system.md) own
the rules; the [audit process](../design/design-audit.md) owns measurement.

## Starting point

PR #67 supplies four type steps, two weights, solid ordinary cards, radius aliases and the
approved Grid control direction. It does not finish the control catalogue or migrate every
feature. The implementation still combines pill actions, rounded fields/cards, skewed shell
tabs and factory cut corners. Several motion and size tokens retain earlier values.

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
