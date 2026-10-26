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
captures, show and quit. Long actions run through the actual preload and Main/native update services.
Claims are persisted before dispatch, so a restart cannot replay a pending action implicitly.
The driver accepts no URL, executable or arbitrary JavaScript. Its API-driven results do not
replace the separate button/human UI gate.

The diagnostic can reject one exact AMF manifest's custom-protocol requests, or withhold launch
health from one exact Host build. Reports must identify these as injected failures. They exercise
real Chromium/native recovery rather than replacing Gateway or provider facts with mock results.
The profile-local trace records rejected module requests and whether the ready module's calls
and shell services have resumed. `show` exposes this same isolated installation for human review.
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

## Recorded installed QA run — 2026-10-26

The real run used Windows 11 Pro x64, build 26200, an owned installation/profile and the
production Ed25519 trust anchor. All updater calls crossed the real preload, Main and native
helpers. No source checkout or development dependencies were needed by the installed process.
The profile held synthetic sentinel rows in the actual BDL, Host-task and AMF-task databases
and a user-file sentinel; this was not a paid-material or signed-in account fixture.

| Artifact | Version | Source commit |
| --- | --- | --- |
| Actual prior public installer | Host 0.7.0 / AMF 0.2.0 | `32a2c9705394e2470e9c0ebccdaa88c949b20ce8` |
| Signed bridge/baseline installer | Host 0.8.0-qa.1 / AMF 0.2.1-qa.1 | `21abe8c391d0a8254405810bd8dd03903caf8635` |
| Independent AMF target | 0.2.1-qa.2 | `34ba2aecc0245d46388f06a8f797829a81098761` |
| Independent Host target | 0.8.0-qa.2 | `2235b9292d0fd2da8093fe15d1ec61c5d2fb5978` |

The AMF inventory digest is
`941a807388287e1a8a507ee2d60d4b9c87bb6be6eb8e4a4ab72bf90a000032d4`; the Host
inventory digest is `a0945bbbba5ff5fb35db27da3d3646735f0f282797ba6b0dcbe565f7691d638c`.
These exact QA versions identify retained immutable artifacts, not production version changes.

| Observation | Measured result |
| --- | --- |
| Legacy bridge | Installed the actual prior public offline installer, then the signed QA bridge over it; root capabilities changed and all three database sentinels, module preference and user file survived startup |
| Official independent publication | AMF and Host were exported and atomically published separately to the restricted ph-r.com feeds; signatures, exact inventory digests and a 206 range were verified; publishing Host retained the AMF feed |
| Independent failure | Before the Host feed existed, its check failed while AMF discovery and staging succeeded |
| AMF delta | Native discovery planned 29,681,263 downloaded bytes and 5,437,644 reused bytes; staging and actual provider/UI activation succeeded |
| Host delta | Seven actual 206 responses transferred 3,481,120 bytes for a 60,875,424-byte Host payload; staging preserved loaded code |
| Runtime/network | Electron 44.9.3 was reused; runtime download was zero. A process-scoped TLS proxy permitted only ph-r.com, rejected an actual GitHub negative probe and recorded no additional denied upstream attempts; all signed runtime sources were outside that allowlist |
| AMF hot activation | Main and the core Host provider kept their process identities while the AMF provider and actual rendered Materials generation changed to the exact signed target |
| Open-page protection | Applying with the actual AMF Materials page open refused before stopping either provider; the page remained mounted |
| AMF UI failure/retry | An exact target custom-protocol request was rejected and recorded; the previous provider/UI and uncommitted data were restored, then the same target succeeded with a fresh import graph after clearing the fault |
| Shell readiness | The real trace confirmed calls were admitted and module shell/image-cache services resumed after both restore and successful replacement; two regression tests first reproduced the missing readiness notification |
| Disabled AMF | Check, restore, stage and apply completed with the saved preference disabled and no regular AMF service process; enabling afterward rendered the new generation without a Host restart |
| Host failure/retry | Withheld health for the exact Host target; the root stopped its trial processes and restored the old Host while retaining the independently updated AMF. Clearing the fault and retrying committed a healthy new Host |
| In-transfer cancellation | Cancelled after more than 256 KiB of encrypted response traffic arrived; the app reported interrupted/cancelled and retained the active pair |
| Helper interruption | Terminated the real staging helper during transfer; Main/provider continued, the active pair stayed unchanged, and the subsequent application restart did not apply the partial update |
| Corruption | Verified a flipped byte inside the served AMF provider range. Both range delivery and full retry were refused with the public `sources_failed_retry_or_offline` error; the original immutable object and installed selection were retained |
| Full fallback | The exact QA object ignored Range and returned 200; native progress reported `fullPack: true`, the correct full pack staged successfully and the active pair stayed unchanged |
| Data retention | All baseline database and user-file sentinels survived Host failure/retry, AMF failure/retry, hot restore and disabled activation; the writer remained BDL 0.11 |
| Human review | On 2026-10-26 the author accepted About/Modules, dark/light display and check/disable/restore behavior in the opened isolated QA installation |

Temporary server rate, corruption and Range faults were cleared. The actual product's latest
selection remains 0.7.0; no production tag, product release or public component-channel promotion
was made. QA discovery remains restricted to the test client, with an independent client's 403
verified. The other UI-development checkout was not changed or stopped.

Raw receipts, process records, profiles, traces and captures are retained locally under the owned
`_local_ops/component-updates/qa-20261026/` tree: in particular `evidence/host-healthy-journal.json`,
`host-failed-trial-recovered-journal.json`, `amf-lifecycle-regression.json`,
`amf-disabled-activation.json`, `transport-cancel.json`, `transport-helper-interrupted.json`,
`transport-corrupt.json`, `transport-full.json` and `transport-server-sanitized.json`.
They are not repository fixtures or release attachments.

Clean-Windows and real bounded-volume low-space acceptance remain unperformed. The author
explicitly retained both as pending on 2026-10-26 because no suitable test environment was
available. This run also does not claim a live cross-writer migration, paid-material production
or real-account regression; the separate controlled migration tests retain their stated scope.
Do not convert these remaining limits into a full U6/N7 or production-release acceptance claim.
