// SPDX-License-Identifier: Apache-2.0

use rcct::planar::{Aabb2, Aabb2Error, Point2};

fn p(x: f64, y: f64) -> Point2 {
    Point2::try_new(x, y).unwrap()
}

fn bounds(x0: f64, y0: f64, x1: f64, y1: f64) -> Aabb2 {
    Aabb2::try_new(p(x0, y0), p(x1, y1)).unwrap()
}

#[test]
fn validates_order_and_builds_from_finite_points() {
    assert_eq!(Aabb2::from_points([]), None);
    assert_eq!(
        Aabb2::try_new(p(1.0, 0.0), p(0.0, 1.0)),
        Err(Aabb2Error::InvertedBounds)
    );
    assert_eq!(
        Aabb2::try_new(p(0.0, 1.0), p(1.0, 0.0)),
        Err(Aabb2Error::InvertedBounds)
    );
    let b = Aabb2::from_points([p(2.0, -1.0), p(-4.0, 3.0), p(0.0, 0.0)]).unwrap();
    assert_eq!(b.min(), p(-4.0, -1.0));
    assert_eq!(b.max(), p(2.0, 3.0));
    assert!(b.contains(p(-4.0, 3.0)));
    assert!(!b.contains(p(-4.0, 4.0)));
}

#[test]
fn contacts_and_degenerate_boxes_are_closed() {
    let a = bounds(0.0, 0.0, 1.0, 1.0);
    let line = bounds(1.0, -1.0, 1.0, 2.0);
    assert_eq!(a.intersection(line), Some(bounds(1.0, 0.0, 1.0, 1.0)));
    assert_eq!(
        a.intersection(bounds(1.0, 1.0, 2.0, 2.0)),
        Some(bounds(1.0, 1.0, 1.0, 1.0))
    );
    assert_eq!(a.intersection(bounds(2.0, 0.0, 3.0, 1.0)), None);
    assert!(bounds(-0.0, 0.0, 0.0, -0.0).contains(p(0.0, -0.0)));
}

#[test]
fn extreme_coordinates_are_selected_without_arithmetic_overflow() {
    let max = f64::MAX;
    let tiny = f64::from_bits(1);
    let a = bounds(-max, -tiny, 0.0, tiny);
    let b = bounds(0.0, -max, max, max);
    let united = a.union(b);
    assert_eq!(united.min(), p(-max, -max));
    assert_eq!(united.max(), p(max, max));
    assert_eq!(a.intersection(b), Some(bounds(0.0, -tiny, 0.0, tiny)));
}

#[test]
fn grid_boxes_obey_union_and_intersection_laws() {
    let boxes: Vec<_> = (-2..=2)
        .flat_map(|x0| {
            (-2..=2).flat_map(move |x1| {
                (-2..=2).flat_map(move |y0| {
                    (-2..=2)
                        .filter(move |&y1| x0 <= x1 && y0 <= y1)
                        .map(move |y1| bounds(x0 as f64, y0 as f64, x1 as f64, y1 as f64))
                })
            })
        })
        .collect();
    for &a in &boxes {
        for &b in &boxes {
            let u = a.union(b);
            assert_eq!(u, b.union(a));
            assert!(u.contains(a.min()) && u.contains(a.max()));
            assert!(u.contains(b.min()) && u.contains(b.max()));
            assert_eq!(a.intersection(b), b.intersection(a));
            if let Some(i) = a.intersection(b) {
                assert!(a.contains(i.min()) && a.contains(i.max()));
                assert!(b.contains(i.min()) && b.contains(i.max()));
            }
        }
    }
}
