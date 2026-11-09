# Changelog

## 0.1.1 (unreleased)

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

## Planned v0.2.0 scope

- Broad geometry API, boundary representation (B-rep), and Boolean operations.
- Required-toolchain/minimum migration to Rust 1.99.1.

These are planned work, not capabilities of the 0.1.1 release.

## 0.1.0

- Start an independent Apache-2.0 pure-Rust CAD-kernel project with original elementary
  3D point, vector, and checked unit-direction types.
- Document floating-point behavior and explicit early-stage scope.
- Add tests, doctests, packaging metadata, and minimum-version/stable CI.

No OCCT compatibility or CAD-kernel functionality is claimed by this release.
