// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    Arrangement2, ArrangementEmbedding2, ArrangementLimits2, EmbeddingLimits2, FaceBuildError2,
    FaceBuildLimits2, FaceBuildResource2, FaceWindingError2, FaceWindingLimits2,
    FaceWindingResource2, PlanarFaces2, Point2, Segment2, build_arrangement2, build_faces2,
    embed_arrangement2, face_windings2,
};
use std::collections::BTreeMap;

fn raw(a: (f64, f64), b: (f64, f64)) -> Segment2 {
    Segment2::try_new(
        Point2::try_new(a.0, a.1).unwrap(),
        Point2::try_new(b.0, b.1).unwrap(),
    )
    .unwrap()
}
fn rect(x: i64, y: i64, w: i64, h: i64, reverse: bool) -> Vec<Segment2> {
    let points = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    (0..4)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % 4]);
            let (a, b) = if reverse { (b, a) } else { (a, b) };
            raw((a.0 as f64, a.1 as f64), (b.0 as f64, b.1 as f64))
        })
        .collect()
}
fn graph(s: &[Segment2]) -> Arrangement2 {
    build_arrangement2(
        s,
        ArrangementLimits2 {
            max_segments: 20,
            max_pairs: 190,
            max_cuts: 800,
            max_vertices: 300,
            max_edges: 600,
            max_edge_uses: 800,
            max_comparisons: 3_000_000,
        },
    )
    .unwrap()
}
fn embedding(g: &Arrangement2) -> ArrangementEmbedding2<'_> {
    embed_arrangement2(
        g,
        EmbeddingLimits2 {
            max_vertices: 300,
            max_half_edges: 1200,
            max_edge_uses: 800,
            max_comparisons: 3_000_000,
            max_boundary_walks: 1200,
            max_walk_steps: 1200,
        },
    )
    .unwrap()
}
fn limits() -> FaceBuildLimits2 {
    FaceBuildLimits2 {
        max_half_edges: 1200,
        max_walks: 1200,
        max_faces: 1201,
        max_orientation_comparisons: 3_000_000,
        max_containment_queries: 100_000,
        max_containment_edge_checks: 3_000_000,
    }
}
fn winding_limits() -> FaceWindingLimits2 {
    FaceWindingLimits2 {
        max_faces: 1201,
        max_half_edges: 1200,
        max_edge_uses: 800,
        max_propagation_checks: 3_000_000,
    }
}
type Box2 = (i64, i64, i64, i64);
fn walk_box(embedding: &ArrangementEmbedding2<'_>, walk: usize) -> Box2 {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for &h in embedding.boundary_walk(walk).unwrap() {
        let vertex = embedding.half_edges()[h].origin();
        let p = embedding.arrangement().vertices()[vertex]
            .point()
            .point_enclosure();
        assert_eq!(p.x().lower(), p.x().upper());
        assert_eq!(p.y().lower(), p.y().upper());
        xs.push(p.x().lower() as i64);
        ys.push(p.y().lower() as i64);
    }
    (
        *xs.iter().min().unwrap(),
        *ys.iter().min().unwrap(),
        *xs.iter().max().unwrap(),
        *ys.iter().max().unwrap(),
    )
}
fn verify_rectangles(boxes: &[Box2], faces: &PlanarFaces2<'_, '_>) {
    let embedding = faces.embedding();
    assert_eq!(faces.faces().len(), boxes.len() + 1);
    let mut id = BTreeMap::new();
    for (face, item) in faces.faces().iter().enumerate().skip(1) {
        let boundary = item.outer_walk().unwrap();
        assert_eq!(faces.face_of_walk(boundary), Some(face));
        id.insert(walk_box(embedding, boundary), face);
    }
    assert_eq!(id.len(), boxes.len());
    for &current in boxes {
        let outer = id[&current];
        let containing =
            |b: Box2| b.0 < current.0 && b.1 < current.1 && b.2 > current.2 && b.3 > current.3;
        let expected_parent = boxes
            .iter()
            .copied()
            .filter(|&b| containing(b))
            .min_by_key(|b| (b.2 - b.0) * (b.3 - b.1))
            .map_or(0, |b| id[&b]);
        let walks: Vec<_> = (0..embedding.boundary_walk_count())
            .filter(|&w| walk_box(embedding, w) == current)
            .collect();
        assert_eq!(walks.len(), 2);
        assert!(walks.iter().any(|&w| faces.face_of_walk(w) == Some(outer)));
        assert!(
            walks
                .iter()
                .any(|&w| faces.face_of_walk(w) == Some(expected_parent))
        );
    }
}

