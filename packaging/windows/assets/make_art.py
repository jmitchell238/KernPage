"""Regenerates the installer artwork (icon, banner, dialog) with Pillow.

Run:  python make_art.py   (writes next to this script)
Palette: the icon's dark teal tile (#046064) with a light teal (#7FC9C4) spine.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

TEAL = (4, 96, 100)       # icon tile, #046064
TEAL_END = (4, 96, 100)   # accent rule under the banner (same as the tile)
DEEP = (3, 66, 70)        # sidebar: dark teal (darker than the tile)
PANEL = (127, 201, 196)   # icon spine, light teal #7FC9C4
WHITE = (255, 255, 255)
OUT = Path(__file__).resolve().parent


def font(size):
    for name in ("segoeuib.ttf", "arialbd.ttf", "DejaVuSans-Bold.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def glyph(size):
    "The app icon (rendered from ../icon/*.svg, see ../icon/make_ico.py)."
    src = OUT.parent / "icon" / "png-master" / "256.png"
    return Image.open(src).convert("RGBA").resize((size, size), Image.LANCZOS)


def banner():
    im = Image.new("RGB", (493, 58), WHITE)
    d = ImageDraw.Draw(im)
    d.rectangle((0, 55, 493, 58), fill=TEAL_END)
    d.rectangle((0, 52, 493, 54), fill=PANEL)
    g = glyph(44)
    im.paste(g, (443, 4), g)
    return im


def dialog():
    # The installer draws its black welcome text on the right, so keep that side white.
    im = Image.new("RGB", (493, 312), (255, 255, 255))
    d = ImageDraw.Draw(im)
    d.rectangle((0, 0, 163, 312), fill=DEEP)
    d.rectangle((163, 0, 165, 312), fill=TEAL)
    g = glyph(96)
    im.paste(g, (34, 40), g)
    d.text((20, 160), "KernPage", font=font(21), fill=WHITE)
    return im


banner().save(OUT / "banner.bmp")
dialog().save(OUT / "dialog.bmp")

