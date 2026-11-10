// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    ArrangementError2, ArrangementLimits2, ArrangementResource2, Point2, Segment2,
    build_arrangement2,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rat {
    n: i128,
    d: i128,
}
fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
impl Rat {
    fn new(mut n: i128, mut d: i128) -> Self {
        assert_ne!(d, 0);
        if d < 0 {
            n = -n;
            d = -d;
        }
        let g = gcd(n, d);
        Self { n: n / g, d: d / g }
    }
    fn integer(x: i64) -> Self {
        Self::new(i128::from(x), 1)
    }
    fn from_f64(x: f64) -> Self {
        if x == 0.0 {
            return Self::new(0, 1);
        }
        let bits = x.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as i32;
        assert!(
            exponent > 0 && exponent < 2047,
            "small oracle expects finite normal bounds"
        );
        let mantissa = i128::from((bits & ((1_u64 << 52) - 1)) | (1_u64 << 52));
        let signed = if bits >> 63 == 0 { mantissa } else { -mantissa };
        let shift = exponent - 1023 - 52;
        if shift >= 0 {
            Self::new(signed << shift, 1)
        } else {
            Self::new(signed, 1_i128 << -shift)
        }
    }
}
impl Ord for Rat {
    fn cmp(&self, b: &Self) -> Ordering {
        (self.n * b.d).cmp(&(b.n * self.d))
    }
}
impl PartialOrd for Rat {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Add for Rat {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.n * b.d + b.n * self.d, self.d * b.d)
    }
}
impl Sub for Rat {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.n * b.d - b.n * self.d, self.d * b.d)
    }
}
impl Mul for Rat {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self::new(self.n * b.n, self.d * b.d)
    }
}
impl Div for Rat {
    type Output = Self;
    fn div(self, b: Self) -> Self {
        Self::new(self.n * b.d, self.d * b.n)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Pt {
    x: Rat,
    y: Rat,
}
impl Pt {
    fn new(x: i64, y: i64) -> Self {
        Self {
            x: Rat::integer(x),
            y: Rat::integer(y),
        }
    }
    fn add(self, b: Self) -> Self {
        Self {
            x: self.x + b.x,
            y: self.y + b.y,
        }
    }
    fn sub(self, b: Self) -> Self {
        Self {
            x: self.x - b.x,
            y: self.y - b.y,
        }
    }
    fn scale(self, t: Rat) -> Self {
        Self {
            x: self.x * t,
            y: self.y * t,
        }
    }
    fn cross(self, b: Self) -> Rat {
        self.x * b.y - self.y * b.x
    }
}
type Segment = (Pt, Pt);
type Use = (usize, Pt, Pt);
type Edge = (Pt, Pt);
fn unit(t: Rat) -> bool {
    Rat::integer(0) <= t && t <= Rat::integer(1)
}
fn param(p: Pt, a: Pt, b: Pt) -> Rat {
    if a.x != b.x {
        (p.x - a.x) / (b.x - a.x)
    } else {
        (p.y - a.y) / (b.y - a.y)
    }
}
fn oracle(segments: &[Segment]) -> (BTreeSet<Pt>, BTreeSet<Edge>, BTreeSet<Use>) {
    let zero = Rat::integer(0);
    let one = Rat::integer(1);
    let mut cuts: Vec<BTreeSet<Rat>> = (0..segments.len())
        .map(|_| BTreeSet::from([zero, one]))
        .collect();
    for i in 0..segments.len() {
        for j in i + 1..segments.len() {
            let (a, b) = segments[i];
            let (c, d) = segments[j];
            let r = b.sub(a);
            let s = d.sub(c);
            let q = c.sub(a);
            let denominator = r.cross(s);
            if denominator != zero {
                let t = q.cross(s) / denominator;
                let u = q.cross(r) / denominator;
                if unit(t) && unit(u) {
                    cuts[i].insert(t);
                    cuts[j].insert(u);
                }
            } else if q.cross(r) == zero {
                let t0 = param(c, a, b);
                let t1 = param(d, a, b);
                let low = zero.max(t0.min(t1));
                let high = one.min(t0.max(t1));
                if low <= high {
                    for t in [low, high] {
                        let point = a.add(r.scale(t));
                        let u = param(point, c, d);
                        cuts[i].insert(t);
                        cuts[j].insert(u);
                    }
                }
            }
        }
    }
    let mut vertices = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut uses = BTreeSet::new();
    for (source, ((a, b), values)) in segments.iter().copied().zip(cuts).enumerate() {
        let mut prior: Option<Pt> = None;
        for t in values {
            let point = a.add(b.sub(a).scale(t));
            vertices.insert(point);
            if let Some(before) = prior {
                edges.insert((before.min(point), before.max(point)));
                uses.insert((source, before, point));
            }
            prior = Some(point);
        }
    }
    (vertices, edges, uses)
}

fn input(seg: Segment) -> Segment2 {
    let p = |v: Pt| Point2::try_new(v.x.n as f64, v.y.n as f64).unwrap();
    Segment2::try_new(p(seg.0), p(seg.1)).unwrap()
}
fn limits() -> ArrangementLimits2 {
    ArrangementLimits2 {
        max_segments: 8,
        max_pairs: 28,
        max_cuts: 128,
        max_vertices: 64,
        max_edges: 128,
        max_edge_uses: 128,
        max_comparisons: 100_000,
    }
}
fn check_case(segments: &[Segment]) {
    let (vertices, edges, uses) = oracle(segments);
    let inputs: Vec<_> = segments.iter().copied().map(input).collect();
    let graph = build_arrangement2(&inputs, limits()).unwrap();
    assert_eq!(graph.sources(), inputs.as_slice());
    assert_eq!(graph.vertices().len(), vertices.len());
    let mut translated = Vec::new();
    for vertex in graph.vertices() {
        let enclosure = vertex.point().point_enclosure();
        let within = |v: Rat, lo: f64, hi: f64| Rat::from_f64(lo) <= v && v <= Rat::from_f64(hi);
        let matches: Vec<_> = vertices
            .iter()
            .copied()
            .filter(|p| {
                within(p.x, enclosure.x().lower(), enclosure.x().upper())
                    && within(p.y, enclosure.y().lower(), enclosure.y().upper())
            })
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "independent exact vertex should be uniquely enclosed: {matches:?}"
        );
        translated.push(matches[0]);
    }
    assert_eq!(
        translated.iter().copied().collect::<BTreeSet<_>>(),
        vertices
    );
    let mut actual_edges = BTreeSet::new();
    for edge in graph.edges() {
        let [i, j] = edge.vertices();
        assert!(i < j);
        let (a, b) = (translated[i], translated[j]);
        assert!(actual_edges.insert((a.min(b), a.max(b))), "duplicate edge");
    }
    assert_eq!(actual_edges, edges);
    let mut actual_uses = BTreeSet::new();
    for edge_use in graph.edge_uses() {
        let [i, j] = graph.edges()[edge_use.edge()].vertices();
        let (a, b) = if edge_use.forward() {
            (translated[i], translated[j])
        } else {
            (translated[j], translated[i])
        };
        assert!(
            actual_uses.insert((edge_use.source(), a, b)),
            "duplicate use"
        );
    }
    assert_eq!(actual_uses, uses);
    let mut per_source = BTreeMap::<usize, Vec<(Pt, Pt)>>::new();
    for edge_use in graph.edge_uses() {
        let [i, j] = graph.edges()[edge_use.edge()].vertices();
        let (a, b) = if edge_use.forward() {
            (translated[i], translated[j])
        } else {
            (translated[j], translated[i])
        };
        per_source
            .entry(edge_use.source())
            .or_default()
            .push((a, b));
    }
    for (index, pieces) in per_source {
        assert_eq!(pieces.first().unwrap().0, segments[index].0);
        assert_eq!(pieces.last().unwrap().1, segments[index].1);
        for pair in pieces.windows(2) {
            assert_eq!(pair[0].1, pair[1].0);
        }
    }
}

#[test]
fn randomized_small_arrangements_match_independent_rational_graph() {
    let mut state = 0x4f9c_b752_u64;
    let mut rand = |n: usize| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 32) as usize) % n
    };
    for _case in 0..400 {
        let count = 1 + rand(6);
        let mut segments = Vec::new();
        for index in 0..count {
            if index > 0 && rand(8) == 0 {
                let (a, b) = segments[rand(index)];
                segments.push((b, a));
                continue;
            }
            let a = Pt::new(rand(9) as i64 - 4, rand(9) as i64 - 4);
            let mut b = Pt::new(rand(9) as i64 - 4, rand(9) as i64 - 4);
            if a == b {
                b.x = b.x + Rat::integer(1);
            }
            segments.push((a, b));
        }
        check_case(&segments);
    }
}

