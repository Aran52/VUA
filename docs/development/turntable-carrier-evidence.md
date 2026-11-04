# Synthetic turntable carrier evidence

> Updated: 2026-10-29
> Scope: offline image transforms; no real X acceptance claim

The reusable Rust carrier selects **±2 luminance levels** on a gray-240 badge plate. A
26 × 12 ECC200 Data Matrix stores the version marker and 12-character declaration code.
The plate locator samples that known grid before falling back to a general detector. This
allows a lower contrast than general detection alone on busy backgrounds.

Four synthetic backgrounds (uniform neutral, gradient, fine checker texture and deterministic
photo-like noise) each carry a different service-vector code and badge corner. They include
a synthetic central Avatar silhouette. The source is 1024 × 1024. Every cell below includes
both the complete image and a crop removing 5% on **each** edge, for eight cases per cell.

| JPEG output width | Quality 85 | Quality 75 |
| --- | --- | --- |
| 1024 (1×) | 8/8 | 8/8 |
| 768 (0.75×) | 8/8 | 8/8 |
| 512 (0.5×) | 8/8 | 8/8 |
| 680 | 8/8 | 8/8 |
| 1200 | 8/8 | 8/8 |
| 2048 | 8/8 | 8/8 |
| Total | 48/48 | 48/48 |

The selected contrast reads **96/96 (100%)**. The minimum search tests integer contrasts
from 1 upward. At ±1 the uniform-background image at width 512, JPEG quality 75, without
cropping fails after 10 earlier cases succeeded. Even if all remaining cases passed, that
contrast could achieve at most 95/96 (98.958%), below the requirement. The test stops rejected
contrasts at their first failure, then runs all 96 cases for a passing contrast; it asserts the
selected constant equals the first passing integer. No success rate for untested cases is implied.

Additional tests reject ordinary images and logo-only images on all four backgrounds, read
all four badge corners, recover a neutral-background repeat after the main badge is removed,
read a code from a later GIF frame, and enforce a 60-frame GIF below 8 MiB with exact reserved
carrier palette entries. Native provider and Main-process tests run without external network.

The enlarged comparison uses only synthetic material: the left side is the unmarked original,
the right side is the VUA inside badge and low-contrast carrier at 3× nearest-neighbour zoom.
The complete original and marked PNGs are retained with local evidence, outside Git.

![Synthetic original and carrier at 3×](evidence/turntable-badge-comparison.png)

Real acceptance remains separate: after the share service is deployed, the author exports
P-02 with Recipe, uploads it to X, then drags each available image-size variant to a second
installed VUA. The same Recipe must open in import preview; the author checks that the
background appears uniform. This report does not claim that external compression or human
visual check has passed.
