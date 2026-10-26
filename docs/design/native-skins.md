# Native skins: art brief

> Updated: 2026-10-27
> Scope: visual direction and original motifs for Frutiger Aero, Tenshi Kaiwai and Jirai Kei
> Status: built-in presets and first motif set implemented; extended scene assets and package loader remain separate

The [skin system](../development/skin-system.md) defines what a skin can change and its package
format. Grid remains the default. The built-in presets consume these palettes through `themes/`.
Tenshi Kaiwai locks Light and Jirai Kei locks Dark. Original SVG motifs live in `packages/design-system/skins/`
and are consumed through `SkinMotif`; custom fonts, scene packs and the community package loader
remain separate work.

Preset display names are fixed English in every UI language: **Grid**, **Frutiger Aero**,
**Tenshi Kaiwai** and **Jirai Kei**. The stored IDs remain `grid`, `frutiger-aero`, `tenshi` and
`jirai`; this naming change does not migrate preferences.

## Visual vocabulary and the first motif set

| Preset | Ten anchors | Implementation |
| --- | --- | --- |
| Frutiger Aero | water, bubbles, glass, specular gloss, sky blue, leaf green, aurora, refraction, nature + technology, humanist optimism | `bubbles-and-sprout.svg`, `water.svg`; a glass header edge, translucent capsule actions and a wet inner rim. [Archive examples](https://frutigeraeroarchive.org/) |
| Tenshi Kaiwai | sky blue, cloud white, wings, halo, bandages, pills, crosses, frills, clouds, weightlessness | `wing-halo-bandage.svg`, `clouds.svg`; blue outlines around white shapes, cloud-white inner borders and no dark/gothic treatment. [NOIKISU](https://noiseandkisses.com/apps/note/2894/) · [Fantage](https://fantage.co.jp/fashion/angel-kaiwai/) |
| Jirai Kei | pink on black, bows, lace, stitched hearts, satin, patent-leather sheen, pearls, chains, safety pins, sweet/dark contrast | `ribbon-lace-heart.svg`, `lace.svg`; dusty pink rather than neon, satin highlights and small metallic details on black-plum surfaces. [Dreamvs](https://dreamvs.jp/blogs/yumemagazine/20250728) · [SPINNS](https://www.spinns.jp/c/taste/w_oshikatsu) |

Reference images inform these original vectors; they are not shipped in VUA. Motifs are decorative,
load locally and have empty alt text. Removing them leaves the same information and controls.
Small windows, manual high contrast, Windows forced colors and resource-saving mode hide them.

## Rules for every asset

- **Rights.** Each asset needs a license that allows redistribution inside VUA (Apache-2.0
  application, free download): CC0, CC-BY 4.0 (credit kept in the skin manifest), SIL OFL for fonts,
  or the author's own work. Non-commercial, no-derivatives, "personal use" and BOOTH-only licenses
  are out. Record the source URL, author, license and download date for each file.
- **No third-party identity.** No characters, logos, brand drinks or trademarks from games, anime,
  VTubers or products; generic shapes only (an energy-drink can is a plain can).
- **Generated images.** Allowed only when the tool's terms grant redistribution; the manifest notes
  the tool. The Japanese community is wary of AI art, so drawn or vector assets come first.
- **Formats.** SVG for flat motifs (Claude draws these, see below); PNG or WebP with transparency
  for textures and painted pieces; WOFF2 for fonts. Every raster ships at 1× and 2×.
- **Legibility.** Motifs decorate empty space only. A screen reads the same with every slot empty,
  and text never sits on a busy texture.

## Current asset provenance

Reviewed 2026-10-27. The runtime references only the six SVG files below. They were drawn in this
workspace for VUA as generic shapes, not imported or traced from a third-party image. Their source
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

The earlier `make-motifs.mjs` and its 22 generated SVGs were committed by Aran52 as `89cbe468`
(2026-10-21) under the same repository terms. Those generated shapes are generic wing, halo, lace,
bow, heart and accessory geometry. The six newer runtime files above are not outputs of that script.
Reference photographs and archived HTML stay in the ignored local research directory; they are not
licensed for bundling by virtue of being viewable online.

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
| Type | a rounded Japanese face for titles (candidates: Zen Maru Gothic, M PLUS Rounded 1c, both OFL); body keeps the default stack |
| Shape | round: radius 14 px on cards, pill controls, no cut corners |
| Effects | faint cloud-white highlights, thin white inner borders, small sparkles and halo arcs; no neon, dark glass or heavy shadows |
| Motion | light and bouncy: a slight overshoot easing (`cubic-bezier(0.34, 1.4, 0.64, 1)`), faster fades |

## Jirai Kei (dark only)

The "landmine" style: black and baby pink, lace and frills, ribbons and bows, hearts with stitches,
crosses, chains and safety pins, patent-leather sheen, a plain energy-drink can, cute-gloomy mood.
Dimensional rather than flat: soft shadows, embossed hearts, leather and satin textures. The
[FASBEE guide](https://fas-bee.com/blogs/blog/jiraikei-fashion-guide) specifically names black,
muted pink and white, with lace and ribbon details supplying the sweet/dark contrast.

| Layer | Direction |
| --- | --- |
| Color roles | canvas `#120A10`, panel `#1E1119`, elevated `#2A1622`, text `#FCEFF5`, secondary `#D6B3C4`, border `#72465B`, disabled `#8E6D7D` |
| VUA accent | rest `#EAA0C2`, hover `#F3BAD6`, pressed `#D285AC`, tint `#3A1A2B`, primary text `#24101A` |
| AMF accent | rest `#C3A6FF`, hover `#D7C4FF`, pressed `#A789EF`, tint `#2C2140`, primary text `#24101A` |
| Rooms and status | control `#EAA0C2`, materials `#C3A6FF`, workshop `#F49ABC`, release `#B69AD1`; success `#72D6A0`, warning `#F0C96A`, error `#FF8299`, info `#B9A8E8` |
| Type | a cute display face for titles (candidates: Mochiy Pop One, Yusei Magic, Kaisei Decol, all OFL); body keeps the default stack |
| Shape | lace edge on cards and dialogs (scalloped mask), radius 10 px, ribbons on headers |
| Effects | soft drop shadows, a satin highlight on primary buttons, a faint lace pattern on the canvas; keep pink decorative rather than neon |
| Motion | slower and softer: base 240 ms, slow 420 ms, ease-in-out, gentle sway on decorations |

## Asset list

Slots marked *planned* remain future work. The first motif set is already used for the page corner,
control-panel corner, canvas and Appearance preview; the other slots are optional scene artwork.

| Slot | Where | Tenshi | Jirai | Format and size |
| --- | --- | --- | --- | --- |
| `page-corner` | sidebar's idle lower area | wing and halo | lace corner with bow | SVG, 192 × 128 viewBox |
| `empty-state` (*planned*) | above empty-state titles | cloud with sparkles | heart with stitches and ribbon | SVG, 120 × 96 |
| `dialog-header` (*planned*) | left of dialog titles | small wing | ribbon bow | SVG, 32 × 32 |
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
    frutiger-aero/motifs/bubbles-and-sprout.svg
    frutiger-aero/motifs/water.svg
    tenshi/motifs/wing-halo-bandage.svg
    tenshi/motifs/clouds.svg
    jirai/motifs/ribbon-lace-heart.svg
    jirai/motifs/lace.svg
  src/motifs/
    SkinMotif.tsx
    motifs.css
```

The built-in CSS presets and original motifs work without the package loader. Custom fonts,
extended scene packs and the `.vuaskin` manifest remain separate work; their absence does not
disable the presets or any application feature.
