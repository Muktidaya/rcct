# rcct

An independent **pure-Rust CAD-kernel project**, starting with a small geometry
foundation and using Open CASCADE Technology (OCCT) as a functional reference.

**Status: unreleased `0.2.0` release preparation.**
The latest published version remains `0.1.1`. The crate provides elementary 3D
geometry plus a first checked planar geometry and topology slice. It is not a
complete CAD kernel, is not API-compatible with OCCT, and cannot import, model,
mesh, or export CAD solids. There are no C++ bindings, native OCCT dependencies,
build scripts, or third-party Rust dependencies.

## Use

The 0.2.0 checkout has not been published. For local development, reference
the checkout by path:

```toml
[dependencies]
rcct = { path = "../rcct" }
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
| `Arrangement2` | A bounded graph of raw straight segments | Face-based Boolean selection is built separately for documented walk types. |

Frames and units are caller-owned, not encoded in these types. The source-level
rustdoc includes an executable `end - start` example and explains why floating-
point translation and subtraction need not undo each other exactly. Source
comments also explain the numerical reasons behind normalization's validation
order and direct component division.

- [Numerical contract](docs/numerics.md): operation semantics, conditioning,
  exceptional inputs, and normalization limits.
- [Checked spatial primitives](docs/spatial.md): finite 3D values, closed boxes,
  and explicitly rounded line, plane, and quadratic curve evaluations.
- [Combinatorial topology](docs/topology.md): scoped identity, declared boundaries,
  edge-use incidence and vertex-link diagnostics without geometric validity claims.
- [Proposed polyhedral Boolean contract](docs/polyhedral-boolean-contract.md):
  intended convex-input milestone, failure and validation boundaries.
- [Raw segment and plane events](docs/segment-plane3.md): exact event classes
  and retained original-support crossing witnesses.
- [Exact 3D orientation](docs/orientation3.md): raw finite-input determinant
  sign, fixed-width bounds and constructed-point limitations.
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
  a restricted bridge-aware path, exact containment, winding and limits.
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

This preparation checkout is an original Rust implementation of geometry foundations.
OCCT is a functional reference, not a runtime dependency or an architectural
constraint. The goal is a Rust-native design that can improve on reference
implementations rather than reproduce their source, API, or internal structure.

- **v0.1.1:** published elementary 3D point/vector/direction API, numerical
  contracts, and independent verification; it supports Rust 1.85.
- **v0.2.0 in progress:** a first bounded slice adds checked finite planar
  points and segments, exact orientation, exact segment-intersection event
  classification, and simple closed-wire validation. A bounded raw-segment graph retains shared points,
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

The project pins an exact compiler in `rust-toolchain.toml` for reproducible
build and release preparation. The current pin is recorded in the
[toolchain provenance](docs/toolchain-provenance.md). The proposed 0.2.0
`rust-version` is 1.100.0; this is a consumer compatibility change from
0.1.1's 1.85 minimum and has not yet been qualified. The pin does not change
the machine-wide default. Confirm official stable metadata at release time:

```sh
python3 tools/release_toolchain.py --check
# Only when deliberately refreshing the pin:
python3 tools/release_toolchain.py --update
```

The update command changes only the project pin and provenance, not
`rust-version`; review that separate compatibility decision and diff. If the
minimum differs from the new pin, add a separately qualified minimum lane or
revise the minimum before claiming support. Manifest and exact-candidate
verification are pending.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --locked --all-features --release
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
python3 tools/generate_numeric_fixtures.py --check
python3 tools/generate_orientation_fixtures.py --check
python3 tools/generate_orientation3_fixtures.py --check
python3 tools/generate_segment_plane_fixtures.py --check
python3 tools/generate_crossing_order_fixtures.py --check
python3 tools/generate_parameter_enclosures.py --check
python3 tools/generate_crossing_coordinate_enclosures.py --check
python3 tools/generate_crossing_point_order.py --check
python3 tools/generate_constructed_orientation.py --check
python3 tools/generate_span_query_fixtures.py --check
python3 tools/generate_boolean_triangle_fixture.py --check
python3 tools/generate_bezier3_fixtures.py --check
python3 tools/check_package.py
```

CI is configured for the explicit project pin across Linux, macOS and Windows,
including Linux/macOS ARM64, with debug and release tests. Earlier verification
records apply only to their exact revisions; this candidate needs its own
qualification. No OCCT installation is required. Fixture, package and pin tools
use Python 3.9+ standard-library modules. The package checker and release
pin tool require Python 3.11+ for the standard-library TOML parser.

## Assurance boundaries

Open source makes implementations inspectable; it does not establish correctness,
security, or suitability for safety-critical use. This release has no safety
qualification, formal verification, independent security audit, or CAD-kernel
validation. Do not infer any such assurance from its tests or its OCCT target.

The development direction is to make numerical assumptions, source provenance,
and operation-specific acceptance evidence traceable. Current tests cover the documented bounded geometry, predicate, and topology
slices; they do not establish general CAD or solid validity. Higher-level algorithms will need their own error
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
