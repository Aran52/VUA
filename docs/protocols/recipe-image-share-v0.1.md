# Recipe image sharing v0.1

> Document version: 0.1.0
> Status: Candidate
> Updated: 2026-10-29
> Owner: AMF

The desktop exports an existing, current 60-frame turntable as one selected PNG frame or
an animated GIF. An export without a Recipe carries VUA's existing logo. An export with a
Recipe first uses the existing source review, requires a fresh public-publication opt-in,
publishes the declaration, verifies its code, and draws the code into a VUA inside badge.
Only the declaration leaves the computer. Paid assets, textures and Avatar files do not.

## Declaration transport and ownership

The Main process alone contacts one build-configured HTTPS origin (`VUA_SHARE_ORIGIN`, default
`https://ph-r.com`). Redirects and credentials are disabled. Requests have a 15-second deadline;
response streams and canonical declarations are limited to 65,536 UTF-8 bytes. Titles have at
most 120 Unicode code points. JSON with duplicate names, nonfinite values or unpaired surrogates
is rejected. Codes are the first 60 bits of SHA-256 over RFC 8785 canonical JSON, encoded as
12 uppercase Crockford base32 characters. The four [fixed vectors](../../schemas/share-links/v0.1/code-vectors.json)
are copied unchanged from the link service (PR #78), including its selection vectors.

`POST /api/shares` yields `{code,url,deleteToken}`. The client verifies the code and exact URL
before using either. Links have no TTL. Every independent publication claim has its own
withdrawal token. All claims this client receives for a code are kept in the AMF data root's
`share-links.json`, written through an exclusive queue, private temporary file, sync and atomic
rename. The renderer receives only `{code,url,recipeId,publishedAt}`. Withdrawal spends this
client's claims; other publishers can keep the declaration available. A save failure after
publication retains ownership and reports where to find and withdraw the link.

## Controlled image carrier

The reusable Rust `share_image` module accepts an RGBA image, code and one of four corners.
Its ECC200 Data Matrix rectangle is 26 × 12 modules, containing `VU1:` and the 12-character
code. This standard rectangular alternative has error correction and finder borders while
using less height than a square QR symbol. Modules use multiples of eight pixels. The controlled
badge plate uses gray 240 and luminance levels 238 and 242 (±2). An
8% inset retains the plate after a 5% crop on every edge. The decoder first locates the known
plate, samples its grid and applies ECC200 correction, then tries amplified residuals with a
general Data Matrix detector. Completely uniform, known neutral Unity background rectangles
may carry additional copies; a pixel-by-pixel check excludes the Avatar silhouette. Arbitrary
backgrounds use the badge as their independent carrier.

PNG exports use 1024 pixels. GIF starts with all 60 frames at 1024 pixels and reduces size to
768, then 512, then frame count to 30, 20 or 10 only as needed to meet 8 MiB. The palette reserves
the exact carrier grays, with no carrier dithering. The GIF reports its actual dimensions and
frame count. A foreground palette is quantized independently. No Recipe is written in metadata.

## Owned artifacts and native calls

The existing one-MiB provider frame limit is retained. `amf.production.exportShareImage`
is a command with closed params `{schemaVersion:"0.1",runId,format:"png"|"gif",index:0..59,code:null|string}`.
Only current baked frames from the selected run can be exported. `amf.production.readShareImage`
is a query with closed params `{schemaVersion:"0.1",inputId}`. `inputId` is exactly
`input-<36 lowercase hexadecimal/hyphen characters>.bin`; it is never a caller-selected path.
Request schemas and positive/negative vectors are owned by [amf-production-flow](../../schemas/amf-production-flow/v0.1).

Main stages input in `<AMF root>/amf-production/share-images`. Both processes check the cache's
resolved containment in the AMF root; regular-file checks reject symlinks. Outputs are opaque
`share-<UUID>.png|gif` IDs with length and media type. Main reads only this private cache,
checks size and file signature, saves to the user's native dialog choice, and removes artifacts.
Derived files and native paths never pass to the renderer. A read returns a PNG preview at
most 480 pixels and 1 MiB, allowing it to fit the retained wire limit.

## Receive and failure states

One PNG, JPEG, GIF or WebP may be dropped onto the window or selected in Recipe import.
Inputs are limited to 20 MiB, 4096 × 4096 pixels, and 128 MiB decoder allocation. GIF tries
successive frames until a code is found, bounded by 120 frames and 64 million decoded pixels.
Native file paths come from Electron preload and stay outside the renderer API. A successful
read retrieves the declaration, verifies its digest and v0.4/v0.5 schema, then opens the existing
import preview with the received image. Import still needs the player's explicit save action.

The installed NSIS build registers `vua://share/<CODE>` at runtime. Development and portable
builds do not register it. Only that exact protocol grammar is accepted. Configured HTTPS
`/r/<CODE>` links and retained `vuar` text use the same preview. Case normalization is ASCII only.
Image has no code, removed link, network failure, oversized image/declaration and invalid
declaration are distinct localized failures. Pending requests survive AMF-disabled navigation.

## Acceptance

Offline tests cover publication and withdrawal ownership, transport bounds, schema/code
verification, native file handling, palette and GIF size, synthetic carrier transforms and false
positives. Two synthetic desktop views cover export and read failure. See the [measured carrier report](../development/turntable-carrier-evidence.md) for the selected contrast. True X recompression and another installed computer remain
an author-run acceptance after the link service is deployed: publish P-02 as PNG with Recipe,
upload to X, drag every available 680/1200/2048 image size into the other computer, and confirm
the same declaration opens while the background appears uniform. Local transform results do
not certify that real-world acceptance.

## Changelog

- 0.1.0 (2026-10-29): add desktop publication ownership, controlled image carrier, PNG/GIF export,
  image and link import, and installed protocol handling.
