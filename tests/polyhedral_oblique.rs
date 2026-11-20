// SPDX-License-Identifier: Apache-2.0
use std::collections::{BTreeMap, BTreeSet};

use rcct::polyhedral::{
    ConvexSolid, Operation3, Plane3h, PolyhedralError, PolyhedralWork, SolidBoundary, boolean3,
};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

// (x,y,z) = (u+v, v+w, -w) reverses orientation and preserves volume.
// Express each fixture's natural (u,v,w) inequalities in the actual input frame.
fn admit_in_frame(rows: &[[i64; 4]]) -> Result<ConvexSolid, PolyhedralError> {
    let planes: Vec<_> = rows
        .iter()
        .map(|&[a, b, c, d]| Plane3h::from_coefficients([a, b - a, b - a - c, d]).unwrap())
        .collect();
    ConvexSolid::from_halfspaces(&planes, &mut work())
}

fn apply(a: &ConvexSolid, b: &ConvexSolid, op: Operation3) -> SolidBoundary {
    boolean3(a, b, op, &mut work()).unwrap()
}

fn assert_solid(boundary: &SolidBoundary, volume: (i128, i128), shells: usize) {
    assert_eq!(boundary.volume(), volume);
    assert_eq!(boundary.shell_count(), shells);
}

// The vertices in each fixture using this oracle lie on the indicated rational
// grid. Invert the linear map using integer arithmetic on that grid.
fn scaled_uvw(vertex: [(i128, i128); 3], scale: i128) -> [i128; 3] {
    let [x, y, z] = vertex.map(|(n, d)| {
        assert_eq!(scale % d, 0);
        n * (scale / d)
    });
    [x - y - z, y + z, -z]
}

fn reflected_cube_and_simplex() -> (ConvexSolid, ConvexSolid) {
    let cube = admit_in_frame(&[
        [-1, 0, 0, 0],
        [1, 0, 0, -1],
        [0, -1, 0, 0],
        [0, 1, 0, -1],
        [0, 0, -1, 0],
        [0, 0, 1, -1],
    ])
    .unwrap();
    let simplex =
        admit_in_frame(&[[-1, 0, 0, 0], [0, -1, 0, 0], [0, 0, -1, 0], [2, 2, 2, -3]]).unwrap();
    (cube, simplex)
}

#[test]
fn simplex_crosses_six_cube_edges_after_orientation_reversal() {
    let (cube, simplex) = reflected_cube_and_simplex();
    // The simplex has volume (3/2)^3 / 6 = 9/16. Outside the cube it
    // has three disjoint tips, each a translated simplex of volume (1/2)^3 / 6.
    let core = apply(&cube, &simplex, Operation3::Intersection);
    assert_solid(&core, (1, 2), 1);
    let points: Vec<_> = (0..core.vertex_count())
        .map(|i| scaled_uvw(core.vertex(i).unwrap(), 2))
        .collect();
    for point in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        assert!(points.contains(&point), "missing edge cut {point:?}");
    }
    assert_solid(&apply(&cube, &simplex, Operation3::Union), (17, 16), 1);
    assert_solid(&apply(&simplex, &cube, Operation3::Difference), (1, 16), 3);
    let readmitted = core.as_convex(&mut work()).unwrap();
    assert_solid(
        &apply(&cube, &readmitted, Operation3::Difference),
        (1, 2),
        1,
    );
}

#[test]
fn constructed_corner_shares_one_vertex_across_every_incident_seam() {
    let (cube, simplex) = reflected_cube_and_simplex();
    for (a, b) in [(&cube, &simplex), (&simplex, &cube)] {
        let result = apply(a, b, Operation3::Intersection);
        // At (u,v,w)=(0,1/2,1), the active outward supports are -u=0,
        // w-1=0 and 2(u+v+w)-3=0. Express them in the reflected frame.
        let supports = [[-1, 1, 1, 0], [0, 0, -1, -1], [2, 0, -2, -3]];
        let event = [(1, 2), (3, 2), (-1, 1)];
        let mut event_ids = Vec::new();
        let mut slacks = Vec::new();
        for id in 0..result.vertex_count() {
            let point = result.vertex(id).unwrap();
            if point == event {
                event_ids.push(id);
            }
            // A separate scale per vertex permits rational seam subdivisions.
            let scale = point
                .iter()
                .map(|&(_, d)| d)
                .try_fold(1, i128::checked_mul)
                .unwrap();
            let [x, y, z] = point.map(|(n, d)| n.checked_mul(scale / d).unwrap());
            let slack = supports.map(|plane| {
                plane
                    .into_iter()
                    .zip([x, y, z, scale])
                    .try_fold(0i128, |sum, (c, h)| sum.checked_sub(h.checked_mul(c)?))
                    .expect("exact support evaluation in the P11 fixture")
            });
            assert!(slack.iter().all(|&s| s >= 0));
            slacks.push(slack);
        }
        let &[event_id] = event_ids.as_slice() else {
            panic!("expected exactly one vertex ID for the constructed corner, got {event_ids:?}");
        };
        // All three slacks are nonnegative on every convex patch. Only this
        // vertex has all three zero, so a patch can contain the corner only
        // by listing it. Record the incoming and outgoing edge of each use.
        let mut seams = BTreeMap::<usize, Vec<(usize, bool)>>::new();
        for face in 0..result.face_count() {
            let ids = result.face_vertices(face).unwrap();
            let Some(i) = ids.iter().position(|&id| id == event_id) else {
                continue;
            };
            assert_eq!(ids.iter().filter(|&&id| id == event_id).count(), 1);
            let plane = result.face_plane(face).unwrap().coefficients();
            let support = supports.iter().position(|&p| p == plane).unwrap();
            let before = ids[(i + ids.len() - 1) % ids.len()];
            let after = ids[(i + 1) % ids.len()];
            seams.entry(before).or_default().push((support, true));
            seams.entry(after).or_default().push((support, false));
        }

        let mut creases = BTreeSet::new();
        for (neighbor, uses) in seams {
            let &[(p, incoming), (q, other_incoming)] = uses.as_slice() else {
                panic!("seam {event_id}–{neighbor} has {uses:?}");
            };
            assert_ne!(incoming, other_incoming, "seam {event_id}–{neighbor}");
            for (i, &s) in slacks[neighbor].iter().enumerate() {
                assert_eq!(s == 0, i == p || i == q, "seam {event_id}–{neighbor}");
            }
            if p != q {
                assert!(creases.insert((p.min(q), p.max(q))));
            }
        }
        // Coplanar subdivision seams are allowed. The three true crease
        // rays each join the independently expected pair of supports once.
        assert_eq!(creases, BTreeSet::from([(0, 1), (0, 2), (1, 2)]));
    }
}

