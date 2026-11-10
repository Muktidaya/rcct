// SPDX-License-Identifier: Apache-2.0

//! Bounded face grouping for simple walks and selected bridge excursions.

use super::embedding::simple_cycle_orientation2;
use super::{
    ArrangementEmbedding2, BridgeAnalysis2, ExactPredicatePoint2, Orientation2,
    PredicateCoordinates, SegmentParameter2, WalkOrientationError2, WalkOrientationLimits2,
    compare_span_boundary_axis, crossing_coordinate_numerator, orient_predicate_coordinates,
    predicate_coordinates, simple_walk_orientation2,
};
use std::cmp::Ordering;
use std::fmt;

/// Logical caps for grouping simple boundary walks into faces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FaceBuildLimits2 {
    /// Maximum input half-edges.
    pub max_half_edges: usize,
    /// Maximum input boundary walks.
    pub max_walks: usize,
    /// Maximum faces, including the unbounded face.
    pub max_faces: usize,
    /// Maximum repeated-vertex and extremum comparisons used for classification.
    pub max_orientation_comparisons: usize,
    /// Maximum exact point-in-walk queries.
    pub max_containment_queries: usize,
    /// Maximum boundary edges examined across containment queries.
    pub max_containment_edge_checks: usize,
}

/// A face grouping resource whose cap was exceeded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceBuildResource2 {
    /// Input half-edges.
    HalfEdges,
    /// Input walks.
    Walks,
    /// Output faces.
    Faces,
    /// Selected orientation comparisons.
    OrientationComparisons,
    /// Point-in-walk queries.
    ContainmentQueries,
    /// Boundary edges examined by containment queries.
    ContainmentEdgeChecks,
}

/// Face grouping failed without returning a partial result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceBuildError2 {
    /// A boundary walk is short or repeats a vertex, such as a bridge walk.
    UnsupportedNonSimpleBoundary {
        /// Index of the rejected boundary walk.
        walk: usize,
    },
    /// Bridge analysis and embedding refer to different arrangement objects.
    DifferentArrangement,
    /// Containment relations did not establish a strictly nested order.
    AmbiguousContainment,
    /// A configured logical limit was reached.
    LimitExceeded(FaceBuildResource2),
    /// A fallible allocation was refused.
    AllocationFailed,
    /// A required graph or orientation invariant was not met.
    InvariantViolation,
}
impl fmt::Display for FaceBuildError2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedNonSimpleBoundary { walk } => {
                write!(f, "boundary walk {walk} is not simple")
            }
            Self::DifferentArrangement => {
                f.write_str("bridge analysis refers to a different arrangement")
            }
            Self::AmbiguousContainment => f.write_str("face containment could not be ordered"),
            Self::LimitExceeded(resource) => write!(f, "face build limit exceeded: {resource:?}"),
            Self::AllocationFailed => f.write_str("face build allocation failed"),
            Self::InvariantViolation => f.write_str("face build invariant failed"),
        }
    }
}
impl std::error::Error for FaceBuildError2 {}

/// One face. Face index zero is the unbounded face.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Face2 {
    outer: Option<usize>,
}
impl Face2 {
    /// Returns its outer walk, or `None` for the unbounded face.
    ///
    /// With bridge-aware grouping the original walk may contain bridge
    /// excursions; its bridge-filtered core is counterclockwise.
    pub const fn outer_walk(&self) -> Option<usize> {
        self.outer
    }
}