#[test]
fn overlaps_t_junctions_and_shared_rational_crossings() {
    let p = Pt::new;
    check_case(&[
        (p(0, 0), p(6, 0)),
        (p(5, 0), p(1, 0)),
        (p(3, -3), p(3, 3)),
        (p(3, 0), p(4, 2)),
        (p(6, 0), p(0, 0)),
    ]);
    check_case(&[
        (p(0, 0), p(3, 0)),
        (p(0, 1), p(1, -2)),
        (p(0, -1), p(1, 2)),
        (p(1, 0), p(0, 0)),
    ]);
    check_case(&[]);
}

#[test]
fn every_resource_limit_fails_without_a_partial_result() {
    let p = Pt::new;
    let inputs = [input((p(0, 0), p(2, 0))), input((p(1, -1), p(1, 1)))];
    let complete = build_arrangement2(&inputs, limits()).unwrap();
    assert_eq!(
        (
            complete.vertices().len(),
            complete.edges().len(),
            complete.edge_uses().len()
        ),
        (5, 4, 4)
    );
    for (resource, test) in [
        (
            ArrangementResource2::Segments,
            ArrangementLimits2 {
                max_segments: 1,
                ..limits()
            },
        ),
        (
            ArrangementResource2::Pairs,
            ArrangementLimits2 {
                max_pairs: 0,
                ..limits()
            },
        ),
        (
            ArrangementResource2::Cuts,
            ArrangementLimits2 {
                max_cuts: 3,
                ..limits()
            },
        ),
        (
            ArrangementResource2::Cuts,
            ArrangementLimits2 {
                max_cuts: 5,
                ..limits()
            },
        ),
        (
            ArrangementResource2::Vertices,
            ArrangementLimits2 {
                max_vertices: 0,
                ..limits()
            },
        ),
        (
            ArrangementResource2::Edges,
            ArrangementLimits2 {
                max_edges: 0,
                ..limits()
            },
        ),
        (
            ArrangementResource2::EdgeUses,
            ArrangementLimits2 {
                max_edge_uses: 0,
                ..limits()
            },
        ),
        (
            ArrangementResource2::Comparisons,
            ArrangementLimits2 {
                max_comparisons: 0,
                ..limits()
            },
        ),
    ] {
        assert!(
            matches!(build_arrangement2(&inputs,test),Err(ArrangementError2::LimitExceeded(actual)) if actual==resource)
        );
        assert_eq!(complete.sources(), &inputs);
        assert_eq!(
            (
                complete.vertices().len(),
                complete.edges().len(),
                complete.edge_uses().len()
            ),
            (5, 4, 4)
        );
    }
    let exact = ArrangementLimits2 {
        max_segments: 2,
        max_pairs: 1,
        max_cuts: 6,
        max_vertices: 5,
        max_edges: 4,
        max_edge_uses: 4,
        max_comparisons: 10_000,
    };
    assert!(build_arrangement2(&inputs, exact).is_ok());
}

