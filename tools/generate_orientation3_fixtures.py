#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate finite-binary64 orient3d fixtures with independent rational math."""
import argparse
from fractions import Fraction
from pathlib import Path
import struct

DEST = Path(__file__).resolve().parents[1] / 'tests/fixtures/orientation3.tsv'
MASK = (1 << 64) - 1

def as_float(bits):
    return struct.unpack('>d', struct.pack('>Q', bits))[0]

def expected(points):
    # Anchor at d, then negate the determinant to match det(b-a,c-a,d-a).
    q = [[Fraction.from_float(as_float(bits)) for bits in p] for p in points]
    a, b, c = [[q[k][i] - q[3][i] for i in range(3)] for k in range(3)]
    det = (a[0] * (b[1]*c[2] - b[2]*c[1])
           + a[1] * (b[2]*c[0] - b[0]*c[2])
           + a[2] * (b[0]*c[1] - b[1]*c[0]))
    det = -det
    return 'positive' if det > 0 else 'negative' if det < 0 else 'coplanar'

def render():
    state = 0x59c3a160a21
    def word():
        nonlocal state
        state ^= (state << 13) & MASK
        state ^= state >> 7
        state ^= (state << 17) & MASK
        return state
    zero = 0
    one = 0x3ff0000000000000
    plus_max = 0x7fefffffffffffff
    minus_max = 0xffefffffffffffff
    up_one = one + 1
    cases = [
        ((zero,zero,zero),(one,zero,zero),(zero,one,zero),(zero,zero,one)),
        ((zero,zero,zero),(one,zero,zero),(zero,zero,one),(zero,one,zero)),
        ((zero,zero,zero),(one,one,one),(one,up_one,one),(one,one,up_one)),
        ((minus_max,minus_max,minus_max),(plus_max,minus_max,minus_max),
         (minus_max,plus_max,minus_max),(minus_max,minus_max,plus_max)),
        ((0x8000000000000000,zero,zero),(zero,zero,zero),(one,zero,zero),(zero,one,zero)),
    ]
    # Every finite exponent field is represented, including subnormals.
    for exponent in range(2047):
        mantissa = word() & ((1 << 52) - 1)
        x = ((exponent << 52) | mantissa | ((exponent & 1) << 63))
        cases.append(((zero,zero,zero),(x,zero,zero),(zero,one,zero),(zero,zero,one)))
    for _ in range(300):
        coords = []
        for _ in range(12):
            sign = (word() & 1) << 63
            exponent = (word() % 2047) << 52
            mantissa = word() & ((1 << 52) - 1)
            coords.append(sign | exponent | mantissa)
        cases.append(tuple(tuple(coords[3*i:3*i+3]) for i in range(4)))
    rows = ['# ax ay az bx by bz cx cy cz dx dy dz expected (hex fields separated by tabs)']
    for case in cases:
        fields = [f'{x:016x}' for p in case for x in p]
        rows.append('\t'.join(fields + [expected(case)]))
    return '\n'.join(rows) + '\n'

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = render()
    if args.check:
        if DEST.read_text() != data:
            raise SystemExit('orientation3 fixtures differ from rational oracle')
    else:
        DEST.write_text(data)

if __name__ == '__main__':
    main()