/// Face ownership of every boundary walk in a borrowed embedding.
///
/// The simple-only and bridge-aware builders have different supported scopes.
/// A face can own multiple clockwise walks from nested disconnected components.
pub struct PlanarFaces2<'e, 'g> {
    embedding: &'e ArrangementEmbedding2<'g>,
    faces: Vec<Face2>,
    walk_faces: Vec<usize>,
}
impl<'e, 'g> PlanarFaces2<'e, 'g> {
    /// Returns the embedded graph used to build this grouping.
    pub const fn embedding(&self) -> &'e ArrangementEmbedding2<'g> {
        self.embedding
    }
    /// Returns faces in deterministic order, with the unbounded face first.
    pub fn faces(&self) -> &[Face2] {
        &self.faces
    }
    /// Returns the face to the left of a boundary walk, if the index exists.
    pub fn face_of_walk(&self, walk: usize) -> Option<usize> {
        self.walk_faces.get(walk).copied()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Relation {
    Outside,
    Boundary,
    Inside,
}
struct ContainmentBudget {
    queries: usize,
    edges: usize,
}
impl ContainmentBudget {
    fn query(&mut self) -> Result<(), FaceBuildError2> {
        if self.queries == 0 {
            return Err(FaceBuildError2::LimitExceeded(
                FaceBuildResource2::ContainmentQueries,
            ));
        }
        self.queries -= 1;
        Ok(())
    }
    fn edge(&mut self) -> Result<(), FaceBuildError2> {
        if self.edges == 0 {
            return Err(FaceBuildError2::LimitExceeded(
                FaceBuildResource2::ContainmentEdgeChecks,
            ));
        }
        self.edges -= 1;
        Ok(())
    }
}
fn reserved<T>(capacity: usize) -> Result<Vec<T>, FaceBuildError2> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| FaceBuildError2::AllocationFailed)?;
    Ok(result)
}
fn walk_point(
    embedding: &ArrangementEmbedding2<'_>,
    walk: usize,
) -> Result<SegmentParameter2, FaceBuildError2> {
    let h = *embedding
        .boundary_walk(walk)
        .and_then(|w| w.first())
        .ok_or(FaceBuildError2::InvariantViolation)?;
    let vertex = embedding
        .half_edges()
        .get(h)
        .ok_or(FaceBuildError2::InvariantViolation)?
        .origin();
    Ok(embedding
        .arrangement()
        .vertices()
        .get(vertex)
        .ok_or(FaceBuildError2::InvariantViolation)?
        .point())
}

// Exact even-odd ray containment for a simple walk. Every query uses retained
// first-generation points and raw support directions, never rounded coordinates.
fn relation(
    embedding: &ArrangementEmbedding2<'_>,
    point: SegmentParameter2,
    walk: usize,
    budget: &mut ContainmentBudget,
) -> Result<Relation, FaceBuildError2> {
    let edges = embedding
        .boundary_walk(walk)
        .ok_or(FaceBuildError2::InvariantViolation)?;
    relation_edges(embedding, point, edges, budget)
}

fn relation_edges(
    embedding: &ArrangementEmbedding2<'_>,
    point: SegmentParameter2,
    edges: &[usize],
    budget: &mut ContainmentBudget,
) -> Result<Relation, FaceBuildError2> {
    budget.query()?;
    let point_coordinates = PredicateCoordinates {
        x: crossing_coordinate_numerator(point.first.start.x, point.first.end.x, &point),
        y: crossing_coordinate_numerator(point.first.start.y, point.first.end.y, &point),
        denominator: point.denominator,
    };
    let graph = embedding.arrangement();
    let mut inside = false;
    for &h in edges {
        budget.edge()?;
        let half = embedding
            .half_edges()
            .get(h)
            .ok_or(FaceBuildError2::InvariantViolation)?;
        let a = graph
            .vertices()
            .get(half.origin())
            .ok_or(FaceBuildError2::InvariantViolation)?
            .point();
        let b = graph
            .vertices()
            .get(half.destination())
            .ok_or(FaceBuildError2::InvariantViolation)?
            .point();
        let (support, along) = embedding
            .directed_support(h)
            .ok_or(FaceBuildError2::InvariantViolation)?;
        let mut turn = orient_predicate_coordinates(
            predicate_coordinates(ExactPredicatePoint2::Input(support.start())),
            predicate_coordinates(ExactPredicatePoint2::Input(support.end())),
            point_coordinates,
        );
        if !along {
            turn = match turn {
                Orientation2::Clockwise => Orientation2::Counterclockwise,
                Orientation2::Counterclockwise => Orientation2::Clockwise,
                Orientation2::Collinear => Orientation2::Collinear,
            };
        }
        if turn == Orientation2::Collinear {
            let use_x = support.start().x() != support.end().x();
            let pa = compare_span_boundary_axis(point, a, use_x, true);
            let pb = compare_span_boundary_axis(point, b, use_x, true);
            if (pa != Ordering::Less && pb != Ordering::Greater)
                || (pb != Ordering::Less && pa != Ordering::Greater)
            {
                return Ok(Relation::Boundary);
            }
        }
        let ay = compare_span_boundary_axis(a, point, false, true);
        let by = compare_span_boundary_axis(b, point, false, true);
        if (ay != Ordering::Greater
            && by == Ordering::Greater
            && turn == Orientation2::Counterclockwise)
            || (by != Ordering::Greater
                && ay == Ordering::Greater
                && turn == Orientation2::Clockwise)
        {
            inside = !inside;
        }
    }
    Ok(if inside {
        Relation::Inside
    } else {
        Relation::Outside
    })
}

