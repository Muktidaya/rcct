// SPDX-License-Identifier: Apache-2.0

//! Translate a location by a displacement and inspect its length and direction.
//! All coordinates below use the same caller-chosen unit and frame; the checked
//! direction is dimensionless and no longer contains the displacement's length.

use rcct::gp::{Point3, UnitDirection3, Vector3};

fn main() -> Result<(), rcct::gp::DirectionError> {
    let start = Point3::new(1.0, 2.0, 3.0);
    let displacement = Vector3::new(3.0, 4.0, 0.0);
    let end = start + displacement;
    // Translation needs the full displacement, not just its unit direction.
    // Direction construction is fallible even though point/vector construction
    // accepts any f64; `?` propagates zero-vector and nonfinite-input errors.
    let direction = UnitDirection3::try_from(displacement)?;
    println!("start={start:?}, end={end:?}");
    println!("distance={}, direction={direction:?}", start.distance(end));
    Ok(())
}
