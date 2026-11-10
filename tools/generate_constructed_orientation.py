#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate mixed raw/constructed orientation signs with a Fraction oracle."""
import argparse
from fractions import Fraction
from pathlib import Path
import random
from generate_crossing_point_order import crossing, value

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "tests" / "fixtures" / "crossing_coordinate_enclosures.tsv"
OUTPUT = ROOT / "tests" / "fixtures" / "constructed_orientation.tsv"


def generate():
    records = [line.split("\t")[:8] for line in SOURCE.read_text(encoding="utf-8").splitlines() if not line.startswith("#")]
    if len(records) != 512:
        raise SystemExit("crossing coordinate source fixture must have 512 cases")
    rng = random.Random(0x5243435437)
    lines = ["# Three blocks: I/C then four source points in hex; exact orientation sign."]
    for _ in range(256):
        blocks = [(rng.choice(("I", "C")), rng.choice(records)) for _ in range(3)]
        points = [tuple(Fraction.from_float(value(x)) for x in record[:2]) if tag == "I" else crossing(record) for tag, record in blocks]
        a, b, c = points
        det = (b[0]-a[0])*(c[1]-a[1]) - (b[1]-a[1])*(c[0]-a[0])
        sign = (det > 0) - (det < 0)
        fields = [entry for tag, record in blocks for entry in (tag, *record)]
        lines.append("\t".join([*fields, str(sign)]))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = generate()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != expected:
            raise SystemExit("constructed orientation fixtures are stale")
        print("Constructed orientation fixtures match the Fraction oracle.")
    else:
        OUTPUT.write_text(expected, encoding="utf-8")
        print(f"Wrote {OUTPUT.relative_to(ROOT)}")

if __name__ == "__main__":
    main()
