// SPDX-License-Identifier: Apache-2.0
//! P05 overlays two partially touching, differently triangulated walls.
#[path = "support/p14_boundary.rs"]
mod boundary;

use std::collections::{BTreeMap, BTreeSet};

use rcct::polyhedral::{
    IndexedFace, Operation3, Plane3h, PolyhedralWork, SolidBoundary, boolean_prismatic3,
};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

enum ContactWall {
    BackWithDiagonal,
    FrontWithFan,
}

// Both boxes have unit height and use the shear (x,y,z) = (u+v/2,v,w).
// In their common plane v=1, A occupies u=[0,2] and B occupies u=[1,3].
// Their opposite-facing walls overlap only on the middle unit square.
fn prism(contact: ContactWall) -> SolidBoundary {
    let (u, v) = match contact {
        ContactWall::BackWithDiagonal => (0i64, 0i64),
        ContactWall::FrontWithFan => (1, 1),
    };
    let mut points = Vec::new();
    for z in [0, 1] {
        for [u, v] in [[u, v], [u + 2, v], [u + 2, v + 1], [u, v + 1]] {
            points.push([(i128::from(2 * u + v), 2), (i128::from(v), 1), (z, 1)]);
        }
    }
    let [left, right, front, back, bottom, top] = [
        [-2, 1, 0, 2 * u],
        [2, -1, 0, -2 * (u + 2)],
        [0, -1, 0, v],
        [0, 1, 0, -(v + 1)],
        [0, 0, -1, 0],
        [0, 0, 1, -1],
    ]
    .map(|row| Plane3h::from_coefficients(row).unwrap());
    let mut patches = vec![
        (left, vec![0, 4, 7, 3]),
        (right, vec![1, 2, 6, 5]),
        (bottom, vec![0, 3, 2, 1]),
        (top, vec![4, 5, 6, 7]),
    ];
    match contact {
        ContactWall::BackWithDiagonal => {
            patches.extend([
                (front, vec![0, 1, 5, 4]),
                (back, vec![3, 7, 6]),
                (back, vec![3, 6, 2]),
            ]);
        }
        ContactWall::FrontWithFan => {
            // Fan center (u,v,w)=(2,1,1/2). One spoke crosses A's diagonal
            // inside the common square at (3/2,1,3/4).
            points.push([(5, 2), (1, 1), (1, 2)]);
            patches.extend([
                (back, vec![3, 7, 6, 2]),
                (front, vec![0, 1, 8]),
                (front, vec![1, 5, 8]),
                (front, vec![5, 4, 8]),
                (front, vec![4, 0, 8]),
            ]);
        }
    }
    let faces: Vec<_> = patches
        .iter()
        .map(|(plane, ids)| IndexedFace {
            plane: *plane,
            vertices: ids,
        })
        .collect();
    let solid = SolidBoundary::from_indexed_faces(&points, &faces, &mut work()).unwrap();
    for (index, face) in faces.iter().enumerate() {
        assert_eq!(solid.face_plane(index), Some(face.plane));
        assert_eq!(solid.face_vertices(index), Some(face.vertices));
    }
    solid
}

fn assert_trim_seams(solid: &SolidBoundary) {
    let vertices: Vec<_> = (0..solid.vertex_count())
        .map(|id| solid.vertex(id).unwrap())
        .collect();
    assert_eq!(
        vertices.iter().collect::<BTreeSet<_>>().len(),
        vertices.len()
    );
    let mut directed = BTreeMap::new();
    for face in 0..solid.face_count() {
        let support = solid.face_plane(face).unwrap().coefficients();
        let ids = solid.face_vertices(face).unwrap();
        for (&from, &to) in ids.iter().zip(ids.iter().cycle().skip(1)) {
            assert!(directed.insert((from, to), support).is_none());
        }
    }
    for &(from, to) in directed.keys() {
        assert!(
            directed.contains_key(&(to, from)),
            "unmatched edge {from}->{to}"
        );
    }

    // At each end of the common square, an exposed piece of the old contact
    // wall meets a slanted side wall. Traverse the complete height by vertex
    // ID. Each pair of successive heights must be one edge in *both* faces.
    // This allows any triangulation while detecting a hanging seam endpoint.
    for (twice_x, contact, side) in [
        (3, [0, 1, 0, -1], [-2, 1, 0, 2]),
        (5, [0, -1, 0, 1], [2, -1, 0, -4]),
    ] {
        let mut crease: Vec<_> = vertices
            .iter()
            .enumerate()
            .filter_map(|(id, &[(x, xd), y, z])| {
                (2 * x == twice_x * xd && y == (1, 1)).then_some((id, z))
            })
            .collect();
        crease.sort_by(|(_, (z, d)), (_, (w, e))| (z * e).cmp(&(w * d)));
        assert_eq!(crease.first().unwrap().1, (0, 1));
        assert_eq!(crease.last().unwrap().1, (1, 1));
        let on_crease: BTreeSet<_> = crease.iter().map(|&(id, _)| id).collect();
        let edge_uses = directed
            .keys()
            .filter(|(a, b)| on_crease.contains(a) && on_crease.contains(b))
            .count();
        assert_eq!(
            edge_uses,
            2 * (crease.len() - 1),
            "edge skips a trim vertex"
        );
        for pair in crease.windows(2) {
            let (a, b) = (pair[0].0, pair[1].0);
            let adjacent = [directed[&(a, b)], directed[&(b, a)]];
            assert!(adjacent.contains(&contact) && adjacent.contains(&side));
        }
    }
}

#[test]
fn partial_coplanar_contact_with_unequal_subdivisions_has_exact_trim_and_seams() {
    let a = prism(ContactWall::BackWithDiagonal);
    let b = prism(ContactWall::FrontWithFan);
    let a_cells = [[0, 0], [1, 0]];
    let b_cells = [[1, 1], [2, 1]];
    let union_cells = [[0, 0], [1, 0], [1, 1], [2, 1]];
    for (first, second, retained_cells) in [(&a, &b, &a_cells), (&b, &a, &b_cells)] {
        let apply = |op| boolean_prismatic3(first, second, op, &mut work()).unwrap();
        assert!(apply(Operation3::Intersection).is_empty());
        boundary::assert_boundary(&apply(Operation3::Difference), retained_cells);
        let union = apply(Operation3::Union);
        assert_eq!(union.shell_count(), 1);
        boundary::assert_boundary(&union, &union_cells);
        assert_trim_seams(&union);
    }
}
