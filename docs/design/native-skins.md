# Native skins: art brief

> Updated: 2026-10-28
> Scope: visual direction and original motifs for Frutiger Aero, Tenshi Kaiwai and Jirai Kei
> Status: built-in presets, motifs, glyph slots and title fonts implemented; extended scene assets and package loader remain separate

The [skin system](../development/skin-system.md) defines what a skin can change and its package
format. Grid remains the default. The built-in presets consume these palettes through `themes/`.
Tenshi Kaiwai locks Light and Jirai Kei locks Dark. Original SVG motifs and the title-font subsets live in `packages/design-system/skins/`;
illustrations render through `SkinMotif`, glyphs and tiles through theme tokens. Scene packs and the
community package loader remain separate work.

The author reviewed the first pass on 2026-10-28: the direction held, but each preset needed to be
truer to its real-world style. Frutiger Aero now recreates Windows Vista's Aero; Jirai Kei adds a pink
energy-drink can marked with three wavy lines and black lace edges; Tenshi Kaiwai adds blue-white
stripes and light techwear details such as zippers.

Preset display names are fixed English in every UI language: **Grid**, **Frutiger Aero**,
**Tenshi Kaiwai** and **Jirai Kei**. The stored IDs remain `grid`, `frutiger-aero`, `tenshi` and
`jirai`; this naming change does not migrate preferences.

## Visual vocabulary and the first motif set

