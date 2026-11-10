// SPDX-License-Identifier: Apache-2.0

//! Grouped winding and selected planar boundaries for simple-face arrangements.

use super::PlanarFaces2;
use std::fmt;

/// Logical caps for grouped face-winding propagation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroupWindingLimits2 {
    /// Maximum source segments and source-group assignments.
    pub max_sources: usize,
    /// Maximum input groups.
    pub max_groups: usize,
    /// Maximum grouped faces.
    pub max_faces: usize,
    /// Maximum input half-edges.
    pub max_half_edges: usize,
    /// Maximum source-edge use records.
    pub max_edge_uses: usize,
    /// Maximum combined face/group and edge/group winding cells.
    pub max_winding_cells: usize,
    /// Maximum edge/group constraint checks during propagation.
    pub max_propagation_checks: usize,
}
/// A grouped-winding resource whose cap was reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupWindingResource2 {
    /// Input sources.
    Sources,
    /// Input groups.
    Groups,
    /// Input faces.
    Faces,
    /// Input half-edges.
    HalfEdges,
    /// Source-edge use records.
    EdgeUses,
    /// Face/group plus edge/group winding cells.
    WindingCells,
    /// Edge/group constraint checks.
    PropagationChecks,
}
/// Grouped winding failed without returning partial values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupWindingError2 {
    /// At least one group is required.
    EmptyGroups,
    /// There must be one assignment per graph source.
    SourceCountMismatch,
    /// A source assignment refers to a nonexistent group.
    InvalidGroup {
        /// Source whose assignment is invalid.
        source: usize,
    },
    /// One group's directed sources are not a consistent closed chain.
    InconsistentGroup {
        /// Group with conflicting constraints.
        group: usize,
    },
    /// A configured logical cap was reached.
    LimitExceeded(GroupWindingResource2),
    /// A winding or edge multiplicity exceeded `i64`.
    ArithmeticOverflow,
    /// A fallible reservation was refused.
    AllocationFailed,
    /// A required graph or face invariant did not hold.
    InvariantViolation,
}
impl fmt::Display for GroupWindingError2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGroups => f.write_str("at least one winding group is required"),
            Self::SourceCountMismatch => {
                f.write_str("source assignment count does not match graph sources")
            }
            Self::InvalidGroup { source } => {
                write!(f, "source {source} has an invalid winding group")
            }
            Self::InconsistentGroup { group } => {
                write!(f, "winding group {group} is not a consistent closed chain")
            }
            Self::LimitExceeded(resource) => {
                write!(f, "group winding limit exceeded: {resource:?}")
            }
            Self::ArithmeticOverflow => f.write_str("group winding arithmetic overflow"),
            Self::AllocationFailed => f.write_str("group winding allocation failed"),
            Self::InvariantViolation => f.write_str("group winding invariant failed"),
        }
    }
}
impl std::error::Error for GroupWindingError2 {}

/// Per-group face windings tied to the face grouping that produced them.
pub struct GroupWindings2<'f, 'e, 'g> {
    faces: &'f PlanarFaces2<'e, 'g>,
    groups: usize,
    values: Vec<i64>,
    half_faces: Vec<usize>,
}
impl<'f, 'e, 'g> GroupWindings2<'f, 'e, 'g> {
    /// Returns the face grouping used by these values.
    pub const fn faces(&self) -> &'f PlanarFaces2<'e, 'g> {
        self.faces
    }
    /// Returns the number of groups.
    pub const fn group_count(&self) -> usize {
        self.groups
    }
    /// Returns one face/group winding when both indices are valid.
    pub fn winding(&self, face: usize, group: usize) -> Option<i64> {
        if face >= self.faces.faces().len() || group >= self.groups {
            return None;
        }
        Some(self.values[face * self.groups + group])
    }
}
fn group_reserved<T>(capacity: usize) -> Result<Vec<T>, GroupWindingError2> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| GroupWindingError2::AllocationFailed)?;
    Ok(result)
}

