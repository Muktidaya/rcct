// SPDX-License-Identifier: Apache-2.0
use rcct::polyhedral::{ConvexSolid, Plane3h, PolyhedralWork};

type Point = [(i128, i128); 3];
type Face = ([i128; 4], Vec<Point>);

#[derive(Clone, Copy)]
pub struct Cuboid {
    lo: [i64; 3],
    hi: [i64; 3],
    denominator: i64,
}

fn ratio(n: i64, d: i64) -> (i128, i128) {
    let (n, d) = (i128::from(n), i128::from(d));
    let (mut a, mut b) = (n.abs(), d);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    (n / a, d / a)
}

impl Cuboid {
    pub const fn new(lo: [i64; 3], hi: [i64; 3]) -> Self {
        Self::scaled(lo, hi, 1)
    }

    /// Bounds in units of 1/denominator on all three axes.
    pub const fn scaled(lo: [i64; 3], hi: [i64; 3], denominator: i64) -> Self {
        assert!(denominator > 0);
        Self {
            lo,
            hi,
            denominator,
        }
    }

    pub fn solid(self) -> ConvexSolid {
        let [x0, y0, z0] = self.lo;
        let [x1, y1, z1] = self.hi;
        let d = self.denominator;
        let planes = [
            [-d, 0, 0, x0],
            [d, 0, 0, -x1],
            [0, -d, 0, y0],
            [0, d, 0, -y1],
            [0, 0, -d, z0],
            [0, 0, d, -z1],
        ]
        .map(|row| Plane3h::from_coefficients(row).unwrap());
        let mut work = PolyhedralWork::new(100_000_000, 128 * 1024 * 1024);
        ConvexSolid::from_halfspaces(&planes, &mut work).unwrap()
    }

    /// The six expected outward rectangles, derived from the supplied intervals.
    pub fn faces(self) -> impl Iterator<Item = Face> {
        (0..3).flat_map(move |axis| {
            [-1_i128, 1].map(|sign| {
                let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                let level = if sign > 0 {
                    self.hi[axis]
                } else {
                    self.lo[axis]
                };
                let (n, d) = ratio(level, self.denominator);
                let mut plane = [0; 4];
                plane[axis] = sign * d;
                plane[3] = -sign * n;
                // In cyclic coordinates, increasing u then v winds around
                // the positive axis. The lower face looks out the other way.
                let mut corners = [
                    (self.lo[u], self.lo[v]),
                    (self.hi[u], self.lo[v]),
                    (self.hi[u], self.hi[v]),
                    (self.lo[u], self.hi[v]),
                ]
                .map(|(s, t)| {
                    let mut p = [0; 3];
                    p[axis] = level;
                    p[u] = s;
                    p[v] = t;
                    p.map(|value| ratio(value, self.denominator))
                });
                if sign < 0 {
                    corners.reverse();
                }
                (plane, corners.to_vec())
            })
        })
    }
}
