#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate an independent exact-rational convex-triangle Boolean oracle."""
import argparse
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "tests" / "fixtures" / "boolean_triangle.tsv"
A = [(0, 0), (6, 0), (0, 6)]
B = [(1, -1), (7, 3), (1, 4)]


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def subtract(a, b):
    return a[0] - b[0], a[1] - b[1]


def clip(polygon, start, end):
    def side(point):
        return cross(subtract(end, start), subtract(point, start))

    result = []
    for p, q in zip(polygon, polygon[1:] + polygon[:1]):
        a, b = side(p), side(q)
        if a >= 0:
            result.append(p)
        if (a < 0 < b) or (b < 0 < a):
            t = a / (a - b)
            result.append((p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])))
    return result


def area(polygon):
    return abs(sum(cross(p, q) for p, q in zip(polygon, polygon[1:] + polygon[:1]))) / 2


def generate():
    a = [tuple(map(Fraction, p)) for p in A]
    b = [tuple(map(Fraction, p)) for p in B]
    intersection = a
    for start, end in zip(b, b[1:] + b[:1]):
        intersection = clip(intersection, start, end)
    aa, bb, ii = area(a), area(b), area(intersection)
    values = [("union", aa + bb - ii), ("intersection", ii),
              ("difference", aa - ii), ("symmetric_difference", aa + bb - 2 * ii)]
    lines = ["# V x_num x_den y_num y_den; A operation twice_area_num twice_area_den"]
    for x, y in sorted(set(a + b + intersection)):
        lines.append(f"V\t{x.numerator}\t{x.denominator}\t{y.numerator}\t{y.denominator}")
    for name, value in values:
        twice = value * 2
        lines.append(f"A\t{name}\t{twice.numerator}\t{twice.denominator}")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = generate()
    if args.check:
        if TARGET.read_text(encoding="utf-8") != expected:
            raise SystemExit("Boolean triangle fixture differs from the Fraction oracle")
        print("Boolean triangle fixture matches the Fraction clipping oracle.")
    else:
        TARGET.write_text(expected, encoding="utf-8")


if __name__ == "__main__":
    main()
