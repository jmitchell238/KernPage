"""Builds png-doc/N.png and ../assets/kernpage-doc.ico (16, 20, 24, 32, 40, 48, 64, 256) for .kern files.
 - 16-64: pixel-snapped kernpage-doc@N.svg rendered to png-doc-hinted/N.png; 256: kernpage-doc.svg master (png-doc-master/N.png).
Run from the repo root after render (regenerate.ps1 does both)."""
import struct
from pathlib import Path
from PIL import Image
D = Path(__file__).resolve().parent
SIZES = [16, 20, 24, 32, 40, 48, 64, 256]
HINTED = {16, 20, 24, 32, 40, 48, 64}
(D / "png-doc").mkdir(exist_ok=True)
blobs = []
for s in SIZES:
    src = D / ("png-doc-hinted" if s in HINTED else "png-doc-master") / f"{s}.png"
    dst = D / "png-doc" / f"{s}.png"
    Image.open(src).convert("RGBA").save(dst)
    blobs.append((s, dst.read_bytes()))
hdr = struct.pack("<HHH", 0, 1, len(blobs)); off = 6 + 16 * len(blobs); ent = b""; body = b""
for s, b in blobs:
    ent += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(b), off + len(body)); body += b
(D.parent / "assets" / "kernpage-doc.ico").write_bytes(hdr + ent + body)
