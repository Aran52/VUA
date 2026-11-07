# Planned connections

> Updated: 2026-11-07

These tools and headset routes have their interface in VUA ahead of a working connection (author
request, 2026-11-06: interface first, backend to follow). VUA describes each one, links its official
page and, where the environment check already observes it, shows whether it is on the computer. A tool's
action reads "In development" and a headset's opens its connection guide until the connection below
exists. Licenses and sources were checked on the date above; recheck them when a connection is built.

## Tools

The cards are listed in `apps/desktop/src/renderer/features/tools/planned-tools.ts`.

| Card | Source | License or price | Steam App ID | Connection needed |
| --- | --- | --- | --- | --- |
| SlimeVR Server | [SlimeVR-Installer](https://github.com/SlimeVR/SlimeVR-Installer) | MIT and Apache-2.0 | — | Official installer, which also installs the SteamVR driver and a Java runtime; discovery and launch |
| VRCT | [GitHub](https://github.com/misyaguziya/VRCT) or BOOTH | MIT, except a chat-bubble model licensed only for VRCT | — | Official release; the model limits mirroring |
| VRCX | [GitHub](https://github.com/vrcx-team/VRCX) | MIT | — | Official installer; VRCX keeps its own VRChat sign-in |
| VRCS | [GitHub](https://github.com/Dreaminko/VRCS) | Verify | — | Already selected in the [N2 table](../../development-outline.md#n2-external-gameplay-tools) |
| XSOverlay | Steam | Paid, Early Access | 1173510 | Shared Steam connection |
| Desktop+ | Steam | Free | 1494460 (`DesktopPlus`, `DesktopPlus.exe`) | Shared Steam connection |
| fpsVR | Steam | Paid | 908520 (`fpsVR`, `fpsVR.exe`) | Shared Steam connection |
| LIV | Steam | Free base application | 755540 | Already in the N2 table |

A Steam card becomes live by adding its identifier to a new version of the
[external tool connection](../../protocols/external-tool-v0.3.md) and moving it out of
`planned-tools.ts`; `SteamToolCard` then draws it with no other interface work. Its logo is already
listed in `apps/desktop/src/electron/tool-logos.ts`.

## Headset routes

The devices and their connections are listed in `apps/desktop/src/renderer/features/home/play-setup-model.ts`.
Steam Link also serves PICO and VIVE standalone, from the headset's own store.
`component` there names the existing environment check that observes the PC software.

| Route | Connections, recommended first | PC software | Observed by |
| --- | --- | --- | --- |
| Meta Quest | Quest Link / Air Link | Meta Horizon Link | `oculus_runtime` |
| | Steam Link | Steam and SteamVR only | Nothing separate to observe |
| | Virtual Desktop | Virtual Desktop Streamer, from vrdesktop.net; the headset app is paid | `virtual_desktop` |
| | ALVR | ALVR, from GitHub or Steam (App ID 3312710), and SteamVR; MIT | `alvr` |
| Steam Frame | SteamVR through its USB wireless adapter | SteamVR | `steamvr` |
| HTC VIVE | VIVE Hub, for VIVE Focus Vision and XR Elite | VIVE Hub and SteamVR | `vive_runtime` |
| | SteamVR, for wired headsets | SteamVR | `steamvr` |
| | Virtual Desktop, ALVR | As above | As above |
| Valve Index | SteamVR | SteamVR | `steamvr` |
| PlayStation VR2 | PlayStation VR2 App (Steam App ID 2580190, free), with the PC adapter | The app and SteamVR | `psvr2` |
| Bigscreen Beyond | Bigscreen Beyond Utility, which installs with SteamVR (Steam App ID 2467050) | SteamVR; base stations required | `bigscreen_beyond` |
| Pimax | [Pimax Play](https://www.pimax.com/support/downloads), which installs the SteamVR driver | Pimax Play and SteamVR | `pimax_runtime` |
| Varjo | [Varjo Base](https://support.varjo.com/hc/en-us/downloads), with OpenVR on | Varjo Base and SteamVR; Aero works up to Varjo Base 4.14 | `varjo_runtime` |
| MeganeX | [MeganeX Configurator](https://shiftall.notion.site/meganex8k-manual-en) | Configurator and SteamVR; base stations; modern NVIDIA GPU | Nothing separate to observe yet |
| Oculus Rift S | Meta Horizon Link, over DisplayPort and USB 3; legacy on Meta's list | Meta Horizon Link | `oculus_runtime` |
| Windows MR | Windows Mixed Reality for SteamVR (Steam App ID 719950), Windows 10 or Windows 11 23H2 only | Guide only; Windows 11 24H2 removed the platform | Nothing separate to observe |
| Another model | Its maker's software, then SteamVR | Guide only | `steamvr` |

PICO keeps its full route and lists Virtual Desktop and ALVR under its details as other streaming
software.

A route becomes live by adding it to `PLAY_ROUTES` in the contracts, with its SOP and play session
in the backend, and removing it from `PLANNED_ROUTES`; `PlayRouteFlow` then draws it. Its connection
guide can stay as the options of that flow. The detailed device requirements remain in the
[N1 delivery plan](../../development/n1-delivery-plan.md).

## Considered and left out

- **VRCVideoCacher** replaces `yt-dlp.exe` in VRChat's folder with its own stub. That changes VRChat's
  installed files, which the [product boundary](../../product-boundary.md) does not authorize on the
  strength of another tool's practice.
- **Working Bigscreen Beyond, Pimax and Varjo routes** follow the device pool in the N1 plan; their
  guides are here now (author request, 2026-11-06).
- **A working WMR route** remains an investigation candidate in the N1 plan. Microsoft removed the
  platform in Windows 11 24H2, so the guide says which Windows versions still run it.