#[test]
fn independent_rectangle_nesting_and_disjoint_oracle() {
    let scenarios = [
        vec![(0, 0, 20, 20), (2, 2, 18, 18), (4, 4, 16, 16)],
        vec![(0, 0, 20, 20), (2, 2, 6, 6), (10, 10, 18, 18)],
        vec![(0, 0, 4, 4), (6, 0, 10, 4), (12, 0, 16, 4)],
    ];
    for boxes in scenarios {
        for mask in 0..8 {
            let mut segments = Vec::new();
            for (i, &(x0, y0, x1, y1)) in boxes.iter().enumerate() {
                segments.extend(rect(x0, y0, x1 - x0, y1 - y0, mask & (1 << i) != 0));
            }
            if mask % 2 == 0 {
                segments.reverse();
            }
            let g = graph(&segments);
            let e = embedding(&g);
            let faces = build_faces2(&e, limits()).unwrap();
            verify_rectangles(&boxes, &faces);
            let winding = face_windings2(&faces, winding_limits()).unwrap();
            assert_eq!(winding.winding(0), Some(0));
            for (face, item) in faces.faces().iter().enumerate().skip(1) {
                let bbox = walk_box(&e, item.outer_walk().unwrap());
                // A point one quarter unit above/right of this rectangle's
                // lower-left corner lies outside its children in these fixtures.
                let sample = (bbox.0 * 4 + 1, bbox.1 * 4 + 1);
                let expected: i64 = boxes
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| {
                        b.0 * 4 < sample.0
                            && sample.0 < b.2 * 4
                            && b.1 * 4 < sample.1
                            && sample.1 < b.3 * 4
                    })
                    .map(|(i, _)| if mask & (1 << i) == 0 { 1 } else { -1 })
                    .sum();
                assert_eq!(winding.winding(face), Some(expected));
            }
        }
    }
}

#[test]
fn randomized_translated_rectangles_match_independent_face_winding_oracle() {
    let patterns = [
        [(0, 0, 20, 20), (2, 2, 18, 18), (4, 4, 16, 16)],
        [(0, 0, 20, 20), (2, 2, 6, 6), (10, 10, 18, 18)],
        [(0, 0, 4, 4), (6, 0, 10, 4), (12, 0, 16, 4)],
    ];
    let mut state = 0x4eb7_9c13_u64;
    let mut rand = |n: usize| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 32) as usize) % n
    };
    for _ in 0..64 {
        let pattern = patterns[rand(patterns.len())];
        let scale = 1 + rand(3) as i64;
        let dx = rand(21) as i64 - 10;
        let dy = rand(21) as i64 - 10;
        let mask = rand(8);
        let boxes: Vec<Box2> = pattern
            .iter()
            .map(|b| {
                (
                    b.0 * scale + dx,
                    b.1 * scale + dy,
                    b.2 * scale + dx,
                    b.3 * scale + dy,
                )
            })
            .collect();
        let mut segments = Vec::new();
        for (i, &b) in boxes.iter().enumerate() {
            segments.extend(rect(b.0, b.1, b.2 - b.0, b.3 - b.1, mask & (1 << i) != 0));
        }
        if rand(2) == 0 {
            segments.reverse();
        }
        let g = graph(&segments);
        let e = embedding(&g);
        let faces = build_faces2(&e, limits()).unwrap();
        verify_rectangles(&boxes, &faces);
        let winding = face_windings2(&faces, winding_limits()).unwrap();
        for (face, item) in faces.faces().iter().enumerate().skip(1) {
            let b = walk_box(&e, item.outer_walk().unwrap());
            let sample = (b.0 * 4 + 1, b.1 * 4 + 1);
            let expected: i64 = boxes
                .iter()
                .enumerate()
                .filter(|(_, b)| {
                    b.0 * 4 < sample.0
                        && sample.0 < b.2 * 4
                        && b.1 * 4 < sample.1
                        && sample.1 < b.3 * 4
                })
                .map(|(i, _)| if mask & (1 << i) == 0 { 1 } else { -1 })
                .sum();
            assert_eq!(winding.winding(face), Some(expected));
        }
    }
}