/// Groups simple boundary walks into bounded and unbounded faces.
///
/// All input walks must be simple; bridges and repeated-vertex boundaries are
/// explicitly rejected. Counterclockwise walks define bounded faces. Each
/// clockwise walk belongs to the innermost strictly containing bounded walk,
/// or to face zero when no such walk exists. Exact ray containment and
/// first-generation point identities are used. Unordered or ambiguous nesting
/// returns an error. Recoverable failure returns no partial result and does not
/// modify the input. Limits count selected work/items, not bytes or wall time.
pub fn build_faces2<'e, 'g>(
    embedding: &'e ArrangementEmbedding2<'g>,
    limits: FaceBuildLimits2,
) -> Result<PlanarFaces2<'e, 'g>, FaceBuildError2> {
    let half_edges = embedding.half_edges().len();
    let walks = embedding.boundary_walk_count();
    if half_edges > limits.max_half_edges {
        return Err(FaceBuildError2::LimitExceeded(
            FaceBuildResource2::HalfEdges,
        ));
    }
    if walks > limits.max_walks {
        return Err(FaceBuildError2::LimitExceeded(FaceBuildResource2::Walks));
    }
    if limits.max_faces == 0 {
        return Err(FaceBuildError2::LimitExceeded(FaceBuildResource2::Faces));
    }
    let capacity = walks
        .checked_add(1)
        .ok_or(FaceBuildError2::LimitExceeded(FaceBuildResource2::Faces))?
        .min(limits.max_faces);
    let mut faces = reserved(capacity)?;
    faces.push(Face2 { outer: None });
    let mut owners = reserved(walks)?;
    owners.resize(walks, usize::MAX);
    let mut orientations = reserved(walks)?;
    let mut remaining = limits.max_orientation_comparisons;
    for (walk, owner) in owners.iter_mut().enumerate() {
        let size = embedding
            .boundary_walk(walk)
            .ok_or(FaceBuildError2::InvariantViolation)?
            .len();
        if size < 3 {
            return Err(FaceBuildError2::UnsupportedNonSimpleBoundary { walk });
        }
        let cost = size
            .checked_mul(size - 1)
            .and_then(|n| n.checked_div(2))
            .and_then(|n| n.checked_add(size - 1))
            .ok_or(FaceBuildError2::LimitExceeded(
                FaceBuildResource2::OrientationComparisons,
            ))?;
        if cost > remaining {
            return Err(FaceBuildError2::LimitExceeded(
                FaceBuildResource2::OrientationComparisons,
            ));
        }
        remaining -= cost;
        let orientation = simple_walk_orientation2(
            embedding,
            walk,
            WalkOrientationLimits2 {
                max_steps: half_edges,
                max_comparisons: cost,
            },
        )
        .map_err(|error| match error {
            WalkOrientationError2::NotSimple => {
                FaceBuildError2::UnsupportedNonSimpleBoundary { walk }
            }
            WalkOrientationError2::AllocationFailed => FaceBuildError2::AllocationFailed,
            WalkOrientationError2::LimitExceeded(_) => {
                FaceBuildError2::LimitExceeded(FaceBuildResource2::OrientationComparisons)
            }
            _ => FaceBuildError2::InvariantViolation,
        })?;
        orientations.push(orientation);
        if orientation == Orientation2::Counterclockwise {
            if faces.len() >= limits.max_faces {
                return Err(FaceBuildError2::LimitExceeded(FaceBuildResource2::Faces));
            }
            *owner = faces.len();
            faces.push(Face2 { outer: Some(walk) });
        }
    }
    let mut budget = ContainmentBudget {
        queries: limits.max_containment_queries,
        edges: limits.max_containment_edge_checks,
    };
    for walk in 0..walks {
        if orientations[walk] != Orientation2::Clockwise {
            continue;
        }
        let point = walk_point(embedding, walk)?;
        let mut best: Option<usize> = None;
        for (candidate, &orientation) in orientations.iter().enumerate() {
            if orientation != Orientation2::Counterclockwise {
                continue;
            }
            if relation(embedding, point, candidate, &mut budget)? != Relation::Inside {
                continue;
            }
            if let Some(current) = best {
                let candidate_point = walk_point(embedding, candidate)?;
                match relation(embedding, candidate_point, current, &mut budget)? {
                    Relation::Inside => best = Some(candidate),
                    Relation::Boundary => return Err(FaceBuildError2::AmbiguousContainment),
                    Relation::Outside => {
                        let current_point = walk_point(embedding, current)?;
                        if relation(embedding, current_point, candidate, &mut budget)?
                            != Relation::Inside
                        {
                            return Err(FaceBuildError2::AmbiguousContainment);
                        }
                    }
                }
            } else {
                best = Some(candidate);
            }
        }
        owners[walk] = best.map_or(0, |index| owners[index]);
    }
    Ok(PlanarFaces2 {
        embedding,
        faces,
        walk_faces: owners,
    })
}

