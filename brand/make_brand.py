"""Rebuilds the KernPage brand kit in brand/ from the approved artwork in packaging/windows/icon/.

Run from the repo root:  C:\\Python312\\python.exe -I brand/make_brand.py

It never edits packaging/. Outputs:
  brand/kernpage-k.svg, kernpage-wordmark.svg, kernpage-icon.svg   recolourable masters (CSS variables)
  brand/<accent>/kernpage-{k,wordmark}-{light,dark}.svg, kernpage-icon.svg   plain static files
  the same as PNG (K and icon 512 px wide, wordmark 1200 px wide, transparent background)
  brand/palette.json   the palette table the README quotes
PNGs are drawn by the `render_svg` example in the kernicons crate (cargo builds it if needed).
"""
import colorsys, json, os, re, struct, subprocess, sys, zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ART = os.path.join(ROOT, "packaging", "windows", "icon")
OUT = os.path.join(ROOT, "brand")

# App accents (crates/kernpage/src/theme.rs, Accent::base).
ACCENTS = {"teal": "#1E8C99", "blue": "#2F6FDB", "purple": "#7B55C8", "graphite": "#55606E", "orange": "#FF8200"}

# The approved teal set. Every colour of another accent is this colour moved by the same
# hue offset, saturation ratio and lightness difference that separates it from teal's base.
TEAL = {
    "dark": "#046064",         # icon tile
    "light": "#7FC9C4",        # stem strip
    "ink": "#0B5A73",          # wordmark letters and K body on light backgrounds
    "accent": "#1696A0",       # the p's fold on light backgrounds
    "line": "#6FB6C6",         # text lines on the arms, light backgrounds
    "accent_on_dark": "#36C4CF",
    "line_on_dark": "#8FB9C6",
}
PAPER = "#FFFFFF"

# Orange is hand-tuned: the hue-shifted set would put a yellowish, low-contrast ink on white.
ORANGE = {
    "dark": "#B35A00",
    "light": "#FFC285",
    "ink": "#58595B",   # Tennessee Smokey grey
    "accent": "#FF8200",
    "line": "#F0C08E",
    "accent_on_dark": "#FF9F3D",
    "line_on_dark": "#E8C9A8",
}


def hx(c):
    return "#%02X%02X%02X" % tuple(round(v * 255) for v in c)


def hls(c):
    c = c.lstrip("#")
    return colorsys.rgb_to_hls(*(int(c[i:i + 2], 16) / 255 for i in (0, 2, 4)))


def shifted(ref, from_base, to_base):
    (h0, l0, s0), (h, l, s), (hb, lb, sb) = hls(from_base), hls(ref), hls(to_base)
    dh = ((h - h0 + 0.5) % 1.0) - 0.5
    nl = min(1.0, max(0.0, lb + (l - l0)))
    ns = min(1.0, max(0.0, sb * (s / s0))) if s0 else sb
    return hx(colorsys.hls_to_rgb((hb + dh) % 1.0, nl, ns))


def palette(accent):
    base = ACCENTS[accent]
    if accent == "teal":
        return dict(TEAL, base=base, paper=PAPER)
    if accent == "orange":
        return dict(ORANGE, base=base, paper=PAPER)
    p = {k: shifted(v, ACCENTS["teal"], base) for k, v in TEAL.items()}
    return dict(p, base=base, paper=PAPER)


def read(name):
    with open(os.path.join(ART, name), encoding="utf-8") as f:
        return f.read()


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


def v(name, fallback):
    return "var(--kp-%s, %s)" % (name, fallback)


# ---- recolourable masters ----------------------------------------------------------------------------
def master_wordmark():
    s = read("kernpage-wordmark.svg")
    for old, var in (("#7FC9C4", "light"), ("#0B5A73", "ink"), ("#1696A0", "accent"), ("#6FB6C6", "line")):
        s = s.replace('"%s"' % old, '"%s"' % v(var, old))
    note = "<!-- KernPage wordmark, recolourable: set the CSS variables kp-ink, kp-light, kp-accent, kp-line (with a double-hyphen prefix) on the inlined svg or a parent. All glyphs are paths. Letters outlined from Poppins (OFL). -->"
    return re.sub(r"<!--.*?-->", note, s, count=1)


def master_k():
    w = master_wordmark()
    group = re.search(r"<g transform=[^>]*>(.*?)</g>", w, re.S).group(1)
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="341 245 780 914">\n'
            "<!-- KernPage K, no tile, transparent: set the CSS variables kp-ink, kp-light, kp-line (double-hyphen prefix) on the inlined svg or a parent. -->"
            + group + "</svg>\n")