/// Computes independent face windings for assigned groups of source segments.
///
/// `source_groups` has one entry per original graph source; `None` ignores that
/// source and `Some(g)` assigns it to group `g`. Each group must independently
/// form a consistent closed directed chain. Winding is zero on the unbounded
/// face. Counts cap named work/items, not exact bytes, all arithmetic or time.
/// Recoverable errors return no partial result and do not mutate inputs.
pub fn group_windings2<'f, 'e, 'g>(
    faces: &'f PlanarFaces2<'e, 'g>,
    source_groups: &[Option<usize>],
    groups: usize,
    limits: GroupWindingLimits2,
) -> Result<GroupWindings2<'f, 'e, 'g>, GroupWindingError2> {
    let embedding = faces.embedding();
    let graph = embedding.arrangement();
    let sources = graph.sources().len();
    let face_count = faces.faces().len();
    let half_count = embedding.half_edges().len();
    let edge_count = graph.edges().len();
    if groups == 0 {
        return Err(GroupWindingError2::EmptyGroups);
    }
    if source_groups.len() != sources {
        return Err(GroupWindingError2::SourceCountMismatch);
    }
    if sources > limits.max_sources {
        return Err(GroupWindingError2::LimitExceeded(
            GroupWindingResource2::Sources,
        ));
    }
    if groups > limits.max_groups {
        return Err(GroupWindingError2::LimitExceeded(
            GroupWindingResource2::Groups,
        ));
    }
    if face_count > limits.max_faces {
        return Err(GroupWindingError2::LimitExceeded(
            GroupWindingResource2::Faces,
        ));
    }
    if half_count > limits.max_half_edges {
        return Err(GroupWindingError2::LimitExceeded(
            GroupWindingResource2::HalfEdges,
        ));
    }
    if graph.edge_uses().len() > limits.max_edge_uses {
        return Err(GroupWindingError2::LimitExceeded(
            GroupWindingResource2::EdgeUses,
        ));
    }
    for (source, assignment) in source_groups.iter().enumerate() {
        if assignment.is_some_and(|g| g >= groups) {
            return Err(GroupWindingError2::InvalidGroup { source });
        }
    }
    let face_cells = face_count
        .checked_mul(groups)
        .ok_or(GroupWindingError2::LimitExceeded(
            GroupWindingResource2::WindingCells,
        ))?;
    let edge_cells = edge_count
        .checked_mul(groups)
        .ok_or(GroupWindingError2::LimitExceeded(
            GroupWindingResource2::WindingCells,
        ))?;
    let cells = face_cells
        .checked_add(edge_cells)
        .ok_or(GroupWindingError2::LimitExceeded(
            GroupWindingResource2::WindingCells,
        ))?;
    if cells > limits.max_winding_cells {
        return Err(GroupWindingError2::LimitExceeded(
            GroupWindingResource2::WindingCells,
        ));
    }
    let mut half_faces = group_reserved(half_count)?;
    half_faces.resize(half_count, usize::MAX);
    for walk in 0..embedding.boundary_walk_count() {
        let face = faces
            .face_of_walk(walk)
            .ok_or(GroupWindingError2::InvariantViolation)?;
        if face >= face_count {
            return Err(GroupWindingError2::InvariantViolation);
        }
        for &h in embedding
            .boundary_walk(walk)
            .ok_or(GroupWindingError2::InvariantViolation)?
        {
            let slot = half_faces
                .get_mut(h)
                .ok_or(GroupWindingError2::InvariantViolation)?;
            if *slot != usize::MAX {
                return Err(GroupWindingError2::InvariantViolation);
            }
            *slot = face;
        }
    }
    let mut weights = group_reserved(edge_cells)?;
    weights.resize(edge_cells, 0_i64);
    for use_ in graph.edge_uses() {
        let assignment = *source_groups
            .get(use_.source())
            .ok_or(GroupWindingError2::InvariantViolation)?;
        if use_.edge() >= edge_count {
            return Err(GroupWindingError2::InvariantViolation);
        }
        if let Some(group) = assignment {
            let index = use_.edge() * groups + group;
            let weight = weights
                .get_mut(index)
                .ok_or(GroupWindingError2::InvariantViolation)?;
            *weight = weight
                .checked_add(if use_.forward() { 1 } else { -1 })
                .ok_or(GroupWindingError2::ArithmeticOverflow)?;
        }
    }
    let mut values = group_reserved(face_cells)?;
    values.resize(face_cells, 0_i64);
    let mut known = group_reserved(face_cells)?;
    known.resize(face_cells, false);
    for known in known.iter_mut().take(groups) {
        *known = true;
    }
    let mut remaining = limits.max_propagation_checks;
    let mut changed = true;
    while changed {
        changed = false;
        for edge in 0..edge_count {
            let left = half_faces[2 * edge];
            let right = half_faces[2 * edge + 1];
            if left >= face_count || right >= face_count {
                return Err(GroupWindingError2::InvariantViolation);
            }
            for group in 0..groups {
                if remaining == 0 {
                    return Err(GroupWindingError2::LimitExceeded(
                        GroupWindingResource2::PropagationChecks,
                    ));
                }
                remaining -= 1;
                let l = left * groups + group;
                let r = right * groups + group;
                let weight = weights[edge * groups + group];
                match (known[l], known[r]) {
                    (true, true) => {
                        if values[r]
                            .checked_add(weight)
                            .ok_or(GroupWindingError2::ArithmeticOverflow)?
                            != values[l]
                        {
                            return Err(GroupWindingError2::InconsistentGroup { group });
                        }
                    }
                    (true, false) => {
                        values[r] = values[l]
                            .checked_sub(weight)
                            .ok_or(GroupWindingError2::ArithmeticOverflow)?;
                        known[r] = true;
                        changed = true;
                    }
                    (false, true) => {
                        values[l] = values[r]
                            .checked_add(weight)
                            .ok_or(GroupWindingError2::ArithmeticOverflow)?;
                        known[l] = true;
                        changed = true;
                    }
                    (false, false) => {}
                }
            }
        }
    }
    if known.iter().any(|&value| !value) {
        return Err(GroupWindingError2::InvariantViolation);
    }
    Ok(GroupWindings2 {
        faces,
        groups,
        values,
        half_faces,
    })
}

