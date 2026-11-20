// SPDX-License-Identifier: Apache-2.0
use std::sync::atomic::AtomicBool;

use rcct::polyhedral::{
    ConvexSolid, IndexedFace, Operation3, Plane3h, PolyhedralError, PolyhedralWork, SolidBoundary,
    boolean3,
};

type Point = [(i128, i128); 3];
const ZERO: (i128, i128) = (0, 1);

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

fn plane(row: [i64; 4]) -> Plane3h {
    Plane3h::from_coefficients(row).unwrap()
}

struct Shell {
    points: Vec<Point>,
    faces: Vec<(Plane3h, Vec<usize>)>,
}

impl Shell {
    fn import(&self, work: &mut PolyhedralWork<'_>) -> Result<SolidBoundary, PolyhedralError> {
        let faces: Vec<_> = self
            .faces
            .iter()
            .map(|(plane, indices)| IndexedFace {
                plane: *plane,
                vertices: indices,
            })
            .collect();
        SolidBoundary::from_indexed_faces(&self.points, &faces, work)
    }

    fn read(boundary: &SolidBoundary) -> Self {
        Self {
            points: (0..boundary.vertex_count())
                .map(|i| boundary.vertex(i).unwrap())
                .collect(),
            faces: (0..boundary.face_count())
                .map(|i| {
                    (
                        boundary.face_plane(i).unwrap(),
                        boundary.face_vertices(i).unwrap().to_vec(),
                    )
                })
                .collect(),
        }
    }
}

// Axis intercepts are 1/2, 2/3, 3/4. Define topology without using a Boolean
// result or the halfspace admission implementation to generate the fixture.
fn tetrahedron() -> Shell {
    Shell {
        points: vec![
            [ZERO; 3],
            [(-2, -4), ZERO, ZERO],
            [ZERO, (6, 9), ZERO],
            [ZERO, ZERO, (3, 4)],
        ],
        faces: vec![
            (plane([-1, 0, 0, 0]), vec![0, 3, 2]),
            (plane([0, -1, 0, 0]), vec![0, 1, 3]),
            (plane([0, 0, -1, 0]), vec![0, 2, 1]),
            (plane([12, 9, 8, -6]), vec![1, 2, 3]),
        ],
    }
}

#[test]
fn rational_tetrahedron_survives_input_drop_and_clips_as_convex() {
    let body = tetrahedron().import(&mut work()).unwrap();
    assert_eq!(body.volume(), (1, 24));
    assert_eq!(
        (body.vertex_count(), body.face_count(), body.shell_count()),
        (4, 4, 1)
    );
    let copied = Shell::read(&body);
    assert_eq!(
        copied.points,
        [
            [ZERO; 3],
            [(1, 2), ZERO, ZERO],
            [ZERO, (2, 3), ZERO],
            [ZERO, ZERO, (3, 4)]
        ]
    );
    assert_eq!(copied.faces, tetrahedron().faces);
    let a = body.as_convex(&mut work()).unwrap();
    let box_planes = [
        [-4, 0, 0, 1],
        [1, 0, 0, -1],
        [0, -1, 0, 0],
        [0, 1, 0, -1],
        [0, 0, -1, 0],
        [0, 0, 1, -1],
    ]
    .map(plane);
    let b = ConvexSolid::from_halfspaces(&box_planes, &mut work()).unwrap();
    let tip = boolean3(&a, &b, Operation3::Intersection, &mut work()).unwrap();
    // x >= 1/4 retains a similar tetrahedron at half the linear scale.
    assert_eq!(tip.volume(), (1, 192));
    assert_eq!(tip.vertex_count(), 4);
}

#[test]
fn indexed_l_prism_has_reentrant_edges_and_conforming_coplanar_faces() {
    // Row-major vertices for three unit squares in an L, extruded from z=0 to z=1.
    let xy: [[i64; 2]; 8] = [
        [0, 0],
        [1, 0],
        [2, 0],
        [0, 1],
        [1, 1],
        [2, 1],
        [0, 2],
        [1, 2],
    ];
    let mut shell = Shell {
        points: Vec::new(),
        faces: Vec::new(),
    };
    for z in [0, 1] {
        shell
            .points
            .extend(xy.map(|[x, y]| [(i128::from(x), 1), (i128::from(y), 1), (z, 1)]));
    }
    for [a, b, c, d] in [[0, 1, 4, 3], [1, 2, 5, 4], [3, 4, 7, 6]] {
        shell.faces.push((plane([0, 0, -1, 0]), vec![d, c, b, a]));
        shell
            .faces
            .push((plane([0, 0, 1, -1]), vec![a + 8, b + 8, c + 8, d + 8]));
    }
    let perimeter = [0, 1, 2, 5, 4, 7, 6, 3, 0];
    for edge in perimeter.windows(2) {
        let (a, b) = (edge[0], edge[1]);
        let [x, y] = xy[a];
        let (nx, ny) = (xy[b][1] - y, x - xy[b][0]);
        shell.faces.push((
            plane([nx, ny, 0, -(nx * x + ny * y)]),
            vec![a, b, b + 8, a + 8],
        ));
    }
    let body = shell.import(&mut work()).unwrap();
    assert_eq!(body.volume(), (3, 1));
    assert_eq!((body.shell_count(), body.face_count()), (1, 14));
    assert_eq!(Shell::read(&body).faces, shell.faces);
    assert_eq!(
        body.as_convex(&mut work()).unwrap_err(),
        PolyhedralError::UnsupportedNonconvex
    );
}