/// Groups boundary walks after removing bridge excursions for containment.
///
/// The bridge analysis must borrow the very same arrangement as `embedding`.
/// Each walk's non-bridge half-edges must form either no cycle or one closed,
/// continuous simple cycle. Walks with multiple core cycles or other repeated
/// vertices are rejected. Bridge half-edges remain in the original walks and
/// both sides are checked to have the same face before publishing the result.
/// Limits include original half-edges and walks; scratch and containment work
/// are bounded by the existing logical caps. Recoverable errors return no
/// partial result and leave the embedding and analysis unchanged.
pub fn build_faces_with_bridges2<'e, 'g>(
    embedding: &'e ArrangementEmbedding2<'g>,
    bridges: &BridgeAnalysis2<'_>,
    limits: FaceBuildLimits2,
) -> Result<PlanarFaces2<'e, 'g>, FaceBuildError2> {
    if !std::ptr::eq(embedding.arrangement(), bridges.arrangement()) {
        return Err(FaceBuildError2::DifferentArrangement);
    }
    let half_edges = embedding.half_edges().len();
    let walks = embedding.boundary_walk_count();
    if half_edges > limits.max_half_edges {
        return Err(FaceBuildError2::LimitExceeded(
            FaceBuildResource2::HalfEdges,
        ));
    }
    if walks > limits.max_walks {
        return Err(FaceBuildError2::LimitExceeded(FaceBuildResource2::Walks));
    }
    if limits.max_faces == 0 {
        return Err(FaceBuildError2::LimitExceeded(FaceBuildResource2::Faces));
    }
    let walk_capacity = walks
        .checked_add(1)
        .ok_or(FaceBuildError2::LimitExceeded(FaceBuildResource2::Walks))?;
    let mut core_edges = reserved(half_edges)?;
    let mut offsets = reserved(walk_capacity)?;
    offsets.push(0);
    for walk in 0..walks {
        for &h in embedding
            .boundary_walk(walk)
            .ok_or(FaceBuildError2::InvariantViolation)?
        {
            let edge = embedding
                .half_edges()
                .get(h)
                .ok_or(FaceBuildError2::InvariantViolation)?
                .edge();
            if !bridges
                .is_bridge(edge)
                .ok_or(FaceBuildError2::InvariantViolation)?
            {
                if core_edges.len() >= half_edges {
                    return Err(FaceBuildError2::InvariantViolation);
                }
                core_edges.push(h);
            }
        }
        offsets.push(core_edges.len());
    }
    let core = |walk: usize| &core_edges[offsets[walk]..offsets[walk + 1]];
    let point_of = |walk: usize| -> Result<SegmentParameter2, FaceBuildError2> {
        let selected = if core(walk).is_empty() {
            embedding
                .boundary_walk(walk)
                .ok_or(FaceBuildError2::InvariantViolation)?
        } else {
            core(walk)
        };
        let h = *selected
            .first()
            .ok_or(FaceBuildError2::InvariantViolation)?;
        let vertex = embedding
            .half_edges()
            .get(h)
            .ok_or(FaceBuildError2::InvariantViolation)?
            .origin();
        Ok(embedding
            .arrangement()
            .vertices()
            .get(vertex)
            .ok_or(FaceBuildError2::InvariantViolation)?
            .point())
    };
    let capacity = walks
        .checked_add(1)
        .ok_or(FaceBuildError2::LimitExceeded(FaceBuildResource2::Faces))?
        .min(limits.max_faces);
    let mut faces = reserved(capacity)?;
    faces.push(Face2 { outer: None });
    let mut owners = reserved(walks)?;
    owners.resize(walks, usize::MAX);
    let mut orientations = reserved(walks)?;
    let mut remaining = limits.max_orientation_comparisons;
    for (walk, owner) in owners.iter_mut().enumerate() {
        let edges = core(walk);
        let size = edges.len();
        if size == 0 {
            orientations.push(None);
            continue;
        }
        if size < 3 {
            return Err(FaceBuildError2::UnsupportedNonSimpleBoundary { walk });
        }
        for index in 0..size {
            let current = embedding
                .half_edges()
                .get(edges[index])
                .ok_or(FaceBuildError2::InvariantViolation)?;
            let next = embedding
                .half_edges()
                .get(edges[(index + 1) % size])
                .ok_or(FaceBuildError2::InvariantViolation)?;
            if current.destination() != next.origin() {
                return Err(FaceBuildError2::UnsupportedNonSimpleBoundary { walk });
            }
        }
        let cost = size
            .checked_mul(size - 1)
            .and_then(|n| n.checked_div(2))
            .and_then(|n| n.checked_add(size - 1))
            .ok_or(FaceBuildError2::LimitExceeded(
                FaceBuildResource2::OrientationComparisons,
            ))?;
        if cost > remaining {
            return Err(FaceBuildError2::LimitExceeded(
                FaceBuildResource2::OrientationComparisons,
            ));
        }
        remaining -= cost;
        let orientation = simple_cycle_orientation2(
            embedding,
            edges,
            WalkOrientationLimits2 {
                max_steps: half_edges,
                max_comparisons: cost,
            },
        )
        .map_err(|error| match error {
            WalkOrientationError2::NotSimple => {
                FaceBuildError2::UnsupportedNonSimpleBoundary { walk }
            }
            WalkOrientationError2::AllocationFailed => FaceBuildError2::AllocationFailed,
            WalkOrientationError2::LimitExceeded(_) => {
                FaceBuildError2::LimitExceeded(FaceBuildResource2::OrientationComparisons)
            }
            _ => FaceBuildError2::InvariantViolation,
        })?;
        orientations.push(Some(orientation));
        if orientation == Orientation2::Counterclockwise {
            if faces.len() >= limits.max_faces {
                return Err(FaceBuildError2::LimitExceeded(FaceBuildResource2::Faces));
            }
            *owner = faces.len();
            faces.push(Face2 { outer: Some(walk) });
        }
    }
    let mut budget = ContainmentBudget {
        queries: limits.max_containment_queries,
        edges: limits.max_containment_edge_checks,
    };
    for walk in 0..walks {
        if orientations[walk] == Some(Orientation2::Counterclockwise) {
            continue;
        }
        let point = point_of(walk)?;
        let mut best: Option<usize> = None;
        for (candidate, &orientation) in orientations.iter().enumerate() {
            if orientation != Some(Orientation2::Counterclockwise) {
                continue;
            }
            match relation_edges(embedding, point, core(candidate), &mut budget)? {
                Relation::Inside => {}
                Relation::Boundary if orientations[walk].is_none() => {
                    return Err(FaceBuildError2::AmbiguousContainment);
                }
                _ => continue,
            }
            if let Some(current) = best {
                let candidate_point = point_of(candidate)?;
                match relation_edges(embedding, candidate_point, core(current), &mut budget)? {
                    Relation::Inside => best = Some(candidate),
                    Relation::Boundary => return Err(FaceBuildError2::AmbiguousContainment),
                    Relation::Outside => {
                        if relation_edges(
                            embedding,
                            point_of(current)?,
                            core(candidate),
                            &mut budget,
                        )? != Relation::Inside
                        {
                            return Err(FaceBuildError2::AmbiguousContainment);
                        }
                    }
                }
            } else {
                best = Some(candidate);
            }
        }
        owners[walk] = best.map_or(0, |index| owners[index]);
    }
    let mut half_owners = reserved(half_edges)?;
    half_owners.resize(half_edges, usize::MAX);
    for (walk, &owner) in owners.iter().enumerate() {
        for &h in embedding
            .boundary_walk(walk)
            .ok_or(FaceBuildError2::InvariantViolation)?
        {
            let slot = half_owners
                .get_mut(h)
                .ok_or(FaceBuildError2::InvariantViolation)?;
            if *slot != usize::MAX {
                return Err(FaceBuildError2::InvariantViolation);
            }
            *slot = owner;
        }
    }
    if half_owners.iter().any(|&owner| owner >= faces.len()) {
        return Err(FaceBuildError2::InvariantViolation);
    }
    for (edge, _) in embedding.arrangement().edges().iter().enumerate() {
        if bridges
            .is_bridge(edge)
            .ok_or(FaceBuildError2::InvariantViolation)?
        {
            let first = *half_owners
                .get(edge * 2)
                .ok_or(FaceBuildError2::InvariantViolation)?;
            let second = *half_owners
                .get(edge * 2 + 1)
                .ok_or(FaceBuildError2::InvariantViolation)?;
            if first == usize::MAX || first != second {
                return Err(FaceBuildError2::InvariantViolation);
            }
        }
    }
    Ok(PlanarFaces2 {
        embedding,
        faces,
        walk_faces: owners,
    })
}