/// How nonzero winding values are interpreted for one operand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FillRule2 {
    /// Any nonzero integer winding is inside.
    NonZero,
    /// Odd integer winding is inside.
    EvenOdd,
}
impl FillRule2 {
    fn inside(self, w: i64) -> bool {
        match self {
            Self::NonZero => w != 0,
            Self::EvenOdd => w % 2 != 0,
        }
    }
}

/// A binary operation on the filled two-dimensional faces of two groups.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanOp2 {
    /// Faces inside either operand.
    Union,
    /// Faces inside both operands.
    Intersection,
    /// Faces inside the first and outside the second operand.
    Difference,
    /// Faces inside exactly one operand.
    SymmetricDifference,
}
impl BooleanOp2 {
    fn select(self, a: bool, b: bool) -> bool {
        match self {
            Self::Union => a || b,
            Self::Intersection => a && b,
            Self::Difference => a && !b,
            Self::SymmetricDifference => a != b,
        }
    }
}

/// Logical caps for selecting and tracing Boolean boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BooleanRegionLimits2 {
    /// Maximum input faces.
    pub max_faces: usize,
    /// Maximum input vertices.
    pub max_vertices: usize,
    /// Maximum input half-edges.
    pub max_half_edges: usize,
    /// Maximum selected directed boundary edges.
    pub max_boundary_edges: usize,
    /// Maximum returned boundary walks.
    pub max_boundary_walks: usize,
    /// Maximum selected half-edges visited during tracing.
    pub max_walk_steps: usize,
    /// Maximum outgoing-ray checks when selecting successors.
    pub max_rotation_checks: usize,
}
/// A Boolean-selection resource whose cap was reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanRegionResource2 {
    /// Input faces.
    Faces,
    /// Input vertices.
    Vertices,
    /// Input half-edges.
    HalfEdges,
    /// Selected directed boundary edges.
    BoundaryEdges,
    /// Returned boundary walks.
    BoundaryWalks,
    /// Selected half-edges visited during tracing.
    WalkSteps,
    /// Outgoing-ray successor checks.
    RotationChecks,
}
/// Boolean selection failed without returning a partial region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanRegionError2 {
    /// An operand group index is invalid.
    InvalidGroup,
    /// A configured logical cap was reached.
    LimitExceeded(BooleanRegionResource2),
    /// A fallible reservation was refused.
    AllocationFailed,
    /// A required embedding or selected-boundary invariant did not hold.
    InvariantViolation,
}
impl fmt::Display for BooleanRegionError2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGroup => f.write_str("Boolean operand group is invalid"),
            Self::LimitExceeded(resource) => {
                write!(f, "Boolean region limit exceeded: {resource:?}")
            }
            Self::AllocationFailed => f.write_str("Boolean region allocation failed"),
            Self::InvariantViolation => f.write_str("Boolean boundary invariant failed"),
        }
    }
}
impl std::error::Error for BooleanRegionError2 {}

