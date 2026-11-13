# Independent scenario preparation v0.2

> Document version: 0.1.0
> Status: Candidate
> Updated: 2026-11-11
> Owner: Environment
> Schemas: [request](../../schemas/scenario-environment/v0.2/request.schema.json), [result](../../schemas/scenario-environment/v0.2/result.schema.json)

This Candidate face serves the play-setup catalog (`platforms`, `devices`, `connections`) and
composed play-setup SOPs directly from the Provider so the desktop UI and MCP read one static,
deterministic source. [scenario-environment v0.1](scenario-environment-v0.1.md) remains active
for `pc_avatar`, `quest_avatar`, `environment.checkScenarioEnvironment` and existing `v0.1`
SOP callers.

## Operations

| Entry | Input | Performs | Result / next action |
| --- | --- | --- | --- |
| `environment.listPlaySetups` | `schemaVersion: "0.2"` | Pure static catalog; no filesystem, network, process or installation probe | `vua.scenario-environment/v0.2` with `platforms`, `devices` (ordered; `connections[0]` is recommended) and `connections` |
| `environment.getScenarioSop` | `schemaVersion: "0.2"`, `selection: {platform, device, connection}` | Validates exact combination without silent normalization; composes deterministic English SOP steps | `performed: "sop_only"`, `engine`, `requirements`, `steps`, `sopVersions`, `lastVerification`, `sources` |

## Combination and engine rules

- Platforms: `vrchat`, `chilloutvr`, `reborn`.
- Devices (15): `pc`, `pico`, `quest`, `steam-frame`, `vive-standalone`, `vive-wired`, `index`,
  `psvr2`, `bigscreen`, `pimax`, `varjo`, `meganex`, `rift-s`, `wmr`, `other-headset`.
- Connections (17): `pico-connect`, `standalone`, `quest-link`, `steam-link`, `virtual-desktop`,
  `alvr`, `steam-frame`, `vive-hub`, `steamvr-direct`, `psvr2-app`, `bigscreen-driver`,
  `pimax-play`, `varjo-base`, `meganex-configurator`, `rift-app`, `wmr-steamvr`, `steamvr-other`.
- `reborn` has no desktop `pc` mode (`platform == "reborn" && device == "pc"` returns
  `vua.scenario.invalid_params`).
- `standalone` applies only when `platform == "vrchat"`.
- `device == "pc"` requires `connection: null`; every headset device requires a non-null
  `connection` listed on that device. Invalid combinations are rejected with
  `vua.scenario.invalid_params` and never silently corrected.
- `engine`:
  - `connection == "standalone"` → `{kind: "headset"}` (`requirements: []`, `nextAction: "follow_sop"`).
  - `platform == "vrchat" && device == "pc"` → `{kind: "route", route: "desktop_play"}`.
  - `platform == "vrchat" && connection == "pico-connect"` → `{kind: "route", route: "pico_pcvr"}`.
  - All other PCVR combinations → `{kind: "tool", toolId: platform}`.

## Step and SOP provenance fields

Each step carries `id` (`steam`, `steam-sign-in`, `platform`, `connection-pc`,
`connection-headset`, `launch`, `store-install`, `store-launch`), `actor` (`vua`, `open`,
`player`), `title`, `explanation`, `expectedResult`, `automatic` (`actor == "vua"`),
`verification` and `onFailure`. Structured identifiers and MCP step prose are English; the
desktop renderer resolves localized five-locale copy by step `id`.

`sopVersions` records `{platform, device, connection}` segment versions (`connection` is `null`
when `selection.connection` is `null`). `lastVerification` records `{date, environment,
softwareVersions}` honestly without claiming unverified hardware play sessions for non-route
headsets. `sources` lists official upstream `https://` references.

Both `environment.listPlaySetups` and `environment.getScenarioSop` are served even when optional
environment deployment services are absent.

## Document changelog

- 0.1.0 (2026-11-11): add `environment.listPlaySetups` and `environment.getScenarioSop` `v0.2`
  play-setup SOP composition while preserving `v0.1` endpoints.
