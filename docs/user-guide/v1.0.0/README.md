# Minimal guide for v1.0.0

> Updated: 2026-10-30
> Scope: freeze row F8; host 1.0.0 and AMF 0.3.0 Beta
> Status: candidate guide; clean-VM walkthrough and all native-language reviews pending

Read in [English](en.md), [简体中文](zh-CN.md), [繁體中文](zh-TW.md),
[日本語](ja.md) or [한국어](ko.md). These are player procedures in the five UI languages;
this index and capture metadata are English under [documentation governance](../../meta/documentation-governance.md).

The procedure starts with installation and ends with one Avatar and outfit in the official SDK.
It is the minimal freeze guide. The complete illustrated N7 guide remains deferred.
The [release notes](../../release/v1.0.0.md) own candidate limitations and pending checks.

## Screenshot provenance

The images render the actual versioned React UI through the existing design-audit/preview
fixtures, with synthetic Gateway facts and stand-in images. They show the UI's labels and states,
not a real material run or successful SDK build. No real account, order, credential, private
material or machine inventory is captured. Grid dark is shown in all five languages.

The committed [capture manifest](capture-manifest.json) records the exact clean source, renderer build identity, capture
time, locale/view, dimensions and SHA-256 for every image. Do not replace an image with a mockup,
a different version or an unrelated installation screenshot.

To reproduce from a clean checkout containing the settled first-use UI:

1. Install the locked workspace dependencies and build the desktop renderer at version 1.0.0.
2. Run the capture command below with Electron available. Its output directory must be new and
   under an ignored local evidence root.
3. Review every selected image for legibility, correct language and synthetic-only data. Raw audit
   JSON, annotated images and logs stay local; only the curated clean images and sanitized
   capture manifest belong here.

~~~powershell
node apps/desktop/scripts/capture-minimal-guide.mjs --out=C:/local/_local_f8/capture
~~~

This script refuses a dirty source, a version other than 1.0.0 or a renderer built from a
different revision. It uses the existing real-renderer fixtures and copies nine states per
language. The raw run also includes any additional views matched by the audit's prefix filter;
those are not additional guide evidence.

## Author captures and walkthrough

Installer and clean-VM images remain **pending author capture**, separately for the real online
and offline 1.0.0 packages on the recorded VM image. Capture installation completion and the
first usable home screen, with the actual artifact version/build and checksum. These images
cannot be supplied by the renderer fixture.

On that same clean VM, follow one language edition without other help, using legally held
Avatar/outfit packages. Record install/first-use outcomes, the material-to-SDK result and any
blocked step. Follow the freeze plan's separate 0.7.0 upgrade, six Steam-tool and blocked-GitHub
update rows. Keep VM inventory and raw evidence local. Native reviewers check all five editions;
Traditional Chinese remains explicitly awaiting native review.

A completed synthetic capture run does not pass F8. Only the author's clean-VM walkthrough and
human review can close its acceptance row. Publication is separate from this guide's delivery.
