# Website tests v0.1

> Document version: 0.4
> Status: Candidate
> Family: website-tests

Additive query/capability `environment.testWebsites`, carried by application contract v0.1 and
Desktop Gateway v1. The existing `environment.checkNetwork` regional query remains unchanged.

Request params contain only `{ urls: string[] }`: one to twelve unique HTTPS URLs, at most
2048 characters each, without credentials, fragments or nonstandard ports. No cookies, custom
headers, script or shell arguments are accepted. Both TS boundaries and the Rust Provider
validate the request. Invalid params return `vua.network.invalid_intent`; absent composition
returns `vua.network.unavailable`, using `errors.network.failed`.

Success contains only `{ websiteTests: WebsiteObservation[] }`, exactly one result per URL in
request order. Each observation has `url`, nonnegative integer `elapsedMs`, `status` and
nullable integer `httpStatus`. Status/code pairs are `reachable`/200–299, `redirected`/300–399,
`http_error`/400–599, or `timeout`, `connection_failed`, `probe_error` with null status code.
An HTTP denial remains a response, not a successful website test. Transport errors contain no
raw logs or response bodies. Partial failures preserve the other observations.

The Rust adapter runs the batch concurrently and bounds redirects and elapsed time. Requests
carry no browser session. A HEAD refusal (405/501) falls back to GET response headers only.
Local website preferences are UI state; no results or account data are persisted by this query.

The four fixed Oracle regional reference origins named by [network onboarding](../architecture/network-onboarding.md)
never follow redirects. Each of these exact URLs opens three independent connections (one client
per connection, one idle pooled slot each). Every connection first completes one discarded HEAD,
with all warm-ups running concurrently; sampling then takes nine rounds, and each round takes one
HEAD per region in Tokyo, US West, US East, Europe order, so the regions in a batch experience the
same moments of the network. Consecutive samples of one region sit at least 150 ms apart.
`elapsedMs` reports the minimum kept sample; a successful regional observation adds one optional
`regional` object with `medianMs`, `p90Ms` (nearest rank), `samples`, `connections` and
`viaProxy`. Ordinary website rows carry the same four fields as before.

A failing connection (transport error, redirect, unexpected HTTP status or an explicit
`Connection: close`) drops the samples of that connection only. A regional value requires at
least five kept samples across at least two connections; an incomplete set reports the existing
failure kinds and `httpStatus` pairing. When several kinds occur together, transport timeouts
outrank connection losses, which outrank HTTP answers, which outrank protocol closures; a set cut
short only by the phase deadline reports a timeout.

`viaProxy` is true when the origin's hostname resolves into `198.18.0.0/15` (the fake-IP range of
TUN-mode proxies such as Clash) or when the environment proxies reqwest honors for this URL apply
(`HTTPS_PROXY`/`https_proxy`, then `ALL_PROXY`/`all_proxy`, minus `NO_PROXY`/`no_proxy`
exclusions). The Windows registry system proxy is outside this measurement.

A batch containing regional references takes a process-wide write lock, so its regional phase
waits for running ordinary batches to finish, and ordinary batches share a matching read lock.
Such a batch measures within a 20-second bound counted from batch start after ordinary URLs run;
ordinary batches keep their 8-second bound, timing, HEAD→GET fallback and redirect limits. In a
mixed batch the ordinary URLs run first and regional references follow, with results returned in
request order.

Ordinary website URLs retain their existing timing, including initial connection setup and
the bounded HEAD/GET fallback; this exception introduces no new method or request field.
The regional UI may present an expected anonymous root 404 as an approximate response time;
the raw observation remains `http_error`, and ordinary website cards still show HTTP 404.
Unexpected HTTP errors, redirects, rate limits and transport failures are not regional latency.

Schema: [website-test.schema.json](../../schemas/website-tests/v0.1/website-test.schema.json).
Implementation: [types and guards](../../packages/contracts/src/website-test.ts),
[Provider route](../../crates/provider-host/src/network_routes.rs),
[adapter](../../crates/project-manager/src/network_probe.rs).

## Document changelog

- 0.4 (2026-10-20): measure each regional reference on three connections with interleaved Tokyo/US West/US East/Europe rounds and a 150 ms per-region spacing; report the minimum with `medianMs`/`p90Ms`/`samples`/`connections`/`viaProxy` in a new optional `regional` observation property; regional batches serialize behind ordinary ones through a process-wide lock with a 20 s bound.
- 0.3 (2026-10-11): specify discarded warm-up and three-response median for the four fixed regional roots, preserving ordinary website behavior and the Candidate v0.1 wire/schema shape.
- 0.2 (2026-10-11): document origin-pinned geographic references and UI treatment of their expected root response, retaining the Candidate v0.1 request/result vocabulary.
- 0.1 (2026-10-07): introduce selected/custom website tests for the compact play-environment cards.
