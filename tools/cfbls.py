"""List the streams inside a compound file (public [MS-CFB] spec). Usage: cfbls.py FILE [STREAM_PATH_TO_HEXDUMP]"""
import struct, sys

def load(path):
    data = open(path, "rb").read()
    assert data[:8] == bytes.fromhex("d0cf11e0a1b11ae1"), "not a compound file"
    sec_shift, mini_shift = struct.unpack_from("<HH", data, 0x1E)
    ss, mss = 1 << sec_shift, 1 << mini_shift
    n_fat, dir_start, _, mini_cutoff, minifat_start, n_minifat, difat_start, n_difat = struct.unpack_from("<IIIIIIII", data, 0x2C)
    sect = lambda i: data[(i + 1) * ss:(i + 2) * ss]
    difat = list(struct.unpack_from("<109I", data, 0x4C))
    d = difat_start
    for _ in range(n_difat):
        blk = struct.unpack(f"<{ss // 4}I", sect(d)); difat += blk[:-1]; d = blk[-1]
    fat = []
    for s in difat[:n_fat]:
        fat += struct.unpack(f"<{ss // 4}I", sect(s))
    def chain(start):
        out, seen = [], set()
        while start < 0xFFFFFFFA and start not in seen:
            seen.add(start); out.append(start); start = fat[start]
        return out
    read = lambda start: b"".join(sect(i) for i in chain(start))
    dirdata = read(dir_start)
    entries = []
    for off in range(0, len(dirdata), 128):
        e = dirdata[off:off + 128]
        nlen = struct.unpack_from("<H", e, 64)[0]
        name = e[:max(nlen - 2, 0)].decode("utf-16le", "replace")
        typ, _, left, right, child = struct.unpack_from("<BBIII", e, 66)
        start, size = struct.unpack_from("<IQ", e, 116)
        entries.append(dict(name=name, type=typ, left=left, right=right, child=child, start=start, size=size & 0xFFFFFFFF))
    root = entries[0]
    ministream = read(root["start"]) if root["start"] < 0xFFFFFFFA else b""
    minifat = []
    for s in chain(minifat_start) if n_minifat else []:
        minifat += struct.unpack(f"<{ss // 4}I", sect(s))
    def stream(e):
        if e["size"] < mini_cutoff:
            out, i = b"", e["start"]
            while i < 0xFFFFFFFA and len(out) < e["size"]:
                out += ministream[i * mss:(i + 1) * mss]; i = minifat[i]
            return out[:e["size"]]
        return read(e["start"])[:e["size"]]
    return entries, stream

def walk(entries, idx, prefix, out):
    if idx >= len(entries) or idx == 0xFFFFFFFF: return
    e = entries[idx]
    walk(entries, e["left"], prefix, out)
    path = prefix + e["name"]
    out.append((path, e))
    if e["type"] in (1, 5): walk(entries, e["child"], path + "/", out)
    walk(entries, e["right"], prefix, out)

if __name__ == "__main__":
    entries, stream = load(sys.argv[1])
    out = []; walk(entries, entries[0]["child"], "", out)
    if len(sys.argv) > 2:
        e = dict(out)[sys.argv[2]]; b = stream(e)
        for i in range(0, min(len(b), int(sys.argv[3]) if len(sys.argv) > 3 else 256), 16):
            print(f"{i:06x}  {b[i:i+16].hex(' '):48s}  {''.join(chr(c) if 32 <= c < 127 else '.' for c in b[i:i+16])}")
    else:
        for path, e in out:
            print(f"{'DIR ' if e['type'] in (1,5) else '    '}{path!r:40s} {e['size']:>8}")
