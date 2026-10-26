# AMF UI runtime v0.1

> Version: 0.1 · Candidate · Owner: Integration · Reviewed: 2026-10-26

The additive Desktop v7 `amfUiRuntime` face coordinates replacement of the first-party AMF
provider and renderer graph while the VUA host stays alive. Its typed authority is
`packages/contracts/src/amf-ui-runtime.ts`. It extends Desktop v6 without changing the frozen
Application, Bridge, installed-layout or component-feed contracts. It is not a plugin loader.

## Generation identity and authority

Main creates each generation from a native-verified signed inventory. A generation projects
only its 64-character manifest digest, random 32-character instance ID, semantic version,
40-character source build and `amf-ui/0.1` ABI. Paths, executables, feeds and import URLs remain
Main-owned. Every attempt, including retrying the same artifact, gets a fresh instance ID for
its complete relative import graph. Each served file is length/hash checked against inventory.

New AMF payloads carry signed `ui/component.json` facts. Native preflight probes the verified
executable's closed `--component-identity` command, without opening user data, and compares
version, source, BDL writer and service build. The running provider handshake must then identify
`amf/<version>@<source>`. The UI entry must independently export the matching version/build,
`amf-ui/0.1`, `amf-hot/0.1` and an unload-admission function. Unknown older payloads use an
explicit restart action. A hot-apply action never silently restarts VUA.

## Transaction and acknowledgements

`snapshot()` and disposable `subscribe()` return schema `0.1`, availability, phase, active
generation, optional candidate, transaction ID and error. Phases are `idle`, `quiescing`,
`loading`, `restoring`. Only the trusted main frame can respond to the current transaction and
phase; stale acknowledgements are rejected. A response accepts or refuses with `busy`,
`restart_required` or `failed`. A bounded missing acknowledgement fails closed.

Before any process stops, the renderer refuses an open AMF surface, modal or unsaved compose
draft. Quiescence does not unmount that surface. Main separately fences AMF commands, checks
nonterminal tasks and shell transfers, and waits for the old process to exit. Host tasks keep
running. Disabled AMF remains disabled and is not started merely to update it.

Native selection uses the durable recovery-v0.2 journal. A separate hot-intent marker prevents
the normal launch acknowledgement from committing a candidate before UI verification. Main
starts the selected provider, verifies its actual handshake and imports the candidate UI graph
with styles inactive. Successful provider/UI health commits selection; only then are old
styles, effects and registrations disposed. Failure stops the candidate and restores the
previous provider and UI. A lost commit reply queries actual recovery state before deciding
which generation to serve. Ordinary root launch recovers an interrupted transaction.

## Data and evidence limits

Hot replacement requires an unchanged BDL writer. Writer-changing component or pair requests
require the [AMF data recovery v0.1](amf-data-recovery-v0.1.md) snapshot, offline migration and
restart trial. The root launcher, host trial fence and target module must all support that
protocol. Older combinations require the transition installer before activation.

Controller and actual Chromium checks cover version mismatch, busy/disabled admission, dirty
state, failed import, cleanup, retry and relative chunks. The ignored native artifact test uses
a real independently built executable/UI under an installation lock, but is not a live host
process update. Real signed remote delivery, actual process-preserving activation, restart,
data recovery, transition installation and clean-Windows/human review remain separate gates.