def master_icon():
    s = read("kernpage.svg")
    for old, var in (("#046064", "dark"), ("#7FC9C4", "light"), ("#FFFFFF", "paper")):
        s = s.replace('"%s"' % old, '"%s"' % v(var, old))
    note = "<!-- KernPage app icon, recolourable: set the CSS variables kp-dark (tile), kp-light, kp-paper (double-hyphen prefix). -->"
    return re.sub(r"<!--.*?-->", note, s, count=1)


def fill(svg, vals):
    return re.sub(r"var\(--kp-(\w+), (#[0-9A-Fa-f]{6})\)", lambda m: vals[m.group(1)], svg)


def values(p, mode):
    if mode == "light":
        return {"dark": p["dark"], "light": p["light"], "ink": p["ink"], "accent": p["accent"], "line": p["line"], "paper": p["paper"]}
    return {"dark": p["dark"], "light": p["light"], "ink": p["paper"], "accent": p["accent_on_dark"], "line": p["line_on_dark"], "paper": p["paper"]}


# ---- PNG helpers -------------------------------------------------------------------------------------
def render_exe():
    exe = os.path.join(ROOT, "target-win", "debug", "examples", "render_svg.exe")
    env = dict(os.environ, CARGO_TARGET_DIR=os.path.join(ROOT, "target-win"))
    subprocess.run(["cargo", "build", "-j", "2", "-p", "kernicons", "--example", "render_svg"], cwd=ROOT, env=env, check=True)
    return exe


def read_png(path):
    d = open(path, "rb").read()
    pos, idat, w, h = 8, b"", 0, 0
    while pos < len(d):
        n, t = struct.unpack(">I4s", d[pos:pos + 8])
        body = d[pos + 8:pos + 8 + n]
        if t == b"IHDR":
            w, h, depth, ctype, _, _, il = struct.unpack(">IIBBBBB", body)
            assert depth == 8 and ctype == 6 and il == 0, "expected 8-bit RGBA"
        elif t == b"IDAT":
            idat += body
        pos += 12 + n
    raw, bpp, stride, rows, prev = zlib.decompress(idat), 4, w * 4, [], bytearray(w * 4)
    for y in range(h):
        f, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line)
        prev = line
    return w, h, rows


def alpha(png, x, y):
    w, h, rows = read_png(png)
    return rows[min(y, h - 1)][min(x, w - 1) * 4 + 3]


def main():
    masters = {"kernpage-k.svg": master_k(), "kernpage-wordmark.svg": master_wordmark(), "kernpage-icon.svg": master_icon()}
    for name, text in masters.items():
        write(os.path.join(OUT, name), text)

    # The masters, filled with the approved teal values, must reproduce the approved files exactly.
    p = palette("teal")
    body = lambda s: s.split("-->", 1)[1]
    assert body(fill(masters["kernpage-wordmark.svg"], values(p, "light"))) == body(read("kernpage-wordmark.svg")), "light wordmark differs from approved"
    assert body(fill(masters["kernpage-wordmark.svg"], values(p, "dark"))) == body(read("kernpage-wordmark-white.svg")), "dark wordmark differs from approved"
    assert body(fill(masters["kernpage-icon.svg"], values(p, "light"))) == body(read("kernpage.svg")), "icon differs from approved"

    exe = render_exe()
    table = {}
    for accent in ACCENTS:
        p = palette(accent)
        table[accent] = p
        d = os.path.join(OUT, accent)
        jobs = []
        for mode in ("light", "dark"):
            jobs.append(("kernpage-k-%s" % mode, fill(masters["kernpage-k.svg"], values(p, mode)), 512))
            jobs.append(("kernpage-wordmark-%s" % mode, fill(masters["kernpage-wordmark.svg"], values(p, mode)), 1200))
        jobs.append(("kernpage-icon", fill(masters["kernpage-icon.svg"], values(p, "light")), 512))
        for stem, svg, width in jobs:
            write(os.path.join(d, stem + ".svg"), svg)
            subprocess.run([exe, os.path.join(d, stem + ".svg"), str(width), os.path.join(d, stem + ".png")], cwd=ROOT, check=True)
        # Transparent where it should be: the icon's tile corner, and the corner of every other mark.
        for stem, _, _ in jobs:
            a = alpha(os.path.join(d, stem + ".png"), 0, 0)
            assert a == 0, "%s/%s corner alpha %d" % (accent, stem, a)
    write(os.path.join(OUT, "palette.json"), json.dumps(table, indent=2) + "\n")
    print("ok")


if __name__ == "__main__":
    sys.exit(main())
