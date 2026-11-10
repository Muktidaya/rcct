// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    Arrangement2, ArrangementLimits2, BooleanOp2, BooleanRegion2, BooleanRegionError2,
    BooleanRegionLimits2, BooleanRegionResource2, EmbeddingLimits2, FaceBuildError2,
    FaceBuildLimits2, FaceWindingLimits2, FillRule2, GroupWindingError2, GroupWindingLimits2,
    GroupWindingResource2, GroupWindings2, Point2, RegionPointBudget2, RegionPointError2,
    RegionPointLocation2, Segment2, build_arrangement2, build_faces2, embed_arrangement2,
    face_windings2, group_windings2, select_boolean2,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rat {
    n: i128,
    d: i128,
}
impl Rat {
    fn new(mut n: i128, mut d: i128) -> Self {
        if d < 0 {
            n = -n;
            d = -d;
        }
        let (mut a, mut b) = (n.abs(), d);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        Self { n: n / a, d: d / a }
    }
    fn add(self, b: Self) -> Self {
        Self::new(self.n * b.d + b.n * self.d, self.d * b.d)
    }
    fn sub(self, b: Self) -> Self {
        Self::new(self.n * b.d - b.n * self.d, self.d * b.d)
    }
    fn mul(self, b: Self) -> Self {
        Self::new(self.n * b.n, self.d * b.d)
    }
    fn cmp(self, b: Self) -> std::cmp::Ordering {
        (self.n * b.d).cmp(&(b.n * self.d))
    }
    fn from_f64(x: f64) -> Self {
        if x == 0.0 {
            return Self::new(0, 1);
        }
        let bits = x.to_bits();
        let exp = ((bits >> 52) & 0x7ff) as i32;
        assert!(exp > 0 && exp < 2047);
        let mantissa = i128::from((bits & ((1_u64 << 52) - 1)) | (1_u64 << 52));
        let signed = if bits >> 63 == 0 { mantissa } else { -mantissa };
        let shift = exp - 1023 - 52;
        if shift >= 0 {
            Self::new(signed << shift, 1)
        } else {
            Self::new(signed, 1_i128 << -shift)
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}
impl Rect {
    fn segments(self, reverse: bool) -> Vec<Segment2> {
        let p = [
            (self.x0, self.y0),
            (self.x1, self.y0),
            (self.x1, self.y1),
            (self.x0, self.y1),
        ];
        (0..4)
            .map(|i| {
                let (a, b) = (p[i], p[(i + 1) % 4]);
                let (a, b) = if reverse { (b, a) } else { (a, b) };
                Segment2::try_new(
                    Point2::try_new(a.0 as f64, a.1 as f64).unwrap(),
                    Point2::try_new(b.0 as f64, b.1 as f64).unwrap(),
                )
                .unwrap()
            })
            .collect()
    }
    // Independent exact quarter-unit point classifier; test points never lie
    // on a rectangle boundary.
    fn inside4(self, x: i64, y: i64) -> bool {
        self.x0 * 4 < x && x < self.x1 * 4 && self.y0 * 4 < y && y < self.y1 * 4
    }
    fn area(self) -> i64 {
        (self.x1 - self.x0) * (self.y1 - self.y0)
    }
}
fn graph(s: &[Segment2]) -> Arrangement2 {
    build_arrangement2(
        s,
        ArrangementLimits2 {
            max_segments: 16,
            max_pairs: 120,
            max_cuts: 500,
            max_vertices: 200,
            max_edges: 400,
            max_edge_uses: 500,
            max_comparisons: 2_000_000,
        },
    )
    .unwrap()
}
fn embedding_limits() -> EmbeddingLimits2 {
    EmbeddingLimits2 {
        max_vertices: 200,
        max_half_edges: 800,
        max_edge_uses: 500,
        max_comparisons: 2_000_000,
        max_boundary_walks: 800,
        max_walk_steps: 800,
    }
}
fn face_limits() -> FaceBuildLimits2 {
    FaceBuildLimits2 {
        max_half_edges: 800,
        max_walks: 800,
        max_faces: 801,
        max_orientation_comparisons: 2_000_000,
        max_containment_queries: 100_000,
        max_containment_edge_checks: 2_000_000,
    }
}
fn group_limits() -> GroupWindingLimits2 {
    GroupWindingLimits2 {
        max_sources: 16,
        max_groups: 4,
        max_faces: 801,
        max_half_edges: 800,
        max_edge_uses: 500,
        max_winding_cells: 10_000,
        max_propagation_checks: 2_000_000,
    }
}
fn region_limits() -> BooleanRegionLimits2 {
    BooleanRegionLimits2 {
        max_faces: 801,
        max_vertices: 200,
        max_half_edges: 800,
        max_boundary_edges: 800,
        max_boundary_walks: 800,
        max_walk_steps: 800,
        max_rotation_checks: 2_000_000,
    }
}
fn evaluate(op: BooleanOp2, a: bool, b: bool) -> bool {
    match op {
        BooleanOp2::Union => a || b,
        BooleanOp2::Intersection => a && b,
        BooleanOp2::Difference => a && !b,
        BooleanOp2::SymmetricDifference => a != b,
    }
}
fn select<'f, 'e, 'g>(
    w: &GroupWindings2<'f, 'e, 'g>,
    a: usize,
    b: usize,
    op: BooleanOp2,
) -> BooleanRegion2<'f, 'e, 'g> {
    select_boolean2(
        w,
        a,
        FillRule2::NonZero,
        b,
        FillRule2::NonZero,
        op,
        region_limits(),
    )
    .unwrap()
}
fn point(g: &Arrangement2, vertex: usize) -> (i64, i64) {
    let p = g.vertices()[vertex].point().point_enclosure();
    assert_eq!(p.x().lower(), p.x().upper());
    assert_eq!(p.y().lower(), p.y().upper());
    (p.x().lower() as i64, p.y().lower() as i64)
}
fn boundaries(region: &BooleanRegion2<'_, '_, '_>) -> BTreeSet<usize> {
    let embedding = region.faces().embedding();
    let h = embedding.half_edges();
    let mut result = BTreeSet::new();
    for i in 0..region.boundary_walk_count() {
        let walk = region.boundary_walk(i).unwrap();
        assert!(!walk.is_empty());
        for j in 0..walk.len() {
            assert!(result.insert(walk[j]));
            assert_eq!(
                h[walk[j]].destination(),
                h[walk[(j + 1) % walk.len()]].origin()
            );
        }
    }
    assert!(region.boundary_walk(region.boundary_walk_count()).is_none());
    result
}
fn verify_region(region: &BooleanRegion2<'_, '_, '_>, a: Rect, b: Rect, op: BooleanOp2) {
    let embedding = region.faces().embedding();
    let g = embedding.arrangement();
    let mut expected = BTreeSet::new();
    for (edge, item) in g.edges().iter().enumerate() {
        let [u, v] = item.vertices();
        let (a0, a1) = (point(g, u), point(g, v));
        let mid = (2 * (a0.0 + a1.0), 2 * (a0.1 + a1.1));
        let normal = (-(a1.1 - a0.1).signum(), (a1.0 - a0.0).signum());
        let left = (mid.0 + normal.0, mid.1 + normal.1);
        let right = (mid.0 - normal.0, mid.1 - normal.1);
        let l = evaluate(op, a.inside4(left.0, left.1), b.inside4(left.0, left.1));
        let r = evaluate(op, a.inside4(right.0, right.1), b.inside4(right.0, right.1));
        if l != r {
            expected.insert(2 * edge + usize::from(!l));
        }
    }
    let actual = boundaries(region);
    assert_eq!(actual, expected, "{a:?} {b:?} {op:?}");
    let mut area2 = 0_i64;
    for h in actual {
        let e = embedding.half_edges()[h];
        let p = point(g, e.origin());
        let q = point(g, e.destination());
        area2 += p.0 * q.1 - p.1 * q.0;
    }
    let ix = (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0);
    let iy = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0);
    let intersection = ix * iy;
    let expected_area = match op {
        BooleanOp2::Union => a.area() + b.area() - intersection,
        BooleanOp2::Intersection => intersection,
        BooleanOp2::Difference => a.area() - intersection,
        BooleanOp2::SymmetricDifference => a.area() + b.area() - 2 * intersection,
    };
    assert_eq!(area2, 2 * expected_area, "{a:?} {b:?} {op:?}");
}

