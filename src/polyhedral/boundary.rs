// SPDX-License-Identifier: Apache-2.0

use super::exact::{P, Q, det, dot, mix, sub3};
use super::{Facet, Plane3h, PolyhedralError, PolyhedralWork, R};
use std::cmp::Ordering;

#[derive(Debug)]
pub(super) struct BoundaryFace {
    pub(super) plane: Plane3h,
    pub(super) indices: Vec<usize>,
}

/// An owned boundary of the regularized material set.
///
/// Each convex face carries an outward support and an oriented loop of exact
/// vertices. Coplanar subdivisions can remain. Incident faces share the same
/// vertex indices along every common edge. A shell is a connected component of
/// the boundary, so an enclosed cavity contributes a separate shell.
#[derive(Debug)]
pub struct SolidBoundary {
    pub(super) vertices: Vec<P>,
    pub(super) faces: Vec<BoundaryFace>,
    volume: Q,
    shells: usize,
}

impl SolidBoundary {
    /// Returns the unique empty material set.
    pub fn empty() -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
            volume: Q::ZERO,
            shells: 0,
        }
    }

    /// Whether this is the explicitly empty result.
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// Number of distinct exact vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// The three Cartesian coordinates as reduced (numerator, positive denominator) pairs.
    pub fn vertex(&self, index: usize) -> Option<[(i128, i128); 3]> {
        Some(self.vertices.get(index)?.map(Q::parts))
    }

    /// Number of convex boundary patches, including coplanar subdivisions.
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// The counterclockwise vertex loop as viewed from outside the material.
    /// The closing edge goes from the last index to the first.
    pub fn face_vertices(&self, index: usize) -> Option<&[usize]> {
        Some(&self.faces.get(index)?.indices)
    }

    /// Oriented face support, with material locally on the nonpositive side.
    pub fn face_plane(&self, index: usize) -> Option<Plane3h> {
        Some(self.faces.get(index)?.plane)
    }

    /// Number of connected boundary shells, including cavity shells.
    pub fn shell_count(&self) -> usize {
        self.shells
    }

    /// Exact volume in cubic coordinate units as a reduced rational pair.
    pub fn volume(&self) -> (i128, i128) {
        self.volume.parts()
    }

    pub(super) fn from_facets(facets: Vec<Facet>, work: &mut PolyhedralWork<'_>) -> R<Self> {
        work.charge(1)?;
        if facets.is_empty() {
            return Ok(Self::empty());
        }
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        work.reserve(&mut faces, facets.len())?;
        for facet in facets {
            check_facet(&facet, work)?;
            let mut indices = Vec::new();
            work.reserve(&mut indices, facet.vertices.len())?;
            for point in facet.vertices {
                let mut index = None;
                for (i, other) in vertices.iter().enumerate() {
                    work.charge(1)?;
                    if *other == point {
                        index = Some(i);
                        break;
                    }
                }
                let id = match index {
                    Some(i) => i,
                    None => {
                        work.reserve(&mut vertices, 1)?;
                        vertices.push(point);
                        vertices.len() - 1
                    }
                };
                work.charge(indices.len())?;
                if indices.contains(&id) {
                    return Err(PolyhedralError::InvalidBoundary);
                }
                indices.push(id);
            }
            faces.push(BoundaryFace {
                plane: facet.plane,
                indices,
            });
        }

        for face in &mut faces {
            face.indices = refine_edges(&face.indices, &vertices, work)?;
        }
        for (i, face) in faces.iter().enumerate() {
            for other in &faces[..i] {
                check_face_pair(face, other, &vertices, work)?;
            }
        }
        let shells = check_incidence(&faces, &vertices, work)?;
        let reference = vertices[0];
        let mut six_volume = Q::ZERO;
        for face in &faces {
            let a = sub3(vertices[face.indices[0]], reference)?;
            for pair in face.indices[1..].windows(2) {
                work.charge(1)?;
                let b = sub3(vertices[pair[0]], reference)?;
                let c = sub3(vertices[pair[1]], reference)?;
                six_volume = six_volume.add(det(a, b, c)?)?;
            }
        }
        if six_volume.sign() <= 0 {
            return Err(PolyhedralError::InvalidBoundary);
        }
        Ok(Self {
            vertices,
            faces,
            shells,
            volume: six_volume.div(Q::int(6))?,
        })
    }
}

