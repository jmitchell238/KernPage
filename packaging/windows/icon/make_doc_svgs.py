"""Writes the pixel-hinted .kern document-icon SVGs kernpage-doc@16/20/24/32.svg (every edge on a whole pixel, no anti-aliasing).
Page = white sheet, thin grey outline, folded top-right corner; the teal K is drawn from pixel rows, centred on the lower page.
Run from the repo root: python -I packaging/windows/icon/make_doc_svgs.py"""
from pathlib import Path
D = Path(__file__).resolve().parent
OUT, WHITE, FOLD = "#7B868C", "#FFFFFF", "#D9E1E4"
TEAL, LIGHT = "#046064", "#7FC9C4"
# size: (left, right, top, bottom, fold, K x0, K y0, K w, K h, stem w, arm w, light stem strip)
SPEC = {
    16: (2, 13, 0, 15, 4, 5, 7, 6, 7, 2, 2, False),
    20: (3, 16, 0, 19, 5, 6, 8, 8, 9, 3, 2, False),
    24: (3, 20, 0, 23, 6, 7, 9, 10, 11, 3, 3, True),
    32: (4, 27, 0, 31, 8, 9, 13, 14, 15, 4, 4, True),
}
for n, (L, R, T, B, f, kx, ky, kw, kh, sw, aw, strip) in SPEC.items():
    g = {}
    def put(x, y, c): g[(x, y)] = c
    for y in range(T, B + 1):
        for x in range(L, R + 1):
            if x > R - f and y < T + f:
                continue  # clipped corner (becomes the fold)
            edge = x in (L, R) or y in (T, B)
            if (x == R - f and y <= T + f) or (y == T + f and x >= R - f):
                edge = True
            put(x, y, OUT if edge else WHITE)
    for r in range(1, f):  # diagonal edge and folded flap
        put(R - f + r, T + r, OUT)
        for x in range(R - f + 1, R - f + r):
            put(x, T + r, FOLD)
    for x in range(R - f + 1, R):
        put(x, T + f, OUT)
    put(R, T + f, OUT)
    for y in range(T + f + 1, T + f + 1):
        pass
    for y in range(T, T + f):  # fold interior row fill below the diagonal
        for x in range(R - f + 1, R + 1):
            if (x, y) not in g and x < R - f + (y - T):
                put(x, y, FOLD)
    # the K
    c = (kh - 1) / 2
    for i in range(kh):
        d = abs(i - c)
        x0 = sw + round(d * (kw - sw - aw) / c)
        for j in range(kw):
            if j < sw or x0 <= j < x0 + aw:
                put(kx + j, ky + i, LIGHT if (strip and j == 0) else TEAL)
    rects = []
    for y in range(n):
        x = 0
        while x < n:
            c0 = g.get((x, y))
            if c0 is None:
                x += 1
                continue
            e = x
            while g.get((e + 1, y)) == c0:
                e += 1
            rects.append(f'<rect x="{x}" y="{y}" width="{e - x + 1}" height="1" fill="{c0}"/>')
            x = e + 1
    svg = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {n} {n}" shape-rendering="crispEdges">\n' + "\n".join(rects) + "\n</svg>\n"
    (D / f"kernpage-doc@{n}.svg").write_text(svg)

# 40/48/64: vector page snapped to the pixel grid (crisp 1-2 px outline) with the real K scaled to whole-pixel height and stem edge.
import re
K = re.findall(r'<path[^>]*/>', (D.parent.parent.parent / "brand" / "kernpage-k.svg").read_text())[:5]
K = [p.replace("var(--kp-light, #7FC9C4)", LIGHT).replace("var(--kp-ink, #0B5A73)", TEAL) for p in K]
# size: (left, right, bottom, fold, outline width, K top, K height)
BIG = {40: (5, 34, 39, 10, 1, 16, 20), 48: (6, 41, 47, 12, 1, 19, 24), 64: (8, 55, 63, 16, 2, 26, 32)}
for n, (L, R, B, f, ow, ky, kh) in BIG.items():
    h = ow / 2
    s = kh / 914
    kw = 780 * s
    kx = round((L + R + 1) / 2 - kw / 2)
    tx, ty = kx - 341 * s, ky - 245 * s
    page = f"M{L+h},{h}H{R-f+1-h}L{R+1-h},{f+h}V{B+1-h}H{L+h}Z"
    flap = f"M{R-f+1-h},{h}V{f+h}H{R+1-h}Z"
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {n} {n}">\n'
           f'<path fill="{WHITE}" stroke="{OUT}" stroke-width="{ow}" stroke-linejoin="miter" d="{page}"/>\n'
           f'<path fill="{FOLD}" stroke="{OUT}" stroke-width="{ow}" stroke-linejoin="miter" d="{flap}"/>\n'
           f'<g transform="translate({tx:.3f} {ty:.3f}) scale({s:.5f})">\n' + "\n".join(K) + "\n</g>\n</svg>\n")
    (D / f"kernpage-doc@{n}.svg").write_text(svg)
