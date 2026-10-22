# Native skins: art brief

> Updated: 2026-10-21
> Scope: art for VUA's two native skins, 天使界隈 (Tenshi) and 地雷系 (Jirai), and what each asset is for
> Status: Draft; the author approves the direction before assets are sourced

The [skin system](../development/skin-system.md) defines what a skin can change and its package
format. This brief fixes the look of the two native skins and lists every asset they need, its
slot, size and format, so assets can be sourced, drawn or generated against one list. The default
skin (purple and orange) stays as it is.

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

## 天使界隈 — Tenshi (light only)

The "angel" street style: clean white and sky blue, pastel lavender, soft gradients, angel wings
and halos, crosses and bandages, pills and syringes treated as cute flat icons, sparkles, clouds,
frills and small hearts. Flat, airy and a little fragile; Y2K digital shine in small doses.

| Layer | Direction |
| --- | --- |
| Color | canvas `#F4F9FF`, panel and elevated `#FFFFFF`, text `#1E2A44` / secondary `#4E5E7E`, border `#CFE3F7`; VUA accent sky blue `#1F6FC2` (white text 5.1:1), AMF accent lavender `#6E52C8`; tints `#E3F1FF` and `#EEE8FF`; room colors sky, periwinkle, lavender, aqua |
| Type | a rounded Japanese face for titles (candidates: Zen Maru Gothic, M PLUS Rounded 1c, both OFL); body keeps the default stack |
| Shape | round: radius 14 px on cards, pill controls, no cut corners |
| Effects | a faint radial highlight behind headings, thin white inner borders, small sparkles; no glow |
| Motion | light and bouncy: a slight overshoot easing (`cubic-bezier(0.34, 1.4, 0.64, 1)`), faster fades |

## 地雷系 — Jirai (dark only)

The "landmine" style: black and baby pink, lace and frills, ribbons and bows, hearts with stitches,
crosses, chains and safety pins, patent-leather sheen, a plain energy-drink can, cute-gloomy mood.
Dimensional rather than flat: soft shadows, embossed hearts, leather and satin textures.

| Layer | Direction |
| --- | --- |
| Color | canvas `#120A10`, panel `#1E1119`, elevated `#2A1622`, text `#FCEFF5` / secondary `#D6B3C4`, border `#4A2638`; VUA accent pink `#FF7EB6` (dark text), AMF accent lilac `#C3A6FF` (dark text); tints `#3A1A2B` and `#2C2140`; room colors pink, lilac, rose, mauve |
| Type | a cute display face for titles (candidates: Mochiy Pop One, Yusei Magic, Kaisei Decol, all OFL); body keeps the default stack |
| Shape | lace edge on cards and dialogs (scalloped mask), radius 10 px, ribbons on headers |
| Effects | soft drop shadows, a satin highlight on primary buttons, a faint lace pattern on the canvas |
| Motion | slower and softer: base 360 ms, ease-in-out, gentle sway on decorations |

## Asset list

Slots marked *planned* are named in the skin system and appear when slice 13 adds them to screens.

| Slot | Where | Tenshi | Jirai | Format and size |
| --- | --- | --- | --- | --- |
| `page-corner` (*planned*) | top-right corner of each page | wing and halo | lace corner with bow | SVG, 160 × 160 |
| `empty-state` (*planned*) | above empty-state titles | cloud with sparkles | heart with stitches and ribbon | SVG, 120 × 96 |
| `dialog-header` (*planned*) | left of dialog titles | small wing | ribbon bow | SVG, 32 × 32 |
| `control-panel-corner` | control center panels | halo arc | lace corner | SVG, 48 × 48 |
| `control-stage-edge` | control center stage border | dotted sparkle line (tile) | lace border (tile) | SVG, 64 × 16, repeats horizontally |
| `control-stage-backdrop` | control center graph background | sky gradient with clouds | black satin with faint lace | PNG/WebP, 1600 × 900 |
| `album-cover` | empty release album | wings framing an empty frame | lace frame with bow | SVG, 320 × 240 |
| `album-photo-corners` | release album photo corners | bandage tape | heart pins | SVG, 40 × 40 each |
| `album-stage-backdrop` | behind the finished Avatar | soft sky gradient | pink spotlight on black | PNG/WebP, 1200 × 1200 |
| canvas pattern | app background | tiny crosses and sparkles | tiny lace and hearts | SVG tile, 96 × 96, very low contrast |
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
packages/design-system/skins/
  make-motifs.mjs      draws every motif in code; edit it and run it to change a motif
  tenshi/motifs/*.svg
  jirai/motifs/*.svg
```

The skin package format (manifest, tokens, schema) lands with the skin loader; until then the motif
files are the only part in the repository, and the palettes above are the proposal for its tokens.
