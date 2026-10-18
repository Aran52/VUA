# AMF interface: the Avatar factory

> Updated: 2026-10-18
> Status: structure approved by the author on 2026-10-17; delivery in progress

The AMF module is one factory. Materials come in, the workshop assembles them on production
lines, and finished Avatars leave through release. The [current plan](current-plan.md) owns the
slice order; this document owns the interface those slices land in.

## The player's goal

A newcomer wants one thing: "put this outfit on my Avatar and get it into VRChat". Each such wish
is one production line. How much the player wants to hand over decides only where the line stops
to ask them; the stations and the layout are the same for everyone.

## Base map

Selecting AMF opens the base map: a two-floor building in the style of a base-management game. A click on a room zooms the camera into it and shows its interior (control center: desks and computers; materials: shelves; release: a stage with the latest finished Avatar; workshop: the production line); the room's Enter button or a double click opens its page. The interiors are animated 2D scenes drawn on canvas at 30 frames per second; the camera eases between the overview and rooms, and while a room is zoomed, doors at the edges lead to the rooms beside, above and below it. Resource-saving mode holds the scenes on a still frame and cuts the camera straight to its view.
Each room shows its live state. A banner gathers what waits for the player and
what needs attention.

| Floor | Room | Holds | Shows on the map |
| --- | --- | --- | --- |
| 2F | **Control center** | Recipes and the package manager (tabs) | saved Recipes |
| 2F | **Materials** | the library and the material browser (tabs) | packages ready for production |
| 1F | **Release** | finished Avatars, the 60-frame turntable, SDK and upload | finished count, latest |
| 1F | **Workshop** | production lines and checks (tabs) | running / waiting / attention counts and the busiest line's stations |

The sidebar lists the four rooms under AMF: control center, materials, workshop and release.
Each room page has a bar with the room's plate, a way back to
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

## Materials room: starting from a chosen Avatar

The library detail of a product the catalog lists as an Avatar shows **Start production**; the
browse tab offers the same while a BOOTH Avatar item page is open. The panel shows the Avatar
with its thumbnail and lists the library's outfits: those whose product or material names
mention the Avatar come first with a **Fits** mark, and the rest fold under "Other outfits"
where they stay selectable. The Avatar's names are its package names without version and
extension ("Meiyun_v2.1.0.zip" → "meiyun") and a name its product title quotes in 「」 or 『』;
the fit compares them with the outfit's names ignoring case, character width and spaces.

When the Avatar's package is not yet a production material — the workshop's material list has
no entry for it — the panel explains the next step (download it from the product page or import
the file) and links to the browse tab. **Start the line** opens the workshop with a new line
already preparing the Avatar and the checked outfits through the same preparation path the
workshop's own button uses. A single Avatar package is named as the Avatar and the outfits come as
automatic materials, so a package without a wearable prefab, such as a bundled lilToon, never
has to be dressed; an Avatar with several packages, or one started without outfits, comes as
automatic materials too, and outfits can join the line later from its materials station.

## Permission modes

The workshop works like an AI agent's permission modes (author ruling, 2026-10-18): the player
chooses how much to hand over, whatever their experience with Unity. All three modes share one
layout; they differ in where the line stops. Each line shows its mode as a button with the mode's
icon and name; the button opens a card below it that lists the three modes, each with a line icon,
its name and one line of explanation, and a check mark at the right of the mode in effect:

| Mode | Stops at |
| --- | --- |
| Manual review, hand icon (default) | every station; each step and plan waits for the player's OK |
| Limited auto, checked box | the assembly plan and the release check |
| Full auto, shield | nowhere; a prepared line that was never assembled assembles by itself |

Choosing full auto first explains that VUA will change the work project without asking and
that this can mess up the Avatar; a restore point before each assembly lets the workshop bring it
back. Uploads to VRChat ask in every mode.

## Release room

The release room is an album of finished Avatars, one spread each, newest first (author request,
2026-10-17). A page turns over the spine from the arrows, the arrow keys, the index tabs or a
dragged page corner; with reduced motion or stacked pages the next spread fades in, and
resource-saving mode cuts to it. The motion is scripted from the motion tokens.

- **Left page:** the Avatar mounted as a photo. Dragging turns it through the 60-frame turntable
  while the turntable shows the current scene, otherwise through the front, side and back views.
  Below it the turntable is baked or baked again.
- **Right page:** the Avatar's name and materials, its outfits with their switches, VUA's checks,
  and **Upload to VRChat** in the room's color. The upload dialog asks for the name on VRChat
  and the player's content rights statement; the page then follows the stages from opening Unity
  to uploading, offers Stop, and shows the VRChat Avatar ID or the SDK's reason in plain words.
- An AI agent's upload request appears on its Avatar's page and opens the same dialog.
- The production records are debugging information in Settings (author request, 2026-10-18).

Paper, binding, pattern and shades are the `--vua-album-*` tokens; the photo corners, the stage
backdrop, page ornaments and the cover are `data-skin-slot` elements.

## How the later slices land here

| Slice | Where |
| --- | --- |
| 5 Unity editor handoff | "Close Unity and continue" on a needs-attention card; "Done checking, close Unity" on the quality check |
| 6 SDK step | release: the SDK checks in plain words |
| 7 Permission modes | the permission modes above; recommendation rules for prefabs |
| 8 Known packages | control center: packages; the materials station |
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
5. **Release:** the album with the 60-frame turntable baked for AMF results and managed upload
   (slice 12, brought forward on 2026-10-17); then the SDK step (slice 6).

`pnpm --filter @vua/desktop preview:factory [dir] [locale] [theme]`
renders the map, the workshop states and the release room with sample data for review;
`--only=<prefix>` renders a subset, for example `--only=release`.