fn check_rectangles(a: Rect, b: Rect, reverse_a: bool, reverse_b: bool) -> bool {
    let mut inputs = a.segments(reverse_a);
    inputs.extend(b.segments(reverse_b));
    let g = graph(&inputs);
    let embedding = embed_arrangement2(&g, embedding_limits()).unwrap();
    let faces = match build_faces2(&embedding, face_limits()) {
        Ok(faces) => faces,
        Err(FaceBuildError2::UnsupportedNonSimpleBoundary { .. }) => return false,
        Err(other) => panic!("unexpected face failure {a:?} {b:?}: {other:?}"),
    };
    let groups = [
        Some(0),
        Some(0),
        Some(0),
        Some(0),
        Some(1),
        Some(1),
        Some(1),
        Some(1),
    ];
    let winding = group_windings2(&faces, &groups, 2, group_limits()).unwrap();
    assert_eq!(winding.winding(0, 0), Some(0));
    assert_eq!(winding.winding(0, 1), Some(0));
    let aggregate = face_windings2(
        &faces,
        FaceWindingLimits2 {
            max_faces: 801,
            max_half_edges: 800,
            max_edge_uses: 500,
            max_propagation_checks: 2_000_000,
        },
    )
    .unwrap();
    for face in 0..faces.faces().len() {
        assert_eq!(
            aggregate.winding(face),
            Some(winding.winding(face, 0).unwrap() + winding.winding(face, 1).unwrap())
        );
    }
    for op in [
        BooleanOp2::Union,
        BooleanOp2::Intersection,
        BooleanOp2::Difference,
        BooleanOp2::SymmetricDifference,
    ] {
        let region = select(&winding, 0, 1, op);
        verify_region(&region, a, b, op);
        if op != BooleanOp2::Difference {
            let reversed = select(&winding, 1, 0, op);
            assert_eq!(
                boundaries(&region),
                boundaries(&reversed),
                "commutativity {op:?}"
            );
        }
        if op == BooleanOp2::Difference || op == BooleanOp2::SymmetricDifference {
            assert!(
                boundaries(&select(&winding, 0, 0, op)).is_empty(),
                "A op A {op:?}"
            );
        }
    }
    true
}

