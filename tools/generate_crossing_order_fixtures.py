#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate exact ordering examples for raw-input segment crossings."""
import argparse
from fractions import Fraction
from pathlib import Path
import random
import struct

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "tests" / "fixtures" / "crossing_order.tsv"
GENERAL_OUTPUT = ROOT / "tests" / "fixtures" / "crossing_order_general.tsv"


def positive_bits(rng):
    value = (rng.randrange(0x7FF) << 52) | rng.getrandbits(52)
    return value or 1


def rational(bits):
    return Fraction.from_float(struct.unpack(">d", bits.to_bytes(8, "big"))[0])


def generate():
    rng = random.Random(0x5243435432)
    lines = ["# Positive binary64 u1 v1 u2 v2 in hex; compare u1/(u1+v1) to u2/(u2+v2)."]
    for _ in range(512):
        values = [positive_bits(rng) for _ in range(4)]
        u1, v1, u2, v2 = map(rational, values)
        left, right = u1 / (u1 + v1), u2 / (u2 + v2)
        order = (left > right) - (left < right)
        lines.append("\t".join([*(f"{v:016x}" for v in values), str(order)]))
    return "\n".join(lines) + "\n"


def generate_general():
    rng = random.Random(0x5243435433)
    lines = ["# Six integer points a,b,c,d,e,f; compare intersections of ab with cd and ef."]
    def sub(a, b):
        return a[0] - b[0], a[1] - b[1]
    def cross(a, b):
        return a[0] * b[1] - a[1] * b[0]
    def parameter(a, b, c, d):
        r, s, q = sub(b, a), sub(d, c), sub(c, a)
        den = cross(r, s)
        if not den:
            return None
        t, u = Fraction(cross(q, s), den), Fraction(cross(q, r), den)
        return t if 0 < t < 1 and 0 < u < 1 else None
    while len(lines) <= 256:
        points = [(rng.randrange(-40, 41), rng.randrange(-40, 41)) for _ in range(6)]
        a, b, c, d, e, f = points
        first, second = parameter(a, b, c, d), parameter(a, b, e, f)
        if first is None or second is None:
            continue
        order = (first > second) - (first < second)
        lines.append("\t".join([*(str(x) for point in points for x in point), str(order)]))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for output, expected in ((OUTPUT, generate()), (GENERAL_OUTPUT, generate_general())):
        if args.check:
            if not output.exists() or output.read_text(encoding="utf-8") != expected:
                raise SystemExit(f"crossing order fixtures are stale: {output.name}")
        else:
            output.write_text(expected, encoding="utf-8")
            print(f"Wrote {output.relative_to(ROOT)}")
    if args.check:
        print("Crossing-order fixtures match the Fraction oracle.")

if __name__ == "__main__":
    main()