/// Logical caps for computing winding values of the aggregate directed sources.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FaceWindingLimits2 {
    /// Maximum grouped faces.
    pub max_faces: usize,
    /// Maximum directed half-edges.
    pub max_half_edges: usize,
    /// Maximum source-edge use records scanned.
    pub max_edge_uses: usize,
    /// Maximum edge constraints examined during propagation.
    pub max_propagation_checks: usize,
}
/// A winding resource whose cap was exceeded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceWindingResource2 {
    /// Grouped faces.
    Faces,
    /// Directed half-edges.
    HalfEdges,
    /// Source-edge use records.
    EdgeUses,
    /// Edge constraints examined during propagation.
    PropagationChecks,
}
/// Winding computation failed without returning partial values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceWindingError2 {
    /// A configured logical count was exceeded.
    LimitExceeded(FaceWindingResource2),
    /// The aggregate directed source uses do not form a consistent closed chain.
    InconsistentChain,
    /// An integer winding or edge multiplicity exceeded `i64`.
    ArithmeticOverflow,
    /// A fallible reservation was refused.
    AllocationFailed,
    /// A required embedding or face invariant was not met.
    InvariantViolation,
}
impl fmt::Display for FaceWindingError2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded(resource) => write!(f, "face winding limit exceeded: {resource:?}"),
            Self::InconsistentChain => {
                f.write_str("directed sources do not form a consistent closed chain")
            }
            Self::ArithmeticOverflow => f.write_str("face winding arithmetic overflow"),
            Self::AllocationFailed => f.write_str("face winding allocation failed"),
            Self::InvariantViolation => f.write_str("face winding invariant failed"),
        }
    }
}
impl std::error::Error for FaceWindingError2 {}

