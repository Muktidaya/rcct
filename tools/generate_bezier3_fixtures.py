#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate independent per-operation rational binary64 Bézier fixtures."""
import argparse
from fractions import Fraction
import math
from pathlib import Path
import struct

DEST = Path(__file__).resolve().parents[1] / 'tests/fixtures/bezier3.tsv'

def bits(value):
    return struct.unpack('>Q', struct.pack('>d', value))[0]

def value(pattern):
    return struct.unpack('>d', struct.pack('>Q', pattern))[0]

def rounded(exact, zero_sign=1.0):
    if not exact:
        return math.copysign(0.0, zero_sign)
    try:
        return float(exact)
    except OverflowError:
        return math.copysign(math.inf, -1.0 if exact < 0 else 1.0)

def multiply(a, b):
    sign = -1.0 if math.copysign(1.0,a) != math.copysign(1.0,b) else 1.0
    return rounded(Fraction.from_float(a) * Fraction.from_float(b), sign)

def add(a, b):
    sign = -1.0 if a == 0 and b == 0 and math.copysign(1.0,a) < 0 and math.copysign(1.0,b) < 0 else 1.0
    return rounded(Fraction.from_float(a) + Fraction.from_float(b), sign)

def blend(a, b, t):
    complement = rounded(Fraction(1) - Fraction.from_float(t))
    left = multiply(complement, a)
    right = multiply(t, b)
    if not all(map(math.isfinite, (complement, left, right))):
        return None
    out = add(left, right)
    return out if math.isfinite(out) else None

def evaluate(a, b, c, t):
    if t == 0: return a
    if t == 1: return c
    x = blend(a,b,t)
    y = blend(b,c,t)
    if x is None or y is None: return None
    return blend(x,y,t)

def render():
    # Specified xorshift64 stream; no interpreter-specific random helpers.
    state = 0xB3A13
    def next_word():
        nonlocal state
        state ^= (state << 13) & 0xffffffffffffffff
        state ^= state >> 7
        state ^= (state << 17) & 0xffffffffffffffff
        return state
    controls = [0, 1, 2, (1<<52)-1, 1<<52, 0x3ff0000000000000,
                0x3fefffffffffffff, 0x7fefffffffffffff, 0x8000000000000000,
                0x8000000000000001, 0xffefffffffffffff]
    parameters = [0, 1, 0x3fd0000000000000, 0x3fe0000000000000,
                  0x3fefffffffffffff, 0x3ff0000000000000]
    rows = ['# a_bits\tb_bits\tc_bits\tt_bits\tresult_bits_or_error']
    cases = [(1,1,1,0x3fe0000000000000),
             (0x8000000000000001,0x8000000000000001,0x8000000000000001,0x3fe0000000000000),
             (0x8000000000000000,0x8000000000000000,0x8000000000000000,0x3fe0000000000000),
             (0x8000000000000000,0,0,0),
             (0,0,0x8000000000000000,0x3ff0000000000000),
             (0x7fefffffffffffff,0xffefffffffffffff,0x7fefffffffffffff,0x3fe0000000000000)]
    for _ in range(256):
        samples = []
        for _ in range(3):
            if next_word() & 1: samples.append(controls[next_word() % len(controls)])
            else:
                sign = (next_word() & 1) << 63
                exponent = (next_word() % 2047) << 52
                samples.append(sign | exponent | (next_word() & ((1 << 52) - 1)))
        t = parameters[next_word() % len(parameters)] if next_word() & 1 else bits((next_word() & ((1 << 53) - 1)) / 2**53)
        cases.append((*samples,t))
    for a,b,c,t in cases:
        out = evaluate(value(a), value(b), value(c), value(t))
        result = 'error' if out is None else f'{bits(out):016x}'
        rows.append(f'{a:016x}\t{b:016x}\t{c:016x}\t{t:016x}\t{result}')
    return '\n'.join(rows) + '\n'

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    expected = render()
    if args.check:
        if DEST.read_text() != expected:
            raise SystemExit('Bézier fixtures differ from the independent rational generator')
    else:
        DEST.write_text(expected)

if __name__ == '__main__':
    main()
