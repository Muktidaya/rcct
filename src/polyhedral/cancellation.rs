// SPDX-License-Identifier: Apache-2.0
//! Deterministic cancellation at real meter polls inside complete Boolean calls.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::panic::Location;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{
    Operation3, PolyhedralError, PolyhedralWork, R, SolidBoundary, boolean_prismatic3,
    boolean_rectilinear3, boolean3,
};

mod fixtures;
use fixtures::{assert_result, boundary, boundary_snapshot, convex_snapshot, cube};

type Site = (&'static str, u32, u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Checkpoint {
    ordinal: usize,
    site: Site,
    remaining: (usize, usize),
}

#[derive(Debug)]
enum Action {
    Record(BTreeMap<Site, (Checkpoint, Checkpoint)>),
    CancelAt {
        target: Checkpoint,
        fired: Option<Checkpoint>,
    },
}

#[derive(Debug)]
pub(super) struct Probe {
    ordinal: usize,
    action: Action,
}

impl Probe {
    // Called immediately before the existing flag load. This probe sets the
    // caller's real flag; only the ordinary meter can return an error.
    pub(super) fn observe(
        &mut self,
        caller: &'static Location<'static>,
        remaining: (usize, usize),
        flag: Option<&AtomicBool>,
    ) {
        self.ordinal += 1;
        let current = Checkpoint {
            site: (caller.file(), caller.line(), caller.column()),
            ordinal: self.ordinal,
            remaining,
        };
        match &mut self.action {
            Action::Record(sites) => {
                sites.entry(current.site).or_insert((current, current)).1 = current;
            }
            Action::CancelAt { target, fired } if current.ordinal == target.ordinal => {
                assert_eq!(current, *target, "execution diverged before cancellation");
                *fired = Some(current);
                flag.expect("replay requires the caller's cancellation flag")
                    .store(true, Ordering::Relaxed);
            }
            Action::CancelAt { .. } => {}
        }
    }
}

const WORK: usize = 100_000_000;
const BYTES: usize = 128 * 1024 * 1024;

fn verify_call<S: Debug + Eq>(
    path: &str,
    op: Operation3,
    apply: impl Fn(&mut PolyhedralWork<'_>) -> R<SolidBoundary>,
    inputs: impl Fn() -> S,
) {
    let original_inputs = inputs();
    let reference_flag = AtomicBool::new(false);
    let mut reference = PolyhedralWork::new(WORK, BYTES).with_cancellation(&reference_flag);
    reference.probe = Some(Probe {
        ordinal: 0,
        action: Action::Record(BTreeMap::new()),
    });
    let answer = apply(&mut reference).unwrap();
    assert!(!reference_flag.load(Ordering::Relaxed));
    assert_result(&answer, op);
    let expected = boundary_snapshot(&answer);
    let probe = reference.probe.take().unwrap();
    let Action::Record(sites) = probe.action else {
        unreachable!()
    };

    // The first and last occurrence distinguish early and late work at a
    // reused poll. A site reached once needs only one replay. The number of
    // replays grows with the code paths exercised, not with the facet count.
    let mut targets: Vec<_> = sites.into_values().flat_map(|(a, b)| [a, b]).collect();
    targets.sort_unstable_by_key(|c| c.ordinal);
    targets.dedup_by_key(|c| c.ordinal);
    assert_eq!(targets.first().unwrap().ordinal, 1);
    assert_eq!(targets.last().unwrap().ordinal, probe.ordinal);
    for target in targets {
        let flag = AtomicBool::new(false);
        let mut meter = PolyhedralWork::new(WORK, BYTES).with_cancellation(&flag);
        meter.probe = Some(Probe {
            ordinal: 0,
            action: Action::CancelAt {
                target,
                fired: None,
            },
        });
        assert_eq!(apply(&mut meter).unwrap_err(), PolyhedralError::Cancelled);
        assert!(flag.load(Ordering::Relaxed));
        assert_eq!(
            meter.remaining(),
            target.remaining,
            "charges before the refused poll"
        );
        let stopped = meter.probe.take().unwrap();
        assert_eq!(stopped.ordinal, target.ordinal);
        let Action::CancelAt { fired, .. } = stopped.action else {
            unreachable!()
        };
        assert_eq!(fired, Some(target));
        assert_eq!(inputs(), original_inputs);

        flag.store(false, Ordering::Relaxed);
        let mut fresh = PolyhedralWork::new(WORK, BYTES).with_cancellation(&flag);
        let retry = apply(&mut fresh).unwrap();
        assert_eq!(boundary_snapshot(&retry), expected);
        let (file, line, column) = target.site;
        println!(
            "P20_CHECK path={path} op={op:?} ordinal={} site={file}:{line}:{column} work={} bytes={}",
            target.ordinal,
            WORK - target.remaining.0,
            BYTES - target.remaining.1
        );
    }
}

#[test]
fn full_boolean_cancellation_checkpoints() {
    let (a, b) = (cube(0, 2), cube(1, 3));
    let (left, right) = (boundary(&a), boundary(&b));
    for op in [
        Operation3::Union,
        Operation3::Intersection,
        Operation3::Difference,
    ] {
        verify_call(
            "convex",
            op,
            |meter| boolean3(&a, &b, op, meter),
            || (convex_snapshot(&a), convex_snapshot(&b)),
        );
        verify_call(
            "prismatic",
            op,
            |meter| boolean_prismatic3(&left, &right, op, meter),
            || (boundary_snapshot(&left), boundary_snapshot(&right)),
        );
        verify_call(
            "rectilinear",
            op,
            |meter| boolean_rectilinear3(&left, &right, op, meter),
            || (boundary_snapshot(&left), boundary_snapshot(&right)),
        );
    }
}
