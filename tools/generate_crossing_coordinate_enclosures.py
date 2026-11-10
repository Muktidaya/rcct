#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate exact crossing-coordinate enclosures with a Fraction oracle."""
import argparse
from fractions import Fraction
import math
from pathlib import Path
import random
import struct

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "tests" / "fixtures" / "crossing_coordinate_enclosures.tsv"


def bits(number):
    return struct.unpack(">Q", struct.pack(">d", number))[0]


def enclosure(exact):
    rounded = float(exact)
    represented = Fraction.from_float(rounded)
    if represented == exact:
        return rounded, rounded
    if represented < exact:
        return rounded, math.nextafter(rounded, math.inf)
    return math.nextafter(rounded, -math.inf), rounded


def sub(a, b):
    return a[0] - b[0], a[1] - b[1]


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def generate():
    rng = random.Random(0x5243435436)
    lines = ["# Four source points as binary64 hex; tight x/y coordinate bounds as binary64 hex."]
    while len(lines) <= 512:
        ex, ey = rng.randrange(-1070, 1011), rng.randrange(-1070, 1011)
        inputs = [(math.ldexp(float(rng.randrange(-50, 51)), ex),
                   math.ldexp(float(rng.randrange(-50, 51)), ey)) for _ in range(4)]
        a, b, c, d = [tuple(Fraction.from_float(v) for v in p) for p in inputs]
        r, s, q = sub(b, a), sub(d, c), sub(c, a)
        den = cross(r, s)
        if not den:
            continue
        t, u = cross(q, s) / den, cross(q, r) / den
        if not (0 < t < 1 and 0 < u < 1):
            continue
        xlo, xhi = enclosure(a[0] + t * r[0])
        ylo, yhi = enclosure(a[1] + t * r[1])
        values = [v for point in inputs for v in point] + [xlo, xhi, ylo, yhi]
        lines.append("\t".join(f"{bits(v):016x}" for v in values))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = generate()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != expected:
            raise SystemExit("crossing coordinate enclosure fixtures are stale")
        print("Crossing-coordinate enclosures match the Fraction oracle.")
    else:
        OUTPUT.write_text(expected, encoding="utf-8")
        print(f"Wrote {OUTPUT.relative_to(ROOT)}")

if __name__ == "__main__":
    main()
