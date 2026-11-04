# Grid hierarchy review

> Updated: 2026-10-25
> Scope: workshop, theme preferences and repositories, with Home and Play refinements
> Status: implemented; author review in the native dev app remains pending

The author adopted the third-party review of PR #68 at `c1498a3`, including its precedence
over the earlier heavy workshop-shadow request. The current rules live in
[controls and motion](../design/controls-and-motion.md); this record describes the delivered
interface and its evidence.

## Delivered hierarchy

| Area | Result |
| --- | --- |
| Workshop | Viewing uses a neutral edge and thin underline. Execution keeps its own icon, short status and progress connector. Future steps stay neutral, waiting steps use amber, and all station cards stay flat. The detail context names the viewed and executing steps independently. |
| Current object | The title uses the observed or selected Avatar, or the sole inspected candidate. Unknown identity stays explicit. A separate material count follows; full material names and line IDs live in collapsed monospace details. Section headings are quieter and rail close actions appear on hover, focus or selection. |
| AMF geometry | Primary buttons, station cards, major containers and large overlays retain the 120-degree top-left cut. Secondary actions, inputs, tags, progress and small menus use quiet rectangles. Host surfaces retain their existing rounded geometry. Global grid texture is reduced; the factory map retains its own scene grid. |
| Theme settings | One Display & performance group has three label/control rows, separators and a persistent resource-saving switch. The selected shell Settings entry uses a semantic tint with readable text. |
| Repositories | One header contains the page title, view switch and primary Add action. The bounded list labels its columns. Refresh and Remove are neutral at rest; Remove uses danger emphasis on hover and confirmation. Disabled refresh slots and package-count alignment remain stable. Not cached has a full tooltip explanation. |
| Home and Play | Modules, Help and Settings become compact utility actions. Tools, modules and settings have dedicated glyphs. Home shows actual active tasks and dated environment observations when available. Play places action and status closer together. |

## Settings evidence correction

The former `first-run-settings` audit called `firstRunReview.run()`. That routine remounted
the onboarding goal page at its end. The old screenshot therefore did not exercise theme
settings, and its zero count cannot support Settings acceptance.

The dedicated `previewSettings()` endpoint now asserts `#settings-theme`, the three actual
appearance choices, three preference rows and a resource-saving switch. It clicks the switch
and restores its original value before capture. The corrected **before** collection uses the
old `c1498a3` interface with the route and theme fixes, so both sides show the same Settings
page in the actual requested theme. That collection exposed three current findings in dark
and two in light: the Settings entry's background, dark text contrast and competing filled
commands. The interface changes address those causes.

A second capture defect was caught by inspecting the comparison sheet: `mount()` cleared
the shell's theme preference. The App therefore rendered dark even in Light-labelled Home,
Settings and Play states. Preview mounts now preserve the requested preference while the
ordinary first-run smoke still tests a new session's Dark default. The runner rejects any
snapshot whose actual `data-theme` differs from the requested theme. The superseded local
collections are not final evidence; the figures and final counts use the corrected run.

## Visual comparison

Each sheet contains the unchanged source screenshots, with dark and light pairs. The before
side is the `c1498a3` interface; the after side is this hierarchy revision. All use the real
components with a synthetic Gateway and the same fixture viewport.

- [Workshop, dark and light](assets/grid-hierarchy-workshop.png)
- [Theme settings, dark and light](assets/grid-hierarchy-settings.png)
- [Repositories, dark and light](assets/grid-hierarchy-repositories.png)

The images establish presentation differences. They do not establish real-material, hardware
or N-stage acceptance. Raw local collections and machine-specific observations remain ignored.

## Verification and limits

- Final filtered audit: **54/54 states, 6,882 elements, zero findings**, with no execution
  or capture failures. Nine views cover running/waiting/finished workshop states, repositories
  and unavailable lifecycle, actual Theme Settings, Home and Play. Each has zh-CN dark/light
  plus en/ja/ko/zh-TW dark overflow passes. Other locales' light-theme growth is not measured.
- Desktop unit suite: **1,472 passed, one existing skip**, with two workers. The initial
  unconstrained run lost two workers; its result is not a pass. Lower concurrency completed
  without unhandled errors. Type checking, production build, module isolation and the
  boundary, translation, contrast and leak checks pass.
- Audit regressions: **57 passed** (49 Node and eight actual Chromium cases). The existing
  first-run/theme/Play smoke passes **84 checks**, including reduced motion, resource saving,
  small-window Settings, launch-state handling and shared theme propagation.
- Three focused browser-shell cases pass: page and modal notifications retain their order,
  pointer/focus interaction and viewport hide/resume; the AMF-owned compact portal remains
  a quiet rectangle while preserving ownership independently of the host.

The rendered workshop regression clicks a future step and the executing step, verifies that
viewing cannot move execution, checks both names in the detail context, then restores the
running selection. Repository regressions exercise disabled refresh, fixed action slots,
right-aligned counts, the Add form and its disabled empty submit, then cancel. Settings
regressions check the route and actual preference controls, including a switch round trip.

Native dev-app checks cover the existing waiting line, future-step selection, dark/light
theme changes, the resource-saving switch and opening/cancelling the empty Add-repository
form. Changed display preferences are restored. Populated/disabled repository rows are
reviewed in the clearly labelled synthetic window.
The separate interactive window labels its synthetic data; its running line permits current
and future selection without starting a real Unity production task. Author visual acceptance
remains the next step.

The full registered audit remains 100 views / 600 locale/theme states. A filtered run only
supports its selected plan and does not refresh the committed full-matrix baseline. The older
optional `smoke:production-review` harness has previously reported fixture/timing failures;
this revision does not use it as a passing acceptance claim.
