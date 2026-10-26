# Windows component update acceptance

> In progress · Owner: Integration · Updated: 2026-10-26

This procedure exercises the real installed application and authenticated official transport.
It is the U6 gate in the [distribution plan](windows-distribution-plan.md), not evidence that
the gate has passed. Keep raw receipts, process observations, profiles and captures local under
an owned QA root. Publish only sanitized versions, source identities and measured results.

## Isolated source and publication

Prepare clean, committed QA source revisions with visibly distinct prerelease versions for the
baseline and independent Host/AMF targets. Do not reuse a published component version for changed
bytes. Keep the actual production trust anchor and private signing key on the publishing machine.
The server receives only public signed inventories/packs and the exported operator tool.

The full installer and independent exports verify the same native/UI source, version, data ABI
and host trial-fence facts. Test the transition from the actual prior installer in an owned
installation/profile. Retain a profile sentinel and validate application/database state after
upgrade; no existing user installation or paid material is an acceptance fixture.

During QA, restrict the new component-feed endpoints to the test client's observed address.
Keep existing product download/release routes and latest product selection intact. Immutable
test objects/pages have explicit QA versions. Check the server configuration before reload,
verify public object digests and an exact 206 range, and verify that an independent client cannot
discover the private QA channel. Installing a server-side publishing runtime is an operator
dependency, never an end-user requirement. Keep it in its own private directory.

## Installed application driver

`scripts/installed-update-qa.mjs init <new-absolute-root>` creates an ownership marker.
`profile <root> <case>` creates a profile below that root. Start its verified installation's
`VUA.exe` with `--vua-smoke-test=<profile>`. Only this explicit packaged diagnostic reads
`component-update-qa.json`; normal startup has no command-file listener or fault injection.

`ready <root> <case>` observes a live Main process. `send <root> <case> <action>` supports snapshot,
check/stage/cancel/apply/restore for a named component, module enable/disable, bounded navigation,
captures and quit. Long actions run through the actual preload and Main/native update services.
Claims are persisted before dispatch, so a restart cannot replay a pending action implicitly.
The driver accepts no URL, executable or arbitrary JavaScript. Its API-driven results do not
replace the separate button/human UI gate.

The diagnostic can reject one exact AMF manifest's custom-protocol requests, or withhold launch
health from one exact Host build. Reports must identify these as injected failures. They exercise
real Chromium/native recovery rather than replacing Gateway or provider facts with mock results.
Default packaged smoke behavior is retained when the QA configuration is absent.

## Required observations

| Scenario | Evidence required |
| --- | --- |
| Transition installer | Actual prior installation, changed root capabilities, retained profile/sentinel/database, real startup |
| Separate detection | Independent fixed signed feeds, independent failures/preferences/cancellation, disabled AMF check |
| Delta delivery | Server requests/ranges and real progress; unchanged runtime makes zero runtime requests/bytes |
| AMF activation | Main/Host process identities survive; AMF process and actual rendered module generation change together |
| Disabled/busy/dirty | No unwanted enable; refusal before stopping AMF; host remains usable |
| AMF failure/retry | Actual protocol/provider failure, restored UI/provider/data, retry uses a fresh import graph |
| Host activation | Staging leaves running code intact; explicit apply restarts into the exact signed target and retains AMF |
| Restart failure | Failed exact target receives no health; root stops its trial and restores a usable compatible selection |
| Cancellation/interruption/corruption | Active pair/data remain intact; restart does not silently apply; explicit retry repairs/reuses verified files |
| Range fallback | A 200 full response is validated and reported as full fallback rather than mislabeled differential transfer |
| Low space | Real bounded-volume failure, no destructive pointer/data switch, usable retry after space is restored |
| GitHub blocked | Actual discovery/object transfer succeeds through ph-r.com without GitHub traffic |
| Clean Windows | Installer/start/update/remove without development tools or prerequisites; retained user data |
| Human UI | About/Modules in both themes; check/download/cancel/apply/restore/disabled actions, clear state and next action |

Record each case's date, source/artifact identities, environment, procedure, measured outcome and
limits. A synthetic inventory test, imported-module check, successful build, screenshot or ready
marker alone does not prove this matrix. Keep U6 open for every missing real observation.
