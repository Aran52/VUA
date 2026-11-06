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
| Avatar editing | Materials, Workshop, Finished Avatars, Recipes, Packages, Unity setup |

The Avatar group follows the player's order: materials become a line in the Workshop, and the line
becomes a Finished Avatar. Recipes, Packages and Unity setup support that path.

- Top bar: the logo opens Home; latency, resources, MCP, Help, Settings and notifications stay on the
  right.
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
3. **Avatar editing:** Materials, Workshop and Finished Avatars with their live facts, and one
   button: New line, or Import materials while the library is empty. With AMF off this part explains
   AMF and offers to turn it on.

## Play

The right part of a route card is the route's one button. It names the next step: check the
environment, please review, install, retry, open the official page, launch or close.

- Ready software launches at once. The steps to review guard installing what is missing.
- The left part opens details: software, connection and download region, the numbered steps and
  notices. Confirming the steps and choosing a PICO download region happen there, next to what they
  refer to, so those two steps also show a button in the details. The steps are a list to read, never
  boxes to tick.
- A failure, an official page to open or a session issue opens the details by itself.
- The running observation overrides an older check, so software installed outside VUA shows up without
  another check.
- Other social platforms (ChilloutVR and Reborn) follow the routes, as cards that install, open and
  close through Steam. They share the Tools cards' connection.

## Cards

Every compact card reads the same way: logo, name and one line on what the item is on the left, and
the next step in words on the right. An item still in development says so in the right part, in place of
an action. A card's details hold whatever a player needs when something is wrong, including reinstalling.

## Latency

Latency sits in the top bar, because a player checks it from any page and it has nothing to do with
choosing a route. Opening it tests the regions once and shows the closest one in the bar. Website
tests sit one level down in the same panel.

## Reinstalling

A Steam application that stopped working reinstalls from its details: Steam asks to uninstall, then VUA
installs it again. Tools and other platforms offer this on each card. Play's details offer it for VRChat
and SteamVR under "Software not working?". Unity setup offers Unity Hub, VCC and ALCOM from their
official pages, because those installers are not VUA's to run.

## From materials to upload

| Place | Main action | Leads to |
| --- | --- | --- |
| Materials | Select a card: **Use as Avatar** in the row under the wall | New-line dialog, then Workshop |
| Workshop, new line | **From materials** or **From a Recipe**, with pictures; **Import a shared Recipe** sits beside them; the start button stays in view | The line |
| Workshop, line | The current station's action; a finished line offers **View finished Avatar** | Finished Avatars |
| Finished Avatars | **Upload to VRChat**, right under the line's name; the 360° preview on the left | Upload dialog |

Selecting stays a single click and opening details a double click (Explorer semantics). The row under
the wall shows the selection's actions, and a one-line hint while nothing is selected.

An empty step points back to the step before it: an empty new-line page offers Import materials, and
a missing Editor offers Unity setup.

A line has one name everywhere: the name the player gave it, else its first material's. Unity's own
Avatar name stays in the line's facts. The triangle on a rail row opens the line's menu: rename, hide,
delete. The Workshop's concurrent-task setting folds away under the rail, because a new player does
not need it.

Finished Avatars shows the 360° preview once it exists, and three still views until then. The preview
takes a Unity job, so the button to create it sits right under the stills; in full auto, following a
finished line to this page creates it without asking.

Recipes open as a list, which says at a glance what each Recipe is; the graph is one click away and
the choice is remembered.

## Help and Settings

A page's first-visit introduction sits at the lower left of the content, because the page's main
action ends at the lower right.

Help keeps common VRChat questions first. Below them one list holds the VUA feature tour, the VRChat
getting-started wizard, the encyclopedia and the game guide. A row does what it says: the tour and the
game guide start directly, without a page holding one button.

The two guides answer two questions and carry the name of what they explain. First use leads to the VUA
feature tour, and the tour ends by offering the VRChat wizard. The wizard's three buttons (previous page,
go to the matching page, next page) sit side by side and stay in place while the text scrolls above them.
Its "full explanation" link, like every other guide link, opens the encyclopedia; using VUA is one of the
encyclopedia's topics, not a separate manual.

A page introduction has one button that closes it for good, beside any link it offers.

Settings opens on its first item, Theme.

## Rulings this changes

These earlier author rulings change here. The 2026-11-06 author message asked for the last two; the rest wait for this review:

| Ruling | Before | Now |
| --- | --- | --- |
| 2026-10-08, 2026-10-09 | Home repeats the directory as tiles; the network tile sits above the play routes | Home shows waiting work and next steps; routes come first |
| 2026-10-18 | Control center first under AMF, holding Recipes and Packages | Materials first; Recipes and Packages are pages of their own |
| F1 (freeze plan) | A station list page for AMF | Home's Avatar section carries the same live facts |
| F11 (freeze plan) | One button inside the route details | The same button on the route card |
| 2026-10-25 third-party review | The line names the viewed step and says when none runs | The station row marks both; the line speaks up only when the running step is elsewhere |
| F9 (freeze plan) | Tool action buttons show symbols, words after 1.2 s of hover | Cards show the next step in words and a line on what the tool is |
| F12, 2026-10-09 | The latency tile sits on Play | Latency sits in the top bar |
| 2026-10-05 guidance | Page introductions offer Later and Don't show again | One button closes them |
| 2026-10-28 guidance | The wizard's buttons say "I want this" and "Got it"; a separate VUA manual | Previous page, Go to, Next page in one row; the manual joined the encyclopedia |

Permission modes, themes, the material card's corner triangle and the Explorer selection semantics are
unchanged.
