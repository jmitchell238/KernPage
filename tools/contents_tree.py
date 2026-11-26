"""Decode the Contents stream into a flat, diffable listing (spec §4).
Usage: contents_tree.py FILE.pub            -> listing on stdout
       contents_tree.py --check FILE.pub... -> decode-health summary per file
Written from docs/spec/PUB-FORMAT-SPEC.md; type table in contents_types.json.
"""
import json, struct, sys
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from cfbls import load, walk

CFG = json.load(open(__file__.rsplit("/", 1)[0] + "/contents_types.json"))
FIXED = {int(k, 16): v for k, v in CFG["fixed"].items()}
NESTED = {int(k, 16) for k in CFG["block"]}
RAW = {int(k, 16) for k in CFG["blob"]}

class Unknown(Exception):
    pass

def items(b, off):
    """Yield (id, type, value) for a block at off; nested values are lists, raw are bytes."""
    total, = struct.unpack_from("<I", b, off)
    pos, end = off + 4, off + total
    while pos < end:
        iid, typ = b[pos], b[pos + 1]; pos += 2
        cls = typ & 0xF8  # low 3 bits are flags; size depends only on the class
        if cls in FIXED:
            n = FIXED[cls]; v = int.from_bytes(b[pos:pos + n], "little") if 0 < n <= 8 else (True if n == 0 else b[pos:pos + n]); pos += n
        elif cls in NESTED:
            n, = struct.unpack_from("<I", b, pos); v = list(items(b, pos)); pos += n
        elif cls in RAW:
            n, = struct.unpack_from("<I", b, pos); v = b[pos + 4:pos + n]; pos += n
        else:
            raise Unknown(f"type 0x{typ:02x} at 0x{pos-2:x}")
        yield iid, typ, v
    if pos != end:
        raise Unknown(f"block at 0x{off:x} misaligned ({pos:x} vs {end:x})")

def contents(path):
    e, s = load(path); o = []; walk(e, e[0]["child"], "", o)
    return s(dict(o)["Contents"])

def index(b):
    t, = struct.unpack_from("<I", b, 0x1A)
    lst = [v for i, ty, v in items(b, t) if i == 3][0]
    objs = {}
    for slot, (_, ty, rec) in enumerate(lst):
        if ty & 0xF8 == 0x78:
            continue
        d = {i: v for i, _, v in rec}
        objs[slot] = d
    return objs

def fmt(v, depth):
    if isinstance(v, list):
        return "\n" + "\n".join(("  " * depth) + f"[{i}] 0x{t:02x} = " + fmt(x, depth + 1) for i, t, x in v)
    if isinstance(v, (bytes, bytearray)):
        if len(v) % 2 == 0 and len(v) >= 4 and all(v[k + 1] == 0 for k in range(0, min(len(v), 16), 2)):
            return repr(v.decode("utf-16le", "replace").rstrip("\x00"))
        return v.hex(" ")
    return str(v)

def listing(path):
    b = contents(path); out = []
    for slot, d in index(b).items():
        out.append(f"### object {slot} kind={d.get(2)} parent={d.get(5)}")
        out.append(fmt(list(items(b, d[4])), 1).lstrip("\n"))
    return "\n".join(out)

if __name__ == "__main__":
    if sys.argv[1] == "--check":
        for p in sys.argv[2:]:
            b = contents(p)
            try:
                objs = index(b); bad = []
                for slot, d in objs.items():
                    try: list(items(b, d[4]))
                    except Unknown as ex: bad.append((slot, d.get(2), str(ex)))
                print(f"{p}: generation={struct.unpack_from('<H', b, 4)[0]} objects={len(objs)} failing={len(bad)} {bad[:3]}")
            except Unknown as ex:
                print(f"{p}: index failed: {ex}")
    else:
        print(listing(sys.argv[1]))
