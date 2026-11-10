# Changelog

## 0.2.0-alpha.1 (unreleased)

- Retain exact rational proper-crossing parameters and compare their order on
  the same directed raw-input segment; other intersection classes remain gated.
- Add checked finite 2D axis-aligned bounds, including closed intersections,
  unions, point containment, and construction from finite points.
- Compare all 5,184 pairs of directed segments on a small integer grid against
  an independent integer parametric intersection oracle.
- Begin the 0.2.0 line with checked finite planar points and segments, exact
  orientation predicates, segment-intersection event classification, and
  simple closed-wire validation. General B-rep and Boolean operations remain
  future milestones.
- Raise the unreleased package minimum to Rust 1.99.1 and retain the pinned
  development toolchain; published 0.1.1 remains supported on Rust 1.85.
- Add independent small-integer orientation-grid checks and Python
  `Fraction`-oracle binary64 cases, plus regressions for product overflow,
  underflow, and coordinate-subtraction overflow.

## 0.1.1 (published 2026-11-09)

- Specify operation-level numerical contracts, caller-defined units, nonfinite
  behavior, overflow/underflow, cancellation, and normalization limitations.
- Add reproducible independent high-precision reference fixtures and explicit
  acceptance budgets, plus adversarial exceptional-value regression tests.
- Expand debug/release verification across a configured Linux/macOS/Windows CI
  matrix, including ARM64, and add packaged-crate/downstream API smoke checks.
- Preserve the v0.1.0 public API and Rust 1.85 minimum. Development commands use
  the already-adopted Rust 1.99.1 pin without making it the package minimum.
- Document verification evidence and limitations without claiming safety
  qualification or robust geometric predicates.
- Add a source-comment/rustdoc pass explaining affine subtraction, frame/unit
  responsibilities, direction copies, and normalization maintenance hazards;
  update the README, contributor guidance, and verification record together.
  The additional example is doctested; runtime behavior is unchanged.

## 0.1.0

- Start an independent Apache-2.0 pure-Rust CAD-kernel project with original elementary
  3D point, vector, and checked unit-direction types.
- Document floating-point behavior and explicit early-stage scope.
- Add tests, doctests, packaging metadata, and minimum-version/stable CI.

No OCCT compatibility or CAD-kernel functionality is claimed by this release.