#[test]
fn coincident_windings_cancel_or_accumulate_and_open_chains_fail() {
    for (reverse, expected) in [(false, 2), (true, 0)] {
        let mut s = rect(0, 0, 4, 4, false);
        s.extend(rect(0, 0, 4, 4, reverse));
        let g = graph(&s);
        let e = embedding(&g);
        let faces = build_faces2(&e, limits()).unwrap();
        let winding = face_windings2(&faces, winding_limits()).unwrap();
        assert_eq!(winding.winding(0), Some(0));
        assert_eq!(winding.winding(1), Some(expected));
    }
    let mut s = rect(0, 0, 4, 4, false);
    s[0] = Segment2::try_new(s[0].end(), s[0].start()).unwrap();
    let g = graph(&s);
    let e = embedding(&g);
    let faces = build_faces2(&e, limits()).unwrap();
    assert!(matches!(
        face_windings2(&faces, winding_limits()),
        Err(FaceWindingError2::InconsistentChain)
    ));
}

#[test]
fn intersecting_and_edge_adjacent_closed_chains_have_correct_windings() {
    for (second, expected) in [
        ((2, -1, 4, 4), vec![0, 1, 1, 2]),
        ((4, 0, 4, 4), vec![0, 1, 1]),
    ] {
        let mut s = rect(0, 0, 4, 4, false);
        s.extend(rect(second.0, second.1, second.2, second.3, false));
        let g = graph(&s);
        let e = embedding(&g);
        let faces = build_faces2(&e, limits()).unwrap();
        let winding = face_windings2(&faces, winding_limits()).unwrap();
        let mut values: Vec<_> = (0..faces.faces().len())
            .map(|f| winding.winding(f).unwrap())
            .collect();
        values.sort();
        assert_eq!(values, expected);
    }
}

#[test]
fn winding_caps_and_failure_are_atomic() {
    let g = graph(&rect(0, 0, 4, 4, false));
    let e = embedding(&g);
    let faces = build_faces2(&e, limits()).unwrap();
    let baseline = face_windings2(&faces, winding_limits()).unwrap();
    for (expected, lim) in [
        (
            FaceWindingResource2::Faces,
            FaceWindingLimits2 {
                max_faces: 1,
                ..winding_limits()
            },
        ),
        (
            FaceWindingResource2::HalfEdges,
            FaceWindingLimits2 {
                max_half_edges: 7,
                ..winding_limits()
            },
        ),
        (
            FaceWindingResource2::EdgeUses,
            FaceWindingLimits2 {
                max_edge_uses: 3,
                ..winding_limits()
            },
        ),
        (
            FaceWindingResource2::PropagationChecks,
            FaceWindingLimits2 {
                max_propagation_checks: 0,
                ..winding_limits()
            },
        ),
    ] {
        assert!(
            matches!(face_windings2(&faces,lim),Err(FaceWindingError2::LimitExceeded(actual)) if actual==expected)
        );
        assert_eq!(baseline.winding(0), Some(0));
        assert_eq!(baseline.winding(1), Some(1));
    }
    let empty = graph(&[]);
    let empty_embedding = embedding(&empty);
    let empty_faces = build_faces2(&empty_embedding, limits()).unwrap();
    let zero = FaceWindingLimits2 {
        max_faces: 1,
        max_half_edges: 0,
        max_edge_uses: 0,
        max_propagation_checks: 0,
    };
    assert_eq!(
        face_windings2(&empty_faces, zero).unwrap().winding(0),
        Some(0)
    );
}

