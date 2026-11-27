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
    "accent": "#1696A0",       # the P's fold on light backgrounds
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


# ---- the approved wordmark artwork (KernPage, capital P built from the K) -------------------------------
WORDMARK_VIEWBOX = "103 211 1249.19 329.50"
WORDMARK_BODY = r'''<g transform="translate(107,215) scale(0.26586) translate(-341,-245)">
<path fill="var(--kp-light, #7FC9C4)" d="M341,285C341,252 395,245 440.8,245V1159C395,1159 341,1152 341,1126Z"/>
<path fill="var(--kp-ink, #0B5A73)" d="M449.8,245C495,245 524.5,250 524.5,278V502L449.8,687Z"/>
<path fill="var(--kp-ink, #0B5A73)" d="M449.8,697L524.5,909V1126C524.5,1154 495,1159 449.8,1159Z"/>
<path fill="var(--kp-ink, #0B5A73)" d="M704,266H1121L775,687H445.3Z"/>
<path fill="var(--kp-ink, #0B5A73)" d="M445.3,697H775L1121,1135H704Z"/>
<path fill="none" stroke="var(--kp-line, #6FB6C6)" stroke-width="5" d="M710.0,366.1H969.4M680.3,411.4H932.7M652.0,456.6H895.9M622.4,503.3H859.2M594.1,548.5H822.4M567.2,595.1H788.5M572.9,795.9H794.1M601.1,842.5H829.5M628.0,889.2H863.4M654.9,934.4H897.3M683.1,981.1H931.3M710.0,1027.7H965.2M736.9,1074.4H999.1"/>
</g>
<g transform="translate(54,40.00)"><path fill="var(--kp-ink, #0B5A73)" d="M396.36 362.86H301.32Q302.49 376.94 311.18 384.91Q319.86 392.89 332.53 392.89Q350.83 392.89 358.58 377.17H394.01Q388.38 395.94 372.42 408.03Q356.47 420.11 333.23 420.11Q314.46 420.11 299.56 411.78Q284.66 403.45 276.33 388.2Q268 372.95 268 353Q268 332.82 276.21 317.57Q284.43 302.31 299.21 294.1Q313.99 285.89 333.23 285.89Q351.77 285.89 366.44 293.87Q381.1 301.84 389.2 316.51Q397.3 331.18 397.3 350.18Q397.3 357.22 396.36 362.86ZM363.27 340.8Q363.04 328.13 354.12 320.5Q345.2 312.87 332.3 312.87Q320.09 312.87 311.76 320.27Q303.43 327.66 301.56 340.8Z"/>
<path fill="var(--kp-ink, #0B5A73)" d="M477.04 286.12V320.62H468.36Q452.87 320.62 445.01 327.89Q437.15 335.17 437.15 353.23V418H404.3V288H437.15V308.18Q443.48 297.86 453.69 291.99Q463.9 286.12 477.04 286.12Z"/>
<path fill="var(--kp-ink, #0B5A73)" d="M607.94 341.74V418H575.09V346.19Q575.09 330.71 567.34 322.38Q559.6 314.05 546.22 314.05Q532.61 314.05 524.75 322.38Q516.89 330.71 516.89 346.19V418H484.04V288H516.89V304.19Q523.46 295.74 533.67 290.93Q543.88 286.12 556.08 286.12Q579.31 286.12 593.62 300.79Q607.94 315.45 607.94 341.74Z"/>
</g><g transform="translate(678.94,215) scale(0.26586) translate(-341,-245)"><path fill="var(--kp-light, #7FC9C4)" d="M341,285C341,252 395,245 440.8,245V1159C395,1159 341,1152 341,1126Z"/><path fill="var(--kp-ink, #0B5A73)" fill-rule="evenodd" d="M449.8,245H895L1085,435V560A230,230 0 0 1 855,790H524.5V1126C524.5,1154 495,1159 449.8,1159Z M524.5,410H826.7L920,503.3V560A65,65 0 0 1 855,625H524.5Z"/><path fill="var(--kp-accent, #1696A0)" d="M895,245V435H1085Z"/></g><g transform="translate(107.30,40.00)"><path fill="var(--kp-ink, #0B5A73)" d="M763.44 353Q763.44 333.05 772.24 317.8Q781.04 302.55 796.29 294.22Q811.54 285.89 830.32 285.89Q849.09 285.89 864.34 294.22Q879.59 302.55 888.39 317.8Q897.19 333.05 897.19 353Q897.19 372.95 888.16 388.2Q879.12 403.45 863.75 411.78Q848.38 420.11 829.38 420.11Q810.6 420.11 795.59 411.78Q780.57 403.45 772 388.2Q763.44 372.95 763.44 353ZM863.4 353Q863.4 334.46 853.66 324.49Q843.93 314.52 829.85 314.52Q815.77 314.52 806.26 324.49Q796.76 334.46 796.76 353Q796.76 371.54 806.03 381.51Q815.3 391.48 829.38 391.48Q838.29 391.48 846.16 387.14Q854.02 382.8 858.71 374.12Q863.4 365.44 863.4 353Z"/>
<path fill="var(--kp-ink, #0B5A73)" d="M864.34,288H897.19V418H864.34Z"/>
<path fill="var(--kp-ink, #0B5A73)" d="M1006.5 306.54V288H1039.59V418.94Q1039.59 437.01 1032.32 451.2Q1025.04 465.4 1010.49 473.73Q995.94 482.06 975.29 482.06Q947.6 482.06 929.89 469.16Q912.17 456.25 909.82 433.96H942.44Q945.02 442.87 953.59 448.15Q962.15 453.43 974.36 453.43Q988.67 453.43 997.59 444.87Q1006.5 436.3 1006.5 418.94V398.76Q1000.17 407.91 989.02 414.01Q977.88 420.11 963.56 420.11Q947.14 420.11 933.53 411.66Q919.92 403.22 912.05 387.85Q904.19 372.48 904.19 352.53Q904.19 332.82 912.05 317.57Q919.92 302.31 933.41 294.1Q946.9 285.89 963.56 285.89Q978.11 285.89 989.14 291.64Q1000.17 297.39 1006.5 306.54ZM972.01 314.75Q962.86 314.75 955.11 319.21Q947.37 323.67 942.56 332.23Q937.75 340.8 937.75 352.53Q937.75 364.26 942.56 373.06Q947.37 381.86 955.23 386.56Q963.09 391.25 972.01 391.25Q981.16 391.25 989.14 386.67Q997.12 382.1 1001.81 373.53Q1006.5 364.97 1006.5 353Q1006.5 341.03 1001.81 332.47Q997.12 323.9 989.14 319.33Q981.16 314.75 972.01 314.75Z"/>
<path fill="var(--kp-ink, #0B5A73)" d="M1174.95 362.86H1079.91Q1081.08 376.94 1089.77 384.91Q1098.45 392.89 1111.12 392.89Q1129.42 392.89 1137.17 377.17H1172.6Q1166.97 395.94 1151.01 408.03Q1135.06 420.11 1111.82 420.11Q1093.05 420.11 1078.15 411.78Q1063.25 403.45 1054.92 388.2Q1046.59 372.95 1046.59 353Q1046.59 332.82 1054.8 317.57Q1063.02 302.31 1077.8 294.1Q1092.58 285.89 1111.82 285.89Q1130.36 285.89 1145.03 293.87Q1159.69 301.84 1167.79 316.51Q1175.89 331.18 1175.89 350.18Q1175.89 357.22 1174.95 362.86ZM1141.86 340.8Q1141.63 328.13 1132.71 320.5Q1123.79 312.87 1110.89 312.87Q1098.68 312.87 1090.35 320.27Q1082.02 327.66 1080.15 340.8Z"/>
</g>'''


# ---- recolourable masters ----------------------------------------------------------------------------
SVG_TMPL = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="%s">\n%s\n%s\n</svg>\n'


def master_wordmark():
    note = "<!-- KernPage wordmark, recolourable: set the CSS variables kp-ink, kp-light, kp-accent, kp-line (with a double-hyphen prefix) on the inlined svg or a parent. All glyphs are paths. Letters outlined from Poppins (OFL); the capital P is built from the K's parts, its top-right corner a folded page corner. -->"
    return SVG_TMPL % (WORDMARK_VIEWBOX, note, WORDMARK_BODY)


def approved_wordmark(p, mode):
    """The static wordmark kept in packaging/windows/icon/ (kernpage-wordmark.svg, kernpage-wordmark-white.svg)."""
    note = "<!-- KernPage wordmark (for %s backgrounds). All glyphs are drawn paths (no font, no text elements). -->" % mode
    return SVG_TMPL % (WORDMARK_VIEWBOX, note, fill(WORDMARK_BODY, values(p, mode)))


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