#[test]
fn randomized_rectangles_match_independent_boundaries_and_area() {
    let mut state = 0x9b13_c4a5_u64;
    let mut rand = |n: usize| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 32) as usize) % n
    };
    let mut supported = 0;
    let mut rejected = 0;
    for case in 0..160 {
        let mut make = || {
            let x = rand(13) as i64 - 6;
            let y = rand(13) as i64 - 6;
            Rect {
                x0: x,
                y0: y,
                x1: x + 1 + rand(6) as i64,
                y1: y + 1 + rand(6) as i64,
            }
        };
        let (a, b) = (make(), make());
        if check_rectangles(a, b, case % 2 == 0, case % 3 == 0) {
            supported += 1;
        } else {
            rejected += 1;
        }
    }
    assert!(supported >= 130, "only {supported} supported cases");
    assert!(
        rejected > 0,
        "expected at least one typed non-simple rejection"
    );
}

#[test]
fn coincident_overlap_containment_and_tangency() {
    let a = Rect {
        x0: 0,
        y0: 0,
        x1: 4,
        y1: 4,
    };
    for b in [
        a,
        Rect {
            x0: 2,
            y0: 0,
            x1: 6,
            y1: 4,
        },
        Rect {
            x0: 1,
            y0: 1,
            x1: 3,
            y1: 3,
        },
        Rect {
            x0: 4,
            y0: 0,
            x1: 8,
            y1: 4,
        },
        Rect {
            x0: 7,
            y0: 1,
            x1: 9,
            y1: 3,
        },
    ] {
        assert!(check_rectangles(a, b, false, true));
    }
    let point_touch = Rect {
        x0: 4,
        y0: 4,
        x1: 6,
        y1: 6,
    };
    assert!(!check_rectangles(a, point_touch, false, false));
}

