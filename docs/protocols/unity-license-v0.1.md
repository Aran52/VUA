# Unity license observations v0.1

> Document version: 0.1.0
> Status: Candidate
> Updated: 2026-10-19
> Owner: Environment

This independent face supplies license metadata and an explicit Editor check. The
[scenario-environment v0.1](scenario-environment-v0.1.md) software check retains its mandatory
`licenseState: "not_checked"`. Sign-in and license activation stay in Unity Hub.

## Methods

Both methods use the existing application and Desktop Gateway envelopes. Params have
`schemaVersion: "0.1"`; unknown keys are rejected.
The command uses its envelope's stable `commandId`: retries replay the durable task receipt
without repeating side effects, and changed arguments for the same ID are refused.

| Method | Kind | Additional params | Result |
| --- | --- | --- | --- |
| `environment.unityLicense` | Query | none | Metadata, processes, last check, `state` and `nextAction` |
| `environment.checkUnityLicense` | Command | `editorPath` | `schemaVersion: "0.1"`, durable `taskId`; task completion carries the check result |

Invalid params return `vua.unity_license.invalid_params`; an undiscovered/unverified Editor
returns `editor_unavailable`, an unowned/changed project `project_unavailable`, failed result
persistence `store_failed`, and missing check authority `unavailable`, under the same prefix.
These envelopes contain stable codes and localization keys, without diagnostic paths or logs.
The desktop module router returns its existing `vua.amf.unavailable` when AMF is not ready.

The query starts no process. Native Windows process enumeration observes Hub and its licensing
client. Existing file and native registry discovery identifies Hub as `classic`, `msix` or `none`
(no detected Hub). `hubRunning`, `licensingClientRunning` and `hubCliPresent` are booleans;
`null` means observation failed. Hub's separate CLI is detected at
`%LOCALAPPDATA%/Unity/bin/unity.exe` and is never executed by this face.

`files` has exactly these entries, in order, with only `kind`, `present` and `modifiedAt`:

| Kind | Metadata location |
| --- | --- |
| `classic_entitlement` | `%LOCALAPPDATA%/Unity/licenses/UnityEntitlementLicense.xml` |
| `msix_entitlement` | `%LOCALAPPDATA%/Packages/UnityTechnologies.UnityHub_2vrhnee42bhxm/LocalCache/Local/Unity/licenses/UnityEntitlementLicense.xml` |
| `legacy_ulf` | `%ProgramData%/Unity/Unity_lic.ulf` |

`present` is a boolean, or `null` when metadata cannot be read. `modifiedAt` is UTC RFC 3339,
or `null` for an absent/unreadable file. Size is used internally to reject empty-file hints.
License contents are never opened, read, logged, returned or uploaded. Full license paths
and account fields never cross this interface.

## Detection rules

These ordered rules produce hints, never an `active` detection claim:

1. Incomplete metadata, Hub discovery or process observations give `unknown` / `run_check`.
2. All three files absent gives `not_found` / `sign_in_and_activate`.
3. Only a nonempty MSIX entitlement is available and the licensing client is stopped:
   `needs_hub_client` / `start_hub`.
4. A nonempty classic/legacy file, or a nonempty MSIX entitlement with its client running,
   gives `likely_active` / `run_check`.
5. Present files that are all empty give `unknown` / `run_check`.

`lastCheck` is `null` before a check or when its record is unreadable. Otherwise it has exactly
`state`, `editorVersion` and `checkedAt`. It is historical evidence for that Editor. For
`likely_active`, when every present file's readable modification time is no later than the last
check, an `active` last check changes the action to `none`, and a `not_activated` last check
changes it to `sign_in_and_activate`. Changed or unreadable times require `run_check`.
Current missing-client and missing-file findings take priority over history.

## Explicit check

The command requires the available AMF provider. It uses AMF's runtime and TaskLane with
`exclusive: true`, waits behind accepted work and runs alone at every capacity. It shares the
admission lock with capacity changes. The check is observed/cancelled through task APIs;
it does not create an AMF run or production queue entry. Pending work never resumes on restart.

The path must match a discovered, identity-verified AMF Editor (`2022.3.22f1` or
`2022.3.22f1c1`). It is checked again after admission and immediately before execution. When
needed, the existing Hub licensing-client startup runs first. Cancellation during that wait
prevents Editor execution. The five-minute timeout applies to the Editor process; Hub readiness
retains its existing bounded wait.

The provider creates and reuses its own `unity-license/project` directory with an ownership
marker, empty Assets and a manifest without dependencies. Unowned projects and linked critical
paths are refused. Player and AMF production projects are never used. Unity runs with
`-batchmode -nographics -quit -projectPath <owned-project> -logFile <local-log>` through the
existing cancellable process runner and its per-invocation Windows Job. Provider containment
also ends the Editor on provider exit. No credentials or activation arguments are supplied.
See [Unity's Editor arguments](https://docs.unity3d.com/2022.3/Documentation/Manual/EditorCommandLineArguments.html).

Completion has `schemaVersion: "0.1"`, `state`, `editorVersion`, `checkedAt` and `reason`
(a fixed path-free sentence or `null`):

- Exit 0, a complete readable log and no license error: `active`.
- `No ULF license found`, `License is not active`, or `No valid Unity Editor license found`:
  `not_activated` when the client is running at completion; `needs_hub_client` when stopped;
  `unknown` when the client cannot be observed.
- Timeout, cancellation, unreadable/oversized log or other failure: `unknown`.

Only the three closed error categories are extracted from local logs. No log line, stdout,
stderr, account or path enters results, events or errors. Negative/unknown outcomes complete
with task warnings; cancellation ends `cancelled`. An executed check atomically saves only
`{state, editorVersion, checkedAt}` to `unity-license/last-check.json` in the provider data
directory, including `unknown` after cancellation. Queued cancellation changes no record.

The desktop routes these operations to AMF when ready. Passive detection also works on the
host when AMF is unavailable; that fallback has no AMF-owned historical check. MCP exposes
only the read-only `vua_unity_license`; real checks require a player-requested UI command.

## Verification

[Schemas and vectors](../../schemas/unity-license/v0.1/) are consumed by Rust and TypeScript.
Synthetic tests inject paths, Hub/process observations and the runner. They cover hints,
privacy, outcomes, persistence, cancellation and exclusion with AMF work. Real Editor
execution remains a manual check; this face does not establish plan item 11's fresh-machine gate.

## Document changelog

- 0.1.0 (2026-10-19): add metadata-only license hints, an exclusive managed Editor check,
  private persisted outcomes and a read-only MCP query.
