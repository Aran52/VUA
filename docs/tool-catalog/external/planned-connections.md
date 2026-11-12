# Planned connections

> Updated: 2026-11-12

These tools and headset routes have their interface in VUA ahead of a working connection (author
request, 2026-11-06: interface first, backend to follow). VUA describes each one, links its official
page and, where the environment check already observes it, shows whether it is on the computer. A tool's
action reads "In development" and a headset's opens its connection guide until the UI card is switched
onto its backend face. Licenses and sources were rechecked on 2026-11-11.

## Tools

Remaining planned cards live in `apps/desktop/src/renderer/features/tools/planned-tools.ts` (`xsoverlay`
and `liv`). Desktop+, fpsVR, SlimeVR Server, VRCT, VRCX and VRCS are now connected to
[external-tool v0.4](../../protocols/external-tool-v0.4.md) (`SteamToolCard`) and
[installer-tool v0.1](../../protocols/installer-tool-v0.1.md) +
[deployment v0.1](../../protocols/environment-deployment-v0.1.md) (`InstallerToolCard`).

| Card | Source | License or price | Steam App ID | Connection status |
| --- | --- | --- | --- | --- |
| [SlimeVR Server](slimevr-server.md) | [SlimeVR-Installer](https://github.com/SlimeVR/SlimeVR-Installer) `v5.0.0` | `MIT AND Apache-2.0` (mirror-eligible with notices) | — | Live (`InstallerToolCard` on `slimevr` in deployment v0.1 + installer-tool v0.1); elevated NSIS `/S` installs SteamVR driver and Java runtime |
| [VRCT](vrct.md) | [GitHub](https://github.com/misyaguziya/VRCT) `v3.6.1` | `MIT`, except VRCT-exclusive chat-bubble model (official GitHub release only; no mirroring) | — | Live (`InstallerToolCard` on `vrct` in deployment v0.1 + installer-tool v0.1) |
| [VRCX](vrcx.md) | [GitHub](https://github.com/vrcx-team/VRCX) `v2026.09.16` | `MIT` (mirror-eligible with license) | — | Live (`InstallerToolCard` on `vrcx` in deployment v0.1 + installer-tool v0.1); VRCX keeps its own VRChat sign-in and database |
| [VRCS](vrcs.md) | [GitHub](https://github.com/Dreaminko/VRCS) `0.3.7` | `AGPL-3.0` (verified 2026-11-11; official GitHub release first, source+license required if mirrored) | — | Live (`InstallerToolCard` on `vrcs` in deployment v0.1 + installer-tool v0.1) |
| XSOverlay | Steam | Paid, Early Access | 1173510 | Planned — pending real Steam installation verification of `installdir` and main executable |
| Desktop+ | Steam | Free | 1494460 (`DesktopPlus`, `DesktopPlus.exe`) | Live (`SteamToolCard` on `desktopplus` in external-tool v0.4) |
| fpsVR | Steam | Paid | 908520 (`fpsVR`, `fpsVR.exe`) | Live (`SteamToolCard` on `fpsvr` in external-tool v0.4) |
| LIV | Steam | Free base application | 755540 | Planned — pending real Steam installation verification of `installdir` and main executable |

A Steam card becomes live by adding its identifier to the
[external tool connection](../../protocols/external-tool-v0.4.md) and moving it out of
`planned-tools.ts`; `SteamToolCard` then draws it with no other interface work. Official-installer
cards (`slimevr`, `vrct`, `vrcx`, `vrcs`) connect to `tools.observeInstaller` / `tools.actInstaller`
and `environment.planDeployment` / `environment.executeDeployment`. Logos are listed in
`apps/desktop/src/electron/tool-logos.ts`.

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

PICO Connect keeps its full route; PICO's other connections are guided like every other headset's.

Steam Frame stays on its SteamVR route. A standalone VRChat build was reported on 2026-06-12
([Steam Deck HQ](https://steamdeckhq.com/news/vrchat-gets-standalone-steam-frame-build/), from SteamDB),
and VRChat's [Getting Started](https://wiki.vrchat.com/wiki/Getting_Started) page does not yet name the
headset. Its "no computer" connection follows once VRChat lists it.

A route becomes live by adding it to `PLAY_ROUTES` in the contracts, with its SOP and play session
in the backend, and pointing `setupEngine` at it; `PlayRouteFlow` then draws it. The detailed device
requirements remain in the [N1 delivery plan](../../development/n1-delivery-plan.md).

## Considered and left out

- **VRCVideoCacher** replaces `yt-dlp.exe` in VRChat's folder with its own stub. That changes VRChat's
  installed files, which the [product boundary](../../product-boundary.md) does not authorize on the
  strength of another tool's practice.
- **Working Bigscreen Beyond, Pimax and Varjo routes** follow the device pool in the N1 plan; their
  guides are here now (author request, 2026-11-06).
- **A working WMR route** remains an investigation candidate in the N1 plan. Microsoft removed the
  platform in Windows 11 24H2, so the guide says which Windows versions still run it.
