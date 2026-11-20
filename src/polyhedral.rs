// SPDX-License-Identifier: Apache-2.0

//! Regularized solid operations on bounded polyhedra.
//!
//! Integer plane coefficients describe exact rational planes. Construction and
//! comparison use checked rational arithmetic throughout. A capacity failure
//! aborts the operation before it returns a boundary. All inputs stay immutable.
//! An operation may produce a nonconvex solid, a cavity, or disjoint shells.
//! Use [`SolidBoundary::as_convex`] to check a result before using it as another
//! convex operand. [`SolidBoundary::from_indexed_faces`] also accepts exact
//! rational vertices and oriented convex face loops with shared index seams.
//! For rectangular patches parallel to the coordinate planes, [`boolean_rectilinear3`]
//! also accepts nonconvex results directly. [`boolean_prismatic3`] admits
//! horizontal caps and vertical walls with rational plane coefficients. Empty solids
//! are represented explicitly.
//!
//! Work and storage limits apply to one or more calls through the same meter.
//! Exact arithmetic has a fixed 128-bit representation and checked intermediates.
//! See the [polyhedral contract](https://github.com/Muktidaya/rcct/blob/root/docs/polyhedral-boolean-contract.md)
//! for the intended domain and acceptance cases.

use std::sync::atomic::{AtomicBool, Ordering};

mod admit;
mod boundary;
#[cfg(test)]
mod cancellation;
mod exact;
mod prismatic;
mod rectilinear;
mod select;

pub use admit::{ConvexSolid, Plane3h};
pub use boundary::{IndexedFace, SolidBoundary};
pub use prismatic::boolean_prismatic3;
pub use rectilinear::boolean_rectilinear3;

use exact::P;

type R<T> = Result<T, PolyhedralError>;

#[derive(Debug)]
struct Facet {
    plane: Plane3h,
    vertices: Vec<P>,
}

/// Failure while admitting a convex operand, constructing a boundary or reusing a result.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PolyhedralError {
    /// A supplied plane has a zero normal.
    InvalidPlane,
    /// The supplied halfspaces do not describe a nonempty full-dimensional convex operand.
    InvalidOperand,
    /// The operand has a recession direction.
    UnboundedOperand,
    /// Exact arithmetic exceeded the supported representation or intermediate width.
    ArithmeticLimit,
    /// The work or requested storage budget was exhausted, or allocation failed.
    ResourceLimit,
    /// The caller requested cancellation at a work checkpoint.
    Cancelled,
    /// The proposed boundary failed a geometry or subdivision consistency check.
    InvalidBoundary,
    /// Aggregate incidence fails the closed orientable manifold requirement.
    NonManifoldResult,
    /// A valid result cannot be reused as a convex operand.
    UnsupportedNonconvex,
    /// A boundary patch falls outside the selected operation's face domain.
    UnsupportedFaceGeometry,
}

impl std::fmt::Display for PolyhedralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PolyhedralError {}

/// Shared budget for bounded loop work and cumulative requested heap storage.
///
/// Units count implementation checkpoints and requested vector backing bytes.
/// Storage charges are cumulative, including superseded buffers. This is a
/// conservative logical charge, without a claim about allocator overhead or RSS.
/// A completed charge is retained when later work fails. A failed charge leaves
/// the remaining allowance for that resource untouched.
#[derive(Debug)]
pub struct PolyhedralWork<'a> {
    work_left: usize,
    bytes_left: usize,
    cancellation: Option<&'a AtomicBool>,
    #[cfg(test)]
    probe: Option<cancellation::Probe>,
}

impl<'a> PolyhedralWork<'a> {
    /// Creates a work meter with explicit limits and cancellation disabled.
    pub fn new(max_work: usize, max_bytes: usize) -> Self {
        Self {
            work_left: max_work,
            bytes_left: max_bytes,
            cancellation: None,
            #[cfg(test)]
            probe: None,
        }
    }

    /// Checks the supplied flag at work and allocation checkpoints.
    pub fn with_cancellation(mut self, flag: &'a AtomicBool) -> Self {
        self.cancellation = Some(flag);
        self
    }

    /// Returns the remaining work units and remaining logical allocation bytes.
    pub fn remaining(&self) -> (usize, usize) {
        (self.work_left, self.bytes_left)
    }

    #[cfg_attr(test, track_caller)]
    fn charge(&mut self, units: usize) -> R<()> {
        #[cfg(test)]
        if let Some(probe) = &mut self.probe {
            probe.observe(
                std::panic::Location::caller(),
                (self.work_left, self.bytes_left),
                self.cancellation,
            );
        }
        if self
            .cancellation
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Err(PolyhedralError::Cancelled);
        }
        self.work_left = self
            .work_left
            .checked_sub(units)
            .ok_or(PolyhedralError::ResourceLimit)?;
        Ok(())
    }

    #[cfg_attr(test, track_caller)]
    fn reserve<T>(&mut self, vec: &mut Vec<T>, additional: usize) -> R<()> {
        self.charge(1)?;
        let needed = vec
            .len()
            .checked_add(additional)
            .ok_or(PolyhedralError::ResourceLimit)?;
        if needed > vec.capacity() {
            let bytes = needed
                .checked_mul(std::mem::size_of::<T>())
                .ok_or(PolyhedralError::ResourceLimit)?;
            self.bytes_left = self
                .bytes_left
                .checked_sub(bytes)
                .ok_or(PolyhedralError::ResourceLimit)?;
            vec.try_reserve_exact(additional)
                .map_err(|_| PolyhedralError::ResourceLimit)?;
        }
        Ok(())
    }
}

/// Boolean set operation, regularized by taking closure of the interior.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Operation3 {
    /// All material from either operand.
    Union,
    /// Material common to both operands.
    Intersection,
    /// Material of the first operand after removing the second.
    Difference,
}

impl Operation3 {
    fn accepts(self, a: bool, b: bool) -> bool {
        match self {
            Self::Union => a || b,
            Self::Intersection => a && b,
            Self::Difference => a && !b,
        }
    }
}

/// Applies the requested regularized set operation to two admitted convex operands.
///
/// For example, face-only contact has an empty volume intersection. A union
/// whose shells meet only at an edge or vertex returns a typed manifold error.
pub fn boolean3(
    a: &ConvexSolid,
    b: &ConvexSolid,
    op: Operation3,
    work: &mut PolyhedralWork<'_>,
) -> R<SolidBoundary> {
    work.charge(1)?;
    let faces = select::select(a, b, op, work)?;
    SolidBoundary::from_facets(faces, work)
}

impl SolidBoundary {
    /// Re-admits the empty result or an owned convex intersection for subsequent operations.
    ///
    /// Every vertex must satisfy every outward facet halfspace. Given a
    /// validated, embedded closed boundary, a supporting plane that excludes a
    /// vertex witnesses nonconvexity. Admission rebuilds the convex face loops,
    /// removing coplanar subdivisions.
    pub fn as_convex(&self, work: &mut PolyhedralWork<'_>) -> R<ConvexSolid> {
        work.charge(1)?;
        if self.is_empty() {
            return Ok(ConvexSolid::empty());
        }
        let mut planes = Vec::new();
        work.reserve(&mut planes, self.faces.len())?;
        for face in &self.faces {
            for &vertex in &self.vertices {
                work.charge(1)?;
                if face.plane.eval(vertex)?.sign() > 0 {
                    return Err(PolyhedralError::UnsupportedNonconvex);
                }
            }
            planes.push(face.plane);
        }
        ConvexSolid::from_halfspaces(&planes, work)
    }
}
