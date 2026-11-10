#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate exact orient2d sign fixtures using Python Fraction only."""
import argparse
from fractions import Fraction
from pathlib import Path
import random
import struct

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "tests" / "fixtures" / "orientation.tsv"


def finite_binary64_bits(rng):
    sign = rng.getrandbits(1) << 63
    exponent = rng.randrange(0x7FF) << 52
    fraction = rng.getrandbits(52)
    return sign | exponent | fraction


def as_float(bits):
    return struct.unpack(">d", bits.to_bytes(8, "big"))[0]


def generate():
    rng = random.Random(0x52434354)
    lines = [
        "# Exact orientation signs for deterministic finite binary64 inputs.",
        "# Fields: case, ax, ay, bx, by, cx, cy (binary64 bits, hex), sign (-1/0/1).",
    ]
    for index in range(256):
        bits = [finite_binary64_bits(rng) for _ in range(6)]
        ax, ay, bx, by, cx, cy = map(as_float, bits)
        determinant = (Fraction(bx) - Fraction(ax)) * (Fraction(cy) - Fraction(ay)) - (
            Fraction(by) - Fraction(ay)
        ) * (Fraction(cx) - Fraction(ax))
        sign = (determinant > 0) - (determinant < 0)
        fields = [f"{value:016x}" for value in bits]
        lines.append("\t".join([f"random-{index:03}", *fields, str(sign)]))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = generate()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != expected:
            raise SystemExit("orientation fixtures are stale; regenerate with this script")
        print("Exact orientation fixtures match the independent Fraction oracle.")
    else:
        OUTPUT.write_text(expected, encoding="utf-8")
        print(f"Wrote {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
