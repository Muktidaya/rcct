# rcct

An independent **pure-Rust CAD-kernel project**, starting with a small geometry
foundation and using Open CASCADE Technology (OCCT) as a functional reference.

**Status: early development, v0.1.1 release candidate.** The crate provides 3D points, vectors,
and checked unit directions. It is not a CAD kernel yet, is not API-compatible
with OCCT, and cannot import, model, mesh, or export CAD solids. There are no C++
bindings, native OCCT dependencies, build scripts, or third-party Rust dependencies.

## Use

```toml
[dependencies]
rcct = "0.1.1"
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
See the API documentation and [numerical contract](docs/numerics.md) for
operation-specific behavior, conditioning, exceptional cases, and units.
[Verification notes](docs/verification.md) distinguish tested acceptance budgets
from universal guarantees and record which platforms were actually run.

## Documentation and examples

Choose the type by what the value represents:

| Type | Meaning | Important responsibility |
| --- | --- | --- |
| `Point3` | A location in a chosen frame | Subtract another point to obtain a displacement; do not add or scale locations as vectors. |
| `Vector3` | A displacement with magnitude | Retain it for translation; its components are unchecked `f64` values. |
| `UnitDirection3` | A checked direction with magnitude discarded | Construction rejects zero/nonfinite vectors; its components are dimensionless. |

Frames and units are caller-owned, not encoded in these types. The source-level
rustdoc includes an executable `end - start` example and explains why floating-
point translation and subtraction need not undo each other exactly. Source
comments also explain the numerical reasons behind normalization's validation
order and direct component division.

- [Numerical contract](docs/numerics.md): operation semantics, conditioning,
  exceptional inputs, and normalization limits.
- [Verification record](docs/verification.md): independent references,
  acceptance budgets, tagged-source compatibility checks, and exact CI evidence.
- [Contribution guide](CONTRIBUTING.md): checks and source-provenance requirements.
- [Displacement example](examples/displacement.rs): a small runnable translation,
  distance, and direction example.

```sh
cargo run --example displacement
cargo test --doc
cargo doc --no-deps --all-features
```

Open `target/doc/rcct/index.html` for documentation built from the current
checkout. The hosted [API documentation](https://docs.rs/rcct) describes the
published crate and may lag this release candidate.

## Scope and development direction

The current release is an original Rust implementation of elementary geometry.
OCCT is a functional reference, not a runtime dependency or an architectural
constraint. The goal is a Rust-native design that can improve on reference
implementations rather than reproduce their source, API, or internal structure.

- **v0.1.1:** numerical contracts and independent verification of the existing
  point/vector/direction API, preserving source compatibility and Rust 1.85.
- **v0.2.0 planned scope:** a broad geometry API, boundary representation (B-rep),
  Boolean operations, and migration of the supported toolchain/minimum to
  Rust 1.99.1. These capabilities are not implemented by v0.1.1.
- **Later direction:** additional modeling algorithms and exchange formats.

The roadmap is a statement of intended scope, not an OCCT compatibility promise,
safety assurance, completed implementation, or delivery schedule.

Development should preserve explicit ownership, geometric invariants, numerical
contracts, and source provenance. Implementations must remain independent and
compatible with this project's permissive license. Do not copy or transliterate
LGPL-licensed OCCT source. See
[CONTRIBUTING.md](https://github.com/Muktidaya/rcct/blob/root/CONTRIBUTING.md).

## Build and check

The repository pins **Rust 1.99.1**, the latest stable release verified on
2026-11-09 against the [official stable channel manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml).
`rust-toolchain.toml` selects the minimal profile plus `rustfmt` and Clippy;
ordinary `cargo` commands use that project toolchain without changing the
machine-wide default. This development-toolchain selection does **not** raise
v0.1.1's supported minimum: `rust-version = "1.85"` preserves the published
v0.1.0 minimum (edition 2024). CI and local checks exercise that minimum
separately. Making Rust 1.99.1 the required minimum is planned for v0.2.0.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --locked --all-features --release
cargo +1.85.0 test --locked --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
python3 tools/generate_numeric_fixtures.py --check
python3 tools/check_package.py
```

CI configures the Rust 1.85 minimum, pinned Rust 1.99.1, and rolling stable
channel across Linux, macOS, and Windows, including Linux/macOS ARM64, with
debug and release tests. The numerical-verification commit passed all 14 hosted
jobs; see the [verification record](docs/verification.md) for the exact revision,
run link, and the requirement to recheck later commits. This crate does
not require an OCCT installation or a C++ toolchain. The fixture generator and
package/downstream checker use Python 3.9+ standard-library tools only.

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
