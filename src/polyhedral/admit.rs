// SPDX-License-Identifier: Apache-2.0

use super::exact::{P, Q, cross, dot};
use super::{Facet, PolyhedralError, PolyhedralWork, R};
use std::cmp::Ordering;

/// Oriented closed halfspace `a*x + b*y + c*z + d <= 0`.
///
/// The coefficients are exact integers. Rational coefficients describe the
/// same halfspace after clearing denominators by a positive common factor.
/// Positive proportional inputs compare equal. Reversing every sign selects
/// the opposite material side of the same geometric plane.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Plane3h([i128; 4]);

impl Plane3h {
    /// Constructs the halfspace and removes common factors from its coefficients.
    /// A zero normal is rejected, regardless of the constant coefficient.
    pub fn from_coefficients(coefficients: [i64; 4]) -> Result<Self, PolyhedralError> {
        if coefficients[..3] == [0; 3] {
            return Err(PolyhedralError::InvalidPlane);
        }
        let divisor = coefficients.iter().fold(0, |mut a, v| {
            let mut b = v.unsigned_abs();
            while b != 0 {
                (a, b) = (b, a % b);
            }
            a
        });
        Ok(Self(
            coefficients.map(|value| i128::from(value) / i128::from(divisor)),
        ))
    }

    /// Returns primitive oriented integer coefficients. The wider return type
    /// also holds the positive counterpart of an input `i64::MIN`.
    pub fn coefficients(self) -> [i128; 4] {
        self.0
    }

    /// Returns the opposite closed halfspace, retaining the support plane.
    pub fn reversed(self) -> Self {
        Self(self.0.map(|value| -value))
    }

    fn coefficient(self, index: usize) -> Q {
        // Every coefficient originates from an i64, up to sign and division by
        // a positive integer. Its unit-denominator rational always fits.
        Q::ratio(self.0[index], 1).expect("primitive plane coefficient fits")
    }

    pub(super) fn normal(self) -> P {
        [
            self.coefficient(0),
            self.coefficient(1),
            self.coefficient(2),
        ]
    }

    pub(super) fn eval(self, point: P) -> R<Q> {
        dot(self.normal(), point)?.add(self.coefficient(3))
    }
}

/// An explicitly empty operand or an owned, bounded, full-dimensional convex
/// polyhedron. Admission enumerates exact halfspace intersections; each
/// boundary facet has one outward polygon and one irredundant support.
#[derive(Debug)]
pub struct ConvexSolid {
    pub(super) planes: Vec<Plane3h>,
    pub(super) faces: Vec<Facet>,
}

impl ConvexSolid {
    /// Returns the explicit empty operand.
    pub fn empty() -> Self {
        Self {
            planes: Vec::new(),
            faces: Vec::new(),
        }
    }

    /// Returns whether this operand represents the empty set.
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// Intersects the given halfspaces and admits a bounded solid with
    /// positive interior. Repeated, scaled and redundant constraints are
    /// normalized. Infeasible or bounded lower-dimensional intersections
    /// give `InvalidOperand`; a nonempty unbounded intersection gives
    /// `UnboundedOperand`. With zero constraints the intersection is all space.
    ///
    /// Triple-plane intersections, side tests and face ordering use exact checked
    /// arithmetic. Work limits, cancellation or an arithmetic limit abort
    /// admission before the owned operand is returned.
    pub fn from_halfspaces(planes: &[Plane3h], work: &mut PolyhedralWork<'_>) -> R<Self> {
        let mut supports = Vec::new();
        for &plane in planes {
            if !contains(&supports, &plane, work)? {
                work.reserve(&mut supports, 1)?;
                supports.push(plane);
            }
        }
        let mut vertices = Vec::new();
        let mut full_rank = false;
        for (i, &a) in supports.iter().enumerate() {
            for (j, &b) in supports.iter().enumerate().skip(i + 1) {
                for &c in &supports[j + 1..] {
                    work.charge(1)?;
                    if let Some(point) = intersection(a, b, c)? {
                        full_rank = true;
                        if inside(&supports, point, work)? && !contains(&vertices, &point, work)? {
                            work.reserve(&mut vertices, 1)?;
                            vertices.push(point);
                        }
                    }
                }
            }
        }

        // With full normal rank the intersection is pointed: if nonempty it
        // has a vertex. Rank-deficient inputs need a separate feasibility
        // check because an infinite slab may have no vertices at all.
        if vertices.is_empty() {
            if !full_rank && rank_deficient_feasible(&supports, work)? {
                return Err(PolyhedralError::UnboundedOperand);
            }
            return Err(PolyhedralError::InvalidOperand);
        }
        if has_recession_ray(&supports, work)? {
            return Err(PolyhedralError::UnboundedOperand);
        }

        let mut solid = Self::empty();
        for plane in supports {
            let mut points = Vec::new();
            for &point in &vertices {
                work.charge(1)?;
                if plane.eval(point)?.sign() == 0 {
                    work.reserve(&mut points, 1)?;
                    points.push(point);
                }
            }
            // For each inequality at least one strict vertex is equivalent
            // to the average of all vertices being strictly interior.
            if points.len() == vertices.len() {
                return Err(PolyhedralError::InvalidOperand);
            }
            if points.len() >= 3 {
                let polygon = face_hull(plane, points, work)?;
                if polygon.len() >= 3 {
                    work.reserve(&mut solid.faces, 1)?;
                    work.reserve(&mut solid.planes, 1)?;
                    solid.faces.push(Facet {
                        plane,
                        vertices: polygon,
                    });
                    solid.planes.push(plane);
                }
            }
        }
        Ok(solid)
    }
}

