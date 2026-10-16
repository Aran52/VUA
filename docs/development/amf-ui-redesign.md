# AMF interface: the Avatar factory

> Updated: 2026-10-17
> Status: structure approved by the author on 2026-10-17; delivery in progress

The AMF module is one factory. Materials come in, the workshop assembles them on production
lines, and finished Avatars leave through release. The [current plan](current-plan.md) owns the
slice order; this document owns the interface those slices land in.

## The player's goal

A newcomer wants one thing: "put this outfit on my Avatar and get it into VRChat". Each such wish
is one production line. The player's familiarity with Unity decides only where the line stops to
ask them; the stations are the same for everyone.

## Base map

Selecting AMF opens the base map: a two-floor building in the style of a base-management game.
Each room shows its live state and opens on click. A banner gathers what waits for the player and
what needs attention.

| Floor | Room | Holds | Shows on the map |
| --- | --- | --- | --- |
| 2F | **Office** | Recipes and the package manager (tabs) | saved Recipes |
| 2F | **Release** | finished Avatars, the 60-frame turntable, SDK and upload | finished count, latest |
| 1F | **Materials** | the library and the material browser (tabs) | packages ready for production |
| 1F | **Workshop** | production lines and checks (tabs) | running / waiting / attention counts and the busiest line's stations |

The sidebar lists the four rooms. Each room page has a bar with the room's plate, a way back to
the map and its tabs. Existing page addresses stay valid; each opens its room and tab.

## A production line

The workshop lists its lines beside the selected line. A line has five stations:

1. **Materials:** a separate work project, packages and imported materials.
2. **Intake check:** Unity reads the actual prefabs and renders their previews.
3. **Assembly plan:** the confirmation, as one sentence with images ("Put the T-shirt on Meiyun,
   each with a menu switch"); details sit under **Adjust**.
4. **Assembly:** VUA binds the outfits and creates their switches.
5. **Quality check:** front, side and back previews and the checks (bindings, menus, shaders,
   missing references, SDK sign-in), then Open in Unity, Save as Recipe, Adjust again, Restore.

Each station shows its state: not started, in progress, waiting for you, done, stopped with a
problem.

**Running:** the current step with live progress. **Stop** ends Unity and keeps the work project
and materials; **Continue** reinspects and carries on. Unity's batch mode has no pause, so Stop
and Continue serve as pause (author decision, 2026-10-17). Live Unity stages with counts, elapsed
and usual durations come from the backend in delivery 1.

**Needs attention:** one card with the cause in plain words, one next action and the technical
detail under "Details". A license failure links to the Unity sign-in and license guide.

## Automation levels

Asked on the first visit to the workshop and changeable on every line:

| Level | Stops at |
| --- | --- |
| Fully automatic | the assembly plan; confirming it plans and assembles in one go |
| Semi-automatic | the assembly plan and the quality check; candidates and switches are shown |
| Manual | every station; every setting is open |

## How the later slices land here

| Slice | Where |
| --- | --- |
| 5 Unity editor handoff | "Close Unity and continue" on a needs-attention card; "Done checking, close Unity" on the quality check |
| 6 SDK step | release: the SDK checks in plain words |
| 7 Familiarity levels | the automation levels above; recommendation rules for prefabs |
| 8 Known packages | office: packages; the materials station |
| 9 Material tuning | an optional station between quality check and release |
| 10 Face tracking | an add-on station after the quality check |
| 11 Unity license | the needs-attention card for license failures |
| 12 Managed upload | release |
| 13 Styles | the map and rooms take the style tokens once its design session settles them |
| 14 Four-language review | every string introduced here |

## Delivery

1. **Progress and stop backend:** Unity log stages with counts, stop and continue, step times.
2. **Base map, rooms and workshop lines** (this delivery): map, room bars and tabs, the line list,
   stations, the stop control, the needs-attention card and the automation levels.
3. **Live progress in the line** once delivery 1 lands.
4. **Materials:** "Start production" from a chosen Avatar in the library.
5. **Release:** finished Avatars with the 60-frame turntable baked for AMF results, then the SDK
   step (slice 6) and upload (slice 12).

`pnpm --filter @vua/desktop preview:factory [dir] [locale] [theme]`
renders the map and the workshop states with sample data for review.
