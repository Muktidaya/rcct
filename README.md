# rcct

An independent **pure-Rust CAD-kernel project**, starting with a small geometry
foundation and using Open CASCADE Technology (OCCT) as a functional reference.

**Status: early development; the current unreleased package is `0.2.0-alpha.1`.**
The latest published version remains `0.1.1`. The crate provides elementary 3D
geometry plus a first checked planar geometry and topology slice. It is not a
complete CAD kernel, is not API-compatible with OCCT, and cannot import, model,
mesh, or export CAD solids. There are no C++ bindings, native OCCT dependencies,
build scripts, or third-party Rust dependencies.

## Use

```toml
[dependencies]
rcct = "0.2.0-alpha.1"
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
| `Point2` | A checked finite planar location | Construct with `Point2::try_new`; nonfinite coordinates are rejected. |
| `Aabb2` | Closed finite axis-aligned planar bounds | Degenerate boxes and boundary contact are allowed. |
| `Segment2` | A checked nonzero finite segment | Construction rejects equal endpoints. |
| `ClosedSegment2` | A finite segment set that may be a point | Point-segment contacts do not return a parameter. |
| `SimpleWire2` | A validated simple closed polygonal wire | Ordered vertices define an implicit closing edge. |
| `Arrangement2` | A bounded graph of raw straight segments | Simple-walk faces and face-based Boolean selections are built separately. |

Frames and units are caller-owned, not encoded in these types. The source-level
rustdoc includes an executable `end - start` example and explains why floating-
point translation and subtraction need not undo each other exactly. Source
comments also explain the numerical reasons behind normalization's validation
order and direct component division.

- [Numerical contract](docs/numerics.md): operation semantics, conditioning,
  exceptional inputs, and normalization limits.
- [Combinatorial topology](docs/topology.md): bounded graph incidence and
  directed vertex-simple wires, edge-use incidence, and vertex-link diagnostics
  without geometric validity claims.
- [Planar geometry slice](docs/planar.md): exact orientation, segment events,
  and simple closed-wire validation contracts.
- [Segment event witnesses](docs/crossing-witness.md): exact proper-crossing,
  endpoint-touch and overlap parameters, ordering, capacity assumptions, and limitations.
- [Repeated-cut prerequisites](docs/repeated-cut-prerequisites.md): exact
  constructed-input, resource and arrangement gates.
- [Bounded constructed predicates](docs/constructed-predicates.md): supported
  first-generation orientation and original-support span split contracts.
- [Original-support span intersections](docs/span-intersection.md): exact
  clipping, cross-support point location, query budgets and scope.
- [Planar arrangement graph](docs/planar-arrangement.md): shared vertices,
  coincident edges, source traversal, resource limits and failure semantics.
- [Planar graph bridges](docs/planar-bridges.md): bounded component and
  graph-edge bridge analysis, including coincident source uses.
- [Planar half-edge embedding](docs/planar-embedding.md): exact radial order,
  boundary walks, simple-walk orientation and limits.
- [Planar faces and winding](docs/planar-faces.md): simple-walk face grouping,
  exact hole containment, aggregate winding and unsupported topology.
- [Planar Boolean regions](docs/planar-boolean.md): per-group winding and
  face-based 2D selection, exact raw-point classification, limits and exclusions.
- [Arithmetic backend evaluation](docs/arithmetic-backend-evaluation.md):
  preliminary license, dependency and unsafe-code tradeoffs.
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
published crate and does not yet document this unreleased candidate.

## Scope and development direction

The current release is an original Rust implementation of elementary geometry.
OCCT is a functional reference, not a runtime dependency or an architectural
constraint. The goal is a Rust-native design that can improve on reference
implementations rather than reproduce their source, API, or internal structure.

- **v0.1.1:** published elementary 3D point/vector/direction API, numerical
  contracts, and independent verification; it supports Rust 1.85.
- **v0.2.0 in progress:** a first bounded slice adds checked finite planar
  points and segments, exact orientation, exact segment-intersection event
  classification, and simple closed-wire validation. The prerelease declares
  Rust 1.99.1 as its minimum. A bounded raw-segment graph retains shared points,
  edge pieces and source traversals. A separate stage groups simple walks into
  faces; per-group winding supports bounded face-based 2D Boolean selection.
- **Remaining v0.2.0 scope:** broader geometry, B-rep, and Boolean operations.
  General CAD and 3D Boolean operations require additional milestones.
- **Later direction:** additional modeling algorithms and exchange formats.

The roadmap is a statement of intended scope, not an OCCT compatibility promise,
safety assurance, completed implementation, or delivery schedule.

Development should preserve explicit ownership, geometric invariants, numerical
contracts, and source provenance. Implementations must remain independent and
compatible with this project's permissive license. Do not copy or transliterate
LGPL-licensed OCCT source. See
[CONTRIBUTING.md](https://github.com/Muktidaya/rcct/blob/root/CONTRIBUTING.md).

## Build and check

The repository pins **Rust 1.99.1** as both its development toolchain and
current package minimum, verified on 2026-11-09 against the [official stable
channel manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml).
`rust-toolchain.toml` selects the minimal profile plus `rustfmt` and Clippy;
ordinary `cargo` commands use that project toolchain without changing the
machine-wide default. Published v0.1.1 retains its Rust 1.85 minimum; the
unreleased 0.2.0 prerelease intentionally raises its minimum to Rust 1.99.1.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --locked --all-features --release
cargo +1.99.1 test --locked --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
python3 tools/generate_numeric_fixtures.py --check
python3 tools/generate_orientation_fixtures.py --check
python3 tools/generate_crossing_order_fixtures.py --check
python3 tools/generate_parameter_enclosures.py --check
python3 tools/generate_crossing_coordinate_enclosures.py --check
python3 tools/generate_crossing_point_order.py --check
python3 tools/generate_constructed_orientation.py --check
python3 tools/generate_span_query_fixtures.py --check
python3 tools/generate_boolean_triangle_fixture.py --check
python3 tools/check_package.py
```

CI configures pinned Rust 1.99.1 and rolling stable across Linux, macOS, and
Windows, including Linux/macOS ARM64, with debug and release tests. The previous
v0.1.1 verification record covers its exact published revision; this candidate
must establish its own CI evidence. See the [verification record](docs/verification.md).
This crate does
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