#[test]
fn extreme_crossings_and_coincident_reversal() {
    let raw = |a: (f64, f64), b: (f64, f64)| {
        Segment2::try_new(
            Point2::try_new(a.0, a.1).unwrap(),
            Point2::try_new(b.0, b.1).unwrap(),
        )
        .unwrap()
    };
    let first = raw((-f64::MAX, 0.), (f64::MAX, 0.));
    let reverse = raw((f64::MAX, 0.), (-f64::MAX, 0.));
    let min = f64::from_bits(1);
    let cutter = raw((min, -1.), (min, 1.));
    let graph = build_arrangement2(&[first, reverse, cutter], limits()).unwrap();
    assert_eq!(
        (
            graph.vertices().len(),
            graph.edges().len(),
            graph.edge_uses().len()
        ),
        (5, 4, 6)
    );
    let center = graph.vertices().iter().find(|vertex| {
        let bounds = vertex.point().point_enclosure();
        bounds.x().lower() == min
            && bounds.x().upper() == min
            && bounds.y().lower() == 0.
            && bounds.y().upper() == 0.
    });
    assert!(center.is_some());
    let tiny = raw((0., 0.), (min, 0.));
    let tiny_reverse = raw((min, 0.), (0., 0.));
    let graph = build_arrangement2(&[tiny, tiny_reverse], limits()).unwrap();
    assert_eq!(
        (
            graph.vertices().len(),
            graph.edges().len(),
            graph.edge_uses().len()
        ),
        (2, 1, 2)
    );
    assert_ne!(
        graph.edge_uses()[0].forward(),
        graph.edge_uses()[1].forward()
    );
}
