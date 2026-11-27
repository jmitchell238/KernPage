# KernPage brand kit

The KernPage logo is a **K** whose stem is a rolled spine (a light strip beside a darker or white body) and whose two arms
are pages with lines of text; the wordmark spells "KernPage" with a capital **P** built like the K (a light strip beside a dark stem and bowl),
whose top-right corner is a folded page corner. Every place the logo and icon are used is listed in
[docs/BRAND-USAGE.md](../docs/BRAND-USAGE.md).
The approved artwork lives in `packaging/windows/icon/` (the teal app icon, which is also what the Windows `.ico` is made from).
This folder holds everything derived from it: transparent marks, every accent colour and recolourable masters.
Do not redraw the shapes; change colours only.

Rebuild everything with `C:\Python312\python.exe -I brand/make_brand.py` from the repo root (needs cargo; the PNGs are
drawn by the `render_svg` example in the `kernicons` crate (sibling folder)). The script also checks that the masters reproduce the approved
teal files exactly and that every PNG has transparent corners.

## Files

| File | What it is | Use it for |
| --- | --- | --- |
| `kernpage-k.svg` | The K alone, no tile, transparent. Recolourable master. | Inline in a web page or app that should follow a theme. |
| `kernpage-wordmark.svg` | The full wordmark, transparent. Recolourable master. | Same, for headers and title lockups. |
| `kernpage-icon.svg` | The tiled app icon. Recolourable master. | Same, when the tile colour should follow a theme. |
| `<accent>/kernpage-k-light.svg` / `.png` | The K in that accent, for light backgrounds. PNG is 512 px wide. | Slides, documents, anything on white or pale backgrounds. |
| `<accent>/kernpage-k-dark.svg` / `.png` | The K for dark backgrounds (white body). | Dark slides, dark mode pages. |
| `<accent>/kernpage-wordmark-light.svg` / `.png` | Wordmark for light backgrounds. PNG is 1200 px wide. | README headers, letterheads, web headers. |
| `<accent>/kernpage-wordmark-dark.svg` / `.png` | Wordmark for dark backgrounds (white letters). | Dark pages, video, dark slides. |
| `<accent>/kernpage-icon.svg` / `.png` | The tiled icon in that accent (512 px). | Avatars, store and web listings, social cards. |
| `palette.json` | The colour table below as data. | Scripts. |
| `make_brand.py` | Generates all of the above. | Rebuilding after a palette change. |

`<accent>` is `teal`, `blue`, `purple`, `graphite` or `orange`: the five accents in the app's Options > Appearance
(`crates/kernpage/src/theme.rs`, `Accent`). **Teal is the brand colour**; use it unless there is a reason to match another theme.
The Windows `.ico` and installer art stay teal.

## Colours

Every accent has the same set of roles; Blue, Purple and Graphite are teal's approved set moved by the same hue offset,
saturation ratio and lightness difference that separate each teal colour from teal's base. Orange (Pantone 151 C) is hand-tuned,
because that shift gives a yellowish ink with too little contrast on white.

| Role | Used for | Teal | Blue | Purple | Graphite | Orange |
| --- | --- | --- | --- | --- | --- | --- |
| base | the app's accent colour (`Accent::base`) | `#1E8C99` | `#2F6FDB` | `#7B55C8` | `#55606E` | `#FF8200` |
| dark | icon tile | `#046064` | `#0352B8` | `#451FAF` | `#303A44` | `#B35A00` |
| light | the stem's light strip | `#7FC9C4` | `#B8CFE3` | `#CFCBE3` | `#A3ABB1` | `#FFC285` |
| ink | wordmark letters and K body on light backgrounds | `#0B5A73` | `#0E39C3` | `#6D2ABA` | `#3A4150` | `#A34700` |
| accent | the P's fold, on light backgrounds | `#1696A0` | `#2372E6` | `#744DCF` | `#53606F` | `#FF8200` |
| accent on dark | the P's fold, on dark backgrounds | `#36C4CF` | `#77A2E1` | `#A893D8` | `#7B8896` | `#FF9F3D` |
| line | text lines on the K's arms, light backgrounds | `#6FB6C6` | `#A9B9DF` | `#CBBDDE` | `#999EA8` | `#F0C08E` |
| line on dark | text lines, dark backgrounds | `#8FB9C6` | `#C5CCE3` | `#DDD5E6` | `#ACAFB5` | `#E8C9A8` |
| paper | white of the arms / letters on dark | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |

On dark backgrounds the ink becomes paper (white), and the accent and line use their "on dark" values.

## Recolourable masters

The three master SVGs take their colours from CSS custom properties, with the teal values as fallbacks:
`fill="var(--kp-ink, #0B5A73)"`. The variables are `--kp-dark`, `--kp-light`, `--kp-accent`, `--kp-paper`,
`--kp-ink` and `--kp-line` (the K and wordmark use `ink`, `light`, `accent`, `line`; the icon uses `dark`, `light`, `paper`).

Inline the SVG in HTML and set the variables on it or on a parent:

```html
<style>
  .logo      { --kp-ink: #6D2ABA; --kp-light: #CFCBE3; --kp-accent: #744DCF; --kp-line: #CBBDDE; }
  @media (prefers-color-scheme: dark) {
    .logo    { --kp-ink: #fff; --kp-accent: #A893D8; --kp-line: #DDD5E6; }
  }
</style>
<div class="logo"><!-- paste kernpage-wordmark.svg here --></div>
```

Browsers ignore these variables when an SVG is loaded through `<img>`, `background-image` or an Open Graph tag, and so does
resvg. For those, use the static files in `<accent>/`, which have the colours already filled in.

## Rules of thumb

- Keep clear space around a mark of at least half the height of the K; do not stretch, rotate or add shadows.
- Light background: `-light` files. Dark background: `-dark` files. Never put the light wordmark on a dark background.
- Below about 32 px use the hinted app icons in `packaging/windows/icon/` (`kernpage@16.svg`, `@20`, `@24`), not these.
- The wordmark letters were outlined from Poppins (SIL Open Font License 1.1), so no font is needed.
