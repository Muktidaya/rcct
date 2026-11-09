// SPDX-License-Identifier: Apache-2.0

//! Experimental, dependency-free geometry foundations for a pure-Rust CAD kernel.
//!
//! This crate implements Cartesian primitives, not a complete CAD
//! kernel or an API-compatible replacement for Open CASCADE Technology (OCCT).
//! Coordinates have caller-chosen units and share one right-handed frame.
//!
//! Use [`gp::Point3`] for locations, [`gp::Vector3`] for displacements, and
//! [`gp::UnitDirection3`] when magnitude should be discarded after validation.
//! Coordinate frames and units are not encoded in the types: callers must
//! reconcile them before combining values. See the [geometry module](gp) for
//! operation-specific numerical limits and unchecked floating-point behavior.
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

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod gp;
