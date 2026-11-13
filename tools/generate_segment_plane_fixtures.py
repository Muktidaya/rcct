#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate raw segment/three-point-plane event fixtures using exact rationals."""
import argparse
from fractions import Fraction
from pathlib import Path
import struct

DEST = Path(__file__).resolve().parents[1] / 'tests/fixtures/segment_plane.tsv'
MASK = (1 << 64) - 1

def value(bits):
    return struct.unpack('>d', struct.pack('>Q', bits))[0]

def classify(bits):
    p = [[Fraction.from_float(value(x)) for x in point] for point in bits]
    a,b,c,start,end = p
    u = [b[i]-a[i] for i in range(3)]
    v = [c[i]-a[i] for i in range(3)]
    n = [u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
    if not any(n): return 'invalid'
    side = lambda point: sum((n[i]*(point[i]-a[i]) for i in range(3)), Fraction())
    x,y = side(start),side(end)
    if not x and not y: return 'contained'
    if not x: return 'start'
    if not y: return 'end'
    if (x > 0) == (y > 0): return 'disjoint'
    t = x/(x-y)
    assert 0 < t < 1
    crossing = [(1-t)*start[i]+t*end[i] for i in range(3)]
    assert side(crossing) == 0
    return 'crossing'

def render():
    state = 0x331fa7a821
    def word():
        nonlocal state
        state ^= (state << 13) & MASK
        state ^= state >> 7
        state ^= (state << 17) & MASK
        return state
    zero,one,minus_one = 0,0x3ff0000000000000,0xbff0000000000000
    max_pos,max_neg = 0x7fefffffffffffff,0xffefffffffffffff
    o = (zero,zero,zero)
    x = (one,zero,zero)
    y = (zero,one,zero)
    z = (zero,zero,one)
    nz = (zero,zero,minus_one)
    cases = [
        (o,x,y,z,nz), (o,x,y,z,(one,one,one)),
        (o,x,y,o,z), (o,x,y,z,o), (o,x,y,o,x),
        (o,x,x,z,nz), (o,x,z,y,(zero,minus_one,zero)),
        (o,(1,zero,zero),(zero,1,zero),(zero,zero,1),(zero,zero,0x8000000000000001)),
        (o,(max_pos,zero,zero),(zero,max_pos,zero),(zero,zero,max_pos),(zero,zero,max_neg)),
        (o,(one,one,one),(one,one+1,one),(one,one,one+1),(one,one,zero)),
    ]
    pool = [0,0x8000000000000000,1,0x8000000000000001,one,minus_one,max_pos,max_neg]
    for _ in range(256):
        coords=[]
        for _ in range(15):
            if word() & 1: coords.append(pool[word() % len(pool)])
            else: coords.append(((word() & 1) << 63) | ((word() % 2047) << 52) | (word() & ((1 << 52)-1)))
        points = tuple(tuple(coords[3*i:3*i+3]) for i in range(5))
        # Segment endpoints must be distinct as exact floating values.
        if all(value(points[3][i]) == value(points[4][i]) for i in range(3)):
            continue
        cases.append(points)
    rows = ['# ax ay az bx by bz cx cy cz sx sy sz ex ey ez event (tab-separated hex)']
    for case in cases:
        fields=[f'{x:016x}' for point in case for x in point]
        rows.append('\t'.join(fields+[classify(case)]))
    return '\n'.join(rows)+'\n'

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    data=render()
    if args.check:
        if DEST.read_text() != data: raise SystemExit('segment-plane fixture mismatch')
    else: DEST.write_text(data)

if __name__ == '__main__': main()