// Cyclic projection preserves the corresponding component of a cross product.
fn projection(plane: Plane3h) -> R<(usize, usize, i8)> {
    let n = plane.normal();
    let k = n
        .iter()
        .position(|q| q.sign() != 0)
        .ok_or(PolyhedralError::InvalidBoundary)?;
    Ok(((k + 1) % 3, (k + 2) % 3, n[k].sign()))
}

fn side(a: P, b: P, p: P, projection: (usize, usize, i8)) -> R<i8> {
    let (x, y, normal_sign) = projection;
    let dx = b[x].sub(a[x])?;
    let dy = b[y].sub(a[y])?;
    let area = dx.mul(p[y].sub(a[y])?)?.sub(dy.mul(p[x].sub(a[x])?)?)?;
    Ok(area.sign() * normal_sign)
}

fn check_facet(facet: &Facet, work: &mut PolyhedralWork<'_>) -> R<()> {
    let points = &facet.vertices;
    if points.len() < 3 {
        return Err(PolyhedralError::InvalidBoundary);
    }
    let projection = projection(facet.plane)?;
    for (i, &a) in points.iter().enumerate() {
        work.charge(1)?;
        let b = points[(i + 1) % points.len()];
        if a == b || facet.plane.eval(a)?.sign() != 0 {
            return Err(PolyhedralError::InvalidBoundary);
        }
        // Each directed edge must support the entire convex patch, and at
        // least one vertex must certify nonzero area on its material side.
        let mut positive = false;
        for &p in points {
            work.charge(1)?;
            match side(a, b, p, projection)? {
                -1 => return Err(PolyhedralError::InvalidBoundary),
                1 => positive = true,
                _ => {}
            }
        }
        if !positive {
            return Err(PolyhedralError::InvalidBoundary);
        }
    }
    Ok(())
}

// Insert exact vertices along an edge in directed parameter order. Normalizing
// before pairing half-edges lets a single edge meet several shorter pieces.
fn refine_edges(ids: &[usize], vertices: &[P], work: &mut PolyhedralWork<'_>) -> R<Vec<usize>> {
    let mut refined = Vec::new();
    for (i, &start) in ids.iter().enumerate() {
        let end = ids[(i + 1) % ids.len()];
        let a = vertices[start];
        let b = vertices[end];
        let d = sub3(b, a)?;
        let axis = d
            .iter()
            .position(|v| v.sign() != 0)
            .ok_or(PolyhedralError::InvalidBoundary)?;
        work.reserve(&mut refined, 1)?;
        refined.push(start);
        let first = refined.len();
        for (id, &p) in vertices.iter().enumerate() {
            work.charge(1)?;
            let coord = p[axis];
            let asc = d[axis].sign() > 0;
            let (lo, hi) = if asc {
                (a[axis], b[axis])
            } else {
                (b[axis], a[axis])
            };
            if coord.cmp_checked(lo)? != Ordering::Greater
                || coord.cmp_checked(hi)? != Ordering::Less
            {
                continue;
            }
            let t = coord.sub(a[axis])?.div(d[axis])?;
            if mix(a, b, t)? != p {
                continue;
            }
            let mut pos = refined.len();
            while pos > first {
                work.charge(1)?;
                let cmp = vertices[refined[pos - 1]][axis].cmp_checked(coord)?;
                if (asc && cmp == Ordering::Less) || (!asc && cmp == Ordering::Greater) {
                    break;
                }
                pos -= 1;
            }
            work.reserve(&mut refined, 1)?;
            work.charge(refined.len() - pos)?;
            refined.insert(pos, id);
        }
    }
    Ok(refined)
}

fn separated_coplanar(
    a: &BoundaryFace,
    b: &BoundaryFace,
    vertices: &[P],
    work: &mut PolyhedralWork<'_>,
) -> R<bool> {
    let projection = projection(a.plane)?;
    for (i, &id) in a.indices.iter().enumerate() {
        let from = vertices[id];
        let to = vertices[a.indices[(i + 1) % a.indices.len()]];
        let mut separated = true;
        for &other in &b.indices {
            work.charge(1)?;
            if side(from, to, vertices[other], projection)? > 0 {
                separated = false;
                break;
            }
        }
        if separated {
            return Ok(true);
        }
    }
    Ok(false)
}

struct Slice {
    lo: P,
    hi: P,
    through_interior: bool,
}

