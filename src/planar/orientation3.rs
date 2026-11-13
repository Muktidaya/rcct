// SPDX-License-Identifier: Apache-2.0

//! Fixed-width dyadic arithmetic for finite raw-input 3D predicates.

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

// The caller establishes finite inputs. Each scaled input is < 2^2098 in
// magnitude and each difference < 2^2099; each cross component is < 2^4199
// and fits 66 limbs. No rounded geometry or normalization is involved.
fn cross(a: Point3, b: Point3, c: Point3) -> [PredicateInteger; 3] {
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
    [
        by.multiply(cz).subtract(bz.multiply(cy)),
        bz.multiply(cx).subtract(bx.multiply(cz)),
        bx.multiply(cy).subtract(by.multiply(cx)),
    ]
}

/// Returns whether the exact cross product of two raw input edges is nonzero.
pub(crate) fn noncollinear3_exact(a: Point3, b: Point3, c: Point3) -> bool {
    debug_assert!(a.is_finite() && b.is_finite() && c.is_finite());
    cross(a, b, c)
        .into_iter()
        .any(|component| component.sign() != Ordering::Equal)
}

// Each cross-times-difference product is < 2^6298, and their three-term sum
// is < 2^6300. Ninety-nine 64-bit limbs provide 6336 bits. These bounds do not
// cover rational or constructed-point inputs.
pub(crate) fn orient3d_exact(a: Point3, b: Point3, c: Point3, d: Point3) -> Ordering {
    debug_assert!(a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite());
    let [x, y, z] = cross(a, b, c);
    let (dx, dy, dz) = (
        difference(d.x, a.x),
        difference(d.y, a.y),
        difference(d.z, a.z),
    );
    signed_product(x, dx)
        .add(signed_product(y, dy))
        .add(signed_product(z, dz))
        .sign()
}
