"""Write an original 200x120 test picture (PNG) using only the standard library.
Pattern: four colored quadrants with a white diagonal, so orientation and cropping are
easy to see when rendered. Usage: make_test_picture.py OUT.png
"""
import struct, sys, zlib

W, H = 200, 120
COLORS = [(200, 40, 40), (40, 140, 60), (40, 80, 200), (230, 180, 30)]  # TL, TR, BL, BR

def pixel(x, y):
    if abs(x * H - y * W) < W * 2:          # white diagonal from top-left to bottom-right
        return (255, 255, 255)
    return COLORS[(x >= W // 2) + 2 * (y >= H // 2)]

rows = b"".join(b"\x00" + bytes(c for x in range(W) for c in pixel(x, y)) for y in range(H))

def chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

png = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
       + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b""))
open(sys.argv[1], "wb").write(png)
