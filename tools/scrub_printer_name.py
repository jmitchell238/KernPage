#!/usr/bin/env python3
"""Replace a real printer name stored in the project-made .pub samples with a neutral one.

The name is stored as UTF-16LE (and, in case a file has one, as ASCII/ANSI). Each occurrence is
overwritten in place with "Sample Printer" padded with spaces to the same number of characters, so
the byte length and the file size do not change. Running it again finds nothing and changes nothing.

Usage: python tools/scrub_printer_name.py "<printer name>" [file.pub ...]
The printer name to remove is given on the command line (it is not stored here, so this script
never repeats it). It must be at least as long as "Sample Printer". With no files it scrubs samples/blank.pub, samples/hello.pub, samples/two-boxes.pub and
samples/gen/*.pub. The third-party samples (tika-testPUBLISHER.pub, lo-fdo59355-1.pub) are never
touched, even when named on the command line.
"""
import glob
import os
import sys

NEUTRAL = "Sample Printer"
SKIP = {"tika-testpublisher.pub", "lo-fdo59355-1.pub"}
ENCODINGS = ("utf-16-le", "ascii")


def default_files(root):
    files = [os.path.join(root, "samples", n) for n in ("blank.pub", "hello.pub", "two-boxes.pub")]
    files += sorted(glob.glob(os.path.join(root, "samples", "gen", "*.pub")))
    return files


def scrub(path, old_name):
    """Return {encoding: count} of replacements made in path."""
    with open(path, "rb") as f:
        data = f.read()
    original_len = len(data)
    counts = {}
    new_name = NEUTRAL.ljust(len(old_name))
    for enc in ENCODINGS:
        old, new = old_name.encode(enc), new_name.encode(enc)
        n = data.count(old)
        if n:
            data = data.replace(old, new)
            counts[enc] = n
    assert len(data) == original_len
    if counts:
        with open(path, "wb") as f:
            f.write(data)
    return counts


def main(argv):
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    if not argv or len(argv[0]) < len(NEUTRAL) or argv[0].lower().endswith(".pub"):
        print(__doc__)
        return 2
    old_name, files = argv[0], argv[1:] or default_files(root)
    total = 0
    for path in files:
        if os.path.basename(path).lower() in SKIP:
            print(f"{path}: skipped (third-party sample)")
            continue
        counts = scrub(path, old_name)
        if counts:
            total += sum(counts.values())
            detail = ", ".join(f"{n} x {enc}" for enc, n in counts.items())
            print(f"{path}: replaced {detail}")
        else:
            print(f"{path}: nothing to change")
    print(f"{total} occurrence(s) replaced in {len(files)} file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
