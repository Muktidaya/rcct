#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Compare raw crossing coordinates with an independent exact Fraction oracle."""
import argparse
from fractions import Fraction
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "tests" / "fixtures" / "crossing_coordinate_enclosures.tsv"
OUTPUT = ROOT / "tests" / "fixtures" / "crossing_point_order.tsv"


def value(word):
    return struct.unpack(">d", int(word, 16).to_bytes(8, "big"))[0]


def sub(a, b):
    return a[0] - b[0], a[1] - b[1]


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def crossing(words):
    values = [Fraction.from_float(value(word)) for word in words]
    a, b, c, d = [tuple(values[i:i+2]) for i in range(0, 8, 2)]
    r, s, q = sub(b, a), sub(d, c), sub(c, a)
    t = cross(q, s) / cross(r, s)
    return a[0] + t * r[0], a[1] + t * r[1]


def generate():
    records = [line.split("\t")[:8] for line in SOURCE.read_text(encoding="utf-8").splitlines() if not line.startswith("#")]
    if len(records) != 512:
        raise SystemExit("crossing coordinate source fixture must have 512 cases")
    lines = ["# Two crossings; exact x/y order and first crossing vs second's first input point."]
    for first, second in zip(records[:256], records[256:]):
        a, b = crossing(first), crossing(second)
        ox = (a[0] > b[0]) - (a[0] < b[0])
        oy = (a[1] > b[1]) - (a[1] < b[1])
        input_point = tuple(Fraction.from_float(value(word)) for word in second[:2])
        input_order = (a > input_point) - (a < input_point)
        lines.append("\t".join([*first, *second, str(ox), str(oy), str(input_order)]))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = generate()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != expected:
            raise SystemExit("crossing point order fixtures are stale")
        print("Crossing-point ordering matches the Fraction oracle.")
    else:
        OUTPUT.write_text(expected, encoding="utf-8")
        print(f"Wrote {OUTPUT.relative_to(ROOT)}")

if __name__ == "__main__":
    main()
