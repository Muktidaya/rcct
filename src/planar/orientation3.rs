// SPDX-License-Identifier: Apache-2.0

//! Fixed-width dyadic arithmetic for the finite raw-input 3D determinant.

use super::{
    ConstructionInteger, CoordinateInteger, PredicateInteger, multiply_predicate_coordinate,
};
use crate::gp::Point3;
use std::cmp::Ordering;

fn difference(a: f64, b: f64) -> CoordinateInteger {
    CoordinateInteger::from_finite_f64(a).subtract(CoordinateInteger::from_finite_f64(b))
}

fn signed_product(left: PredicateInteger, right: CoordinateInteger) -> ConstructionInteger {
    ConstructionInteger {
        limbs: multiply_predicate_coordinate(left, right),
        negative: left.negative != right.negative,
    }
    .canonical()
}

// Caller guarantees finite coordinates. Each scaled input has magnitude below
// 2^2098, each difference below 2^2099. Each cross component is below 2^4199,
// each cross-times-difference product below 2^6298, and the three-term sum
// below 2^6300. Ninety-nine 64-bit limbs provide 6336 bits. These bounds do not
// cover rational or constructed-point inputs.
pub(crate) fn orient3d_exact(a: Point3, b: Point3, c: Point3, d: Point3) -> Ordering {
    debug_assert!(a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite());
    let (bx, by, bz) = (
        difference(b.x, a.x),
        difference(b.y, a.y),
        difference(b.z, a.z),
    );
    let (cx, cy, cz) = (
        difference(c.x, a.x),
        difference(c.y, a.y),
        difference(c.z, a.z),
    );
    let (dx, dy, dz) = (
        difference(d.x, a.x),
        difference(d.y, a.y),
        difference(d.z, a.z),
    );
    let x = by.multiply(cz).subtract(bz.multiply(cy));
    let y = bz.multiply(cx).subtract(bx.multiply(cz));
    let z = bx.multiply(cy).subtract(by.multiply(cx));
    signed_product(x, dx)
        .add(signed_product(y, dy))
        .add(signed_product(z, dz))
        .sign()
}
