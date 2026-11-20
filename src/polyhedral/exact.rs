// SPDX-License-Identifier: Apache-2.0
//! Fixed-capacity exact arithmetic for the polyhedral construction path.
//!
//! Reduced numerators and positive denominators have magnitudes at most
//! `i128::MAX`. Intermediate products and sums also have to fit a signed
//! 128-bit word. A limit can therefore be reported even when an alternative
//! evaluation order would produce a representable result. Each scalar
//! operation uses fixed storage and bounded integer work.

use super::PolyhedralError::{self, ArithmeticLimit};
use std::cmp::Ordering;

type R<T> = Result<T, PolyhedralError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(super) struct Q {
    num: i128,
    den: i128,
}

pub(super) type P = [Q; 3];

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

// Callers supply a positive denominator as the second argument, so the gcd
// fits a positive i128, even if the first argument is i128::MIN.
fn factor(numerator: i128, denominator: i128) -> i128 {
    gcd(numerator.unsigned_abs(), denominator as u128) as i128
}

impl Q {
    pub(super) const ZERO: Self = Self::int(0);
    pub(super) const ONE: Self = Self::int(1);

    pub(super) const fn int(n: i64) -> Self {
        Self {
            num: n as i128,
            den: 1,
        }
    }

    pub(super) fn ratio(n: i128, d: i128) -> R<Self> {
        if d == 0 {
            return Err(ArithmeticLimit);
        }
        let divisor = gcd(n.unsigned_abs(), d.unsigned_abs());
        let magnitude = i128::try_from(n.unsigned_abs() / divisor).map_err(|_| ArithmeticLimit)?;
        let den = i128::try_from(d.unsigned_abs() / divisor).map_err(|_| ArithmeticLimit)?;
        Ok(Self {
            num: if (n < 0) != (d < 0) {
                -magnitude
            } else {
                magnitude
            },
            den,
        })
    }

    pub(super) fn sign(self) -> i8 {
        self.num.signum() as i8
    }

    pub(super) fn parts(self) -> (i128, i128) {
        (self.num, self.den)
    }

    pub(super) fn neg(self) -> R<Self> {
        Ok(Self {
            num: -self.num,
            ..self
        })
    }

    pub(super) fn add(self, other: Self) -> R<Self> {
        let common = factor(self.den, other.den);
        let left_scale = other.den / common;
        let right_scale = self.den / common;
        let left = self.num.checked_mul(left_scale).ok_or(ArithmeticLimit)?;
        let right = other.num.checked_mul(right_scale).ok_or(ArithmeticLimit)?;
        let sum = left.checked_add(right).ok_or(ArithmeticLimit)?;
        // Any factor still shared with the least common denominator divides
        // `common`. Cancel it before multiplying the denominator terms.
        let cancel = factor(sum, common);
        let den = right_scale
            .checked_mul(other.den / cancel)
            .ok_or(ArithmeticLimit)?;
        Self::ratio(sum / cancel, den)
    }

    pub(super) fn sub(self, other: Self) -> R<Self> {
        self.add(other.neg()?)
    }

    pub(super) fn mul(self, other: Self) -> R<Self> {
        let left_cancel = factor(self.num, other.den);
        let right_cancel = factor(other.num, self.den);
        let num = (self.num / left_cancel)
            .checked_mul(other.num / right_cancel)
            .ok_or(ArithmeticLimit)?;
        let den = (self.den / right_cancel)
            .checked_mul(other.den / left_cancel)
            .ok_or(ArithmeticLimit)?;
        Self::ratio(num, den)
    }

    pub(super) fn div(self, other: Self) -> R<Self> {
        self.mul(Self::ratio(other.den, other.num)?)
    }

    pub(super) fn cmp_checked(self, other: Self) -> R<Ordering> {
        let signs = self.sign().cmp(&other.sign());
        if signs != Ordering::Equal {
            return Ok(signs);
        }
        // Equal integer quotients can be stripped from both fractions.
        // Reciprocating the remaining proper fractions reverses their order.
        // Denominators strictly decrease, and no cross-product is needed.
        let (mut a, mut b) = (self.num.unsigned_abs(), self.den as u128);
        let (mut c, mut d) = (other.num.unsigned_abs(), other.den as u128);
        let mut reversed = self.num < 0;
        loop {
            let order = (a / b).cmp(&(c / d));
            let (ar, cr) = (a % b, c % d);
            if order != Ordering::Equal || ar == 0 || cr == 0 {
                let result = order.then_with(|| ar.cmp(&cr));
                return Ok(if reversed { result.reverse() } else { result });
            }
            (a, b) = (b, ar);
            (c, d) = (d, cr);
            reversed = !reversed;
        }
    }
}

