# rcct

An independent **pure-Rust CAD-kernel project**, starting with a small geometry
foundation and using Open CASCADE Technology (OCCT) as a functional reference.

**Status: early development.** Version 0.1.0 provides tested 3D points, vectors,
and checked unit directions. It is not a CAD kernel yet, is not API-compatible
with OCCT, and cannot import, model, mesh, or export CAD solids. There are no C++
bindings, native OCCT dependencies, build scripts, or third-party Rust dependencies.

## Use

```toml
[dependencies]
rcct = "0.1"
```

```rust
use rcct::gp::{Point3, UnitDirection3, Vector3};

let displacement = Vector3::new(3.0, 4.0, 0.0);
let destination = Point3::ORIGIN + displacement;
assert_eq!(destination.distance(Point3::ORIGIN), 5.0);

let direction = UnitDirection3::try_from(displacement).expect("finite, nonzero vector");
assert!((direction.as_vector().norm() - 1.0).abs() < 1e-14);
```

The `gp` module distinguishes points, displacement vectors, and unit directions.
Point and vector coordinates are ordinary `f64` values and permit nonfinite
inputs. Direction construction rejects zero and nonfinite vectors. Units are
chosen by the caller; there is no implicit tolerance or robust-predicate claim.
See the API documentation for numerical behavior and exceptional cases.

## Scope and development direction

The current release is an original Rust implementation of elementary geometry.
OCCT is a functional reference, not a runtime dependency or an architectural
constraint. The goal is a Rust-native design that can improve on reference
implementations rather than reproduce their source, API, or internal structure. The intended direction is geometry and numerical foundations,
then curves and surfaces, topology and boundary representation, modeling
algorithms, and exchange formats. These later layers are unimplemented; this
outline is not a compatibility guarantee or delivery schedule.

Development should preserve explicit ownership, geometric invariants, numerical
contracts, and source provenance. Implementations must remain independent and
compatible with this project's permissive license. Do not copy or transliterate
LGPL-licensed OCCT source. See
[CONTRIBUTING.md](https://github.com/Muktidaya/rcct/blob/root/CONTRIBUTING.md).

## Build and check

Requires Rust 1.85 or newer (edition 2024).

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo package --locked
```

CI checks the declared minimum Rust version and stable Rust. This crate does not
require an OCCT installation or a C++ toolchain.

## Assurance boundaries

Open source makes implementations inspectable; it does not establish correctness,
security, or suitability for safety-critical use. This release has no safety
qualification, formal verification, independent security audit, or CAD-kernel
validation. Do not infer any such assurance from its tests or its OCCT target.

The development direction is to make numerical assumptions, source provenance,
and operation-specific acceptance evidence traceable. Current tests cover the
small geometry API only. Higher-level algorithms will need their own error
analysis, degeneracy cases, differential checks where appropriate, and explicit
qualification work for any regulated use.

## License and attribution

Licensed under the **Apache License, Version 2.0** (`Apache-2.0`). See
[LICENSE](LICENSE) and [NOTICE.md](NOTICE.md).

The initial implementation is original Rust code implementing standard
Cartesian geometry. No OCCT source was copied, translated, or linked into it.
OCCT's own license remains separate; a language rewrite alone does not remove
copyright obligations. Contributions must preserve this independent,
permissively licensed implementation boundary.

[Open CASCADE Technology](https://github.com/Open-Cascade-SAS/OCCT) is developed by
Open Cascade SAS and its contributors. `rcct` is an independent project, not an
official Open Cascade release and not affiliated with or endorsed by Open
Cascade SAS. Upstream names identify the reference project; no upstream logos
are used.