#[test]
fn non_dyadic_triangle_boolean_areas_match_fraction_clipping() {
    let point = |x: f64, y: f64| Point2::try_new(x, y).unwrap();
    let triangles = [
        [(0., 0.), (6., 0.), (0., 6.)],
        [(1., -1.), (7., 3.), (1., 4.)],
    ];
    let mut input = Vec::new();
    for triangle in triangles {
        for i in 0..3 {
            let (a, b) = (triangle[i], triangle[(i + 1) % 3]);
            input.push(Segment2::try_new(point(a.0, a.1), point(b.0, b.1)).unwrap());
        }
    }
    let graph = graph(&input);
    let embedding = embed_arrangement2(&graph, embedding_limits()).unwrap();
    let faces = build_faces2(&embedding, face_limits()).unwrap();
    let groups = [Some(0), Some(0), Some(0), Some(1), Some(1), Some(1)];
    let winding = group_windings2(&faces, &groups, 2, group_limits()).unwrap();
    // Vertices and areas generated independently by exact Fraction clipping of
    // the two input triangles; enclosures identify each rational point uniquely.
    let r = Rat::new;
    let mut candidates = Vec::new();
    let mut areas = Vec::new();
    for line in include_str!("fixtures/boolean_triangle.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        match fields[0] {
            "V" => candidates.push((
                r(fields[1].parse().unwrap(), fields[2].parse().unwrap()),
                r(fields[3].parse().unwrap(), fields[4].parse().unwrap()),
            )),
            "A" => {
                let op = match fields[1] {
                    "union" => BooleanOp2::Union,
                    "intersection" => BooleanOp2::Intersection,
                    "difference" => BooleanOp2::Difference,
                    "symmetric_difference" => BooleanOp2::SymmetricDifference,
                    _ => panic!("unknown operation"),
                };
                areas.push((
                    op,
                    r(fields[2].parse().unwrap(), fields[3].parse().unwrap()),
                ));
            }
            _ => panic!("unknown fixture row"),
        }
    }
    assert_eq!((candidates.len(), areas.len()), (10, 4));
    let mut vertices = Vec::new();
    for vertex in graph.vertices() {
        let p = vertex.point().point_enclosure();
        let contains = |value: Rat, lo: f64, hi: f64| {
            value.cmp(Rat::from_f64(lo)) != std::cmp::Ordering::Less
                && value.cmp(Rat::from_f64(hi)) != std::cmp::Ordering::Greater
        };
        let matches: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|&(x, y)| {
                contains(x, p.x().lower(), p.x().upper())
                    && contains(y, p.y().lower(), p.y().upper())
            })
            .collect();
        assert_eq!(matches.len(), 1);
        vertices.push(matches[0]);
    }
    for (op, expected_area2) in areas {
        let region = select(&winding, 0, 1, op);
        let mut area2 = r(0, 1);
        for h in boundaries(&region) {
            let edge = embedding.half_edges()[h];
            let (a, b) = (vertices[edge.origin()], vertices[edge.destination()]);
            area2 = area2.add(a.0.mul(b.1).sub(a.1.mul(b.0)));
        }
        assert_eq!(area2, expected_area2, "{op:?}");
    }
}

