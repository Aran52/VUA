# VUA share-link service

A small Node service for public Recipe declarations and BOOTH product selections. It serves
five-language material pages with no player titles, scripts, third-party resources or analytics.
The [Candidate protocol](../../docs/protocols/share-links-v0.1.md) owns wire behavior, code
vectors, validation, privacy and withdrawal semantics.

## Develop and verify

Use Node 24.14 or newer and the repository's pinned pnpm. From the workspace root:

```sh
pnpm install --frozen-lockfile
pnpm --filter @vua/recipe-links test
pnpm --filter @vua/recipe-links typecheck
```

Tests use Web Requests, synthetic documents, in-memory storage and isolated temporary
directories, without network requests. `test` builds the existing contract guard and this
service. The `share-links` job in `.github/workflows/ts.yml` runs both checks on Linux.
The service has no Rust, browser or native database dependency. Ajv validates the bundled
2020-12 schemas; the existing contract package supplies Recipe semantic checks. Dependency
licenses remain in the installed packages (Ajv and its runtime dependencies use MIT).

For a foreground local preview, build the dependency graph and configure a dedicated
absolute data directory outside the checkout:

```sh
pnpm --filter @vua/recipe-links... build
PUBLIC_ORIGIN=https://ph-r.com DATA_DIR=/tmp/vua-share-preview TRUST_PROXY=0 pnpm --filter @vua/recipe-links start
```

PowerShell can set the same variables with `$env:PUBLIC_ORIGIN`, `$env:DATA_DIR` and
`$env:TRUST_PROXY`. Set the data directory to a dedicated absolute Windows path. The listener
is `http://127.0.0.1:6191`; returned public URLs use `PUBLIC_ORIGIN`, never the incoming Host.
The preview's synthetic documents must not be copied into production storage.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `PUBLIC_ORIGIN` | `https://ph-r.com` | HTTPS origin with no path/query/credentials |
| `DATA_DIR` | required | Dedicated absolute directory for public documents and credential hashes |
| `PORT` | `6191` | Loopback-only TCP listener, 1–65535 |
| `PUBLISHES_PER_HOUR` | `20` | Sliding-hour attempts per source, including invalid attempts; 1–1000 |
| `TRUST_PROXY` | `0` | `1` trusts one `X-Real-IP` from a loopback peer; proxy must overwrite it |

Run one Node writer process per data directory. The default rate limiter keeps at most 10,000
source/day buckets and discards expired counts and salts. It persists no IPs, request logs or
analytics. Restarting resets these ephemeral limits. Do not add a process cluster, multiple
hosts or persistent request logging without a new privacy/storage design.

Local-directory storage avoids a native SQLite build and extra database process on the small
existing host. The storage interface has an in-memory test implementation and can later accept
a transactional adapter without changing HTTP consumers. Records are atomically replaced,
with synced writes, mode 0600 and a mode-0700 directory on Unix. Only credential hashes are
stored. The local client retains the actual credentials; the server cannot recover them.

## Prepare a reviewed build

Production installation is performed by the author after review; this slice does not modify
the live server. Build from the reviewed commit on a development or CI machine:

```sh
pnpm install --frozen-lockfile
pnpm --filter @vua/recipe-links... build
```

Stage a clean repository export plus `packages/contracts/dist` and
`services/recipe-links/dist` in `/opt/vua-links`. Do not copy `.git`, local operations,
credentials, profiles, private evidence or an existing data directory. The build bundles exact
Recipe/selection schemas into `services/recipe-links/dist/schemas`; runtime validation never
downloads schema URLs. Keep the workspace manifests and lockfile, the contract package and
this service's package manifest. Install only their runtime dependency graph on the server:

```sh
pnpm --filter @vua/recipe-links... install --prod --frozen-lockfile --ignore-scripts
```

Build outside the production host to avoid competing with the bot for memory. Review the
provided memory limits against actual headroom before enabling the service. Preserve the
previous service build and proxy configuration as a rollback point.

## systemd

Install Node at the system path used in
[`deploy/vua-recipe-links.service`](deploy/vua-recipe-links.service), or adjust `ExecStart` to
the actual system Node executable. Copy the reviewed unit to `/etc/systemd/system/`, check
the public origin and chosen port, then:

```sh
sudo systemd-analyze verify /etc/systemd/system/vua-recipe-links.service
sudo systemctl daemon-reload
sudo systemctl enable --now vua-recipe-links.service
sudo systemctl is-active vua-recipe-links.service
```

