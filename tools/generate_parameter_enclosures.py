#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate tight binary64 parameter enclosures with an independent Fraction oracle."""
import argparse
from fractions import Fraction
import math
from pathlib import Path
import random
import struct

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "tests" / "fixtures" / "parameter_enclosures.tsv"
GENERAL_OUTPUT = ROOT / "tests" / "fixtures" / "parameter_enclosures_general.tsv"


def value(bits):
    return struct.unpack(">d", bits.to_bytes(8, "big"))[0]


def bits(number):
    return struct.unpack(">Q", struct.pack(">d", number))[0]


def positive(rng, exponent=None):
    if exponent is None:
        exponent = rng.randrange(0x7FF)
    return (exponent << 52) | (rng.getrandbits(52) or 1)


def enclosure(exact):
    rounded = float(exact)
    rounded_exact = Fraction.from_float(rounded)
    if rounded_exact == exact:
        return rounded, rounded
    if rounded_exact < exact:
        return rounded, math.nextafter(rounded, math.inf)
    return math.nextafter(rounded, -math.inf), rounded


def generate():
    rng = random.Random(0x5243435434)
    pairs = [(1, 0x7FEFFFFFFFFFFFFF), (0x7FEFFFFFFFFFFFFF, 1),
             (0x3FF0000000000000, 0x3FF0000000000000),
             (0x3FF0000000000000, 0x4000000000000000)]
    for _ in range(384):
        exponent = rng.randrange(0x7FF)
        pairs.append((positive(rng, exponent), positive(rng, exponent)))
    for _ in range(384):
        pairs.append((positive(rng), positive(rng)))
    lines = ["# u, v, lower, upper: binary64 bits in hex for exact u/(u+v)."]
    for ub, vb in pairs:
        u, v = Fraction.from_float(value(ub)), Fraction.from_float(value(vb))
        exact = u / (u + v)
        lower, upper = enclosure(exact)
        lines.append("\t".join(f"{b:016x}" for b in (ub, vb, bits(lower), bits(upper))))
    return "\n".join(lines) + "\n"


def generate_general():
    rng = random.Random(0x5243435435)
    lines = ["# Four integer points a,b,c,d; tight binary64 bounds for the ab/cd crossing parameter."]
    def sub(a, b): return a[0] - b[0], a[1] - b[1]
    def cross(a, b): return a[0] * b[1] - a[1] * b[0]
    while len(lines) <= 256:
        points = [(rng.randrange(-50, 51), rng.randrange(-50, 51)) for _ in range(4)]
        a, b, c, d = points
        r, s, q = sub(b, a), sub(d, c), sub(c, a)
        den = cross(r, s)
        if not den: continue
        t, u = Fraction(cross(q, s), den), Fraction(cross(q, r), den)
        if not (0 < t < 1 and 0 < u < 1): continue
        lower, upper = enclosure(t)
        lines.append("\t".join([*(str(x) for point in points for x in point), f"{bits(lower):016x}", f"{bits(upper):016x}"]))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for output, expected in ((OUTPUT, generate()), (GENERAL_OUTPUT, generate_general())):
        if args.check:
            if not output.exists() or output.read_text(encoding="utf-8") != expected:
                raise SystemExit(f"parameter enclosure fixtures are stale: {output.name}")
        else:
            output.write_text(expected, encoding="utf-8")
            print(f"Wrote {output.relative_to(ROOT)}")
    if args.check:
        print("Parameter-enclosure fixtures match the Fraction oracle.")

if __name__ == "__main__":
    main()
