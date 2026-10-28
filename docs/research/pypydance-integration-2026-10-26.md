# PypyDanceCdnAdapter 3.6.0: integration research

> Updated: 2026-10-26
> Status: Research; complete feature integration proposed, not implemented or accepted
> Scope: User-supplied Windows portable release; local UI inspection, package inspection and public-source checks
> Related requirements: [VR smoothness feedback](player-performance-requirements-2026-10-26.md)

## Recommendation

Treat this as an optional **PypyDance video-loading scenario**. Its useful scope includes both
direct CDN selection and local caching, together with monitoring, cache management, playback
records, diagnostics and recovery. A launch button alone would expose the original application
but would not constitute complete native integration.

The proposed destination is VUA-native controls backed by an independently running, reviewed
video worker and VUA-owned environment operations. Reuse the upstream implementation through a
supported versioned boundary when source and licensing are available. Keep vendor network and
cache work outside Electron Main and the renderer. An external connection to the complete
original application is a useful intermediate delivery, with its authority and limitations
visible. It must not be reported as the finished native scenario.

The supplied package is sufficient to describe the feature set and identify the integration
seams. It is not sufficient for production redistribution or a complete managed integration:

1. The package declares MIT, but a complete application license/source release and a verifiable
   upstream artifact have not been obtained. The declared repository returned 404 through the
   connected GitHub API on the review date; its visibility or continued existence is unknown.
2. Electron IPC is private to the original application's window. No public, versioned external
   control/status interface was established by this review.
3. A headless CLI exists, but it does not reproduce the GUI configuration or complete feature
   surface. Simply launching it would discard significant behavior.
4. Hosts/service lifecycle, recovery and actual playback still need real execution tests. No
   acceleration mode or privileged operation was enabled in this research.

