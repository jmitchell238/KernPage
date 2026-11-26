"""Exploratory decoder for the `Contents` stream. Hypothesis-testing tool, written from
observation of sample files (see docs/spec). Usage: contents_probe.py FILE.pub [max_depth]

Hypothesis H1: a "block" = u32 total length (incl. the 4 length bytes) followed by items.
Each item = u8 id, u8 type, then a payload whose size depends on type.
Unknown type bytes are reported, not guessed, so wrong hypotheses show up immediately.
"""
import struct, sys
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from cfbls import load, walk

FIXED = {}      # type byte -> payload size; filled from evidence as we confirm
BLOCK = set()   # type bytes whose payload is a nested length-prefixed block of items
BLOB = set()    # type bytes whose payload is u32 length (incl. itself) + opaque bytes/UTF-16 text
unknown = {}

def parse_block(b, off, depth, maxdepth, out):
    total, = struct.unpack_from("<I", b, off)
    end = off + total
    pos = off + 4
    while pos + 2 <= end:
        iid, typ = b[pos], b[pos + 1]
        pos += 2
        if typ in FIXED:
            n = FIXED[typ]; val = b[pos:pos + n]
            out.append(("  " * depth) + f"@{pos-2:05x} id={iid:3d} type=0x{typ:02x} val={val.hex()}")
            pos += n
        elif typ in BLOCK:
            sub_total, = struct.unpack_from("<I", b, pos)
            out.append(("  " * depth) + f"@{pos-2:05x} id={iid:3d} type=0x{typ:02x} BLOCK len={sub_total}")
            if depth < maxdepth:
                parse_block(b, pos, depth + 1, maxdepth, out)
            pos += sub_total
        elif typ in BLOB:
            n, = struct.unpack_from("<I", b, pos); raw = b[pos + 4:pos + n]
            txt = raw.decode("utf-16le", "replace").rstrip("\x00") if len(raw) % 2 == 0 else ""
            out.append(("  " * depth) + f"@{pos-2:05x} id={iid:3d} type=0x{typ:02x} BLOB len={n} {txt[:60]!r}")
            pos += n
        else:
            unknown.setdefault(typ, []).append(pos - 2)
            out.append(("  " * depth) + f"@{pos-2:05x} id={iid:3d} type=0x{typ:02x} ??? stop")
            return False
    ok = pos == end
    out.append(("  " * depth) + f"-- block @{off:05x} len={total} {'ALIGNED' if ok else f'MISALIGNED (pos {pos:05x} vs end {end:05x})'}")
    return ok

if __name__ == "__main__":
    import json
    cfg = json.loads(open(__file__.rsplit("/", 1)[0] + "/contents_types.json").read())
    FIXED.update({int(k, 16): v for k, v in cfg["fixed"].items()})
    BLOCK.update(int(k, 16) for k in cfg["block"])
    BLOB.update(int(k, 16) for k in cfg.get("blob", []))
    e, s = load(sys.argv[1]); o = []; walk(e, e[0]["child"], "", o)
    b = s(dict(o)["Contents"])
    maxdepth = int(sys.argv[2]) if len(sys.argv) > 2 else 2
    out = []
    off = struct.unpack_from("<H", b, 2)[0]          # first block offset (observed: header u16[1])
    trailer = struct.unpack_from("<I", b, 0x1A)[0]
    for start in sorted({off, trailer}):
        out.append(f"=== top-level block @{start:05x}")
        if not parse_block(b, start, 0, maxdepth, out): break
    print("\n".join(out if "--all" in sys.argv else out[:200]))
    if unknown:
        print("UNKNOWN TYPE BYTES:", {hex(k): [hex(x) for x in v[:3]] for k, v in unknown.items()})
