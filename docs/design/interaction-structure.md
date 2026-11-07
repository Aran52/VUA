# Interaction structure

> Updated: 2026-11-07
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
| Play | Launch, Tools |
| Avatar | Materials, Workshop, Finished Avatars, Recipes, Packages, Unity setup |

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
mark while lines only run. Lines the player hid stay out of the count. An idle app makes no requests
for it.

The AMF module answers one request at a time, and the production list takes seconds, so pages share one
read: a page draws the last list at once and reads again after a task changes or when the list is over a
minute old. The page just opened goes first. The new-line dialog always reads, because starting a line is
a decision.

## Home

Home has three parts, and the first appears only when it has content:

1. **Current work:** running tasks, then lines that need attention or review, at most three of each.
   A row opens its task's page or its line.
2. **Play:** the combination chosen on Launch. It starts from here when its software is ready; otherwise
   its button opens Launch. Home reads play sessions every five seconds; Launch reads them every 1.5
   seconds.
3. **Avatar:** Materials, Workshop and Finished Avatars with their live facts, and one
   button: New line, or Import materials while the library is empty. With AMF off this part explains
   AMF and offers to turn it on.

Home holds both goals and stays in one accent, as every page does.

## Launch

Launch asks what the player plays and how, then lists what that takes. One card stands for the chosen
combination. Its right part is the one button and names the next step: check the environment, please
review, install, retry, open the official page, launch or close. Its left part opens the choices and the
steps. A first visit opens them; a player who has chosen sees just the card, and Home shows the same one.

- **Platform:** VRChat, ChilloutVR or Reborn. ChilloutVR and Reborn take the device and connection steps
  through Steam; settings, tutorials and Avatar work stay with VRChat. Reborn's store page names no desktop
  mode, so it has no PC mode.
- **Device:** PC mode by default, or the one headset whose software is on this computer. Once the player
  has chosen, what is installed no longer changes the choice. A green dot marks a device whose software is
  here. Less common headsets sit behind their own entry: Bigscreen Beyond, Pimax, Varjo, MeganeX, Oculus
  Rift S and Windows MR, then any other model SteamVR supports.
- **Connection:** asked only where a device has several, with the recommended one first, or the one
  already on this computer. Standalone headsets (PICO, Meta Quest, VIVE standalone) offer their own
  streaming, Steam Link, Virtual Desktop and ALVR, and "No PC" where the headset has its own VRChat. Wired
  headsets have one route and skip the question.
- **Steps:** they change with the choice, and each says who does it: VUA, VUA opening the official page, or
  the player. Software steps tick off from what the computer reports. The steps are a list to read, never
  boxes to tick. Steps that come from official guides say so.
- PC mode and PICO Connect run their own flow: ready software launches at once, and the steps to review
  guard installing what is missing. Confirming them and choosing a PICO download region happen in the
  details, next to what they refer to.
- Other combinations install and open the platform through Steam, and the official pages cover the
  connection. The [planned connections](../tool-catalog/external/planned-connections.md) list what each
  needs to become a full flow.
- A failure, an official page to open or a session issue opens the details by itself.
- The running observation overrides an older check, so software installed outside VUA shows up without
  another check.

`features/home/play-setup-model.ts` holds the combinations and composes their steps, in the shape a later
provider contract will serve, so the app and an agent read one source.

## Cards

Every compact card reads the same way: logo, name and one line on what the item is on the left, and
the next step in words on the right. A tool still in development says so in the right part, in place of
an action. A card's details hold whatever a player needs when something is wrong, including reinstalling.
Installing uses one icon on every card. Library cards keep the same rows on every card, so shop and
variant line up across the wall. A left part that opens details in place carries a small caret that turns
when they open. Details scroll into view when they open and close with Esc.

Tools still in development have the same card, with their source, license and official page in the
details. Tools that replace VRChat's own files stay out; see the planned connections.

## Latency

Latency sits in the top bar, because a player checks it from any page and it has nothing to do with
choosing a route. Opening it tests the regions once and shows the closest one in the bar. Website
tests sit one level down in the same panel.

## Reinstalling

A Steam application that stopped working reinstalls from its details: Steam asks to uninstall, then VUA
installs it again. Tools and other platforms offer this on each card. Play's details offer it for VRChat
and SteamVR under "Software not working?", with PICO's own software on its official page. Unity setup
offers Unity Hub, VCC and ALCOM from their official pages, because those installers are not VUA's to run.

## From materials to upload

| Place | Main action | Leads to |
| --- | --- | --- |
| Materials | Select a card: **Use as Avatar** in the row under the wall | New-line dialog, then Workshop |
| Workshop, new line | **From materials** or **From a Recipe**, with pictures; **Import a shared Recipe** sits beside them; the start button stays in view | The line |
| Workshop, line | The current station's action; a finished line offers **View finished Avatar** | Finished Avatars |
| Finished Avatars | **Upload to VRChat**, right under the line's name; the 360° preview on the left | Upload dialog |

