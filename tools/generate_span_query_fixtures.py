#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate exact bounded-span crossing classifications with Fraction."""
import argparse
from fractions import Fraction
from pathlib import Path
import random
import struct

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "tests" / "fixtures" / "crossing_order.tsv"
OUTPUT = ROOT / "tests" / "fixtures" / "span_query.tsv"
OVERLAP_OUTPUT = ROOT / "tests" / "fixtures" / "span_overlap_query.tsv"


def value(bits):
    return Fraction.from_float(struct.unpack(">d", int(bits, 16).to_bytes(8, "big"))[0])


def ratio(u, v):
    u, v = value(u), value(v)
    return u / (u + v)


def positive(rng):
    exponent = rng.randrange(0x7FF)
    fraction = rng.getrandbits(52) or 1
    return f"{((exponent << 52) | fraction):016x}"


def generate():
    rng = random.Random(0x5243435438)
    rows = [line.split("\t") for line in SOURCE.read_text(encoding="utf-8").splitlines() if not line.startswith("#")]
    if len(rows) != 512:
        raise SystemExit("crossing order source fixture must have 512 cases")
    lines = ["# lower u/v, upper u/v, cutter u/v in binary64 hex; exact clipped class."]
    for i, row in enumerate(rows):
        a, b = row[:2], row[2:4]
        ta, tb = ratio(*a), ratio(*b)
        if ta == tb:
            raise SystemExit("source contains equal span boundaries")
        if ta > tb:
            a, b, ta, tb = b, a, tb, ta
        c = a if i % 8 == 0 else b if i % 8 == 1 else [positive(rng), positive(rng)]
        tc = ratio(*c)
        kind = "Disjoint" if tc < ta or tc > tb else "BoundaryTouch" if tc == ta or tc == tb else "ProperCrossing"
        lines.append("\t".join([*a, *b, *c, kind]))
    return "\n".join(lines) + "\n"


def generate_overlap():
    rng = random.Random(0x5243435439)
    rows = [line.split("\t") for line in SOURCE.read_text(encoding="utf-8").splitlines() if not line.startswith("#")]
    if len(rows) != 512:
        raise SystemExit("crossing order source fixture must have 512 cases")
    coordinates = ["0000000000000000", "0000000000000001", "3fd0000000000000",
                   "3fe0000000000000", "3fe8000000000000", "3fefffffffffffff", "3ff0000000000000"]
    lines = ["# lower u/v, upper u/v, cutter x endpoints; exact clipped class and boundary sources."]
    for row in rows:
        a, b = row[:2], row[2:4]
        ta, tb = ratio(*a), ratio(*b)
        if ta == tb: raise SystemExit("source contains equal span boundaries")
        if ta > tb: a, b, ta, tb = b, a, tb, ta
        c, d = rng.sample(coordinates, 2)
        tc, td = value(c), value(d)
        low_c, high_c = (tc, td) if tc < td else (td, tc)
        low_label, high_label = ("C", "D") if tc < td else ("D", "C")
        low, high = max(ta, low_c), min(tb, high_c)
        kind = "Disjoint" if low > high else "BoundaryTouch" if low == high else "Overlap"
        lo_label = "A" if ta >= low_c else low_label
        hi_label = "B" if tb <= high_c else high_label
        lines.append("\t".join([*a, *b, c, d, kind,
                                lo_label if kind != "Disjoint" else "-",
                                hi_label if kind == "Overlap" else "-"]))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for output, expected in ((OUTPUT, generate()), (OVERLAP_OUTPUT, generate_overlap())):
        if args.check:
            if not output.exists() or output.read_text(encoding="utf-8") != expected:
                raise SystemExit(f"span query fixtures are stale: {output.name}")
        else:
            output.write_text(expected, encoding="utf-8")
            print(f"Wrote {output.relative_to(ROOT)}")
    if args.check:
        print("Span-query fixtures match the Fraction oracle.")

if __name__ == "__main__":
    main()