// Intersect a convex polygon with a nonparallel plane. The contact is a
// closed interval on their common line. A straddling polygon has relative
// interior along the open interval even when both endpoints are vertices.
fn section(
    face: &BoundaryFace,
    plane: Plane3h,
    axis: usize,
    vertices: &[P],
    work: &mut PolyhedralWork<'_>,
) -> R<Option<Slice>> {
    let mut interval: Option<(P, P)> = None;
    let mut positive = false;
    let mut negative = false;
    let mut p = vertices[*face
        .indices
        .last()
        .ok_or(PolyhedralError::InvalidBoundary)?];
    let mut h = plane.eval(p)?;
    for &id in &face.indices {
        work.charge(1)?;
        let next = vertices[id];
        let hn = plane.eval(next)?;
        negative |= hn.sign() < 0;
        positive |= hn.sign() > 0;
        let event = if hn.sign() == 0 {
            Some(next)
        } else if h.sign() * hn.sign() < 0 {
            Some(mix(p, next, h.div(h.sub(hn)?)?)?)
        } else {
            None
        };
        if let Some(q) = event {
            if let Some((lo, hi)) = &mut interval {
                if q[axis].cmp_checked(lo[axis])? == Ordering::Less {
                    *lo = q;
                }
                if q[axis].cmp_checked(hi[axis])? == Ordering::Greater {
                    *hi = q;
                }
            } else {
                interval = Some((q, q));
            }
        }
        p = next;
        h = hn;
    }
    Ok(interval.map(|(lo, hi)| Slice {
        lo,
        hi,
        through_interior: positive && negative,
    }))
}

fn check_face_pair(
    a: &BoundaryFace,
    b: &BoundaryFace,
    vertices: &[P],
    work: &mut PolyhedralWork<'_>,
) -> R<()> {
    work.charge(1)?;
    let line = super::exact::cross(a.plane.normal(), b.plane.normal())?;
    let Some(axis) = line.iter().position(|v| v.sign() != 0) else {
        if a.plane.eval(vertices[b.indices[0]])?.sign() == 0
            && !separated_coplanar(a, b, vertices, work)?
            && !separated_coplanar(b, a, vertices, work)?
        {
            return Err(PolyhedralError::InvalidBoundary);
        }
        return Ok(());
    };
    let Some(first) = section(a, b.plane, axis, vertices, work)? else {
        return Ok(());
    };
    let Some(second) = section(b, a.plane, axis, vertices, work)? else {
        return Ok(());
    };
    let lo = if first.lo[axis].cmp_checked(second.lo[axis])? == Ordering::Less {
        second.lo
    } else {
        first.lo
    };
    let hi = if first.hi[axis].cmp_checked(second.hi[axis])? == Ordering::Greater {
        second.hi
    } else {
        first.hi
    };
    match lo[axis].cmp_checked(hi[axis])? {
        Ordering::Greater => return Ok(()),
        Ordering::Less if first.through_interior || second.through_interior => {
            return Err(PolyhedralError::InvalidBoundary);
        }
        _ => {}
    }
    // Every endpoint of an intersection must occur explicitly on both
    // loops. Seams then pair by identity, and an isolated vertex contact is
    // admitted only if the global link test finds one cyclic fan.
    for face in [a, b] {
        for endpoint in [lo, hi] {
            work.charge(face.indices.len())?;
            if !face.indices.iter().any(|&id| vertices[id] == endpoint) {
                return Err(PolyhedralError::InvalidBoundary);
            }
        }
    }
    Ok(())
}

struct Dart {
    from: usize,
    to: usize,
    face: usize,
    prev: usize,
    twin: Option<usize>,
    visited: bool,
}