#[test]
fn subdivided_cell_and_rejected_bridge() {
    let mut s = rect(0, 0, 4, 4, false);
    s.push(raw((0., 0.), (4., 4.)));
    let g = graph(&s);
    let e = embedding(&g);
    let faces = build_faces2(&e, limits()).unwrap();
    assert_eq!(faces.faces().len(), 3);
    assert_eq!(
        (0..e.boundary_walk_count())
            .filter(|&w| faces.face_of_walk(w) == Some(0))
            .count(),
        1
    );
    let mut s = rect(0, 0, 4, 4, false);
    s.push(raw((4., 0.), (6., 0.)));
    let g = graph(&s);
    let e = embedding(&g);
    assert!(matches!(
        build_faces2(&e, limits()),
        Err(FaceBuildError2::UnsupportedNonSimpleBoundary { .. })
    ));
}

#[test]
fn tangent_loops_are_rejected_but_shared_edge_subdivision_is_supported() {
    let mut s = rect(0, 0, 6, 6, false);
    let diamond = [(0., 3.), (2., 2.), (3., 3.), (2., 4.)];
    s.extend((0..4).map(|i| raw(diamond[i], diamond[(i + 1) % 4])));
    let g = graph(&s);
    let e = embedding(&g);
    assert!(matches!(
        build_faces2(&e, limits()),
        Err(FaceBuildError2::UnsupportedNonSimpleBoundary { .. })
    ));

    let mut s = rect(0, 0, 6, 6, false);
    s.extend(rect(0, 2, 2, 2, false));
    let g = graph(&s);
    let e = embedding(&g);
    let faces = build_faces2(&e, limits()).unwrap();
    let winding = face_windings2(&faces, winding_limits()).unwrap();
    let mut values: Vec<_> = (0..faces.faces().len())
        .map(|f| winding.winding(f).unwrap())
        .collect();
    values.sort();
    assert_eq!(values, vec![0, 1, 2]);
}

#[test]
fn concave_containment_does_not_use_bounding_boxes() {
    let polygon = [
        (0., 0.),
        (10., 0.),
        (10., 10.),
        (7., 10.),
        (7., 3.),
        (3., 3.),
        (3., 10.),
        (0., 10.),
    ];
    let mut s: Vec<_> = (0..polygon.len())
        .map(|i| raw(polygon[i], polygon[(i + 1) % polygon.len()]))
        .collect();
    s.extend(rect(1, 3, 1, 1, false));
    s.extend(rect(4, 4, 2, 2, false));
    let g = graph(&s);
    let e = embedding(&g);
    let faces = build_faces2(&e, limits()).unwrap();
    assert_eq!(faces.faces().len(), 4);
    let ids: BTreeMap<_, _> = faces
        .faces()
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, f)| (walk_box(&e, f.outer_walk().unwrap()), i))
        .collect();
    let outer = ids[&(0, 0, 10, 10)];
    for (bbox, expected_parent) in [((1, 3, 2, 4), outer), ((4, 4, 6, 6), 0)] {
        let own = ids[&bbox];
        let owned: Vec<_> = (0..e.boundary_walk_count())
            .filter(|&w| walk_box(&e, w) == bbox)
            .map(|w| faces.face_of_walk(w).unwrap())
            .collect();
        assert_eq!(owned.len(), 2);
        assert!(owned.contains(&own));
        assert!(owned.contains(&expected_parent));
    }
}

