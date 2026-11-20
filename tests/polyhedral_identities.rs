// SPDX-License-Identifier: Apache-2.0
//! Exact outward boundaries for idempotence, separation, face contact and slab removal.
#[path = "support/cuboid.rs"]
mod cuboid;
#[path = "support/oriented_boundary.rs"]
mod oriented_boundary;

use std::collections::{BTreeMap, BTreeSet};

use cuboid::Cuboid;
use rcct::polyhedral::{ConvexSolid, Operation3, PolyhedralWork, boolean3};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

const UNIT: Cuboid = Cuboid::new([0; 3], [1; 3]);

// Each expected cuboid is an entire, disconnected component. Matching the
// six oriented regions of each box locates the material on both sides of
// the gap, even when a plane carries patches from both surviving boxes.
fn check(a: &ConvexSolid, b: &ConvexSolid, op: Operation3, volume: i128, components: &[Cuboid]) {
    let result = boolean3(a, b, op, &mut work()).unwrap();
    let faces: Vec<_> = components.iter().flat_map(|b| b.faces()).collect();
    let allowed: BTreeSet<_> = faces.iter().map(|(plane, _)| *plane).collect();
    let mut edges = BTreeMap::new();
    for face in 0..result.face_count() {
        assert!(allowed.contains(&result.face_plane(face).unwrap().coefficients()));
        let ids = result.face_vertices(face).unwrap();
        assert!(ids.len() >= 3);
        for (&start, &end) in ids.iter().zip(ids.iter().cycle().skip(1)) {
            assert_ne!(start, end);
            let uses = edges
                .entry((start.min(end), start.max(end)))
                .or_insert([0, 0]);
            uses[usize::from(start < end)] += 1;
        }
    }
    assert!(edges.values().all(|uses| *uses == [1, 1]));
    oriented_boundary::assert_oriented_faces(&result, faces);
    assert_eq!(result.shell_count(), components.len());
    assert_eq!(result.volume(), (volume, 1));
    assert_eq!(result.is_empty(), components.is_empty());
    if components.is_empty() {
        assert_eq!((result.vertex_count(), result.face_count()), (0, 0));
    }
}

#[test]
fn identical_cubes_have_exactly_one_exterior_boundary_or_empty() {
    let a = UNIT.solid();
    check(&a, &a, Operation3::Union, 1, &[UNIT]);
    check(&a, &a, Operation3::Intersection, 1, &[UNIT]);
    check(&a, &a, Operation3::Difference, 0, &[]);
}

#[test]
fn separated_cubes_preserve_the_two_components_and_directed_difference() {
    let far = Cuboid::new([2; 3], [3; 3]);
    let (a, b) = (UNIT.solid(), far.solid());
    check(&a, &b, Operation3::Union, 2, &[UNIT, far]);
    check(&a, &b, Operation3::Intersection, 0, &[]);
    check(&a, &b, Operation3::Difference, 1, &[UNIT]);
}

#[test]
fn shared_face_cubes_merge_the_interface_and_preserve_the_first_difference() {
    let a = UNIT.solid();
    let b = Cuboid::new([1, 0, 0], [2, 1, 1]).solid();
    let joined = Cuboid::new([0; 3], [2, 1, 1]);
    check(&a, &b, Operation3::Union, 2, &[joined]);
    check(&a, &b, Operation3::Intersection, 0, &[]);
    check(&a, &b, Operation3::Difference, 1, &[UNIT]);
}

#[test]
fn through_slab_subtraction_retains_both_outward_oriented_boxes() {
    let a = Cuboid::new([0; 3], [3; 3]).solid();
    let cutter = Cuboid::new([1, -1, -1], [2, 4, 4]).solid();
    let left = Cuboid::new([0; 3], [1, 3, 3]);
    let right = Cuboid::new([2, 0, 0], [3; 3]);
    check(&a, &cutter, Operation3::Difference, 18, &[left, right]);
}