#[test]
fn evenodd_and_nonzero_fill_differ_on_double_winding() {
    let a = Rect {
        x0: 0,
        y0: 0,
        x1: 4,
        y1: 4,
    };
    let mut s = a.segments(false);
    s.extend(a.segments(false));
    let g = graph(&s);
    let e = embed_arrangement2(&g, embedding_limits()).unwrap();
    let f = build_faces2(&e, face_limits()).unwrap();
    let groups = vec![Some(0); 8];
    let w = group_windings2(&f, &groups, 2, group_limits()).unwrap();
    assert_eq!(w.winding(1, 0), Some(2));
    assert_eq!(w.winding(1, 1), Some(0));
    let nonzero = select_boolean2(
        &w,
        0,
        FillRule2::NonZero,
        1,
        FillRule2::NonZero,
        BooleanOp2::Union,
        region_limits(),
    )
    .unwrap();
    let parity = select_boolean2(
        &w,
        0,
        FillRule2::EvenOdd,
        1,
        FillRule2::NonZero,
        BooleanOp2::Union,
        region_limits(),
    )
    .unwrap();
    assert_eq!(boundaries(&nonzero).len(), 4);
    assert!(boundaries(&parity).is_empty());
}

#[test]
fn ignored_open_chord_does_not_appear_in_selected_boundary() {
    let a = Rect {
        x0: 0,
        y0: 0,
        x1: 4,
        y1: 4,
    };
    let mut s = a.segments(false);
    s.push(
        Segment2::try_new(
            Point2::try_new(0., 0.).unwrap(),
            Point2::try_new(4., 4.).unwrap(),
        )
        .unwrap(),
    );
    let g = graph(&s);
    let e = embed_arrangement2(&g, embedding_limits()).unwrap();
    let faces = build_faces2(&e, face_limits()).unwrap();
    let groups = [Some(0), Some(0), Some(0), Some(0), None];
    let winding = group_windings2(&faces, &groups, 2, group_limits()).unwrap();
    let region = select(&winding, 0, 1, BooleanOp2::Union);
    let boundary = boundaries(&region);
    assert_eq!(boundary.len(), 4);
    let chord = g
        .edge_uses()
        .iter()
        .find(|item| item.source() == 4)
        .unwrap()
        .edge();
    assert!(!boundary.contains(&(2 * chord)) && !boundary.contains(&(2 * chord + 1)));
    let mut budget = RegionPointBudget2::new(region.boundary_edge_count());
    assert_eq!(
        region.locate_point(Point2::try_new(2., 2.).unwrap(), &mut budget),
        Ok(RegionPointLocation2::Inside)
    );
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn region_point_queries_match_independent_rectangle_neighborhoods() {
    let a = Rect {
        x0: 0,
        y0: 0,
        x1: 4,
        y1: 4,
    };
    let b = Rect {
        x0: 2,
        y0: -1,
        x1: 6,
        y1: 3,
    };
    let mut inputs = a.segments(false);
    inputs.extend(b.segments(true));
    let graph = graph(&inputs);
    let embedding = embed_arrangement2(&graph, embedding_limits()).unwrap();
    let faces = build_faces2(&embedding, face_limits()).unwrap();
    let groups = [
        Some(0),
        Some(0),
        Some(0),
        Some(0),
        Some(1),
        Some(1),
        Some(1),
        Some(1),
    ];
    let winding = group_windings2(&faces, &groups, 2, group_limits()).unwrap();
    let in8 =
        |r: Rect, x: i64, y: i64| r.x0 * 8 < x && x < r.x1 * 8 && r.y0 * 8 < y && y < r.y1 * 8;
    for op in [
        BooleanOp2::Union,
        BooleanOp2::Intersection,
        BooleanOp2::Difference,
        BooleanOp2::SymmetricDifference,
    ] {
        let region = select(&winding, 0, 1, op);
        let cost = region.boundary_edge_count();
        let mut budget = RegionPointBudget2::new(usize::MAX);
        for xq in (-4..=28).step_by(2) {
            for yq in (-8..=20).step_by(2) {
                let mut inside = 0;
                for (dx, dy) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
                    let p = (2 * xq + dx, 2 * yq + dy);
                    if evaluate(op, in8(a, p.0, p.1), in8(b, p.0, p.1)) {
                        inside += 1;
                    }
                }
                let expected = match inside {
                    0 => RegionPointLocation2::Outside,
                    4 => RegionPointLocation2::Inside,
                    _ => RegionPointLocation2::Boundary,
                };
                let point = Point2::try_new(xq as f64 / 4., yq as f64 / 4.).unwrap();
                assert_eq!(
                    region.locate_point(point, &mut budget),
                    Ok(expected),
                    "{op:?} point {xq}/4,{yq}/4"
                );
            }
        }
        let before = budget.remaining();
        assert_eq!(before, usize::MAX - cost * 255);
        if cost > 0 {
            let mut insufficient = RegionPointBudget2::new(cost - 1);
            assert_eq!(
                region.locate_point(Point2::try_new(0., 0.).unwrap(), &mut insufficient),
                Err(RegionPointError2::WorkLimitExceeded)
            );
            assert_eq!(insufficient.remaining(), cost - 1);
            let mut exact = RegionPointBudget2::new(cost);
            assert!(
                region
                    .locate_point(Point2::try_new(0., 0.).unwrap(), &mut exact)
                    .is_ok()
            );
            assert_eq!(exact.remaining(), 0);
        }
        assert_eq!(budget.remaining(), before);
    }
}