/// Selected two-dimensional faces and oriented boundary walks tied to a graph.
///
/// Each returned boundary half-edge has a selected face on its left and an
/// unselected face on its right. Walks can share vertices and are not promised
/// to be simple rings. Isolated point or edge contacts are not retained as
/// lower-dimensional Boolean results. No new coordinates are constructed.
pub struct BooleanRegion2<'f, 'e, 'g> {
    faces: &'f PlanarFaces2<'e, 'g>,
    selected: Vec<bool>,
    boundary: Vec<usize>,
    offsets: Vec<usize>,
}
impl<'f, 'e, 'g> BooleanRegion2<'f, 'e, 'g> {
    /// Returns the underlying face grouping.
    pub const fn faces(&self) -> &'f PlanarFaces2<'e, 'g> {
        self.faces
    }
    /// Returns whether a valid face is selected.
    pub fn contains_face(&self, face: usize) -> Option<bool> {
        self.selected.get(face).copied()
    }
    /// Returns the number of closed boundary walks.
    pub fn boundary_walk_count(&self) -> usize {
        self.offsets.len() - 1
    }
    /// Returns half-edge indices of one closed oriented boundary walk.
    pub fn boundary_walk(&self, walk: usize) -> Option<&[usize]> {
        let next = walk.checked_add(1)?;
        Some(&self.boundary[*self.offsets.get(walk)?..*self.offsets.get(next)?])
    }
}
fn region_reserved<T>(capacity: usize) -> Result<Vec<T>, BooleanRegionError2> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| BooleanRegionError2::AllocationFailed)?;
    Ok(result)
}