`DynamicUser` and `StateDirectory` create the service's private durable directory. The service
binds only to loopback; expose it through the selected HTTPS proxy. Startup output contains a
fixed readiness/failure message, without IPs, URLs, request bodies, codes or credentials.
No request middleware logging is enabled.

## Existing PH-R Nginx integration

The existing host uses Nginx and existing certificate management. Preserve those listeners,
the five-language portal, `/bot/`, statistics, immutable download routes and the retired API
responses. Do not install a competing Caddy listener or re-run historical portal generators.

Back up the current virtual-host configuration first. Install
[`recipe-links-proxy.conf`](deploy/recipe-links-proxy.conf) as the named Nginx snippet, then
include [`nginx-locations.conf`](deploy/nginx-locations.conf) inside the existing HTTPS server.
Review location precedence against the current configuration, especially a retired `/api/`
prefix. The exact/longer `/api/shares` locations are scoped exceptions, not restoration of the
old API. The case-insensitive route handles `/r/`, `/R/` and API case variants.

The proxy replaces `X-Real-IP` with `$remote_addr`; it must not pass a client-provided value
through unchanged. If another trusted edge proxy is later added, establish its client-address
policy explicitly. Access logging is disabled for these locations; the scoped error sink avoids
recording client IPs. The service pages must not inherit the portal's CSP or theme injection:
their CSP authorizes one inline style hash, not the portal's script/font resources. The snippet
owns its `add_header` scope to avoid an additional inherited CSP, and supplies HSTS/nosniff.
Preserve any stricter existing host-wide HSTS policy when adapting that line.

The 64 KiB proxy limit returns the same JSON `too_large` error. Request/response buffering and
proxy caching are disabled, so publication receipts are not stored in proxy caches/temp files.
Check before graceful reload:

```sh
sudo nginx -t
sudo systemctl reload nginx
```

Then check a material page, anonymous `/bot/status.json` (still 401), the release checksum URL
(still 200), and retired `/api/` and `/vrc/` (still 410). Never replace an immutable release or
publish private configuration while collecting evidence.

## Standalone Caddy and automatic HTTPS

For a separate host with free ports 80/443, use
[`deploy/Caddyfile.example`](deploy/Caddyfile.example). Replace `shares.example.com` with the
chosen DNS hostname and use the same HTTPS origin in the Node unit. The public DNS must point
to that host and its certificate-challenge ports must be reachable. Caddy's
[automatic HTTPS](https://caddyserver.com/docs/automatic-https) manages certificates and renewal.
The proxy replaces the trusted source header; no access-log directive is enabled, and the
standalone logger excludes HTTP access/error events that can contain request metadata. Validate
the adapted configuration with `caddy validate --config /etc/caddy/Caddyfile` before reload.
No DNS, certificate or firewall change is performed by this repository slice.

## Backup and recovery

Stop the Node service for a consistent directory backup, preserve the private directory and
its ownership/modes, and restart it. Back up the entire `DATA_DIR`, including all per-code
records; the canonical text and hashes together define outstanding publication claims. Treat
the backup as private and restrict the archive to its operator. Do not publish it or back it
up into the served website. No IP counter database needs copying.

Restore only while the service is stopped. Restore the compatible build/configuration and
whole data snapshot, verify directory access, restart, then read an existing known code and
withdraw a synthetic test publication. An old data backup can restore a previously withdrawn
claim: reconcile that consequence with the author before reverting live data. Roll back code
or proxy changes independently when the data format did not change. Corrupt records fail
closed; preserve them for private inspection instead of overwriting them.

## Author's deployment acceptance

Use `curl` to POST a synthetic declaration from the schema examples, with
`Content-Type: application/json`, to the configured HTTPS `/api/shares`. Keep the returned
credential private. Open the returned `/r/<CODE>` URL on a phone: verify a readable material
list, BOOTH links, the purchase notice, `Open in VUA` and `Get VUA`, without the player title.
DELETE with `Authorization: Bearer <deleteToken>`, then verify both JSON and material-page
reads return 404 if no other publisher retains the same declaration. Use a unique synthetic
title for this check to avoid touching anyone else's claim. Desktop custom-protocol handling
and image/pixel-code reading are owned by the separate turntable-share client slice.

Five-language strings await native review. HTTP/schema/storage tests and local browser checks
do not establish production HTTPS, actual phone behavior or the full X-image-to-client flow.