#[test]
fn region_point_queries_respect_extreme_and_subnormal_boundaries() {
    let tiny = f64::from_bits(1);
    for (lo, hi, inside, boundary, outside) in [
        (0., tiny * 4., (tiny, tiny), (0., tiny), Some((-tiny, tiny))),
        (-f64::MAX, f64::MAX, (0., 0.), (f64::MAX, 0.), None),
    ] {
        let p = |x: f64, y: f64| Point2::try_new(x, y).unwrap();
        let vertices = [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi)];
        let inputs: Vec<_> = (0..4)
            .map(|i| Segment2::try_new(vertices[i], vertices[(i + 1) % 4]).unwrap())
            .collect();
        let g = graph(&inputs);
        let e = embed_arrangement2(&g, embedding_limits()).unwrap();
        let f = build_faces2(&e, face_limits()).unwrap();
        let groups = [Some(0), Some(0), Some(0), Some(0)];
        let w = group_windings2(&f, &groups, 2, group_limits()).unwrap();
        let region = select(&w, 0, 1, BooleanOp2::Union);
        let mut budget = RegionPointBudget2::new(region.boundary_edge_count() * 3);
        assert_eq!(
            region.locate_point(p(inside.0, inside.1), &mut budget),
            Ok(RegionPointLocation2::Inside)
        );
        assert_eq!(
            region.locate_point(p(boundary.0, boundary.1), &mut budget),
            Ok(RegionPointLocation2::Boundary)
        );
        if let Some((x, y)) = outside {
            assert_eq!(
                region.locate_point(p(x, y), &mut budget),
                Ok(RegionPointLocation2::Outside)
            );
        }
    }
}