| Preset | Ten anchors | Implementation |
| --- | --- | --- |
| Frutiger Aero | Windows Vista's Aero: a glass frame with glare streaks, a white client area, Segoe UI, glossy push buttons that light up blue, Explorer's selection, task dialogs with a footer band, aurora, orbs and bubbles | `bubbles-and-sprout.svg` and `orb.svg`; the frame, buttons, selection and dialogs are drawn by the theme's tokens. Dark mode follows Vista's black glass. [Archive examples](https://frutigeraeroarchive.org/) |
| Tenshi Kaiwai | sky blue, cloud white, blue-white stripes, wings, halo, bandages, pills, clouds, light techwear (zippers, harness hardware), pixel hearts | `wing-halo-bandage.svg`, `clouds.svg`, `zipper.svg`, `pixel-heart.svg`; blue outlines around white shapes, a second hairline inside the edge, a striped band on cards and striped selection, zipper tape under the header and along dialogs, and no dark/gothic treatment. [NOIKISU](https://noiseandkisses.com/apps/note/2894/) · [Fantage](https://fantage.co.jp/fashion/angel-kaiwai/) |
| Jirai Kei | pink on black, black lace, bows, stitched hearts, a pink energy-drink can, satin, patent-leather sheen, chains, safety pins, sweet/dark contrast | `claw-can.svg`, `lace.svg`, `lace-black.svg`; dusty pink rather than neon, a dashed pink stitch inside containers, black lace hems on cards, under the header and along dialogs, and a soft card shadow. [Dreamvs](https://dreamvs.jp/blogs/yumemagazine/20250728) · [SPINNS](https://www.spinns.jp/c/taste/w_oshikatsu) |

Reference images inform these original vectors; they are not shipped in VUA. Motifs are decorative,
load locally and have empty alt text. Removing them leaves the same information and controls.
Small windows, manual high contrast, Windows forced colors and resource-saving mode hide them.

## Rules for every asset

- **Rights.** Each asset needs a license that allows redistribution inside VUA (Apache-2.0
  application, free download): CC0, CC-BY 4.0 (credit kept in the skin manifest), SIL OFL for fonts,
  or the author's own work. Non-commercial, no-derivatives, "personal use" and BOOTH-only licenses
  are out. Record the source URL, author, license and download date for each file.
- **No third-party identity.** No characters, logos, brand drinks or trademarks from games, anime,
  VTubers or products; generic shapes only. Jirai's energy-drink can carries three wavy lines and no
  logo, letters or product name (author ruling, 2026-10-28).
- **Generated images.** Allowed only when the tool's terms grant redistribution; the manifest notes
  the tool. The Japanese community is wary of AI art, so drawn or vector assets come first.
- **Formats.** SVG for flat motifs (Claude draws these, see below); PNG or WebP with transparency
  for textures and painted pieces; WOFF2 for fonts. Every raster ships at 1× and 2×.
- **Legibility.** Motifs decorate empty space only. A screen reads the same with every slot empty,
  and text never sits on a busy texture.

## Current asset provenance

Reviewed 2026-10-28. The runtime references the six illustrations below and the glyph files listed
after them. All were drawn in this workspace for VUA as generic shapes, not imported or traced from
a third-party image. Their source
and redistribution terms are the repository's Apache-2.0 `LICENSE`, with the VUA project as the
origin. No downloaded font, character, brand logo or reference photograph is bundled.

| Asset | Created | Reference use |
| --- | --- | --- |
| `frutiger-aero/motifs/bubbles-and-sprout.svg` | 2026-10-26 | Frutiger Aero Archive supplied visual direction only; its images and Microsoft assets are not redistributed |
| `frutiger-aero/motifs/water.svg` | 2026-10-26 | Original wave paths and bubbles; no background image was copied |
| `tenshi/motifs/wing-halo-bandage.svg` | 2026-10-26 | Original wing, halo, bandage and pill shapes; no fashion photo or character art is included |
| `tenshi/motifs/clouds.svg` | 2026-10-26 | Original cloud and sparkle outlines; NOIKISU/Fantage references remain external reading |
| `jirai/motifs/ribbon-lace-heart.svg` | 2026-10-26 | Original bow, stitched heart, lace and chain geometry; no shop photography or brand mark is included |
| `jirai/motifs/lace.svg` | 2026-10-26 | Original low-contrast geometric tile; Dreamvs/SPINNS references remain external reading |
| `jirai/motifs/claw-can.svg` | 2026-10-28 | Original can, bow, heart and lace drawn in code by `make-motifs.mjs`; replaces `ribbon-lace-heart.svg` in the page corner, workshop header and Appearance preview |

The glyph slots use files drawn in code by `make-motifs.mjs`, which regenerates them exactly:

| Slot | Tenshi | Jirai | Frutiger Aero |
| --- | --- | --- | --- |
| mark: room names, section and dialog titles | `dialog-header.svg` (wing) | `dialog-header.svg` (bow) | `orb.svg` (glass orb, 2026-10-28) |
| selected: the current navigation item | `pixel-heart.svg` (2026-10-28) | `heart.svg` (stitched heart, 2026-10-28) | `bubble.svg` (2026-10-28) |
| empty: above empty-state titles | `empty-state.svg` (cloud and halo) | `empty-state.svg` (stitched heart and bow) | `empty-state.svg` (bubbles, leaf and water, 2026-10-28) |
| tiles: under the header, along dialogs; Jirai also on card hems | `zipper.svg` (2026-10-28) | `lace-black.svg` (2026-10-28) | none; Vista's glass frame and footer band take this place |

The earlier `make-motifs.mjs` and its 22 generated SVGs were committed by Aran52 as `89cbe468`
(2026-10-21) under the same repository terms. Those generated shapes are generic wing, halo, lace,
bow, heart and accessory geometry. The six illustrations above are not outputs of that script.
Reference photographs and archived HTML stay in the ignored local research directory; they are not
licensed for bundling by virtue of being viewable online.

## Title fonts

The author chose these on 2026-10-28 after a coverage test: the Japanese candidates this brief used
to list (Zen Maru Gothic, M PLUS Rounded 1c, Mochiy Pop One, Yusei Magic, Kaisei Decol) have no
simplified Chinese forms such as 车 and 变, and mainland players come first.

| Preset | Title font | Source | License |
| --- | --- | --- | --- |
| Tenshi Kaiwai | Resource Han Rounded CN Medium (rounded; simplified, traditional and Japanese forms) | [v0.990 release](https://github.com/CyanoHao/Resource-Han-Rounded/releases/tag/v0.990), `RHR-CN-0.990.7z`, SHA-256 `e7005f7b…9dc7`; the TrueType release, because the subsetter cannot read v1.910's CFF2 variable font | SIL OFL 1.1; Reserved Font Name 'Source' (from Adobe's Source Han Sans), which the font's own name does not use |
| Jirai Kei | ZCOOL KuaiLe (playful display face; simplified Chinese and Latin) | [google/fonts](https://github.com/google/fonts/tree/main/ofl/zcoolkuaile), `ZCOOLKuaiLe-Regular.ttf`, SHA-256 `812a6fc1…b71d` | SIL OFL 1.1, no Reserved Font Name |
| Frutiger Aero | Segoe UI, Microsoft YaHei UI, Meiryo UI, Malgun Gothic: Vista's own UI fonts | installed with Windows; not shipped | Windows' license |

`skins/make-fonts.mjs` subsets each source to the UI copy, printable ASCII and the 3,755 common
simplified characters (GB2312 level 1), keeps the copyright and license records, and writes WOFF2
beside the font's `OFL.txt`: about 774 KiB for Tenshi and 442 KiB for Jirai. Characters outside the
subset, and Jirai's traditional and Japanese text, fall back to the default font. The source files
stay local; [third-party notices](../../THIRD_PARTY_NOTICES.md) carry the copyright lines and license.

## Tenshi Kaiwai (light only)

The "angel" street style: clean white and sky blue, pastel lavender, soft gradients, angel wings
and halos, crosses and bandages, pills and syringes treated as cute flat icons, sparkles, clouds,
frills and small hearts. Flat, airy and a little fragile; Y2K digital shine in small doses. The
[NOIKISU overview](https://noiseandkisses.com/apps/note/2894/) supports the emphasis on soft
blue and white, purity and an unreal, weightless feeling.

| Layer | Direction |
| --- | --- |
| Color roles | canvas `#F4F9FF`, panel `#FFFFFF`, elevated `#F7FCFF`, text `#1E2A44`, secondary `#4E5E7E`, border `#B5CDE5`, disabled `#8C9AB0` |
| VUA accent | rest `#1F6FC2`, hover `#185EA6`, pressed `#124E88`, tint `#E3F1FF`, primary text `#FFFFFF` |
| AMF accent | rest `#6E52C8`, hover `#5C43B0`, pressed `#4B3796`, tint `#EEE8FF`, primary text `#FFFFFF` |
| Rooms and status | control `#347BC1`, materials `#6C7CCF`, workshop `#7A62BF`, release `#2F8F97`; success `#207B4B`, warning `#8A6100`, error `#B93A54`, info `#386FA6` |
| Type | Resource Han Rounded CN for titles (see Title fonts); body keeps the default stack |
| Shape | round: radius 14 px on cards, pill controls, no cut corners |
| Effects | faint cloud-white highlights, a second hairline inside the edge, blue-white stripes, zipper tape, small sparkles and halo arcs; no neon, dark glass or heavy shadows |
| Motion | light and bouncy: a slight overshoot easing (`cubic-bezier(0.34, 1.4, 0.64, 1)`), faster fades |

## Jirai Kei (dark only)

The "landmine" style: black and baby pink, lace and frills, ribbons and bows, hearts with stitches,
crosses, chains and safety pins, patent-leather sheen, a pink energy-drink can, cute-gloomy mood.
Dimensional rather than flat: soft shadows, embossed hearts, leather and satin textures. The
[FASBEE guide](https://fas-bee.com/blogs/blog/jiraikei-fashion-guide) specifically names black,
muted pink and white, with lace and ribbon details supplying the sweet/dark contrast.

| Layer | Direction |
| --- | --- |
| Color roles | canvas `#120A10`, panel `#1E1119`, elevated `#2A1622`, text `#FCEFF5`, secondary `#D6B3C4`, border `#72465B`, disabled `#8E6D7D` |
| VUA accent | rest `#EAA0C2`, hover `#F3BAD6`, pressed `#D285AC`, tint `#3A1A2B`, primary text `#24101A` |
| AMF accent | rest `#C3A6FF`, hover `#D7C4FF`, pressed `#A789EF`, tint `#2C2140`, primary text `#24101A` |
| Rooms and status | control `#EAA0C2`, materials `#C3A6FF`, workshop `#F49ABC`, release `#B69AD1`; success `#72D6A0`, warning `#F0C96A`, error `#FF8299`, info `#B9A8E8` |
| Type | ZCOOL KuaiLe for titles (see Title fonts); body keeps the default stack |
| Shape | black lace hems on cards, under the header and along dialogs, a dashed stitch inside containers, radius 10 px |
| Effects | soft drop shadows, a satin highlight on primary buttons, a faint lace pattern on the canvas; keep pink decorative rather than neon |
| Motion | slower and softer: base 240 ms, slow 420 ms, ease-in-out, gentle sway on decorations |

## Frutiger Aero (dark and light)

Windows Vista's Aero, as the author asked on 2026-10-28. Light mode is the classic look; dark mode
follows Vista's black glass in Media Center and the Sidebar.

| Layer | Direction |
| --- | --- |
| Frame | the header is glass: a white gloss over the top half, diagonal glare streaks and a blue tint, blurred over the aurora canvas; the main area is a white client surface with a rounded top-left corner |
| Type | Segoe UI with Microsoft YaHei UI, Meiryo UI and Malgun Gothic, Vista's own fonts; headings in Vista's dark blue |
| Shape | small radii: 3 px controls, 2 px fields and tags, 4 px panels, 6 px dialogs |
| Buttons | secondary buttons are Vista's gray glossy push buttons that turn light blue on hover; primary buttons are glossy blue (VUA) or green (AMF) with a dark outline |
| Selection | Explorer's light-blue gradient with a blue border on the current row and navigation item |
| Dialogs | a glass frame around the dialog, the title in dark blue and a gray footer band for the buttons |
| Motifs | an orb before titles, bubbles for the current item, bubbles and a leaf for empty states, the bubbles-and-sprout corner |

## Asset list

Slots marked *planned* remain future work. The first motif set is already used for the page corner,
control-panel corner, canvas and Appearance preview; the other slots are optional scene artwork.

| Slot | Where | Tenshi | Jirai | Format and size |
| --- | --- | --- | --- | --- |
| `page-corner` | sidebar's idle lower area | wing and halo | lace corner with bow | SVG, 192 × 128 viewBox |
| `empty-state` | above empty-state titles | cloud with sparkles | heart with stitches and ribbon | SVG, 120 × 96 |
| `dialog-header` | left of dialog titles, room names and section titles | small wing | ribbon bow | SVG, 32 × 32 |
| `control-panel-corner` | selected production-line header | wing, halo and bandage | bow and stitched heart | SVG, 192 × 128 viewBox |
| `control-stage-edge` | control center stage border | dotted sparkle line (tile) | lace border (tile) | SVG, 64 × 16, repeats horizontally |
| `control-stage-backdrop` | control center graph background | sky gradient with clouds | black satin with faint lace | PNG/WebP, 1600 × 900 |
| `album-cover` | empty release album | wings framing an empty frame | lace frame with bow | SVG, 320 × 240 |
| `album-photo-corners` | release album photo corners | bandage tape | heart pins | SVG, 40 × 40 each |
| `album-stage-backdrop` | behind the finished Avatar | soft sky gradient | pink spotlight on black | PNG/WebP, 1200 × 1200 |
| canvas pattern | app background | `clouds.svg`, 160 × 128 | `lace.svg`, 96 × 64 | low-contrast SVG tile |
| icons (*optional*) | replace line icons | rounded, slightly thicker | default icons with heart variants for check and bell | SVG, 24 × 24, stroke 1.8 |
| room plates | factory map rooms | wing plates | ribbon plates | SVG, 220 × 56 |
| factory scene pack | room interiors | angel workers with small wings, clouds, pill crates | workers with bows, energy-drink crates, heart lamps | sprite sheets per the scene-pack format, 400 × 190 space |
| fonts | titles | see Type | see Type | WOFF2 subset for zh/ja/ko titles, with license file |
| preview image | Appearance picker | — | — | PNG, 640 × 360, 1× and 2× |

## Who makes what

- **Claude draws the SVG motifs**: page corners, empty states, dialog headers, panel corners, stage
  edges, album covers and photo corners, canvas patterns and room plates. They are drawn with
  `currentColor` and the skin's tokens where possible, so a later palette change recolors them.
  The first set is in `packages/design-system/skins/` (see below).
- **Sourced or generated**: backdrops and textures (satin, leather, sky), fonts, the factory scene
  sprites and the optional icon set. The task for sourcing is written for ChatGPT; anything it cannot
  find under the rules above is generated by the author.

## Files

```text
packages/design-system/
  skins/
    make-motifs.mjs          draws the generated motifs, glyphs and tiles
    make-fonts.mjs           subsets the title fonts
    frutiger-aero/motifs/    bubbles-and-sprout.svg, orb.svg, bubble.svg, empty-state.svg
    tenshi/motifs/           wing-halo-bandage.svg, clouds.svg, zipper.svg, pixel-heart.svg, ...
    tenshi/fonts/            ResourceHanRoundedCN-Title.woff2, OFL.txt
    jirai/motifs/            claw-can.svg, lace.svg, lace-black.svg, heart.svg, ...
    jirai/fonts/             ZCOOLKuaiLe-Title.woff2, OFL.txt
  src/motifs/
    SkinMotif.tsx
    motifs.css
```

`frutiger-aero/motifs/water.svg` and `jirai/motifs/ribbon-lace-heart.svg` stay in the repository
but no preset uses them since the 2026-10-28 review. The built-in presets work without the package
loader; extended scene packs and the `.vuaskin` manifest remain separate work, and their absence
does not disable the presets or any application feature.