#[test]
fn cavity_output_can_be_read_back_but_cannot_be_used_as_convex() {
    fn cube(lo: i64, hi: i64) -> ConvexSolid {
        let planes = [
            [-1, 0, 0, lo],
            [1, 0, 0, -hi],
            [0, -1, 0, lo],
            [0, 1, 0, -hi],
            [0, 0, -1, lo],
            [0, 0, 1, -hi],
        ]
        .map(plane);
        ConvexSolid::from_halfspaces(&planes, &mut work()).unwrap()
    }
    let loaded = {
        let cavity = boolean3(
            &cube(0, 3),
            &cube(1, 2),
            Operation3::Difference,
            &mut work(),
        )
        .unwrap();
        Shell::read(&cavity).import(&mut work()).unwrap()
    };
    assert_eq!(loaded.volume(), (26, 1));
    assert_eq!(loaded.shell_count(), 2);
    assert!(matches!(
        loaded.as_convex(&mut work()),
        Err(PolyhedralError::UnsupportedNonconvex)
    ));
    assert!(
        SolidBoundary::from_indexed_faces(&[], &[], &mut work())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn rejects_invalid_indices_and_coordinate_aliases() {
    let vertices_only = SolidBoundary::from_indexed_faces(&[[ZERO; 3]], &[], &mut work());
    assert_eq!(vertices_only.unwrap_err(), PolyhedralError::InvalidBoundary);
    let mut missing = tetrahedron();
    missing.faces[0].1[1] = usize::MAX;
    assert_eq!(
        missing.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );

    let mut repeated = tetrahedron();
    repeated.faces[0].1.push(0); // Closing edge is implicit.
    assert_eq!(
        repeated.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );

    let mut alias = tetrahedron();
    alias.points.push([(5, 10), ZERO, ZERO]);
    alias.faces[1].1[1] = 4;
    assert_eq!(
        alias.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );

    let mut unused = tetrahedron();
    unused.points.push([(1, 1); 3]);
    assert_eq!(
        unused.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );
}

#[test]
fn rejects_malformed_geometry_and_open_boundaries() {
    let mut open = tetrahedron();
    open.faces.pop();
    assert_eq!(
        open.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );

    let mut misplaced = tetrahedron();
    misplaced.faces[3].0 = plane([12, 9, 8, -7]);
    assert_eq!(
        misplaced.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );

    let mut reversed_loop = tetrahedron();
    reversed_loop.faces[2].1.reverse();
    assert_eq!(
        reversed_loop.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );

    let mut inverted = tetrahedron();
    for (support, indices) in &mut inverted.faces {
        *support = support.reversed();
        indices.reverse();
    }
    assert_eq!(
        inverted.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );
}

#[test]
fn both_incident_faces_must_declare_the_same_edge_subdivision() {
    let mut divided = tetrahedron();
    divided.points.push([(1, 4), ZERO, ZERO]);
    divided.faces[1].1 = vec![0, 4, 1, 3];
    assert_eq!(
        divided.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );
    divided.faces[2].1 = vec![0, 2, 1, 4];
    let conforming = divided.import(&mut work()).unwrap();
    assert_eq!(conforming.vertex_count(), 5);
    assert_eq!(conforming.volume(), (1, 24));
}

#[test]
fn two_closed_shells_meeting_at_one_vertex_are_nonmanifold() {
    let mut shell = tetrahedron();
    // Reflect a second tetrahedron through the origin. The shared vertex
    // now has two disconnected face fans even though every edge is paired.
    shell.points.extend([
        [(-1, 2), ZERO, ZERO],
        [ZERO, (-2, 3), ZERO],
        [ZERO, ZERO, (-3, 4)],
    ]);
    shell.faces.extend([
        (plane([1, 0, 0, 0]), vec![0, 5, 6]),
        (plane([0, 1, 0, 0]), vec![0, 6, 4]),
        (plane([0, 0, 1, 0]), vec![0, 4, 5]),
        (plane([-12, -9, -8, -6]), vec![4, 6, 5]),
    ]);
    assert_eq!(
        shell.import(&mut work()).unwrap_err(),
        PolyhedralError::NonManifoldResult
    );
}

#[test]
fn rational_syntax_limits_and_cancellation_have_distinct_results() {
    let mut source = tetrahedron();
    source.points[1][0] = (1, 0);
    assert_eq!(
        source.import(&mut work()).unwrap_err(),
        PolyhedralError::InvalidBoundary
    );
    source.points[1][0] = (i128::MIN, 1);
    assert_eq!(
        source.import(&mut work()).unwrap_err(),
        PolyhedralError::ArithmeticLimit
    );
    source.points[1][0] = (1, 2);
    let mut reference = work();
    let before = reference.remaining();
    assert_eq!(source.import(&mut reference).unwrap().volume(), (1, 24));
    let one_short = before.0 - reference.remaining().0 - 1;
    let mut tiny = PolyhedralWork::new(one_short, usize::MAX);
    assert_eq!(
        source.import(&mut tiny).unwrap_err(),
        PolyhedralError::ResourceLimit
    );
    assert!(tiny.remaining().0 < one_short);
    let one_byte_short = before.1 - reference.remaining().1 - 1;
    let mut almost_enough = PolyhedralWork::new(usize::MAX, one_byte_short);
    assert_eq!(
        source.import(&mut almost_enough).unwrap_err(),
        PolyhedralError::ResourceLimit
    );
    assert!(almost_enough.remaining().1 < one_byte_short);
    let mut no_storage = PolyhedralWork::new(1_000, 0);
    assert_eq!(
        source.import(&mut no_storage).unwrap_err(),
        PolyhedralError::ResourceLimit
    );
    let stop = AtomicBool::new(true);
    let mut cancelled = work().with_cancellation(&stop);
    assert_eq!(
        source.import(&mut cancelled).unwrap_err(),
        PolyhedralError::Cancelled
    );
    assert_eq!(source.import(&mut work()).unwrap().volume(), (1, 24));
}
