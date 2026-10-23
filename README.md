# VUA — VRC Ultra Assistant

English | [简体中文](README_ZH.md) | [日本語](README_JA.md) | [한국어](README_KO.md)

**VUA (VRC Ultra Assistant)** is a Windows-first, local-first scenario installer, manager and
launcher for VRChat players. A scenario brings together a goal, the required software and the
steps to prepare, launch and recover it: play on a PC screen, stream to a PICO, or edit an Avatar
with a selected module. Start with what you want to do; VUA helps you work out what comes next.

## What you can do with VUA

### 1. Game assistant

> **Materials checked and cleared.**

Prepare the software and settings your play setup needs, then learn the basics of movement, menus, safety options and devices. Choose your goal and hardware; deployment is intended to explain what is needed and guide installation.

The first account guide opens official registration pages in the built-in browser: Steam and VRChat for play, with optional Unity and BOOTH accounts for creators. Users complete registration, Steam library actions and official account linking themselves. Upload guidance explains the full VRChat account and New User or higher requirements; local Avatar preparation can start before upload eligibility.

Planned optional connections cover tracking, VR overlays, sleep utilities and capture/streaming. VUA will detect independent installations, open official Steam purchase/install routes and launch the applications; Steam/upstream manages updates. The tracking tools also receive setup guidance, with PICO eye tracking included in the first play release. See the [selected tools and support scope](docs/development-outline.md#n2-external-gameplay-tools).

### 2. Avatar production

> **Sugar, spice, and everything nice.**

AMF is the first-party editing module installed with VUA and enabled for new users. You can turn
it off in Avatar editing or Settings → Modules; existing choices are remembered. Its asset library and private BDL
stay together, and disabling it preserves your data.

Combine your own materials, capture the choices in a Recipe, and share the Recipe so others can reproduce the setup with assets they obtain themselves. The goal is automated Avatar production with checks and controlled changes before handing the result to the official SDK; a Recipe contains references and settings, not paid assets.

## How it works

Start with your goal and available materials, review the proposed steps, and follow the results. A Build Record keeps production steps, inspection evidence and errors together so that you can see what happened and what needs attention.

Project and package management prepares the required Unity environment and dependencies, including VPM repositories, package installation, updates and removal. Existing ALCOM/VCC projects are inspected read-only; editing starts from a user-requested VUA-managed copy. Final login and upload remain your actions in the official VRChat SDK.

## Safety boundaries

- VUA is an independent third-party assistant, with no affiliation with or endorsement by VRChat Inc.; its integration policy permits documented external interfaces, OSC, launch options, necessary local logs and documented configuration fields, never VRChat client injection, hooks, patches or EAC bypass.
- Initial account guidance uses isolated temporary browser sessions. VUA application features do not collect passwords or extract login cookies/tokens into application data, logs, or agent context; first delivery does not remember VRChat sign-in. Later web-reading and experimental persistence are deferred and are not a claim of platform approval; see the [account boundary](docs/product-boundary.md#account-onboarding-user-ruling-2026-09-30).
- Account changes must be initiated by you through an allowed path; VUA does not take over your account from a cloud service or automatically upload Avatars on your behalf.
- Only data needed for the feature is kept, locally by default, without unnecessary friend-activity tracking or profiling; purchased materials stay local and shared Recipes exclude their contents.
- Undocumented client behavior, hidden configuration fields and uncontrolled API automation are outside the default scope; a successful technical check is not a guarantee of appearance, behavior or production safety.

[VRChat Creator Guidelines](https://hello.vrchat.com/creator-guidelines) · [Configuration File](https://docs.vrchat.com/docs/configuration-file)

## Development progress

[v0.7.0 Stone Age (Beta)](https://github.com/VUA-Project/VUA/releases/tag/v0.7.0) brings the first Avatar production path: choose an Avatar and outfits from your library, and VUA assembles them in its own Unity project, checks them with the VRChat SDK and uploads after you confirm. Online and offline Windows x64 NSIS installers include AMF. Development follows the [N1–N7 sequence](docs/development-outline.md).

The project remains in Beta. Publication does not declare N1/N5 or other pending physical, material and human-review outcomes accepted. See the [v0.7.0 scope, checks and limitations](docs/release/v0.7.0.md).

### Current delivery and starting points

- Download **VUA-0.7.0-windows-x64-setup.exe** from the [official download page](https://ph-r.com/projects/vua/#download) or [GitHub release](https://github.com/VUA-Project/VUA/releases/tag/v0.7.0), or choose **VUA-0.7.0-windows-x64-offline.exe** when downloads are unreliable. Both sources serve the same files. Run the installer, then open VUA from the Start menu. Node, pnpm, Rust and a source checkout are not needed. This build is unsigned; Windows may display an unfamiliar-publisher warning.
- Start with the first-use tour and guide, or open Play, Tools or Avatar editing directly. AMF is enabled for new users and can be disabled in Settings → Modules. Steam/PICO/Unity/account actions remain with their official applications.
- To update VUA, finish your work, exit VUA and run the new installer. AMF also supports separate updates through Settings → Modules or About, followed by an explicit restart. Application data remains in `%APPDATA%\VUA`. Existing ZIP users can switch to the installer while retaining that profile. The release notes name remaining tests and manual handoffs. Use [Issues](https://github.com/VUA-Project/VUA/issues/new/choose) for problems.
- For developers: [install dependencies, launch the desktop app and select checks](apps/desktop/README.md#development-commands), then follow [Contributing](CONTRIBUTING.md).

## Documentation

- [Documentation guide](docs/README.md) — the smallest route for every task
- [Product boundary](docs/product-boundary.md)
- [Architecture](docs/architecture/system.md)
- [v0.7.0 release notes](docs/release/v0.7.0.md)
- [Contributing](CONTRIBUTING.md) · [Security policy](SECURITY.md)

## Milestones
Dates are the times GitHub recorded for each release and pull request.

| Date | Milestone |
| --- | --- |
| 2026-09-07 | [v0.5.0](https://github.com/VUA-Project/VUA/releases/tag/v0.5.0): the first path from the desktop app through the Rust orchestrator to a real Unity 2022.3 project, assembling an Avatar with an outfit |
| 2026-10-01 | [Purpose-driven environment deployment](https://github.com/VUA-Project/VUA/pull/60) with regional Unity sources |
| 2026-10-08 | [BOOTH library sync, selective downloads and local file management](https://github.com/VUA-Project/VUA/pull/66) |
| 2026-10-11 | [v0.6.6 Ibis](https://github.com/VUA-Project/VUA/releases/tag/v0.6.6), the first public Beta, with AMF as a bundled module |
| 2026-10-12 | [NSIS installers and independent AMF updates](https://github.com/VUA-Project/VUA/pull/80) |
| 2026-10-15 | [Material-first Avatar production, Recipes and the local MCP adapter](https://github.com/VUA-Project/VUA/pull/84) |
| 2026-10-17 | [Release room with the 60-frame turntable and managed upload](https://github.com/Aran52/VUA/pull/4) |
| 2026-10-18 | [VN3 terms parsing and checks by use](https://github.com/Aran52/VUA/pull/31) |
| 2026-10-22 | [v0.7.0 Stone Age](https://github.com/VUA-Project/VUA/releases/tag/v0.7.0), the first release with the Avatar production path |

## License

VUA is licensed under [Apache-2.0](LICENSE). External applications retain their own licenses, including [VRCFaceTracking (Apache-2.0)](https://github.com/benaclejames/VRCFaceTracking/blob/master/LICENSE) and [Space Calibrator (MIT core; separate third-party licenses)](https://github.com/hyblocker/OpenVR-SpaceCalibrator/blob/develop/LICENSE). See [third-party notices](THIRD_PARTY_NOTICES.md) for all planned connections, upstream attribution and conditions for any future redistribution.

[NOTICE](NOTICE) · [Trademark guidance](TRADEMARKS.md)

Copyright 2026 Aran52.
