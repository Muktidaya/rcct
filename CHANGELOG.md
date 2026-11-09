# Changelog

## Unreleased

- Pin the project toolchain to Rust 1.99.1 with rustfmt and Clippy, and raise the
  development minimum to match the latest stable release verified on 2026-11-09.
- Test the pinned version and rolling stable channel in CI. The published 0.1.0
  package and its original Rust 1.85 minimum are unchanged.

## 0.1.0

- Start an independent Apache-2.0 pure-Rust CAD-kernel project with original elementary
  3D point, vector, and checked unit-direction types.
- Document floating-point behavior and explicit early-stage scope.
- Add tests, doctests, packaging metadata, and minimum-version/stable CI.

No OCCT compatibility or CAD-kernel functionality is claimed by this release.
