// SPDX-License-Identifier: Apache-2.0

use rcct::gp::{Point3, UnitDirection3, Vector3};

fn main() -> Result<(), rcct::gp::DirectionError> {
    let start = Point3::new(1.0, 2.0, 3.0);
    let displacement = Vector3::new(3.0, 4.0, 0.0);
    let end = start + displacement;
    let direction = UnitDirection3::try_from(displacement)?;
    println!("start={start:?}, end={end:?}");
    println!("distance={}, direction={direction:?}", start.distance(end));
    Ok(())
}