/// Selects filled faces and traces their boundary for two winding groups.
///
/// This is a face-based regularized 2D result, not a complete set-theoretic
/// Boolean of points and edges. Inputs must come from a successful simple-boundary face grouping and
/// grouped-winding computation. Boundary half-edges are oriented with selected
/// material on the left. At a vertex the clockwise predecessor among selected
/// boundary rays is followed; every selected edge is checked to have one
/// predecessor. Recoverable failure returns no partial region. Limits count
/// named operations and items, not exact bytes or wall time.
pub fn select_boolean2<'f, 'e, 'g>(
    windings: &GroupWindings2<'f, 'e, 'g>,
    first: usize,
    first_rule: FillRule2,
    second: usize,
    second_rule: FillRule2,
    operation: BooleanOp2,
    limits: BooleanRegionLimits2,
) -> Result<BooleanRegion2<'f, 'e, 'g>, BooleanRegionError2> {
    if first >= windings.groups || second >= windings.groups {
        return Err(BooleanRegionError2::InvalidGroup);
    }
    let faces = windings.faces;
    let embedding = faces.embedding();
    let graph = embedding.arrangement();
    let nfaces = faces.faces().len();
    let nvertices = graph.vertices().len();
    let half_count = embedding.half_edges().len();
    if nfaces > limits.max_faces {
        return Err(BooleanRegionError2::LimitExceeded(
            BooleanRegionResource2::Faces,
        ));
    }
    if nvertices > limits.max_vertices {
        return Err(BooleanRegionError2::LimitExceeded(
            BooleanRegionResource2::Vertices,
        ));
    }
    if half_count > limits.max_half_edges {
        return Err(BooleanRegionError2::LimitExceeded(
            BooleanRegionResource2::HalfEdges,
        ));
    }
    let mut selected = region_reserved(nfaces)?;
    for face in 0..nfaces {
        let a = windings
            .winding(face, first)
            .ok_or(BooleanRegionError2::InvariantViolation)?;
        let b = windings
            .winding(face, second)
            .ok_or(BooleanRegionError2::InvariantViolation)?;
        selected.push(operation.select(first_rule.inside(a), second_rule.inside(b)));
    }
    if selected.first().copied() != Some(false) {
        return Err(BooleanRegionError2::InvariantViolation);
    }
    let mut is_boundary = region_reserved(half_count)?;
    is_boundary.resize(half_count, false);
    let mut boundary_count = 0_usize;
    for edge in 0..graph.edges().len() {
        let left = windings.half_faces[2 * edge];
        let right = windings.half_faces[2 * edge + 1];
        if left >= nfaces || right >= nfaces {
            return Err(BooleanRegionError2::InvariantViolation);
        }
        if selected[left] != selected[right] {
            if boundary_count >= limits.max_boundary_edges {
                return Err(BooleanRegionError2::LimitExceeded(
                    BooleanRegionResource2::BoundaryEdges,
                ));
            }
            let h = 2 * edge + usize::from(!selected[left]);
            is_boundary[h] = true;
            boundary_count += 1;
        }
    }
    if boundary_count > limits.max_walk_steps {
        return Err(BooleanRegionError2::LimitExceeded(
            BooleanRegionResource2::WalkSteps,
        ));
    }
    let mut positions = region_reserved(half_count)?;
    positions.resize(half_count, usize::MAX);
    for vertex in 0..nvertices {
        for (position, &h) in embedding
            .outgoing(vertex)
            .ok_or(BooleanRegionError2::InvariantViolation)?
            .iter()
            .enumerate()
        {
            *positions
                .get_mut(h)
                .ok_or(BooleanRegionError2::InvariantViolation)? = position;
        }
    }
    let mut next = region_reserved(half_count)?;
    next.resize(half_count, usize::MAX);
    let mut predecessors = region_reserved(half_count)?;
    predecessors.resize(half_count, 0_u8);
    let mut remaining = limits.max_rotation_checks;
    for h in 0..half_count {
        if !is_boundary[h] {
            continue;
        }
        let half = embedding.half_edges()[h];
        let outgoing = embedding
            .outgoing(half.destination())
            .ok_or(BooleanRegionError2::InvariantViolation)?;
        let position = *positions
            .get(half.twin())
            .ok_or(BooleanRegionError2::InvariantViolation)?;
        if position >= outgoing.len() {
            return Err(BooleanRegionError2::InvariantViolation);
        }
        let mut successor = None;
        for offset in 1..=outgoing.len() {
            if remaining == 0 {
                return Err(BooleanRegionError2::LimitExceeded(
                    BooleanRegionResource2::RotationChecks,
                ));
            }
            remaining -= 1;
            let index = if offset <= position {
                position - offset
            } else {
                outgoing.len() - (offset - position)
            };
            let candidate = outgoing[index];
            if is_boundary[candidate] {
                successor = Some(candidate);
                break;
            }
        }
        let successor = successor.ok_or(BooleanRegionError2::InvariantViolation)?;
        next[h] = successor;
        predecessors[successor] = predecessors[successor]
            .checked_add(1)
            .ok_or(BooleanRegionError2::InvariantViolation)?;
        if predecessors[successor] > 1 {
            return Err(BooleanRegionError2::InvariantViolation);
        }
    }
    let mut visited = region_reserved(half_count)?;
    visited.resize(half_count, false);
    let mut boundary = region_reserved(boundary_count)?;
    let offset_capacity = boundary_count
        .min(limits.max_boundary_walks)
        .checked_add(1)
        .ok_or(BooleanRegionError2::LimitExceeded(
            BooleanRegionResource2::BoundaryWalks,
        ))?;
    let mut offsets = region_reserved(offset_capacity)?;
    offsets.push(0);
    for start in 0..half_count {
        if !is_boundary[start] || visited[start] {
            continue;
        }
        if offsets.len() > limits.max_boundary_walks {
            return Err(BooleanRegionError2::LimitExceeded(
                BooleanRegionResource2::BoundaryWalks,
            ));
        }
        let mut current = start;
        loop {
            if current >= half_count || !is_boundary[current] {
                return Err(BooleanRegionError2::InvariantViolation);
            }
            if visited[current] {
                if current != start {
                    return Err(BooleanRegionError2::InvariantViolation);
                }
                break;
            }
            visited[current] = true;
            boundary.push(current);
            current = next[current];
        }
        offsets.push(boundary.len());
    }
    Ok(BooleanRegion2 {
        faces,
        selected,
        boundary,
        offsets,
    })
}
