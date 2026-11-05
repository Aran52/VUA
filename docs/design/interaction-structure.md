# Interaction structure

> Updated: 2026-11-06
> Status: proposed; the author reviews it locally on `slice/ui-structure`

This document owns how the desktop app is organized for the player: the directory, the name of each
page, what Home does and where a task's main action sits. The
[design standard](design-standard.md) owns tokens, components and accessibility, and
[controls and motion](controls-and-motion.md) owns control sizes and motion. The
[desktop architecture](../architecture/desktop.md) owns how the shell is built.

## Four rules

1. **One directory.** The sidebar is the only list of the app's pages. Home, room bars and search do
   not add another list with other names.
2. **One name per page.** The sidebar, the page title, search, the tour and links use the same word.
3. **One next step per object.** A play route, a material, a production line and a finished Avatar
   each show one main action, named by what happens next. Details explain that step and never
   repeat it, except where the step depends on what the details show.
4. **Home says what is waiting.** The work that needs the player comes first, then each goal's next
   step.

## Directory

| Group | Pages, in order |
| --- | --- |
| VRChat | Play, Tools |
| Avatar editing · AMF | Materials, Workshop, Finished Avatars, Recipes, Packages, Unity setup |

The Avatar group follows the player's order: materials become a line in the Workshop, and the line
becomes a Finished Avatar. Recipes, Packages and Unity setup support that path.

- Top bar: the logo opens Home; MCP, Help, Settings and notifications stay on the right.
- Unity setup keeps its page ID `env-create` and works without AMF, so it stays in the group when
  AMF is off, beside the entry that turns AMF on. It reads as Avatar work and uses the Avatar accent.
- `features/shell/directory.ts` holds the directory. The theme address bar and search read it.
- Materials keeps two tabs, Library and Browse, because they are two views of one job. No other AMF
  page has tabs at the top level.
- Retired pages: the `avatar-hub` and `environment-hub` hubs open Home, because an independently
  updated AMF UI can still link to them. Stored `help-tour` and `help-game-assistant` routes open
  Help.

The directory shows a count on Workshop while lines wait for review or need attention, and a small
mark while lines only run. Lines the player hid stay out of the count. The summary reads again when
a task changes and when Home or the Workshop opens or closes, so an idle app makes no requests for it.

## Home

Home has three parts, and the first appears only when it has content:

1. **Current work:** running tasks, then lines that need attention or review, at most three of each.
   A row opens its task's page or its line.
2. **VRChat:** the two available routes. A route whose software is ready launches from here; otherwise
   its button opens that route's details on Play. Home reads play sessions every five seconds; Play
   reads them every 1.5 seconds.
3. **Avatar editing · AMF:** Materials, Workshop and Finished Avatars with their live facts, and one
   button: New line, or Import materials while the library is empty. With AMF off this part explains
   AMF and offers to turn it on.

## Play

The right half of a route card is the route's one button. It names the next step: check the
environment, confirm the checklist, install, retry, open the official page, launch or close.

- Ready software launches at once. The checklist guards installing what is missing.
- The left half opens details: software, connection and download region, checklist and notices.
  Confirming the checklist and choosing a PICO download region happen there, next to what they refer
  to, so those two steps also show a button in the details.
- A failure, an official page to open or a session issue opens the details by itself.
- The running observation overrides an older check, so software installed outside VUA shows up without
  another check.
- Routes come first and the regional latency tile follows them.

## From materials to upload

| Place | Main action | Leads to |
| --- | --- | --- |
| Materials | Select a card: **Use as Avatar** in the row under the wall | New-line dialog, then Workshop |
| Workshop, new line | **From materials** or **From a Recipe**; the start button stays in view | The line |
| Workshop, line | The current station's action; a finished line offers **View finished Avatar** | Finished Avatars |
| Finished Avatars | **Upload to VRChat**, right under the Avatar's name | Upload dialog |

Selecting stays a single click and opening details a double click (Explorer semantics). The row under
the wall shows the selection's actions, and a one-line hint while nothing is selected.

An empty step points back to the step before it: an empty new-line page offers Import materials, and
a missing Editor offers Unity setup.

Recipes open as a list, which says at a glance what each Recipe is; the graph is one click away and
the choice is remembered.

## Help and Settings

A page's first-visit introduction sits at the lower left of the content, because the page's main
action ends at the lower right.

Help keeps common VRChat questions first. Below them one list holds the encyclopedia, the beginner
wizard, the feature tour and the game guide. A row does what it says: the tour and the game guide start
directly, without a page holding one button.

Settings opens on its first item, Theme.

## Rulings this changes

These earlier author rulings change here, pending this review:

| Ruling | Before | Now |
| --- | --- | --- |
| 2026-10-08, 2026-10-09 | Home repeats the directory as tiles; the network tile sits above the play routes | Home shows waiting work and next steps; routes come first |
| 2026-10-18 | Control center first under AMF, holding Recipes and Packages | Materials first; Recipes and Packages are pages of their own |
| F1 (freeze plan) | A station list page for AMF | Home's Avatar section carries the same live facts |
| F11 (freeze plan) | One button inside the route details | The same button on the route card |
| 2026-10-25 third-party review | The line names the viewed step and says when none runs | The station row marks both; the line speaks up only when the running step is elsewhere |

The first-use notice, the wizard and the tour keep the 2026-10-28 rulings. Permission modes, themes and
the Explorer selection semantics are unchanged.
