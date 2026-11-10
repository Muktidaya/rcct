// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    Arrangement2, ArrangementEmbedding2, ArrangementLimits2, BooleanOp2, BooleanRegionLimits2,
    BridgeAnalysis2, BridgeAnalysisLimits2, EmbeddingLimits2, FaceBuildError2, FaceBuildLimits2,
    FaceBuildResource2, FaceWindingError2, FaceWindingLimits2, FillRule2, GroupWindingLimits2,
    Point2, RegionPointBudget2, RegionPointLocation2, Segment2, analyze_bridges2,
    build_arrangement2, build_faces_with_bridges2, build_faces2, embed_arrangement2,
    face_windings2, group_windings2, select_boolean2,
};

fn raw(a: (f64, f64), b: (f64, f64)) -> Segment2 {
    Segment2::try_new(
        Point2::try_new(a.0, a.1).unwrap(),
        Point2::try_new(b.0, b.1).unwrap(),
    )
    .unwrap()
}
fn square(x: f64, y: f64, size: f64) -> Vec<Segment2> {
    let p = [(x, y), (x + size, y), (x + size, y + size), (x, y + size)];
    (0..4).map(|i| raw(p[i], p[(i + 1) % 4])).collect()
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
fn bridges(g: &Arrangement2) -> BridgeAnalysis2<'_> {
    analyze_bridges2(
        g,
        BridgeAnalysisLimits2 {
            max_vertices: 300,
            max_edges: 600,
            max_adjacency_visits: 1200,
            max_stack_depth: 300,
            max_components: 300,
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
fn owner_of_halfedge(f: &rcct::planar::PlanarFaces2<'_, '_>, h: usize) -> usize {
    let e = f.embedding();
    let walk = (0..e.boundary_walk_count())
        .find(|&w| e.boundary_walk(w).unwrap().contains(&h))
        .unwrap();
    f.face_of_walk(walk).unwrap()
}
fn assert_euler(g: &Arrangement2, b: &BridgeAnalysis2<'_>, f: &rcct::planar::PlanarFaces2<'_, '_>) {
    assert_eq!(
        f.faces().len(),
        g.edges().len() + b.component_count() + 1 - g.vertices().len()
    );
    for edge in 0..g.edges().len() {
        if b.is_bridge(edge) == Some(true) {
            assert_eq!(
                owner_of_halfedge(f, edge * 2),
                owner_of_halfedge(f, edge * 2 + 1)
            );
        }
    }
}
fn source_edge(g: &Arrangement2, source: usize) -> usize {
    g.edge_uses()
        .iter()
        .find(|u| u.source() == source)
        .unwrap()
        .edge()
}

#[test]
fn outer_and_inner_spurs_receive_the_correct_face() {
    for (spur, expected_face) in [(((4., 0.), (6., 0.)), 0), (((0., 2.), (2., 2.)), 1)] {
        let mut s = square(0., 0., 4.);
        s.push(raw(spur.0, spur.1));
        let g = graph(&s);
        let e = embedding(&g);
        let b = bridges(&g);
        assert!(matches!(
            build_faces2(&e, limits()),
            Err(FaceBuildError2::UnsupportedNonSimpleBoundary { .. })
        ));
        let f = build_faces_with_bridges2(&e, &b, limits()).unwrap();
        assert_eq!(f.faces().len(), 2);
        assert_euler(&g, &b, &f);
        let edge = source_edge(&g, 4);
        assert_eq!(b.is_bridge(edge), Some(true));
        assert_eq!(owner_of_halfedge(&f, edge * 2), expected_face);
        assert!(matches!(
            face_windings2(
                &f,
                FaceWindingLimits2 {
                    max_faces: 10,
                    max_half_edges: 100,
                    max_edge_uses: 100,
                    max_propagation_checks: 1000
                }
            ),
            Err(FaceWindingError2::InconsistentChain)
        ));
        let labels = [Some(0), Some(0), Some(0), Some(0), None];
        let w = group_windings2(
            &f,
            &labels,
            2,
            GroupWindingLimits2 {
                max_sources: 10,
                max_groups: 2,
                max_faces: 10,
                max_half_edges: 100,
                max_edge_uses: 100,
                max_winding_cells: 1000,
                max_propagation_checks: 1000,
            },
        )
        .unwrap();
        let region = select_boolean2(
            &w,
            0,
            FillRule2::NonZero,
            1,
            FillRule2::NonZero,
            BooleanOp2::Union,
            BooleanRegionLimits2 {
                max_faces: 10,
                max_vertices: 100,
                max_half_edges: 100,
                max_boundary_edges: 100,
                max_boundary_walks: 100,
                max_walk_steps: 100,
                max_rotation_checks: 1000,
            },
        )
        .unwrap();
        let mut budget = RegionPointBudget2::new(region.boundary_edge_count());
        let sample = if expected_face == 0 {
            Point2::try_new(5., 0.).unwrap()
        } else {
            Point2::try_new(1., 2.).unwrap()
        };
        let expected = if expected_face == 0 {
            RegionPointLocation2::Outside
        } else {
            RegionPointLocation2::Inside
        };
        assert_eq!(region.locate_point(sample, &mut budget), Ok(expected));
    }
}

#[test]
fn detached_trees_are_grouped_inside_and_outside_nested_rings() {
    let mut s = square(0., 0., 10.);
    s.extend(square(3., 3., 4.));
    s.push(raw((4., 4.), (5., 4.)));
    s.push(raw((5., 4.), (5., 5.)));
    s.push(raw((12., 0.), (13., 0.)));
    let g = graph(&s);
    let e = embedding(&g);
    let b = bridges(&g);
    let f = build_faces_with_bridges2(&e, &b, limits()).unwrap();
    assert_eq!(f.faces().len(), 3);
    assert_euler(&g, &b, &f);
    let inner = source_edge(&g, 8);
    let inner_branch = source_edge(&g, 9);
    let outer = source_edge(&g, 10);
    assert_ne!(owner_of_halfedge(&f, inner * 2), 0);
    assert_eq!(owner_of_halfedge(&f, outer * 2), 0);
    let inner_face = owner_of_halfedge(&f, inner * 2);
    assert_eq!(owner_of_halfedge(&f, inner_branch * 2), inner_face);
    let inner_ring_ccw = f
        .faces()
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(face, item)| {
            let walk = item.outer_walk()?;
            let edge = e
                .boundary_walk(walk)?
                .first()
                .map(|&h| e.half_edges()[h].edge())?;
            g.edge_uses()
                .iter()
                .any(|u| u.edge() == edge && (4..8).contains(&u.source()))
                .then_some(face)
        })
        .unwrap();
    assert_eq!(inner_face, inner_ring_ccw);
}

#[test]
fn forest_is_one_unbounded_face_and_opposite_uses_cancel() {
    let s = [
        raw((0., 0.), (1., 0.)),
        raw((1., 0.), (2., 1.)),
        raw((1., 0.), (2., -1.)),
    ];
    let g = graph(&s);
    let e = embedding(&g);
    let b = bridges(&g);
    let f = build_faces_with_bridges2(&e, &b, limits()).unwrap();
    assert_eq!(f.faces().len(), 1);
    assert_euler(&g, &b, &f);
    let w = face_windings2(
        &f,
        FaceWindingLimits2 {
            max_faces: 10,
            max_half_edges: 100,
            max_edge_uses: 100,
            max_propagation_checks: 1000,
        },
    );
    assert!(matches!(w, Err(FaceWindingError2::InconsistentChain)));
    let s = [raw((0., 0.), (1., 0.)), raw((1., 0.), (0., 0.))];
    let g = graph(&s);
    let e = embedding(&g);
    let b = bridges(&g);
    let f = build_faces_with_bridges2(&e, &b, limits()).unwrap();
    assert_eq!(f.faces().len(), 1);
    assert_euler(&g, &b, &f);
    assert_eq!(
        face_windings2(
            &f,
            FaceWindingLimits2 {
                max_faces: 10,
                max_half_edges: 100,
                max_edge_uses: 100,
                max_propagation_checks: 1000
            }
        )
        .unwrap()
        .winding(0),
        Some(0)
    );
}

#[test]
fn multiple_core_cycles_and_tangent_loops_remain_unsupported() {
    let mut s = square(0., 0., 2.);
    s.extend(square(4., 0., 2.));
    s.push(raw((2., 1.), (4., 1.)));
    let g = graph(&s);
    let e = embedding(&g);
    let b = bridges(&g);
    assert!(matches!(
        build_faces_with_bridges2(&e, &b, limits()),
        Err(FaceBuildError2::UnsupportedNonSimpleBoundary { .. })
    ));
    let mut s = square(0., 0., 6.);
    let p = [(0., 3.), (2., 2.), (3., 3.), (2., 4.)];
    s.extend((0..4).map(|i| raw(p[i], p[(i + 1) % 4])));
    let g = graph(&s);
    let e = embedding(&g);
    let b = bridges(&g);
    assert!(matches!(
        build_faces_with_bridges2(&e, &b, limits()),
        Err(FaceBuildError2::UnsupportedNonSimpleBoundary { .. })
    ));
}

#[test]
fn mismatched_graph_and_resource_failures_are_atomic() {
    let s = square(0., 0., 4.);
    let g = graph(&s);
    let other = graph(&s);
    let e = embedding(&g);
    let b = bridges(&g);
    let other_b = bridges(&other);
    assert!(matches!(
        build_faces_with_bridges2(&e, &other_b, limits()),
        Err(FaceBuildError2::DifferentArrangement)
    ));
    let baseline = build_faces_with_bridges2(&e, &b, limits()).unwrap();
    for (resource, lim) in [
        (
            FaceBuildResource2::HalfEdges,
            FaceBuildLimits2 {
                max_half_edges: 0,
                ..limits()
            },
        ),
        (
            FaceBuildResource2::Walks,
            FaceBuildLimits2 {
                max_walks: 0,
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
            matches!(build_faces_with_bridges2(&e,&b,lim),Err(FaceBuildError2::LimitExceeded(actual)) if actual==resource)
        );
        assert_eq!(baseline.faces().len(), 2);
    }
}

#[test]
fn generated_spur_positions_obey_euler_and_bridge_ownership() {
    for i in 0..32 {
        let size = 10 + (i / 8);
        let inside_y = 1 + (i % 8);
        let outside_y = 1 + ((i + 3) % 8);
        let mut s = square(0., 0., size as f64);
        s.push(raw((0., inside_y as f64), (2., inside_y as f64)));
        s.push(raw(
            (size as f64, outside_y as f64),
            ((size + 2) as f64, outside_y as f64),
        ));
        let g = graph(&s);
        let e = embedding(&g);
        let b = bridges(&g);
        let f = build_faces_with_bridges2(&e, &b, limits()).unwrap();
        assert_euler(&g, &b, &f);
        assert_eq!(f.faces().len(), 2);
        assert_eq!(owner_of_halfedge(&f, source_edge(&g, 4) * 2), 1);
        assert_eq!(owner_of_halfedge(&f, source_edge(&g, 5) * 2), 0);
    }
}

#[test]
fn no_bridge_path_matches_existing_simple_face_grouping() {
    for offset in 0..12 {
        let mut s = square(offset as f64, 0., 8.);
        s.extend(square((offset + 2) as f64, 2., 2.));
        let g = graph(&s);
        let e = embedding(&g);
        let b = bridges(&g);
        assert!((0..g.edges().len()).all(|edge| b.is_bridge(edge) == Some(false)));
        let original = build_faces2(&e, limits()).unwrap();
        let extended = build_faces_with_bridges2(&e, &b, limits()).unwrap();
        assert_eq!(original.faces(), extended.faces());
        for walk in 0..e.boundary_walk_count() {
            assert_eq!(original.face_of_walk(walk), extended.face_of_walk(walk));
        }
    }
}

#[test]
fn extreme_and_subnormal_interior_spurs_preserve_face_ownership() {
    let tiny = f64::from_bits(1);
    for (lo, hi, mid, tip) in [
        (0., tiny * 4., tiny * 2., tiny),
        (-f64::MAX, f64::MAX, 0., 1.),
    ] {
        let corners = [(lo, lo), (hi, lo), (hi, hi), (lo, hi)];
        let mut s: Vec<_> = (0..4)
            .map(|i| raw(corners[i], corners[(i + 1) % 4]))
            .collect();
        s.push(raw((lo, mid), (tip, mid)));
        let g = graph(&s);
        let e = embedding(&g);
        let b = bridges(&g);
        let f = build_faces_with_bridges2(&e, &b, limits()).unwrap();
        assert_eq!(f.faces().len(), 2);
        assert_euler(&g, &b, &f);
        assert_eq!(owner_of_halfedge(&f, source_edge(&g, 4) * 2), 1);
    }
}
