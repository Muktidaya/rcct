// SPDX-License-Identifier: Apache-2.0

//! Experimental, dependency-free geometry foundations for a pure-Rust CAD kernel.
//!
//! The `gp` primitives share a right-handed frame and caller-chosen units.
//! [`planar`] builds bounded segment arrangements and selects faces by winding.
//! [`spatial`] provides checked 3D primitives and exact raw-input predicates,
//! and [`topology`] describes combinatorial boundaries and incidence.
//!
//! The experimental [`polyhedral`] module admits convex halfspace operands
//! and exact indexed planar-face boundaries. [`polyhedral::boolean3`] takes
//! convex operands. [`polyhedral::boolean_rectilinear3`] accepts boundaries
//! built from axis-aligned rectangular patches, including nonconvex results
//! from earlier operations. [`polyhedral::boolean_prismatic3`] accepts horizontal
//! caps and vertical planar walls on independent supports. All three use checked
//! fixed-width rational arithmetic and return owned, in-memory boundaries.
//!
//! Use [`gp::Point3`] for locations, [`gp::Vector3`] for displacements, and
//! [`gp::UnitDirection3`] when magnitude should be discarded after validation.
//! Coordinate frames and units are not encoded in the types: callers must
//! reconcile them before combining values. See the [geometry module](gp) for
//! 3D operation-specific numerical limits, and the [planar module](planar) for
//! checked finite points, exact orientation, and wire-validation contracts.
//!
//! ```
//! use rcct::gp::{Point3, UnitDirection3, Vector3};
//!
//! let start = Point3::ORIGIN;
//! let displacement = Vector3::new(3.0, 4.0, 0.0);
//! let end = start + displacement;
//! assert_eq!(start.distance(end), 5.0);
//! let direction = UnitDirection3::try_from(displacement)?;
//! assert!((direction.as_vector().norm() - 1.0).abs() < 1e-15);
//! # Ok::<(), rcct::gp::DirectionError>(())
//! ```
//!
//! The initial planar slice accepts finite points and classifies exact
//! orientation without choosing a tolerance:
//!
//! ```
//! use rcct::planar::{orient2d, Orientation2, Point2};
//!
//! let a = Point2::try_new(0.0, 0.0).unwrap();
//! let b = Point2::try_new(1.0, 0.0).unwrap();
//! let c = Point2::try_new(0.0, 1.0).unwrap();
//! assert_eq!(orient2d(a, b, c), Orientation2::Counterclockwise);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod gp;
pub mod planar;
pub mod polyhedral;
pub mod spatial;
pub mod topology;