#[test]
fn partial_oblique_contact_removes_only_the_common_face_area() {
    let left = admit_in_frame(&[
        [-1, 0, 0, 0],
        [1, 0, 0, -1],
        [0, -1, 0, 0],
        [0, 1, 0, -2],
        [0, 0, -1, 0],
        [0, 0, 1, -2],
    ])
    .unwrap();
    let right = admit_in_frame(&[
        [-1, 0, 0, 1],
        [1, 0, 0, -2],
        [0, -2, 0, 1],
        [0, 2, 0, -5],
        [0, 0, -3, 1],
        [0, 0, 3, -5],
    ])
    .unwrap();
    assert_solid(&apply(&left, &right, Operation3::Intersection), (0, 1), 0);
    let joined = apply(&left, &right, Operation3::Union);
    assert_solid(&joined, (20, 3), 1);
    let mut remaining_area2 = 0;
    for face in 0..joined.face_count() {
        let vertices: Vec<_> = joined
            .face_vertices(face)
            .unwrap()
            .iter()
            .map(|&i| scaled_uvw(joined.vertex(i).unwrap(), 6))
            .collect();
        if vertices.iter().all(|p| p[0] == 6) {
            let area2: i128 = vertices
                .iter()
                .zip(vertices.iter().cycle().skip(1))
                .map(|(p, q)| p[1] * q[2] - p[2] * q[1])
                .sum();
            remaining_area2 += area2.abs();
        }
    }
    // At u=1 the two footprints have areas 4 and 8/3; their overlap
    // covers (3/2)*(4/3)=2. Twice the overlap disappears from the boundary.
    assert_eq!(remaining_area2, 2 * 6 * 6 * 8 / 3);
}

fn slab(lo_quarters: i64, hi_quarters: i64) -> ConvexSolid {
    admit_in_frame(&[
        [-4, 0, 0, lo_quarters],
        [4, 0, 0, -hi_quarters],
        [0, -1, 0, 0],
        [0, 1, 0, -1],
        [0, 0, -1, 0],
        [0, 0, 1, -1],
    ])
    .unwrap()
}

#[test]
fn exact_sliver_and_gap_have_indistinguishable_binary64_support_positions() {
    // At 2^53 the next larger binary64 value is two units away.
    let n: i64 = 1 << 53;
    let q = 4 * n;
    assert_eq!((q + 1) as f64 / 4.0, (q + 2) as f64 / 4.0);
    let overlap = apply(
        &slab(q - 4, q + 2),
        &slab(q + 1, q + 8),
        Operation3::Intersection,
    );
    assert_solid(&overlap, (1, 4), 1);
    let points: Vec<_> = (0..overlap.vertex_count())
        .map(|i| scaled_uvw(overlap.vertex(i).unwrap(), 4))
        .collect();
    for end in [q + 1, q + 2] {
        assert!(points.contains(&[i128::from(end), 0, 0]));
    }
    let left = slab(q - 4, q + 1);
    let right = slab(q + 2, q + 8);
    assert_solid(&apply(&left, &right, Operation3::Intersection), (0, 1), 0);
    assert_solid(&apply(&left, &right, Operation3::Union), (11, 4), 2);
}

#[test]
fn rank_two_constraints_distinguish_infeasibility_from_an_unbounded_prism() {
    // u and v must both be >=1; the free direction in physical coordinates
    // is (0,1,-1). The third constraint switches between u+v<=1 and <=3.
    let mut planes = [[-1, 0, 0, 1], [0, -1, 0, 1], [1, 1, 0, -1]];
    assert!(matches!(
        admit_in_frame(&planes),
        Err(PolyhedralError::InvalidOperand)
    ));
    planes[2][3] = -3;
    assert!(matches!(
        admit_in_frame(&planes),
        Err(PolyhedralError::UnboundedOperand)
    ));
}
