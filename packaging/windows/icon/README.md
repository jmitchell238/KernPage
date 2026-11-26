# KernPage app icon and wordmark

Redrawn as clean geometry from the owner's mock-ups. The icon is a dark teal (#046064) rounded tile (transparent corners) with a big
white K: the stem is a rolled spine (white, with a light teal #7FC9C4 strip on its right) and the two arms are white pages with thin
grey text lines, meeting at the middle with a thin teal gap.

- `kernpage.svg` - master (1408 grid), used for 32 px and up.
- `kernpage@16.svg`, `@20`, `@24` - pixel-hinted versions (every edge on a whole pixel, stepped tile corners, no shadows, no text lines)
  for the smallest sizes: a two-tone stem and two solid arms.
- `kernpage-wordmark.svg` - the word "Kernpage" in dark teal (#0B5A73) for light backgrounds; `kernpage-wordmark-white.svg` is the
  version for dark backgrounds. The K is the icon's K recoloured; the "p" bowl is a page outline with a folded corner in the accent
  teal (#1696A0). Every glyph is a path (no font reference and no text elements).
  Wordmark letters outlined from Poppins (SIL Open Font License 1.1, Indian Type Foundry).
- `png-master/`, `png-hinted/`, `png/` - rendered outputs; `png/` holds the sizes packed into `../assets/kernpage.ico`
  (16, 20, 24, 32, 40, 48, 64, 256). The launcher embeds `png-master/64.png`, `png-master/256.png` and `png/32.png`.

Regenerate everything from the repo root with `powershell packaging/windows/icon/regenerate.ps1`
(renders the SVGs with the `render_svg` example in the `kernicons` crate (sibling folder), then runs `make_icons.py` and `make_ico.py`).
Then run `python packaging/windows/assets/make_art.py` to refresh the installer banner and dialog art.
To preview one SVG at any width: `render_svg <in.svg> <width_px> <out.png>`.

## .kern document icon

`../assets/kernpage-doc.ico` is the icon Explorer shows for `.kern` files (`KernPage.Document\DefaultIcon`): a white page with a thin grey
outline, a folded top-right corner and the app icon's teal K (#046064, light #7FC9C4 stem strip) centred on the lower page. It is our own
page shape, so it reads as a document, not the app tile. `.pub` files keep the app icon.

- `kernpage-doc.svg` - 256 px master (256 grid).
- `kernpage-doc@16/20/24/32.svg` - pixel-art hinted sizes written by `make_doc_svgs.py` (every edge on a whole pixel, stepped K).
- `kernpage-doc@40/48/64.svg` - also written by `make_doc_svgs.py`: vector page with the outline snapped to whole pixels and the K scaled
  to a whole-pixel height, so the edges stay crisp.
- `png-doc-hinted/`, `png-doc-master/`, `png-doc/` - rendered outputs; `png-doc/` is packed (16, 20, 24, 32, 40, 48, 64, 256) into the .ico by `make_doc_ico.py`.

`regenerate.ps1` rebuilds it after the app icon (it renders with the `render_svg` example from the sibling `../kernicons` checkout).
The installer ships it as its own file, `kernpage-doc.ico` (`KernDocIcon` component in `kernpage.wxs`).