pub(super) fn sub3(a: P, b: P) -> R<P> {
    Ok([a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?])
}

pub(super) fn cross(a: P, b: P) -> R<P> {
    Ok([
        a[1].mul(b[2])?.sub(a[2].mul(b[1])?)?,
        a[2].mul(b[0])?.sub(a[0].mul(b[2])?)?,
        a[0].mul(b[1])?.sub(a[1].mul(b[0])?)?,
    ])
}

pub(super) fn dot(a: P, b: P) -> R<Q> {
    a[0].mul(b[0])?.add(a[1].mul(b[1])?)?.add(a[2].mul(b[2])?)
}

pub(super) fn mix(a: P, b: P, t: Q) -> R<P> {
    let delta = sub3(b, a)?;
    Ok([
        a[0].add(delta[0].mul(t)?)?,
        a[1].add(delta[1].mul(t)?)?,
        a[2].add(delta[2].mul(t)?)?,
    ])
}

pub(super) fn det(a: P, b: P, c: P) -> R<Q> {
    dot(a, cross(b, c)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduces_before_enforcing_bounds_and_normalizes_signs() {
        assert_eq!(Q::ratio(0, i128::MIN), Ok(Q::ZERO));
        assert_eq!(Q::ratio(i128::MIN, i128::MIN), Ok(Q::ONE));
        assert_eq!(Q::ratio(i128::MIN, 2), Q::ratio(-(1 << 126), 1));
        assert_eq!(Q::ratio(10, -15), Q::ratio(-2, 3));
        assert_eq!(Q::ratio(i128::MIN, 1), Err(ArithmeticLimit));
        assert_eq!(Q::ratio(1, i128::MIN), Err(ArithmeticLimit));
        assert_eq!(Q::ratio(0, 0), Err(ArithmeticLimit));
    }

    #[test]
    fn arithmetic_cancels_large_factors_before_products() {
        let max = i128::MAX;
        let a = Q::ratio(max, max - 1).unwrap();
        let b = Q::ratio(max - 1, max).unwrap();
        assert_eq!(a.mul(b), Ok(Q::ONE));
        assert_eq!(a.div(a), Ok(Q::ONE));
        let small = Q::ratio(1, max).unwrap();
        assert_eq!(small.sub(small), Ok(Q::ZERO));
        assert_eq!(small.add(small), Q::ratio(2, max));
        assert_eq!(Q::ONE.div(Q::ZERO), Err(ArithmeticLimit));
        let large = Q::ratio(max, 1).unwrap();
        assert_eq!(large.add(Q::ONE), Err(ArithmeticLimit));
        assert_eq!(small.mul(small), Err(ArithmeticLimit));
    }

    #[test]
    fn exact_order_handles_extreme_values_and_agrees_with_small_cross_products() {
        let m = i128::MAX;
        let a = Q::ratio(m - 2, m - 1).unwrap();
        let b = Q::ratio(m - 1, m).unwrap();
        assert_eq!(a.cmp_checked(b), Ok(Ordering::Less));
        assert_eq!(
            a.neg().unwrap().cmp_checked(b.neg().unwrap()),
            Ok(Ordering::Greater)
        );
        for n in -6..=6 {
            for d in 1..=6 {
                for p in -6..=6 {
                    for q in 1..=6 {
                        let left = Q::ratio(n, d).unwrap();
                        let right = Q::ratio(p, q).unwrap();
                        assert_eq!(left.cmp_checked(right), Ok((n * q).cmp(&(p * d))));
                        assert_eq!(left.add(right), Q::ratio(n * q + p * d, d * q));
                        assert_eq!(left.sub(right), Q::ratio(n * q - p * d, d * q));
                        assert_eq!(left.mul(right), Q::ratio(n * p, d * q));
                        assert_eq!(left.div(right), Q::ratio(n * q, d * p));
                    }
                }
            }
        }
    }

    #[test]
    fn rational_geometry_preserves_orientation_and_interpolation() {
        let origin = [Q::ZERO; 3];
        let x = [Q::ONE, Q::ZERO, Q::ZERO];
        let y = [Q::ZERO, Q::ONE, Q::ZERO];
        let z = [Q::ZERO, Q::ZERO, Q::ONE];
        assert_eq!(det(x, y, z), Ok(Q::ONE));
        assert_eq!(det(x, z, y), Ok(Q::int(-1)));
        let t = Q::ratio(1, 3).unwrap();
        assert_eq!(mix(origin, x, t), Ok([t, Q::ZERO, Q::ZERO]));
    }
}