/// Aggregate integer winding values tied to the grouped faces that produced them.
pub struct FaceWindings2<'f, 'e, 'g> {
    faces: &'f PlanarFaces2<'e, 'g>,
    values: Vec<i64>,
}
impl<'f, 'e, 'g> FaceWindings2<'f, 'e, 'g> {
    /// Returns the face grouping used by this result.
    pub const fn faces(&self) -> &'f PlanarFaces2<'e, 'g> {
        self.faces
    }
    /// Returns the aggregate winding for one face; the unbounded face is zero.
    pub fn winding(&self, face: usize) -> Option<i64> {
        self.values.get(face).copied()
    }
}
fn winding_reserved<T>(capacity: usize) -> Result<Vec<T>, FaceWindingError2> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| FaceWindingError2::AllocationFailed)?;
    Ok(result)
}

/// Computes aggregate source winding on grouped faces for a consistent chain.
///
/// Each directed source use contributes +1 along its source direction; opposite
/// coincident uses cancel. Starting with zero on the unbounded face, every edge
/// imposes `left = right + signed_uses`. Inconsistent constraints return
/// `InconsistentChain`; checked integer arithmetic reports overflow. The result
/// is immutable and no partial values are returned on recoverable failure.
/// This computes one aggregate winding, not a per-source or Boolean region.
pub fn face_windings2<'f, 'e, 'g>(
    faces: &'f PlanarFaces2<'e, 'g>,
    limits: FaceWindingLimits2,
) -> Result<FaceWindings2<'f, 'e, 'g>, FaceWindingError2> {
    let embedding = faces.embedding();
    let graph = embedding.arrangement();
    let face_count = faces.faces().len();
    let half_count = embedding.half_edges().len();
    if face_count > limits.max_faces {
        return Err(FaceWindingError2::LimitExceeded(
            FaceWindingResource2::Faces,
        ));
    }
    if half_count > limits.max_half_edges {
        return Err(FaceWindingError2::LimitExceeded(
            FaceWindingResource2::HalfEdges,
        ));
    }
    if graph.edge_uses().len() > limits.max_edge_uses {
        return Err(FaceWindingError2::LimitExceeded(
            FaceWindingResource2::EdgeUses,
        ));
    }
    let mut half_faces = winding_reserved(half_count)?;
    half_faces.resize(half_count, usize::MAX);
    for walk in 0..embedding.boundary_walk_count() {
        let face = faces
            .face_of_walk(walk)
            .ok_or(FaceWindingError2::InvariantViolation)?;
        if face >= face_count {
            return Err(FaceWindingError2::InvariantViolation);
        }
        for &h in embedding
            .boundary_walk(walk)
            .ok_or(FaceWindingError2::InvariantViolation)?
        {
            let slot = half_faces
                .get_mut(h)
                .ok_or(FaceWindingError2::InvariantViolation)?;
            if *slot != usize::MAX {
                return Err(FaceWindingError2::InvariantViolation);
            }
            *slot = face;
        }
    }
    let mut weights = winding_reserved(graph.edges().len())?;
    weights.resize(graph.edges().len(), 0_i64);
    for use_ in graph.edge_uses() {
        let weight = weights
            .get_mut(use_.edge())
            .ok_or(FaceWindingError2::InvariantViolation)?;
        *weight = weight
            .checked_add(if use_.forward() { 1 } else { -1 })
            .ok_or(FaceWindingError2::ArithmeticOverflow)?;
    }
    let mut values = winding_reserved(face_count)?;
    values.resize(face_count, None::<i64>);
    *values
        .get_mut(0)
        .ok_or(FaceWindingError2::InvariantViolation)? = Some(0);
    let mut remaining = limits.max_propagation_checks;
    let mut changed = true;
    while changed {
        changed = false;
        for (edge, &weight) in weights.iter().enumerate() {
            if remaining == 0 {
                return Err(FaceWindingError2::LimitExceeded(
                    FaceWindingResource2::PropagationChecks,
                ));
            }
            remaining -= 1;
            let left = half_faces[edge * 2];
            let right = half_faces[edge * 2 + 1];
            if left >= face_count || right >= face_count {
                return Err(FaceWindingError2::InvariantViolation);
            }
            match (values[left], values[right]) {
                (Some(l), Some(r)) => {
                    if r.checked_add(weight)
                        .ok_or(FaceWindingError2::ArithmeticOverflow)?
                        != l
                    {
                        return Err(FaceWindingError2::InconsistentChain);
                    }
                }
                (Some(l), None) => {
                    values[right] = Some(
                        l.checked_sub(weight)
                            .ok_or(FaceWindingError2::ArithmeticOverflow)?,
                    );
                    changed = true;
                }
                (None, Some(r)) => {
                    values[left] = Some(
                        r.checked_add(weight)
                            .ok_or(FaceWindingError2::ArithmeticOverflow)?,
                    );
                    changed = true;
                }
                (None, None) => {}
            }
        }
    }
    let mut result = winding_reserved(face_count)?;
    for value in values {
        result.push(value.ok_or(FaceWindingError2::InvariantViolation)?);
    }
    Ok(FaceWindings2 {
        faces,
        values: result,
    })
}
