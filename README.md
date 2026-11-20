# rcct

Geometry foundations for an independent **pure-Rust CAD kernel**.
The 0.2 series includes planar arrangements, checked 3D primitives,
combinatorial topology and experimental solid Booleans on exact planar faces.
The crate has no third-party dependencies. Open CASCADE Technology (OCCT)
serves as a functional reference.

Version 0.2.1 is a documentation patch for the geometry API published in
[0.2.0](https://crates.io/crates/rcct/0.2.0).

## Use

Add the following dependency to your application's `Cargo.toml`.
The 0.2 series requires **Rust 1.100.0** or newer. Existing `rcct::gp` imports
and constructors from 0.1.1 still apply. Projects that require Rust 1.85 can
continue using 0.1.1. [rcct on crates.io](https://crates.io/crates/rcct)

```toml
[dependencies]
rcct = "0.2.1"
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
See the API documentation and [numerical contract](https://github.com/Muktidaya/rcct/blob/root/docs/numerics.md) for
operation-specific behavior, conditioning, exceptional cases, and units.
[Verification notes](https://github.com/Muktidaya/rcct/blob/root/docs/verification.md) distinguish tested acceptance budgets
from universal guarantees and record which platforms were actually run.

### Use a source checkout

To work on a local copy, clone [the repository](https://github.com/Muktidaya/rcct)
beside your application and point Cargo at the directory containing the crate's
`Cargo.toml`.

```toml
[dependencies]
rcct = { path = "../rcct" }
```

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

- [Numerical contract](https://github.com/Muktidaya/rcct/blob/root/docs/numerics.md): operation semantics, conditioning,
  exceptional inputs, and normalization limits.
- [Checked spatial primitives](https://github.com/Muktidaya/rcct/blob/root/docs/spatial.md): finite 3D values, closed boxes,
  and explicitly rounded line, plane, and quadratic curve evaluations.
- [Combinatorial topology](https://github.com/Muktidaya/rcct/blob/root/docs/topology.md): scoped identity, declared boundaries,
  edge-use incidence and vertex-link diagnostics without geometric validity claims.
- [Polyhedral Boolean contract](https://github.com/Muktidaya/rcct/blob/root/docs/polyhedral-boolean-contract.md) describes
  convex halfspace operands, admission of rational vertices and indexed planar
  faces, and chaining operations on nonconvex solids with vertical walls.
- [Raw segment and plane events](https://github.com/Muktidaya/rcct/blob/root/docs/segment-plane3.md): exact event classes
  and retained original-support crossing witnesses.
- [Exact 3D orientation](https://github.com/Muktidaya/rcct/blob/root/docs/orientation3.md): raw finite-input determinant
  sign, fixed-width bounds and constructed-point limitations.
- [Planar geometry slice](https://github.com/Muktidaya/rcct/blob/root/docs/planar.md): exact orientation, segment events,
  and simple closed-wire validation contracts.
- [Segment event witnesses](https://github.com/Muktidaya/rcct/blob/root/docs/crossing-witness.md): exact proper-crossing,
  endpoint-touch and overlap parameters, ordering, capacity assumptions, and limitations.
- [Repeated-cut prerequisites](https://github.com/Muktidaya/rcct/blob/root/docs/repeated-cut-prerequisites.md): exact
  constructed-input, resource and arrangement gates.
- [Bounded constructed predicates](https://github.com/Muktidaya/rcct/blob/root/docs/constructed-predicates.md): supported
  first-generation orientation and original-support span split contracts.
- [Original-support span intersections](https://github.com/Muktidaya/rcct/blob/root/docs/span-intersection.md): exact
  clipping, cross-support point location, query budgets and scope.
- [Planar arrangement graph](https://github.com/Muktidaya/rcct/blob/root/docs/planar-arrangement.md): shared vertices,
  coincident edges, source traversal, resource limits and failure semantics.
- [Planar graph bridges](https://github.com/Muktidaya/rcct/blob/root/docs/planar-bridges.md): bounded component and
  graph-edge bridge analysis, including coincident source uses.
- [Planar half-edge embedding](https://github.com/Muktidaya/rcct/blob/root/docs/planar-embedding.md): exact radial order,
  boundary walks, simple-walk orientation and limits.
- [Planar faces and winding](https://github.com/Muktidaya/rcct/blob/root/docs/planar-faces.md): simple-walk face grouping,
  a restricted bridge-aware path, exact containment, winding and limits.
- [Planar Boolean regions](https://github.com/Muktidaya/rcct/blob/root/docs/planar-boolean.md): per-group winding and
  face-based 2D selection, exact raw-point classification, limits and exclusions.
- [Arithmetic backend evaluation](https://github.com/Muktidaya/rcct/blob/root/docs/arithmetic-backend-evaluation.md):
  preliminary license, dependency and unsafe-code tradeoffs.
- [Verification record](https://github.com/Muktidaya/rcct/blob/root/docs/verification.md): independent references,
  acceptance budgets, tagged-source compatibility checks, and exact CI evidence.
- [Contribution guide](https://github.com/Muktidaya/rcct/blob/root/CONTRIBUTING.md)
  covers checks and source-provenance requirements.
- [Displacement example](https://github.com/Muktidaya/rcct/blob/root/examples/displacement.rs): a small runnable translation,
  distance, and direction example.
- [Convex clipping example](https://github.com/Muktidaya/rcct/blob/root/examples/convex_clip.rs) intersects a unit cube and
  a tetrahedron and reads the exact volume of the result.

```sh
cargo run --example displacement
cargo run --example convex_clip
cargo test --doc
cargo doc --no-deps --all-features
```

Open `target/doc/rcct/index.html` for documentation built from the checkout.
The published crate's [API documentation is available on docs.rs](https://docs.rs/rcct).

## Geometry supported in 0.2

- **v0.1.1:** published elementary 3D point/vector/direction API, numerical
  contracts, and independent verification; it supports Rust 1.85.
- **v0.2.0 bounded planar solids.** Checked finite planar primitives and exact segment
  events feed bounded arrangements and face-based 2D Booleans. The spatial
  module adds exact raw-input 3D orientation and segment–plane classification.
  The topology module records combinatorial boundary uses and incidence.
  The experimental polyhedral module admits closed boundaries from exact
  rational vertices and outward-oriented convex planar faces. A convex
  boundary can be re-admitted as an operand for `boolean3`. A result whose
  faces are axis-aligned rectangles can go directly into
  `boolean_rectilinear3` for another union, intersection or difference.
  `boolean_prismatic3` accepts horizontal caps and vertical walls on
  independently oriented rational planes. It takes the returned boundary
  directly, including a nonconvex result. Each face must be an admitted
  convex planar patch.
- **Release limits.** Solid Boolean operations use planar faces and checked
  fixed-width rational arithmetic. Results are owned in memory. Curved or
  trimmed surface operations, saving and reopening exact solids, and Boolean
  operations on arbitrary nonconvex B-rep operands are outside the 0.2 series.

Development should preserve explicit ownership, geometric invariants, numerical
contracts, and source provenance. Implementations must remain independent and
compatible with this project's permissive license. Do not copy or transliterate
LGPL-licensed OCCT source. See
[CONTRIBUTING.md](https://github.com/Muktidaya/rcct/blob/root/CONTRIBUTING.md).

## Build and check

The project pins an exact compiler in `rust-toolchain.toml` for reproducible
build and release preparation. The current pin is recorded in the
[toolchain provenance](https://github.com/Muktidaya/rcct/blob/root/docs/toolchain-provenance.md). The manifest records
the minimum supported compiler in `rust-version`. Check the official stable
metadata when preparing a release.

```sh
python3 tools/release_toolchain.py --check
# Only when deliberately refreshing the pin:
python3 tools/release_toolchain.py --update
```

The update command changes only the project pin and provenance, not
`rust-version`; review that separate compatibility decision and diff. If the
minimum differs from the new pin, add a separately qualified minimum lane or
revise the minimum before claiming support.

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
including Linux/macOS ARM64, with debug and release tests. Verification
records identify the commit tested. No OCCT installation is required.
Fixture generators use Python 3.9+ standard-library modules. The package checker
and release pin tool require Python 3.11+ for the standard-library TOML parser.

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
[LICENSE.txt](https://github.com/Muktidaya/rcct/blob/root/LICENSE.txt) and [NOTICE.md](https://github.com/Muktidaya/rcct/blob/root/NOTICE.md).

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