#[test]
fn region_point_query_handles_non_dyadic_corners_exactly() {
    let point = |x: f64, y: f64| Point2::try_new(x, y).unwrap();
    let triangles = [
        [(0., 0.), (6., 0.), (0., 6.)],
        [(1., -1.), (7., 3.), (1., 4.)],
    ];
    let mut inputs = Vec::new();
    for triangle in triangles {
        for i in 0..3 {
            let (a, b) = (triangle[i], triangle[(i + 1) % 3]);
            inputs.push(Segment2::try_new(point(a.0, a.1), point(b.0, b.1)).unwrap());
        }
    }
    let graph = graph(&inputs);
    let embedding = embed_arrangement2(&graph, embedding_limits()).unwrap();
    let faces = build_faces2(&embedding, face_limits()).unwrap();
    let groups = [Some(0), Some(0), Some(0), Some(1), Some(1), Some(1)];
    let winding = group_windings2(&faces, &groups, 2, group_limits()).unwrap();
    let region = select(&winding, 0, 1, BooleanOp2::Intersection);
    let mut budget = RegionPointBudget2::new(region.boundary_edge_count() * 8);
    let r = Rat::new;
    for (x, y) in [
        (3., 1.),
        (3., 3.),
        (5., 0.),
        (4.6, 1.4),
        (3., f64::from_bits((3.0_f64).to_bits() + 1)),
        (1., 0.),
        (1., 4.),
        (2., 0.5),
    ] {
        let (xq, yq) = (Rat::from_f64(x), Rat::from_f64(y));
        let signs = [
            xq,
            yq,
            r(6, 1).sub(xq).sub(yq),
            r(6, 1)
                .mul(yq.add(r(1, 1)))
                .sub(r(4, 1).mul(xq.sub(r(1, 1)))),
            r(25, 1).sub(xq).sub(r(6, 1).mul(yq)),
            xq.sub(r(1, 1)),
        ];
        let expected = if signs
            .iter()
            .any(|v| v.cmp(r(0, 1)) == std::cmp::Ordering::Less)
        {
            RegionPointLocation2::Outside
        } else if signs
            .iter()
            .any(|v| v.cmp(r(0, 1)) == std::cmp::Ordering::Equal)
        {
            RegionPointLocation2::Boundary
        } else {
            RegionPointLocation2::Inside
        };
        assert_eq!(
            region.locate_point(point(x, y), &mut budget),
            Ok(expected),
            "point {x:?},{y:?}"
        );
    }
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn invalid_groups_open_chains_and_resource_failures_are_atomic() {
    let a = Rect {
        x0: 0,
        y0: 0,
        x1: 4,
        y1: 4,
    };
    let b = Rect {
        x0: 2,
        y0: -1,
        x1: 6,
        y1: 3,
    };
    let mut s = a.segments(false);
    s.extend(b.segments(false));
    let g = graph(&s);
    let e = embed_arrangement2(&g, embedding_limits()).unwrap();
    let f = build_faces2(&e, face_limits()).unwrap();
    let groups = [
        Some(0),
        Some(0),
        Some(0),
        Some(0),
        Some(1),
        Some(1),
        Some(1),
        Some(1),
    ];
    let baseline = group_windings2(&f, &groups, 2, group_limits()).unwrap();
    assert!(matches!(
        group_windings2(&f, &groups[..7], 2, group_limits()),
        Err(GroupWindingError2::SourceCountMismatch)
    ));
    let mut bad = groups;
    bad[0] = Some(2);
    assert!(matches!(
        group_windings2(&f, &bad, 2, group_limits()),
        Err(GroupWindingError2::InvalidGroup { source: 0 })
    ));
    bad = groups;
    bad[0] = None;
    assert!(matches!(
        group_windings2(&f, &bad, 2, group_limits()),
        Err(GroupWindingError2::InconsistentGroup { group: 0 })
    ));
    let split = [
        Some(0),
        Some(1),
        Some(0),
        Some(1),
        Some(0),
        Some(1),
        Some(0),
        Some(1),
    ];
    assert!(matches!(
        group_windings2(&f, &split, 2, group_limits()),
        Err(GroupWindingError2::InconsistentGroup { .. })
    ));
    for (expected, lim) in [
        (
            GroupWindingResource2::Sources,
            GroupWindingLimits2 {
                max_sources: 7,
                ..group_limits()
            },
        ),
        (
            GroupWindingResource2::Groups,
            GroupWindingLimits2 {
                max_groups: 1,
                ..group_limits()
            },
        ),
        (
            GroupWindingResource2::Faces,
            GroupWindingLimits2 {
                max_faces: 0,
                ..group_limits()
            },
        ),
        (
            GroupWindingResource2::HalfEdges,
            GroupWindingLimits2 {
                max_half_edges: 0,
                ..group_limits()
            },
        ),
        (
            GroupWindingResource2::EdgeUses,
            GroupWindingLimits2 {
                max_edge_uses: 0,
                ..group_limits()
            },
        ),
        (
            GroupWindingResource2::WindingCells,
            GroupWindingLimits2 {
                max_winding_cells: 0,
                ..group_limits()
            },
        ),
        (
            GroupWindingResource2::PropagationChecks,
            GroupWindingLimits2 {
                max_propagation_checks: 0,
                ..group_limits()
            },
        ),
    ] {
        assert!(
            matches!(group_windings2(&f,&groups,2,lim),Err(GroupWindingError2::LimitExceeded(actual)) if actual==expected)
        );
        assert_eq!(baseline.winding(0, 0), Some(0));
    }
    let original = select(&baseline, 0, 1, BooleanOp2::Union);
    let expected = boundaries(&original);
    assert!(matches!(
        select_boolean2(
            &baseline,
            0,
            FillRule2::NonZero,
            2,
            FillRule2::NonZero,
            BooleanOp2::Union,
            region_limits()
        ),
        Err(BooleanRegionError2::InvalidGroup)
    ));
    for (resource, lim) in [
        (
            BooleanRegionResource2::Faces,
            BooleanRegionLimits2 {
                max_faces: 0,
                ..region_limits()
            },
        ),
        (
            BooleanRegionResource2::Vertices,
            BooleanRegionLimits2 {
                max_vertices: 0,
                ..region_limits()
            },
        ),
        (
            BooleanRegionResource2::HalfEdges,
            BooleanRegionLimits2 {
                max_half_edges: 0,
                ..region_limits()
            },
        ),
        (
            BooleanRegionResource2::BoundaryEdges,
            BooleanRegionLimits2 {
                max_boundary_edges: 0,
                ..region_limits()
            },
        ),
        (
            BooleanRegionResource2::WalkSteps,
            BooleanRegionLimits2 {
                max_walk_steps: 0,
                ..region_limits()
            },
        ),
        (
            BooleanRegionResource2::RotationChecks,
            BooleanRegionLimits2 {
                max_rotation_checks: 0,
                ..region_limits()
            },
        ),
        (
            BooleanRegionResource2::BoundaryWalks,
            BooleanRegionLimits2 {
                max_boundary_walks: 0,
                ..region_limits()
            },
        ),
    ] {
        assert!(
            matches!(select_boolean2(&baseline,0,FillRule2::NonZero,1,FillRule2::NonZero,BooleanOp2::Union,lim),Err(BooleanRegionError2::LimitExceeded(actual)) if actual==resource)
        );
        assert_eq!(boundaries(&original), expected);
    }
}

#[test]
fn empty_graph_produces_empty_region_at_exact_limits() {
    let g = graph(&[]);
    let e = embed_arrangement2(&g, embedding_limits()).unwrap();
    let f = build_faces2(&e, face_limits()).unwrap();
    let group_limits = GroupWindingLimits2 {
        max_sources: 0,
        max_groups: 2,
        max_faces: 1,
        max_half_edges: 0,
        max_edge_uses: 0,
        max_winding_cells: 2,
        max_propagation_checks: 0,
    };
    let w = group_windings2(&f, &[], 2, group_limits).unwrap();
    assert_eq!(w.winding(0, 0), Some(0));
    assert_eq!(w.winding(0, 1), Some(0));
    assert!(matches!(
        group_windings2(&f, &[], 0, group_limits),
        Err(GroupWindingError2::EmptyGroups)
    ));
    let region_limits = BooleanRegionLimits2 {
        max_faces: 1,
        max_vertices: 0,
        max_half_edges: 0,
        max_boundary_edges: 0,
        max_boundary_walks: 0,
        max_walk_steps: 0,
        max_rotation_checks: 0,
    };
    let region = select_boolean2(
        &w,
        0,
        FillRule2::NonZero,
        1,
        FillRule2::EvenOdd,
        BooleanOp2::Union,
        region_limits,
    )
    .unwrap();
    assert_eq!(region.contains_face(0), Some(false));
    assert_eq!(region.boundary_walk_count(), 0);
    let mut budget = RegionPointBudget2::new(0);
    assert_eq!(
        region.locate_point(Point2::try_new(0., 0.).unwrap(), &mut budget),
        Ok(RegionPointLocation2::Outside)
    );
    assert_eq!(budget.remaining(), 0);
}