fn contains<T: Eq>(items: &[T], wanted: &T, work: &mut PolyhedralWork<'_>) -> R<bool> {
    for item in items {
        work.charge(1)?;
        if item == wanted {
            return Ok(true);
        }
    }
    Ok(false)
}

fn inside(planes: &[Plane3h], point: P, work: &mut PolyhedralWork<'_>) -> R<bool> {
    for &plane in planes {
        work.charge(1)?;
        if plane.eval(point)?.sign() > 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Cramer's rule in row-vector form: the three cross products are the
/// columns of the adjugate; each is weighted by its plane's right-hand side.
fn intersection(a: Plane3h, b: Plane3h, c: Plane3h) -> R<Option<P>> {
    let [x, y, z] = [a.normal(), b.normal(), c.normal()];
    let yz = cross(y, z)?;
    let determinant = dot(x, yz)?;
    if determinant == Q::ZERO {
        return Ok(None);
    }
    let zx = cross(z, x)?;
    let xy = cross(x, y)?;
    let [u, v, w] = [a, b, c].map(|plane| plane.reversed().coefficient(3));
    let mut point = [Q::ZERO; 3];
    for i in 0..3 {
        point[i] = yz[i]
            .mul(u)?
            .add(zx[i].mul(v)?)?
            .add(xy[i].mul(w)?)?
            .div(determinant)?;
    }
    Ok(Some(point))
}

fn rank_deficient_feasible(planes: &[Plane3h], work: &mut PolyhedralWork<'_>) -> R<bool> {
    // Any feasible point lies in a closed coordinate orthant. Intersecting
    // with that orthant gives a pointed, nonempty polyhedron with a vertex.
    // Its active constraints include at least one coordinate plane when the
    // real plane normals have rank below three. Test all such vertices.
    let axes = [
        Plane3h([1, 0, 0, 0]),
        Plane3h([0, 1, 0, 0]),
        Plane3h([0, 0, 1, 0]),
    ];
    for (i, &axis) in axes.iter().enumerate() {
        let candidates = axes[i + 1..].iter().chain(planes);
        for (j, &b) in candidates.clone().enumerate() {
            for &c in candidates.clone().skip(j + 1) {
                work.charge(1)?;
                if let Some(point) = intersection(axis, b, c)?
                    && inside(planes, point, work)?
                {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn has_recession_ray(planes: &[Plane3h], work: &mut PolyhedralWork<'_>) -> R<bool> {
    // Full normal rank precludes recession lines. A nontrivial pointed
    // recession cone therefore has an extreme ray, the common nullspace of
    // two independent active normals. Either cross-product sign may be
    // feasible for the homogeneous inequalities.
    for (i, &a) in planes.iter().enumerate() {
        for &b in &planes[i + 1..] {
            work.charge(1)?;
            let ray = cross(a.normal(), b.normal())?;
            if ray == [Q::ZERO; 3] {
                continue;
            }
            let (mut positive, mut negative) = (false, false);
            for &plane in planes {
                work.charge(1)?;
                let sign = dot(plane.normal(), ray)?.sign();
                positive |= sign > 0;
                negative |= sign < 0;
                if positive && negative {
                    break;
                }
            }
            if !(positive && negative) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn face_hull(plane: Plane3h, mut points: Vec<P>, work: &mut PolyhedralWork<'_>) -> R<Vec<P>> {
    // Dropping a nonzero normal component makes projection injective on
    // this plane. Cyclic coordinate order preserves the corresponding
    // component of the right-handed cross product.
    let axis = plane.0[..3]
        .iter()
        .position(|v| *v != 0)
        .expect("nonzero normal");
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    for i in 1..points.len() {
        let mut j = i;
        while j > 0 {
            work.charge(1)?;
            let left = points[j - 1];
            let right = points[j];
            let order = match left[u].cmp_checked(right[u])? {
                Ordering::Equal => left[v].cmp_checked(right[v])?,
                other => other,
            };
            if order != Ordering::Greater {
                break;
            }
            points.swap(j - 1, j);
            j -= 1;
        }
    }
    let mut hull = Vec::new();
    let max_hull = points
        .len()
        .checked_mul(2)
        .ok_or(PolyhedralError::ResourceLimit)?;
    work.reserve(&mut hull, max_hull)?;
    for point in &points {
        hull_push(&mut hull, *point, 0, (u, v), work)?;
    }
    hull.pop();
    let lower = hull.len();
    for point in points.iter().rev() {
        hull_push(&mut hull, *point, lower, (u, v), work)?;
    }
    hull.pop();
    if plane.0[axis] < 0 {
        work.charge(hull.len())?;
        hull.reverse();
    }
    Ok(hull)
}

fn hull_push(
    hull: &mut Vec<P>,
    point: P,
    start: usize,
    (u, v): (usize, usize),
    work: &mut PolyhedralWork<'_>,
) -> R<()> {
    while hull.len() - start >= 2 {
        work.charge(1)?;
        let a = hull[hull.len() - 2];
        let b = hull[hull.len() - 1];
        let turn = b[u]
            .sub(a[u])?
            .mul(point[v].sub(a[v])?)?
            .sub(b[v].sub(a[v])?.mul(point[u].sub(a[u])?)?)?;
        if turn.sign() > 0 {
            break;
        }
        hull.pop();
    }
    hull.push(point);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admit<const N: usize>(rows: [[i64; 4]; N]) -> R<ConvexSolid> {
        let planes = rows.map(|row| Plane3h::from_coefficients(row).unwrap());
        ConvexSolid::from_halfspaces(&planes, &mut PolyhedralWork::new(20_000, 1_000_000))
    }

    #[test]
    fn rank_deficient_feasibility_precedes_unboundedness() {
        let invalid = admit([[-1, 0, 0, 2], [1, 0, 0, -1]]);
        assert_eq!(invalid.unwrap_err(), PolyhedralError::InvalidOperand);
        // Unbounded strip in a displaced plane containing no coordinate axis.
        let strip = admit([[1, 1, 1, -4], [-1, -1, -1, 4], [0, -1, 0, 2]]);
        assert_eq!(strip.unwrap_err(), PolyhedralError::UnboundedOperand);
        assert_eq!(admit([]).unwrap_err(), PolyhedralError::UnboundedOperand);
    }

    #[test]
    fn truncated_ray_region_with_finite_vertices_is_unbounded() {
        let prism = [
            [1, 0, 0, -1],
            [-1, 0, 0, 0],
            [0, 1, 0, -1],
            [0, -1, 0, 0],
            [0, 0, -1, 0],
        ];
        assert_eq!(admit(prism).unwrap_err(), PolyhedralError::UnboundedOperand);
    }

    #[test]
    fn redundant_edge_vertex_and_plane_contacts_do_not_create_facets() {
        let solid = admit([
            [-1, 0, 0, 0],
            [0, -1, 0, 0],
            [0, 0, -1, 0],
            [1, 1, 1, -1],
            [6, 6, 6, -6],
            [-1, -1, 0, 0],
            [-1, -1, -1, 0],
        ])
        .unwrap();
        assert_eq!(solid.planes.len(), 4);
        for facet in solid.faces {
            let [a, b, c] = facet.vertices.as_slice() else {
                panic!("expected triangle")
            };
            let normal = cross(
                super::super::exact::sub3(*b, *a).unwrap(),
                super::super::exact::sub3(*c, *a).unwrap(),
            )
            .unwrap();
            assert!(dot(normal, facet.plane.normal()).unwrap().sign() > 0);
        }
    }

    #[test]
    fn primitive_oriented_coefficients_include_extreme_inputs() {
        let row = Plane3h::from_coefficients([i64::MIN, 0, 0, 1]).unwrap();
        assert_eq!(row.reversed().coefficients(), [1_i128 << 63, 0, 0, -1]);
        assert_eq!(row.reversed().reversed(), row);
        assert_eq!(
            Plane3h::from_coefficients([i64::MIN, 0, 0, i64::MIN])
                .unwrap()
                .coefficients(),
            [-1, 0, 0, -1]
        );
    }
}
