"""Builds png/N.png (the sizes packed into kernpage.ico) for the app icon. Run from the repo root.
 - 32 and up: the master kernpage.svg rendered by the render_svg example (png-master/N.png).
 - 16, 20, 24: the pixel-hinted kernpage@N.svg rendered to png-hinted/N.png, so edges stay crisp at tiny sizes."""
from pathlib import Path
from PIL import Image
D = Path(__file__).resolve().parent
SIZES = [16, 20, 24, 32, 40, 48, 64, 256]
HINTED = {16, 20, 24}
(D / "png").mkdir(exist_ok=True)
for s in SIZES:
    pick = D / ("png-hinted" if s in HINTED else "png-master") / f"{s}.png"
    Image.open(pick).convert("RGBA").save(D / "png" / f"{s}.png")