Selecting stays a single click and opening details a double click (Explorer semantics): Ctrl adds or
removes a card, Shift extends from the last click, arrow keys move and select, and Enter opens. The row
under the wall appears with a selection and holds its actions.

An empty step points back to the step before it: an empty new-line page offers Import materials, and
a missing Editor offers Unity setup.

A line has one name everywhere: the name the player gave it, else its first material's. Unity's own
Avatar name stays in the line's facts. The triangle on a rail row opens the line's menu: rename, hide,
delete. The Workshop's concurrent-task setting folds away under the rail, because a new player does
not need it.

Finished Avatars shows the 360° preview once it exists, and three still views until then. The preview
takes a Unity job, so the button to create it sits right under the stills; in full auto, following a
finished line to this page creates it without asking.

Recipes open as cards: the Avatar's and outfits' pictures, whether this PC has the materials, and one
button that starts a line. A Recipe saved without a BOOTH id finds its pictures through the production list.
Whether a Recipe's materials are here takes seconds to work out, so plans run one at a time, the open Recipe
first, and each keeps its answer until a task changes or five minutes pass. A card's details open beside the cards and list each material with its state.
With two or more Recipes the graph is one click away, since it shows what Recipes share, and the choice is remembered. Its captions keep one size at every zoom and sit clear of nodes and each other; one with no clear spot waits for the pointer. Node positions survive leaving the page. Renaming and deleting a Recipe wait for the
engine; their menu entries say so.

## Materials

Rows load in batches as the wall scrolls and stay in the library's order, so a local import sorts among
products. Selection, Ctrl+A, arrow keys and details cover the whole list; Ctrl+A reads the rows still to come
first. Cancel filter sits on the count row. The list has title with its variation, shop, status, size and
added, sortable by name, shop, size and added, and it keeps dragged widths. "Recently added" follows BOOTH's
own list among account products, and local imports fall between them by their own time. BOOTH's 3D categories
read in the player's language; descriptions stay as the shop wrote them, with a copy button.

One row of controls sits under the title: search, source, download state, BOOTH category, VN3 terms, order
and view. Their values name them, so they carry no labels above. The count and a way to clear filters sit
under that row, and page buttons appear only when there is more than one page.

- **Order:** recently added by default, then first added, name, shop, price and largest on disk; newest on
  BOOTH joins them once a row has BOOTH's date. The choice and the view are remembered. An order other than the default shows its value on each
  card, so it explains itself; the list's title and shop headers sort too. Filters and the place in the
  wall last until the app closes.
- **VN3 terms:** a panel with the terms state (all, has VN3, no VN3, not read yet, see the document) and
  the uses the terms must allow, grouped as VN3 words them, each with how many materials would match.
  Unread terms can be read from the panel. A matching card says so only when it is conditional, needs the
  creator's permission or has a special note.
- **Details:** a page in place of the wall, which stays where it was. The pictures stay in view beside the
  name, the one main action, facts, VN3 terms, files and description. Esc or the arrow beside the title
  returns; the left and right arrow keys move to the neighbouring cards of the current page.

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

## Direct actions

- A production line renames from its menu, with a double click or with F2, as a file does in a file
  manager.
- Adding a package repository opens below the page's title row, so the view switch never moves. A click on
  a common repository (Avatar Optimizer, VRCFury, Poiyomi) fills the form; the review step still applies.
- The VRChat wizard pages with the left and right arrow keys as well as its three buttons.
- The album's page controls sit under its tabs, in view without scrolling.
- A disabled start button says what it waits for.

## A line whose work project is gone

A work project can leave the computer, most often to free disk space after an upload. Finished Avatars and the
Workshop then say so plainly and offer a new line from the same materials. It reads as a neutral fact and stays
out of the Workshop's count. The Avatar's views stay, because finishing a step copies them next to the line's
record; the turntable stays in the project.

## Rulings this changes

These earlier author rulings change here. The author's 2026-11-06 message asked for the two guidance rows and the 2026-11-07 message approved the last three; the rest wait for this review:

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
| 2026-11-06 | ChilloutVR and Reborn install and open, with no preparation | They also take Launch's device and connection steps |
| 2026-10-12 | Materials pages through results, Cancel filter follows the pagination, and the list has title, shop, variant and source columns | Materials scrolls, Cancel filter sits on the count row, and the list has title with variant, shop, status, size and added |
| 2026-10-31 | The corner triangle has a 72 px tail, and a product on BOOTH is green | The triangle stays 40 px with no tail, and such a product is grey until the pointer reaches it |

Permission modes, themes and the Explorer selection semantics are unchanged.