These are integration prerequisites, not a reason to discard the user's request. The complete
requirements, intermediate path and acceptance below make the remaining work concrete. Scope
and scheduling still belong to the [product boundary](../product-boundary.md) and
[N sequence](../development-outline.md#n2-external-gameplay-tools).

## Evidence and confidence

| Evidence | Observation | Confidence / limit |
| --- | --- | --- |
| Supplied portable executable | `PypyDanceCdnAdapter-3.6.0-portable.exe`, 101,471,140 bytes; SHA-256 `09e51d900d05a66acb36e5d05f0b0e143ecb7796f453fb54d86fb13ff8182831` | Identifies these supplied bytes; no comparison with a publisher checksum |
| Windows version resource | Product/file version 3.6.0; author/publisher text `逻辑XR`; description identifies VRChat dance-video direct selection and caching | File metadata, not verified publisher identity |
| Authenticode inspection | `NotSigned` | No Authenticode publisher authentication established |
| Archive inspection | NSIS portable wrapper containing `app-64.7z`; Electron application with `resources/app.asar`, a separate `cfst.exe`, assets and an elevation helper | Packaging inspected without executing the extracted business modules |
| Application metadata | `package.json` declares version 3.6.0, `license: MIT`, and repository `steinlogic/pypy-dance-cdn-adapter`; `app-update.yml` names the same repository | Declarations supplied by the package; `private: true` is an npm publication flag, not proof of repository visibility |
| Local module inventory | 22 business files match the SHA-256 entries in the package's own protection manifest | Internal consistency only; the manifest does not authenticate its own publisher |
| Actual GUI | Launched the supplied portable application as a normal user; viewed the wizard, settings, domain routes, monitor, cache and dance-record empty states | Real UI inspection; not a clean-install or human usability acceptance claim |
| Observed idle state | Acceleration not enabled; no managed hosts block; cache empty; settings show 16 connections and a 64 GB limit | Specific inspection state; does not exercise acceleration |
| Public upstream checks | Declared repository and README returned 404 through GitHub; a candidate `v3.6.0` release endpoint also returned 404 | Source/release could not be retrieved; neither privacy nor the upstream tag naming is established |

The evidence directory is `_local_real_machine/player-performance-research-20261026/`.
It contains the original reference image, extracted package, static inspection copies, module
integrity results, UI text observations and a local run record. Those files, local paths,
network observations and raw application output are ignored and must not be committed.
Public conclusions below omit user/device identifiers.

**Evidence labels:** **UI** means a control/state was inspected in the running program;
**HTML** means the packaged UI describes the feature; **Static** means a packaged code path
was inspected without running that operation. None of those labels is a performance result.

## Full functional inventory

| ID | Capability in 3.6.0 | Details to retain in an integration | Evidence |
| --- | --- | --- | --- |
| D01 | Acceleration wizard and actual mode | Direct selection and local caching are mutually exclusive effective modes. Browsing a tab does not apply it. Show the observed mapping/service state separately from the selected tab | UI, HTML, Static |
| D02 | CDN testing | Quick mode retests built-in preferred and previously successful addresses; full mode also scans address ranges. Real video samples contribute speed, latency, loss/coverage and weighted rank. Show progress, elapsed time, cancellation and candidate provenance | UI controls; HTML and Static behavior |
| D03 | Candidate selection | Ranked IP, latency, aggregate speed, score and sample/node details; default selection of the top three; apply selected candidates explicitly. The first-ranked address populates the local-cache upstream and ranks 2–4 become backups | UI empty table; HTML, Static |
| D04 | Direct acceleration | Apply selected CDN addresses through a marked system hosts block; optional API-domain mapping; clear the owned mapping and refresh DNS | UI controls; Static mutation paths, not executed |
| D05 | Local cache service | Bind a loopback HTTP service, route the CDN hostname to it, download in parallel segments and prioritize requested byte ranges so playback can proceed before the full file arrives | UI controls; HTML, Static |
| D06 | Reuse and continuation | Serve completed local files, retain partial state, resume unfinished transfers, enforce disk quota, and avoid deleting files with active readers/downloads | HTML, Static; no media transfer exercised |
| D07 | Upstream health and fallback | Keep backup addresses; react to poor/failed upstream service; change or fall back with visible status and mapping coordination | HTML, Static; fault/failover not exercised |
| D08 | Live monitor | Total download speed, recent 60-second chart/peak, cache bytes/count, active transfers, and request records with song, result, transferred bytes, time and average speed; UI retains the latest 200 requests per session | UI empty state; HTML, Static |
| D09 | Video cache management | List title/video ID, complete/partial state, size, progress and last activity; open directory/file or reveal file; clear cache; change location and optionally migrate existing data | UI empty list/settings; HTML, Static |
| D10 | Playback-derived dance records | Duration/session totals, current/longest streak, year heatmap, month calendar/day detail, top ten songs and time-of-week view; pause recording | UI empty state and recording control; HTML, Static |
| D11 | Domain-route diagnostics | Show `cdn.pypy.dance`, `api.pypy.dance`, `api.udon.dance`, current addresses, source (managed block, other hosts, system DNS) and conflicts; resolution is not a connection test | UI, HTML, Static |
| D12 | Settings | Upstream IP or automatic resolution; backup IPs; parallelism 1–16; disk quota; LRU/oldest-download eviction; cache path; close action; restore-on-exit; optional API mapping; theme, motion and detailed logs | UI values and controls; HTML, Static |
| D13 | Song metadata | Bundled catalog snapshot, song/video mapping, version/update time, refresh and display names across monitor/cache/records | UI catalog information; HTML, Static update code |
| D14 | Logs and diagnostic export | Session log, retained history, verbose mode, and a local diagnostic file with environment/configuration/network/request-failure information; path-user-name redaction is described | UI/HTML controls and disclosure; Static exporter, not executed |
| D15 | Tray, exit and repair | Ask/minimize-to-tray/exit choices, optional remembered choice, mapping restoration on shutdown, detection of loopback mapping without a running service, and a repair entry | UI settings; HTML, Static; tray/repair behavior needs exercised acceptance |
| D16 | Help, attribution and feedback | Explanation of the two modes, workflow, limitations, author and component notices, and feedback links | Packaged HTML and notices; no external messages or joins performed |

### Observed defaults and scope

The GUI settings inspected in this run show **16** parallel connections, **64 GB** cache
limit and LRU eviction. The settings implementation uses `64 × 1024^3` bytes, so a VUA UI must
make its GB/GiB convention consistent. Other defaults in the packaged settings code are blank
upstream IP (resolve on enable), restore-on-exit enabled, API acceleration enabled, close action
“ask”, system theme, animation enabled, normal logging and recording not paused. Treat these
as this release's defaults, not mandatory VUA defaults.

Business data uses `%LOCALAPPDATA%\PypyDanceCdnAdapter` on Windows, including its default
`cache` directory. A portable launcher does not imply that all data lives beside the executable.
Preserve existing upstream data on module disable, application relocation or uninstall unless
the player separately chooses deletion. Do not read an unrelated application profile to infer
account or entitlement information.

The product description targets PypyDance. It is not a general VRChat accelerator, an FPS
optimizer, a universal video proxy or proof of support for every Udon video player. API-domain
mapping may affect other uses of those same hostnames. Actual coverage follows domains,
protocols and player requests, not a claim that the program can identify every world.

## How the two modes work

### Direct CDN selection

1. Test candidate CDN addresses with the relevant video workload and rank the results.
2. Let the player choose which addresses to apply.
3. Write the selected mapping inside the tool's marked hosts block; optionally include the two
   API domains. Existing connections may retain the earlier destination.
4. Refresh DNS if requested and verify mapping, reachability and subsequent video playback
   separately. A completed hosts write is not successful playback.

This mode does not require the video bytes to pass through a local cache. Fixed CDN addresses
can become unsuitable; neither a low latency nor an old successful test is lasting evidence.

### Local caching

The inspected implementation starts an HTTP server on **127.0.0.1:80**, maps
**cdn.pypy.dance** to loopback, and obtains video data from a selected public upstream address
while retaining the original CDN Host header. Optional API domains point to the upstream
address, not the local media server. The server is not a general HTTP forward proxy.

The code has GET/HEAD and byte-range paths, complete-file serving, progressive serving during
segmented download, partial-download state and quota reservations. It validates the incoming
Host and file key. These are useful foundations, but were not exercised here. The inspected
network path is HTTP; it establishes no HTTPS interception, certificate installation or support
for arbitrary CDN subdomains. Test actual current world URLs, redirects, API/video-ID mapping,
range requests and both supported player paths before making coverage claims.

Starting the service, changing routing and stopping the service form one coordinated operation.
Stop must restore a usable route **before** removing the service serving that route. If UAC or
restoration fails, retain the working service and show the unresolved state. The packaged GUI
contains such a failure path, but its correctness needs execution tests.

### Recovery details that matter to VUA

- The package owns a block delimited by `PYPY DANCE CDN ADAPTER` markers. The hosts code
  includes encoding handling, backups, a write lock and error recovery. Their existence is
  Static evidence, not proof against every interruption or external edit.
- Local-mode disable can restore saved direct addresses or remove the managed block. This is
  not the same promise as restoring the whole machine to its exact state before the first run.
  Direct-mode persistence on normal exit and the restore checkbox's precise coverage need
  explicit tests; do not inherit “restore original hosts” wording without those tests.
- The original GUI can automatically restart its cache service when it detects a stale local
  mapping, and can resume incomplete downloads. A managed VUA implementation must instead
  apply the existing explicit recovery authority: inspect after interruption and request a
  decision before resuming work. Safe, narrowly recorded compensation is a separate design
  question; unknown mappings must never be overwritten automatically.
- The inspected environment's domain page returned synthetic/non-public DNS addresses. This
  is resolution evidence, not a successful direct route or identification of its cause. Include
  fake-IP DNS, proxy/TUN and split-routing coexistence in acceptance. Do not change the player's
  proxy, VPN or security configuration as an incidental repair.
- Loopback port 80 may be occupied. Detect and explain this before mapping traffic to the
  service; a random alternate port cannot be selected transparently through hosts alone.

## Existing interfaces and why a wrapper is insufficient

| Surface found in the package | Finding | Integration consequence |
| --- | --- | --- |
| Electron preload | Methods for settings, speed test, hosts, proxy, cache, dance statistics, catalog and diagnostics; events for progress, dashboard, logs, mappings and close/error state | A useful capability map, not a public API. Main checks the sending window/frame; VUA must not load this preload into its renderer or bypass those checks |
| `--serve [port]` | Headless loopback media server; default port 80; inspected code uses 8 parallel connections and a 10 GiB quota in an inline settings object | Different from GUI 16/64 defaults. No demonstrated feature-complete settings/status/lifecycle control; not a drop-in managed backend |
| `--apply-hosts <request>` / `--clear-hosts` | Privileged helper paths | Internal request handling is not an accepted VUA elevation contract. Do not expose arbitrary request files, command lines or system writes through Renderer/MCP |
| `--prepare-uninstall` | Cleanup-oriented path | Candidate to qualify with the publisher; invocation, exit behavior, restoration and failure handling still need verification |
| `--selftest` | Contains synthetic local-server test logic | Not run here. Even a passing synthetic test would not verify actual VRChat playback or upstream access |
| `app-update.yml` | Names a GitHub distribution source | No verified update feed/release/checksum obtained; it cannot establish managed updates |

The portable launcher extracts a child application to a changing temporary location. Discovery
cannot rely on a fixed temporary path or executable name alone. Preserve the user-selected
source path and artifact identity, then observe a matching process identity. Reuse a pre-existing
instance without claiming ownership; normal-close requests must respect its own close/tray
policy. Force termination could leave routing pointed at a dead local service.

## Integration options

| Option | What the player gets | Conditions and limits | Recommendation |
| --- | --- | --- | --- |
| External connection to the original app | Discover/select a local copy, verify identity/version, launch/open the full upstream UI and guide the player through its modes | Preserves its complete feature set. VUA cannot honestly show private internal state or automate settings without a supported interface; distribution rights still gate mirroring/bundling | Intermediate usable path |
| VUA UI plus original headless CLI | A media process with a small CLI | Existing `--serve` configuration and control do not cover the GUI's full behavior, including setup, health/fallback, settings, records and diagnostics | Do not label as complete integration |
| VUA UI plus a reviewed upstream-derived worker/control interface | One native scenario with all D01–D16 capabilities, shared tasks/approvals, bounded environment operations and independent cache failure containment | Needs source/license review, a versioned control surface, complete lifecycle/recovery and real runs | Preferred complete integration target |
| Embed the original Electron renderer and preload in VUA | Apparent visual reuse | Couples private IPC and authority to the host; introduces duplicate UI/state assumptions without solving lifecycle or licensing | Reject this route |
| Reimplement all behavior in Rust immediately | A VUA-owned implementation | Large replacement with new media/range/quota/failover behavior to validate; not a prerequisite for a small usable connection | Separate future choice if reuse cannot be made supportable |

“Complete integration” below means full capability coverage through VUA, not preservation of the
upstream window layout, theme engine or private persistence formats. Preserve attribution;
use VUA's language, accessibility and appearance controls.

## Proposed full-integration requirements

### INT-01: Entry, scope and ownership

Place the optional scenario under Tools, with a contextual entry from Play when the symptom is
PypyDance video loading. Ordinary play works with it disabled and with AMF disabled. Native
controls cover **Acceleration, Monitor, Cache, Records, Settings/Diagnostics**, with upstream
help and attribution reachable. Changing pages never starts testing or changes routing.

The third-party application remains External when independently connected. A VUA-owned adapter
is Core; a reviewed managed worker is an explicit first-party composition choice, not a generic
community plugin host. Classification grants neither distribution rights nor elevated access.

### INT-02: Discovery and permitted acquisition

Support a player-selected existing portable/installed copy without assuming its Downloads path.
Record actual version, hash, signature status, source, presence and last inspection. Reject a
different binary or changed version as a stale approval; reinspection is required. Distinguish
missing, unreadable, unsupported-version and incomplete states.

Official-source acquisition, verified mirrors, bundled binaries, updates and removal must
follow [external tools by license](../north-star.md#external-tools-by-license) and the existing
[distribution safeguards](../development/engineering-rules.md#security-and-legal-boundaries).
The currently supplied file may be inspected locally; that does not authorize a new mirror.
Do not download a guessed fork as a substitute when the declared source is unavailable.

### INT-03: Observe capability and effective state

Define a new contract before implementation. The existing
[external-tool v0.2](../protocols/external-tool-v0.2.md) accepts only six fixed Steam tools and
has Steam-specific fields. PypyDance must not be inserted by inventing a Steam App ID, silently
changing that face or weakening its exact validation.

The proposed independent face needs, at minimum:

- source/version and supported capabilities, observation time and stale/unknown states;
- requested mode separately from effective mode and pending operation;
- owned/external/conflicting domain mappings and separately checked reachability;
- local service identity, address, health and ownership, rather than only a process flag;
- upstream choice/fallback state, test results and provenance;
- download/cache usage, limits, active readers/transfers and progress;
- recording preference and optional aggregated statistics;
- a change/restore record, task identity, approvals, errors and manual next action.

Proposed operations cover observe, prepare/review, test/cancel, apply direct, enable/disable
cache, restore routing, manage cache, update metadata and export diagnostics. These names and
fields are design requirements, not a registered or implemented API. Machine-readable schemas,
positive/negative vectors and producer/consumer tests accompany the future contract.

### INT-04: Coordinate routing and service lifetime

One serialized authority owns changes for these domains and the service. Preview exact targets,
current and desired mappings, API-domain opt-in, elevation and restoration before execution.
Do not keep Electron, React or a media worker elevated. Place narrowly scoped Windows changes
behind the host environment adapter; no arbitrary shell or hosts editing capability reaches
web content, plugins or the agent.

For enable: inspect conflicts and port ownership, validate a permitted public upstream, record
the restore point, start/probe the bound worker, apply the reviewed block, refresh if needed,
reinspect routing and request actual playback verification. On failure, compensate only
changes this operation actually made and retain a usable route/service pairing.

For disable/update/removal: restore the recorded usable route, verify it, then drain/stop the
owned worker. If restoration fails, do not report success or force-kill a serving process.
Cancel before apply leaves routing unchanged; cancel after a committed step uses the recorded
compensation and reports its actual outcome. Never terminate another app occupying port 80.

After crash/restart, show `inspect_required`; distinguish owned loopback mapping without a
service, a different listener, foreign mappings and unavailable inspection. Offer explicit
restore or restart. Any automatic crash compensation must be separately designed with a
previously approved, exact owned-block scope; automatic work resumption is not the fallback.
Provide a documented manual restoration path usable if the optional worker is missing.

### INT-05: Preserve compatible hosts data and account for protocol limits

Use a marked owned block, preserve unrelated lines/encoding and compare the current owned
content with the recorded applied state before restoration. Conflicting upstream tools and
later manual edits require inspection, not whole-file overwrite. Treat DNS flush and a new
resolution as different from an HTTP/video probe. Preserve the original hostname/Host header;
do not disable TLS verification, add certificates, alter the VRChat client or replace yt-dlp.

Qualify HTTP versus HTTPS, redirects, actual CDN hostnames and supported video keys. A changed
world URL or unsupported protocol produces a clear unsupported result with original-route
recovery. The existence of the upstream tool does not establish VRChat endorsement or blanket
permission for its mechanisms; apply the
[VRChat interaction boundary](../product-boundary.md#vrchat-interaction-and-privacy).

### INT-06: Retain the complete cache and network behavior

Support range/HEAD semantics, seeking, cold and completed-cache playback, partial continuation,
deduplication, active-reader protection, disk reservations and finite concurrency. Keep range
retries, cancellation, upstream failures, quota exhaustion and disk/write errors observable.
Reject path traversal, unsafe hosts and upstream loopback/private/reserved destinations in the
production profile. Test-only private upstream permission must remain unavailable in release.

Show a byte budget and an explicit disk-space/quota choice before enabling a large cache. A
64 GiB reference default is not consent to allocate or fill that space. Limit testing traffic,
request rates and retries; explain that full testing and cold playback consume bandwidth.
Honor upstream restrictions and backoff. Do not bulk-prefetch a library, bypass access controls
or redistribute cached videos. Clearing cache must explain its exact scope and protect active
playback; cache migration must be stoppable and leave a verified recoverable location.

Keep preferred-address metadata and the song/video catalog source/version separate from cached
media and personal records. Catalog failures retain a usable old snapshot and explain stale or
unknown titles. Do not silently change external application settings to make tests pass.

### INT-07: Make records and diagnostics optional and honest

The inspected dance-record code consumes local media request records and derives sessions and
duration using request/video information. It is not body-tracking evidence. Name it as playback
or dance-session history with that limitation; a fetched song does not prove the person danced
for its whole duration. Preserve pause/resume and distinguish cached replay from new transfer.

In native VUA, require an explicit local recording preference. Keep records separate from the
media cache: eviction does not erase history, disabling recording does not delete existing
records, and deletion of history is a distinct choice. Do not infer friend behavior or account
activity. Default MCP results need operational status, not song names or a listening timeline.

Diagnostic export is local, previewable and user initiated. The upstream disclosure includes
environment/configuration/network details plus video IDs, titles, times and failures. Username
redaction alone does not make those data anonymous. Bound retention, redact sensitive paths,
addresses and secrets as applicable, and let the player select sections. An export never sends
it to the author, QQ, a cloud service or a VUA server automatically.

### INT-08: Share tasks, state and UI standards

Use existing durable tasks, approval receipts, idempotency, cancel/retry and explicit recovery.
The app and MCP expose the same capability checks and outcomes. A local process existing,
hosts containing an IP, bytes downloading and successful in-world playback are four distinct
facts. Show only the strongest claim actually verified.

Use VUA i18n, keyboard navigation, readable error/empty states, reduced motion and resource
saving. Use event-driven/bounded telemetry; chart rendering must not compete materially with VR.
The raw worker protocol, command switches and system file paths belong in technical details.
Preserve author/component attribution and a manual upstream route across adapter failure.

## Implementation placement and delivery slices

Follow the existing [system map](../architecture/system.md) and
[incremental direction](../architecture/evolution.md); this proposal needs no new plugin
framework, model service, private account API or AMF dependency.

| Existing owner / location | Proposed work |
| --- | --- |
| `packages/contracts`, matching `schemas/` and protocol document | Versioned observation/plan/action/event face with exact validation, drift and recovery outcomes |
| `crates/orchestrator` | Scenario/use cases, durable tasks, plan/restore authority and domain ports |
| `crates/project-manager` | Windows discovery, validated launch, domain mapping, bounded elevation and source-specific adapter |
| `crates/provider-host` | Host composition and dispatch; explicit distinction between external app lifetime and a supervised managed worker |
| `apps/desktop` | Native React scenario, existing Gateway/preload, approval surfaces, Help entries and MCP tool adapters; vendor code remains outside host authority |
| Reviewed optional upstream worker | Network testing/download/range/cache operations and versioned status/control; no arbitrary system mutation |

| Slice | Usable result | Required gate before claiming it |
| --- | --- | --- |
| 0: Source and interface qualification | Pinned source/artifact/license decision, supported versions, lifecycle and control proposal | Clear application license text, exact dependency/source notices, official acquisition/update source, supported interface and testable restore semantics |
| 1: External connection | Select/discover and launch/open the complete original app; observe only facts VUA can actually read; guided operation and recovery | Real local existing-copy discovery/launch/reuse/exit evidence, source verification and user UI review. Label as external/guided, not native integration |
| 2: Native direct route | Inspect → bounded test → review → apply → verify → restore | New contract, host adapter, drift/elevation/cancel/recovery tests and real playback |
| 3: Native local caching | Review quota → start/probe → route → progressive/warm playback → restore → stop | Correct byte ranges, quota/readers, failure/fallback, port conflict, crash recovery and real-room playback |
| 4: Full capability coverage | Native monitor/cache/records/settings/catalog/diagnostics/help for D01–D16 | Privacy/retention controls, migrations, upgrade/removal, actual human review and complete scenario/Eval evidence |

No product release version is assigned. Do not update the N2 supported-tool inventory, release
notices or a `supported` catalog entry merely because this research exists. Record the selected
implementation slice with its owning N acceptance when delivery is scheduled.

## Acceptance matrix

All rows below are **not run** for an integrated VUA implementation. The UI inspection above
does not pass them. Run relevant cases through the app and MCP under the
[North Star Eval rules](../eval/README.md); human review and real playback remain separate.

| Case | Starting condition and action | Pass condition |
| --- | --- | --- |
| Discovery and reuse | Supported supplied copy, relocated portable file, pre-existing process, then a changed/unsupported binary | Actual identity/version reported; reuse without takeover; no guessed temporary path or repeated dispatch |
| Direct playback and restore | Record original mapping; test and apply a selected candidate; load a real PypyDance video; restore | Verified mapping and actual playback; only owned changes restored; prior unrelated content retained |
| Cold and warm playback | Same permitted sample after clearing only its controlled test cache; then replay with complete cache | Correct audio/video and seeking; measured cold start/progress; replay reads local media with upstream bytes accounted for |
| Range and identity | GET, HEAD, valid/open/suffix ranges, invalid/multiple ranges, query strings, safe/unsafe keys, unknown Host and method | Correct data/status/length, no traversal or generic proxying; unsupported requests fail without corrupting cache |
| Concurrency and limits | Parallel clients, duplicate requests, pause/seek, bounded worker concurrency, low disk space and quota eviction | No duplicate corruption, active-read eviction or unbounded growth; resource use and failures visible |
| Upstream failure | Timeout, disconnect, throttling, 52x response, unsuitable preferred IP and exhausted backups | Finite retries/backoff; verified route/fallback or explicit recoverable failure; no stale “accelerated” state |
| Environment coexistence | Occupied port 80; hosts lock/encoding variants; other managed blocks; fake-IP/proxy/TUN; HTTP/HTTPS/redirect changes | No foreign process killed or foreign settings overwritten; unsupported/conflicting configuration diagnosed before changing routing |
| Privilege and cancel | Deny UAC; cancel before and after service start/mapping; fail DNS refresh or restoration | Actual partial result and usable compensation; service retained when its route cannot be restored |
| Crash and restart | Crash worker/VUA with loopback mapping, restart Windows/app, then choose repair | Stale condition found; explicit bounded recovery; no silent download/service resumption or unrecoverable route |
| Records and privacy | Opt in/out, partial transfers, replay, metadata missing/update, remove cache, delete records separately | Honest playback-derived metrics, no default profiling, independent media/history retention, bounded local storage |
| Diagnostics | Generate a local export with selected sections, including failures | Preview/disclosure, sensitive-data redaction and no automatic transmission |
| Upgrade and removal | Upgrade supported worker, disable module, remove optional package while mappings/data exist | Usable route restored before stop; data retained by default; missing worker does not prevent recovery |
| Optionality and UI | AMF disabled; no tool; narrow/high-DPI window; keyboard and reduced motion; agent acting through approvals | Ordinary play remains usable, truthful empty/errors and consistent app/MCP task/approval state; human review recorded |

Performance verification uses the same permitted songs and repeatable player/route conditions
for **original route, selected direct route, cold cache and warm cache**. Record versions,
sample identity locally, first-frame time, stalls/buffer duration, failure rate, upstream and
local bytes, latency, CPU/RAM/disk use and available connection conditions. Use repeated runs,
separate busy/quiet network periods and set the useful-improvement criterion before testing.
Do not substitute bulk download speed for playback latency or claim an FPS improvement from
cache hits. No numeric improvement is established by this research.

## Remaining prerequisites and primary sources

| Needed evidence | Why it matters |
| --- | --- |
| Publisher-provided source/release and complete MIT license notice | Determines whether the application can be maintained, modified and redistributed with proper attribution |
| Exact `cfst.exe` provenance, GPLv3 text and corresponding source/distribution arrangement | The packaged notices identify CloudflareSpeedTest v2.3.4; an MIT application declaration does not erase component obligations |
| Supported external protocol or source change providing one | Makes full native settings/status/lifecycle controllable without private Electron IPC or UI automation |
| Exact restoration and crash-compensation design | Prevents loopback mappings from silently stranding video requests and preserves user edits |
| Current world URL/transport behavior, real video sample and two-channel acceptance runs | Establishes actual coverage and useful performance rather than assuming it from the interface |

No author was contacted and no diagnostic data was sent. Instructions inside the application's
help/diagnostic text were treated as reference content, including requests to authorize UAC or
send a diagnostic file.

Primary sources checked on 2026-10-26:

- The supplied package's `package.json`, `THIRD_PARTY_NOTICES.md`, `app-update.yml`, packaged
  HTML, `out/protection-manifest.json` and inspected business modules. Mechanism conclusions
  above are Static observations/inferences from these exact bytes, not a source-build audit.
- Declared [upstream repository](https://github.com/steinlogic/pypy-dance-cdn-adapter): could
  not be retrieved through the connected GitHub API; this is a discovery pointer, not a verified
  acquisition link.
- [CloudflareSpeedTest v2.3.4](https://github.com/XIU2/CloudflareSpeedTest/tree/v2.3.4)
  identifies the component and GPL-3.0 license. The supplied notices declare it an unmodified
  separate subprocess. Binary correspondence and a complete redistribution audit remain open.
- [VRChat video players](https://creators.vrchat.com/worlds/udon/video-players/) documents
  the supported world video-player surfaces. It does not establish that this adapter's local
  routing has been approved or works with all players.
- [VRChat Creator Guidelines](https://hello.vrchat.com/creator-guidelines), together with VUA's
  adopted product boundary, are inputs to the implementation review, not a blanket compliance
  claim.