fn check_incidence(
    faces: &[BoundaryFace],
    vertices: &[P],
    work: &mut PolyhedralWork<'_>,
) -> R<usize> {
    let mut darts = Vec::new();
    let mut first_dart = Vec::new();
    work.reserve(&mut first_dart, faces.len())?;
    for (face, boundary) in faces.iter().enumerate() {
        let ids = &boundary.indices;
        let start = darts.len();
        work.reserve(&mut darts, ids.len())?;
        first_dart.push(start);
        for (i, &from) in ids.iter().enumerate() {
            work.charge(1)?;
            darts.push(Dart {
                from,
                to: ids[(i + 1) % ids.len()],
                face,
                prev: start + (if i == 0 { ids.len() - 1 } else { i - 1 }),
                twin: None,
                visited: false,
            });
        }
    }
    for i in 0..darts.len() {
        if darts[i].twin.is_some() {
            continue;
        }
        let (u, v) = (darts[i].from, darts[i].to);
        let mut twin = None;
        for (j, dart) in darts.iter().enumerate().skip(i + 1) {
            work.charge(1)?;
            if (dart.from, dart.to) == (u, v) {
                return Err(PolyhedralError::NonManifoldResult);
            }
            if (dart.from, dart.to) == (v, u) && twin.replace(j).is_some() {
                return Err(PolyhedralError::NonManifoldResult);
            }
        }
        let j = twin.ok_or(PolyhedralError::InvalidBoundary)?;
        darts[i].twin = Some(j);
        darts[j].twin = Some(i);
    }

    // Around an oriented vertex, crossing the incoming edge takes one
    // outgoing corner to its successor. A manifold has precisely one orbit,
    // even when two independently closed shells otherwise touch at a point.
    let mut started = Vec::new();
    work.reserve(&mut started, vertices.len())?;
    started.resize(vertices.len(), false);
    for i in 0..darts.len() {
        if darts[i].visited {
            continue;
        }
        let vertex = darts[i].from;
        if started[vertex] {
            return Err(PolyhedralError::NonManifoldResult);
        }
        started[vertex] = true;
        let mut current = i;
        while !darts[current].visited {
            work.charge(1)?;
            darts[current].visited = true;
            let prev = darts[current].prev;
            current = darts[prev].twin.ok_or(PolyhedralError::InvalidBoundary)?;
        }
        if current != i {
            return Err(PolyhedralError::InvalidBoundary);
        }
    }
    let mut seen = Vec::new();
    let mut stack = Vec::new();
    work.reserve(&mut seen, faces.len())?;
    work.reserve(&mut stack, faces.len())?;
    seen.resize(faces.len(), false);
    let mut shells = 0;
    for i in 0..faces.len() {
        if seen[i] {
            continue;
        }
        check_shell_orientation(i, faces, vertices, work)?;
        shells += 1;
        stack.push(i);
        seen[i] = true;
        while let Some(face) = stack.pop() {
            let start = first_dart[face];
            for dart in &darts[start..start + faces[face].indices.len()] {
                work.charge(1)?;
                let twin = dart.twin.ok_or(PolyhedralError::InvalidBoundary)?;
                let other = darts[twin].face;
                if !seen[other] {
                    seen[other] = true;
                    stack.push(other);
                }
            }
        }
    }
    Ok(shells)
}

// Global parity assigns material to alternating nested regions. Matching every
// edge in opposite directions enforces consistent orientation *within* a
// shell. One strict face-interior sample then decides whether that entire shell
// has the right orientation for its nesting depth.
fn check_shell_orientation(
    index: usize,
    faces: &[BoundaryFace],
    vertices: &[P],
    work: &mut PolyhedralWork<'_>,
) -> R<()> {
    let face = &faces[index];
    let a = vertices[face.indices[0]];
    let b = vertices[face.indices[1]];
    let chart = projection(face.plane)?;
    let mut sample = None;
    for &id in &face.indices[2..] {
        work.charge(1)?;
        let c = vertices[id];
        if side(a, b, c, chart)? > 0 {
            let third = Q::ratio(1, 3)?;
            let mut p = [Q::ZERO; 3];
            for k in 0..3 {
                p[k] = a[k].add(b[k])?.add(c[k])?.mul(third)?;
            }
            sample = Some(p);
            break;
        }
    }
    let origin = sample.ok_or(PolyhedralError::InvalidBoundary)?;
    // A direction parallel to a support or passing through an edge is
    // exceptional. For directions (1,k,k²), each such plane excludes at most
    // two k's. If the sample lies on an edge's infinite line, a nonparallel
    // ray meets that line only at the origin, outside the actual edge.
    let mut k = Q::ZERO;
    loop {
        work.charge(1)?;
        let d = [Q::ONE, k, k.mul(k)?];
        if let Some(odd) = crossing_parity(origin, d, faces, vertices, work)? {
            let to_material = dot(face.plane.normal(), d)?.sign() < 0;
            return if odd == to_material {
                Ok(())
            } else {
                Err(PolyhedralError::InvalidBoundary)
            };
        }
        k = k.add(Q::ONE)?;
    }
}