#[test]
fn nested_ring_in_face_with_non_dyadic_crossing() {
    let mut s = rect(0, 0, 6, 6, false);
    s.push(raw((0., 1.), (6., 4.)));
    s.push(raw((0., 5.), (6., 1.)));
    s.extend([
        raw((1., 0.25), (2., 0.25)),
        raw((2., 0.25), (2., 0.5)),
        raw((2., 0.5), (1., 0.5)),
        raw((1., 0.5), (1., 0.25)),
    ]);
    let g = graph(&s);
    let e = embedding(&g);
    let faces = build_faces2(&e, limits()).unwrap();
    assert_eq!(faces.faces().len(), 6);
    let bottom = g
        .edge_uses()
        .iter()
        .find(|use_| use_.source() == 0)
        .unwrap();
    let bottom_half = bottom.edge() * 2 + usize::from(!bottom.forward());
    let bottom_walk = (0..e.boundary_walk_count())
        .find(|&w| e.boundary_walk(w).unwrap().contains(&bottom_half))
        .unwrap();
    let expected_parent = faces.face_of_walk(bottom_walk).unwrap();
    assert_ne!(expected_parent, 0);
    let child_walks: Vec<_> = (0..e.boundary_walk_count())
        .filter(|&w| {
            e.boundary_walk(w).unwrap().iter().all(|&h| {
                let point = g.vertices()[e.half_edges()[h].origin()]
                    .point()
                    .point_enclosure();
                point.x().lower() >= 1.
                    && point.x().upper() <= 2.
                    && point.y().lower() >= 0.25
                    && point.y().upper() <= 0.5
            })
        })
        .collect();
    assert_eq!(child_walks.len(), 2);
    assert!(
        child_walks
            .iter()
            .any(|&w| faces.face_of_walk(w) == Some(expected_parent))
    );
    assert!(matches!(
        face_windings2(&faces, winding_limits()),
        Err(FaceWindingError2::InconsistentChain)
    ));
}

#[test]
fn subnormal_and_extreme_nested_squares() {
    for (outer, inner) in [(f64::MAX, 1.), (f64::from_bits(4), f64::from_bits(1))] {
        let ring = |lo: f64, hi: f64| {
            [
                raw((lo, lo), (hi, lo)),
                raw((hi, lo), (hi, hi)),
                raw((hi, hi), (lo, hi)),
                raw((lo, hi), (lo, lo)),
            ]
        };
        let mut s = Vec::from(ring(-outer, outer));
        s.extend(ring(-inner, inner));
        let g = graph(&s);
        let e = embedding(&g);
        let faces = build_faces2(&e, limits()).unwrap();
        assert_eq!(faces.faces().len(), 3);
        let counts: Vec<_> = (0..3)
            .map(|f| {
                (0..e.boundary_walk_count())
                    .filter(|&w| faces.face_of_walk(w) == Some(f))
                    .count()
            })
            .collect();
        assert_eq!(counts.iter().sum::<usize>(), 4);
        assert_eq!(counts[0], 1);
        assert!(counts[1..].contains(&2));
    }
}

#[test]
fn caps_and_failure_are_atomic() {
    let g = graph(&rect(0, 0, 4, 4, false));
    let e = embedding(&g);
    let baseline = build_faces2(&e, limits()).unwrap();
    for (expected, lim) in [
        (
            FaceBuildResource2::HalfEdges,
            FaceBuildLimits2 {
                max_half_edges: 7,
                ..limits()
            },
        ),
        (
            FaceBuildResource2::Walks,
            FaceBuildLimits2 {
                max_walks: 1,
                ..limits()
            },
        ),
        (
            FaceBuildResource2::Faces,
            FaceBuildLimits2 {
                max_faces: 1,
                ..limits()
            },
        ),
        (
            FaceBuildResource2::OrientationComparisons,
            FaceBuildLimits2 {
                max_orientation_comparisons: 0,
                ..limits()
            },
        ),
        (
            FaceBuildResource2::ContainmentQueries,
            FaceBuildLimits2 {
                max_containment_queries: 0,
                ..limits()
            },
        ),
        (
            FaceBuildResource2::ContainmentEdgeChecks,
            FaceBuildLimits2 {
                max_containment_edge_checks: 0,
                ..limits()
            },
        ),
    ] {
        assert!(
            matches!(build_faces2(&e,lim),Err(FaceBuildError2::LimitExceeded(actual)) if actual==expected)
        );
        assert_eq!(baseline.faces().len(), 2);
        assert_eq!(e.boundary_walk_count(), 2);
    }
    let empty = graph(&[]);
    let empty_embedding = embedding(&empty);
    let zero = FaceBuildLimits2 {
        max_half_edges: 0,
        max_walks: 0,
        max_faces: 1,
        max_orientation_comparisons: 0,
        max_containment_queries: 0,
        max_containment_edge_checks: 0,
    };
    assert_eq!(
        build_faces2(&empty_embedding, zero).unwrap().faces().len(),
        1
    );
}