fn crossing_parity(
    origin: P,
    dir: P,
    faces: &[BoundaryFace],
    vertices: &[P],
    work: &mut PolyhedralWork<'_>,
) -> R<Option<bool>> {
    let mut odd = false;
    for face in faces {
        work.charge(1)?;
        let slope = dot(face.plane.normal(), dir)?;
        if slope.sign() == 0 {
            return Ok(None);
        }
        let t = face.plane.eval(origin)?.neg()?.div(slope)?;
        if t.sign() <= 0 {
            continue;
        }
        let mut hit = origin;
        for k in 0..3 {
            hit[k] = hit[k].add(t.mul(dir[k])?)?;
        }
        let chart = projection(face.plane)?;
        let mut inside = true;
        let mut on_edge = false;
        for (j, &id) in face.indices.iter().enumerate() {
            work.charge(1)?;
            let next = face.indices[(j + 1) % face.indices.len()];
            match side(vertices[id], vertices[next], hit, chart)? {
                -1 => {
                    inside = false;
                    break;
                }
                0 => on_edge = true,
                _ => {}
            }
        }
        if inside {
            if on_edge {
                return Ok(None);
            }
            odd = !odd;
        }
    }
    Ok(Some(odd))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A deliberately independent assembly path for invalid draft boundaries.
    fn tetrahedron(offset: i64, size: i64, inward: bool) -> Vec<Facet> {
        let xyz = [
            [offset, offset, offset],
            [offset + size, offset, offset],
            [offset, offset + size, offset],
            [offset, offset, offset + size],
        ];
        [
            ([-1, 0, 0, offset], [0, 3, 2]),
            ([0, -1, 0, offset], [0, 1, 3]),
            ([0, 0, -1, offset], [0, 2, 1]),
            ([1, 1, 1, -3 * offset - size], [1, 2, 3]),
        ]
        .into_iter()
        .map(|(coefficients, ids)| {
            let mut plane = Plane3h::from_coefficients(coefficients).unwrap();
            let mut vertices = ids.map(|i| xyz[i].map(Q::int)).to_vec();
            if inward {
                plane = plane.reversed();
                vertices.reverse();
            }
            Facet { plane, vertices }
        })
        .collect()
    }

    fn validate(facets: Vec<Facet>) -> R<SolidBoundary> {
        SolidBoundary::from_facets(facets, &mut PolyhedralWork::new(usize::MAX, usize::MAX))
    }

    #[test]
    fn rejects_missing_faces_open_loops_and_inconsistent_support() {
        let mut missing = tetrahedron(0, 3, false);
        missing.pop();
        assert_eq!(
            validate(missing).unwrap_err(),
            PolyhedralError::InvalidBoundary
        );
        let mut open = tetrahedron(0, 3, false);
        open[1].vertices.pop();
        assert_eq!(
            validate(open).unwrap_err(),
            PolyhedralError::InvalidBoundary
        );
        let mut off_plane = tetrahedron(0, 3, false);
        off_plane[0].plane = Plane3h::from_coefficients([-1, 0, 0, 1]).unwrap();
        assert_eq!(
            validate(off_plane).unwrap_err(),
            PolyhedralError::InvalidBoundary
        );
    }

    #[test]
    fn validates_orientation_of_every_disconnected_shell() {
        assert_eq!(
            validate(tetrahedron(0, 1, true)).unwrap_err(),
            PolyhedralError::InvalidBoundary
        );
        let mut shells = tetrahedron(0, 3, false);
        shells.extend(tetrahedron(5, 1, true));
        assert_eq!(
            validate(shells).unwrap_err(),
            PolyhedralError::InvalidBoundary
        );
        let mut valid = tetrahedron(0, 3, false);
        valid.extend(tetrahedron(5, 1, false));
        let boundary = validate(valid).unwrap();
        assert_eq!(boundary.shell_count(), 2);
        assert_eq!(boundary.volume(), (14, 3));
    }

    #[test]
    fn nested_boundary_is_inward_for_a_cavity() {
        let mut incorrect = tetrahedron(0, 6, false);
        incorrect.extend(tetrahedron(1, 1, false));
        assert_eq!(
            validate(incorrect).unwrap_err(),
            PolyhedralError::InvalidBoundary
        );

        let mut cavity = tetrahedron(0, 6, false);
        cavity.extend(tetrahedron(1, 1, true));
        let boundary = validate(cavity).unwrap();
        assert_eq!(boundary.shell_count(), 2);
        assert_eq!(boundary.volume(), (215, 6));
    }

    #[test]
    fn rejects_coplanar_duplicates_and_crossing_patches() {
        let mut duplicate = tetrahedron(0, 2, false);
        duplicate.extend(tetrahedron(0, 2, false));
        assert_eq!(
            validate(duplicate).unwrap_err(),
            PolyhedralError::InvalidBoundary
        );
        let mut intersecting = tetrahedron(0, 4, false);
        intersecting.extend(tetrahedron(1, 2, false));
        assert_eq!(
            validate(intersecting).unwrap_err(),
            PolyhedralError::InvalidBoundary
        );
    }
}
