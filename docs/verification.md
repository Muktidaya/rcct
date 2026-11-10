# v0.1.1 verification record

This is a reproducible acceptance record, not a proof or safety qualification.
The production arithmetic and public API are unchanged from v0.1.0. Source
compatibility and numerical results are separate questions; passing a consumer
smoke test is not an exhaustive semver proof or bit-for-bit portability promise.

## Independent numerical references

`tools/generate_numeric_fixtures.py` generates
[`tests/fixtures/numerical.tsv`](../tests/fixtures/numerical.tsv) without importing
rcct, OCCT, or a platform `hypot` implementation. Inputs are exact binary64 bit
patterns. Python's standard-library `Fraction` evaluates products, sums, and
coordinate differences exactly. `Decimal` evaluates square roots and normalized
components at 1800 significant digits, with a second 2000-digit computation
required to produce identical bytes. That stability check is strong empirical
evidence for these fixtures, not a proof of correct rounding for all inputs.

The deterministic corpus contains 128 cases: 21 norms, 21 normalizations,
27 dot products, 27 cross products, and 32 point distances. It covers exponent
extremes, normal/subnormal boundaries, near-parallel vectors, cancellation,
large adjacent coordinates, and scale separation. No random generator or
external dataset is involved. Nonfinite classifications, intermediate overflow,
underflow, and signed-zero semantics are tested separately from real arithmetic.

Acceptance budgets are explicit in the fixture data and described in
[the fixture notes](../tests/fixtures/README.md):

| Operation | Corpus acceptance budget |
| --- | --- |
| Norm | 4 ULPs of the rounded independent result |
| Normalized component | 8 ULPs plus 4 minimum subnormals |
| Dot / cross component | `8 * 2^-53 * sum(abs(exact products)) + 8 * 2^-1074`, rounded upward |
| Distance | 8 ULPs; zero allowance for exactly representable adjacent-coordinate cases |

Here an ULP is the local binary64 spacing, with `2^-1074` at zero/subnormal
results. The product-scale budget avoids dividing by a cancellation-dominated
answer. Norm/normalization/distance budgets accommodate the tested operation
chains and platform variation; they do not establish a universal bound for
Rust's unspecified-precision `hypot`. Intermediate-overflow cases do not use
finite-accuracy bounds. See [the numerical contract](numerics.md).

## Downstream and package verification

`tools/check_package.py` packages the candidate with locked, offline resolution,
extracts it into a temporary directory, checks selected shipped source/provenance
files against the checkout, and runs its complete tests in debug and release.
It then compiles and executes the identical
[`tests/api_compat.rs`](../tests/api_compat.rs) source in two separate downstream
consumers: one using the repository's `v0.1.0` tag, the other using the extracted
candidate. This is a tagged-source comparison, not a newly downloaded registry
artifact. Builds use an isolated target directory and explicit toolchain.

The consumer checks existing constructors, public fields, constants, methods,
operators, documented derives, const accessor, direction conversion and exhaustive
error variants. It does not test every possible downstream program. The crate
continues to have no third-party Rust dependencies or build script, and forbids
unsafe Rust. The package includes its numerical fixtures and generator.

## Executed locally on 2026-11-09

Host: macOS ARM64 (`aarch64-apple-darwin`). Python: 3.9.6.

- Rust 1.85.0: source tests, extracted-package tests, and both downstream consumers
  pass in debug and release. The declared `rust-version` remains `1.85`.
- Rust 1.99.1 (`58cf8f9c3`, LLVM 23.1.1): the same test/package/consumer checks pass.
- Installed `stable`: resolves to the same Rust 1.99.1 compiler; checks pass, but
  this is not evidence from a third distinct compiler version.
- Pinned-toolchain formatting, warnings-denied Clippy, warnings-denied rustdoc,
  fixture byte-for-byte regeneration, and `git diff --check` pass.
- Generated rustdoc HTML contains the operation table and correct module-anchor
  links. Visual browser/IDE-hover inspection has not been performed.

Commands (from a checkout with the `v0.1.0` tag):

```sh
cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets --all-features -- -D warnings
cargo test --locked --offline --all-features
cargo test --locked --offline --all-features --release
cargo +1.85.0 test --locked --offline --all-features
cargo +1.85.0 test --locked --offline --all-features --release
RUSTDOCFLAGS="-D warnings" cargo doc --locked --offline --no-deps --all-features
python3 tools/generate_numeric_fixtures.py --check
python3 tools/check_package.py --toolchain 1.85.0
python3 tools/check_package.py --toolchain 1.99.1
python3 tools/check_package.py --toolchain stable
git diff --check
```

For an intentionally uncommitted candidate only, pass `--allow-dirty` to the
package script. Local preparation used this option while changes were in flight;
release checks should run from the final clean commit. No registry publication
or remote CI execution follows from running these commands.

## Hosted matrix and remaining gates

The workflow configures both profiles and package/downstream checks for:

| Hosted runner | Architecture | Toolchains |
| --- | --- | --- |
| `ubuntu-24.04` | x86-64 | 1.85.0, 1.99.1, stable |
| `ubuntu-24.04-arm` | ARM64 | 1.85.0, 1.99.1, stable |
| `macos-15` | ARM64 | 1.85.0, 1.99.1, stable |
| `windows-2022` | x86-64 | 1.85.0, 1.99.1, stable |

The numerical-verification commit
[`7691b52ca1c29aa3b110be82a9c04e34ac22c288`](https://github.com/Muktidaya/rcct/commit/7691b52ca1c29aa3b110be82a9c04e34ac22c288)
passed [CI run 42683054858, attempt 1](https://github.com/Muktidaya/rcct/actions/runs/42683054858)
on 2026-11-09. All 14 expected jobs completed successfully: the 12 matrix jobs
above and two quality jobs for Rust 1.99.1/stable. None were skipped, failed, or
missing. Final clean package/downstream checks also passed locally on the three
listed toolchains for that commit.

The subsequent source-comment/documentation pass adds one doctest, taking the
suite to 15 integration tests and 3 doctests, without changing implementation
expressions. Its local tests, documentation, and package checks are separate
from the hosted result above. That earlier green run does not establish hosted
results for a later documentation commit.

Before publication, run the full matrix on the exact outgoing commit, review
failures without silently loosening budgets, repeat clean packaging, and review
source/history for unintended disclosure. Windows ARM64, macOS x86-64, other
architectures, changed floating-point modes, and broader Rust versions are not
established by this record.

B-rep, Boolean operations, a broad geometry API, and the required Rust 1.99.1
baseline belong to the planned v0.2.0 scope. This release establishes only the
current primitive-level contracts and finite verification evidence. Formal
proofs, adaptive/exact geometric predicates, security audit and application-
specific safety qualification remain outside this evidence.

## 0.2.0-alpha.1 development slice

The candidate starts from the verified `root` commit
`c83f03a12fc7d6db323a1b5b94aa63308bbcff00` (2026-11-09). The package version
is an unreleased prerelease; no tag, hosted workflow, or registry package has
been created for it. Its declared minimum is Rust 1.99.1. The existing 0.1.1
release and its Rust 1.85 support record above remain historical and unchanged.

The initial planar slice passed all checks below locally on Rust 1.99.1,
including the packaged candidate and its downstream compatibility consumer in
debug and release. These results apply only to the exact candidate state
recorded by the implementation handoff, not to an unrun hosted matrix:

- `cargo +1.99.1 fmt --all -- --check`
- `cargo +1.99.1 clippy --locked --offline --all-targets --all-features -- -D warnings`
- `cargo +1.99.1 test --locked --offline --all-features`
- `cargo +1.99.1 test --locked --offline --all-features --release`
- `RUSTDOCFLAGS="-D warnings" cargo +1.99.1 doc --locked --offline --no-deps --all-features`
- `python3 tools/generate_numeric_fixtures.py --check`
- `python3 tools/generate_orientation_fixtures.py --check`
- `python3 tools/check_package.py --toolchain 1.99.1 --allow-dirty`

The added predicate regression compares every ordered triple on a 5 by 5 integer
coordinate grid against an `i128` determinant, compares 256 deterministic
binary64 triples against a separate Python `Fraction` oracle, and includes
maximum finite, subnormal, and overflowing-subtraction examples. Segment and
wire tests exercise the event and validation classes documented in
`docs/planar.md`. This evidence is finite and is not a proof, fuzzing result, or
CAD-kernel qualification.

### Independent planar review and bounds

An additional local review compared 5,184 directed small-grid segment pairs,
3,000 randomized integer segment pairs, and 5,000 wide-exponent orientation
triples against a separate Python rational-arithmetic probe. A further 10,000
near-collinear wide-scale orientation triples matched that oracle. The probe was
an independent local experiment, not a bundled test or a proof. A persistent
integer parametric oracle now covers the 5,184 directed segment pairs in
`tests/planar_segment_oracle.rs`. Closed finite planar bounds are covered by
`tests/planar_bounds.rs`, including degenerate boxes, contact, signed zero,
extremes, and small-grid union/intersection laws.

### Proper-crossing parameter order

The bounded witness retains raw source segments and a rational parameter;
its capacity argument and excluded cases are in `docs/crossing-witness.md`.
`tests/crossing_witness.rs` compares 512 wide-exponent and 256 general integer
cases against independently generated Python `Fraction` order fixtures, plus
equal unreduced fractions, extreme values, differing source segments and
nonproper cases. On Rust 1.99.1, formatting, Clippy, debug/release tests,
warnings-denied documentation, all three fixture checks, and the dirty-candidate
package/downstream check passed locally. No hosted CI or formal proof is claimed.

An independent local probe additionally compared 2,000 pairs of general proper
crossings with independently scaled coordinate axes across the finite binary64
exponent range against Python `Fraction` parameters. The probe was temporary;
its results are not a proof or a bundled test. Endpoint witnesses and their
ordering alongside proper crossings are covered by the persistent exhaustive
small-grid integer oracle and an adversarial maximum/subnormal scale case.

Another independent temporary `Fraction` probe checked 2,000 endpoint/proper
comparisons and 2,000 collinear overlaps with independently scaled axes. It
checked retained endpoints and order against rational parameters. Persistent
small-grid tests additionally compare overlap boundaries alongside proper and
touch events. These are finite tests, not a proof.

### Point containment and parameter enclosures

The exact segment containment query is compared against an independent integer
oracle for 15,000 small-grid segment/point combinations and explicitly covers
maximum finite and subnormal coordinates. Tight parameter enclosures are checked
against 1,028 deterministic Python `Fraction` cases, including 256 general
crossings, exact endpoints, nonrepresentable values, and ratios below the least
subnormal or arbitrarily close to one. The independent generator records both
binary64 endpoints and uses `nextafter` after comparing the rounded value to the
exact rational. This corpus is finite evidence, not a formal proof.

Rust 1.99.1 formatting, Clippy, debug/release tests, rustdoc, all four fixture
checks, and the dirty-candidate package/downstream check passed locally for
these increments. Hosted CI was not run.

### Degenerate point segments

`intersect_closed_segments` is compared with an independent integer oracle for
all 6,561 ordered endpoint-pair combinations on a 3 by 3 grid, including
zero-length point segments. Separate cases cover signed zero and
maximum/subnormal coordinates. Point contact returns an input point without a
parameter; nonzero event witnesses remain separate APIs.
Rust 1.99.1 formatting, Clippy, debug/release tests, rustdoc, all fixture
checks, and the dirty-candidate package/downstream check passed locally after
this change. Hosted CI was not run.

### Retained point contacts and crossing-coordinate bounds

The point-on-segment witness is checked across the independent small-grid point
oracle and against existing event parameters, including extreme coordinates.
Proper-crossing coordinate bounds are checked against 512 deterministic Python
`Fraction` cases with independently scaled axes. Each case is also checked after
source swap and first-segment reversal. Explicit cases cover a non-dyadic third,
a minimum-subnormal exact crossing hidden by parameter rounding, negative and
positive half-subnormal values, and exact zero. These are finite tests, not a
formal proof. Local Rust 1.99.1 formatting, Clippy, debug/release tests, rustdoc,
all fixture checks and dirty-candidate package/downstream checks passed. Hosted
CI was not run.

### Exact crossing-point identity

Exact raw-input crossing points are compared against each other and against
finite input points in 256 deterministic Python `Fraction` cases. A separate
512-case coordinate corpus checks equality under source swap and first-segment
reversal. Explicit tests distinguish an exact one-third from its neighboring
binary64 input even though their enclosures overlap, and cover equal unreduced
constructions and extreme coordinates. The 165-limb bound and retained-input
limitations are recorded in `docs/crossing-witness.md`; repeated-cut and backend
requirements are in `docs/repeated-cut-prerequisites.md`.

On Rust 1.99.1, local formatting, Clippy, debug/release tests, rustdoc, fixture
checks and the dirty-candidate package/downstream check passed. The crossing
witness tests and packaged checks were rerun after extending the oracle to input
point comparisons. Hosted CI and formal verification were not run.

### First-generation constructed orientation and source spans

An independent temporary Python `Fraction` probe compared 2,000 crossing-point
identity/order pairs and 5,000 mixed raw/first-generation constructed orientation
triples sampled from the existing crossing-coordinate corpus against the Rust
results. The persistent constructed
orientation fixture adds 256 independently generated mixed cases, while focused
tests cover equal constructions, overlapping floating enclosures with different
exact signs, exponent extremes, budget exhaustion and unchanged inputs on
failure. Source-span tests check repeated one-third/two-thirds splits, 512
Fraction-derived event orders, different sources, endpoint/out-of-span rejection
and unchanged split budgets on failure. The temporary probes are not bundled;
these are finite tests and reviewed bounds, not a formal proof.

Rust 1.99.1 formatting, Clippy, debug/release tests, rustdoc, all fixture checks
and the dirty-candidate package/downstream check passed locally. Hosted CI and
formal verification were not run.

### Original-support span and raw-cutter queries

Two deterministic Python `Fraction` corpora independently supply 512 point-event
and 512 overlap-clipping results. Focused tests also cover non-dyadic span
boundaries, single-point clipped overlaps, reversed support traversal, repeated
queries and splits, and unchanged inputs and budget on exhaustion. The source
parameter-comparison capacity argument remains bounded to first-generation raw
inputs; the query count budget is not a time or memory limit.

On Rust 1.99.1, formatting, Clippy with warnings denied, debug and release
tests and doctests, warnings-denied rustdoc, all eight fixture checks, and the
dirty-candidate packaged crate and downstream compatibility checks in both
profiles passed locally. These finite checks are not a formal proof. Hosted CI
and visual inspection of rendered documentation were not run.

### Bounded straight-segment arrangement graph

An independent reduced signed-`i128` rational oracle constructs expected
vertices, undirected pieces, and directed source uses for 400 generated small
arrangements. Tests compare all three sets and check each source's ordered,
continuous traversal. Focused cases cover partial and reversed coincident
segments, T-junctions, concurrent rational crossings, extreme finite and
subnormal coordinates, every resource-limit class, and unchanged inputs and
previously returned graphs on failure. The located-point review also exercises
repeated transfers by using each retained result as the next input.

On Rust 1.99.1, formatting, Clippy with warnings denied, debug/release tests and
doctests, warnings-denied rustdoc, all eight fixture checks, and the
dirty-candidate packaged-crate/downstream checks in both profiles passed
locally. These finite checks are not a formal proof. At this local record,
hosted CI and visual inspection of rendered documentation had not been run.

### Half-edge rotation and simple-walk orientation

The arrangement review traced cut ordering, exact vertex identity, coincident
edge merging, source traversal, private draft failure, and the documented
distinction between count limits and byte/time guarantees. An independent
signed-`i128` angular oracle checks radial order and successor relationships
across 400 generated small graphs. The independent rational graph corpus also
compares simple-walk orientation against rational signed areas, including a
triangle with non-dyadic vertices. Focused tests cover nested and disconnected
components, bridges, exact resource boundaries, and nearly collinear rays and
triangles at maximum finite and minimum subnormal scales. These tests are
finite evidence, not a formal proof or independent third-party audit.

On Rust 1.99.1, formatting, Clippy with warnings denied, debug/release tests and
doctests, warnings-denied rustdoc, all eight fixture checks, and dirty-candidate
packaged-crate/downstream checks in both profiles passed locally. At this local
record, hosted CI and visual inspection of rendered documentation had not run.

### Located points and spans

An independent signed-`i128` rational determinant and closed-bounds oracle
checks 2,400 point-on-support cases, including reversed supports and points
outside the target. The earlier span-pair corpus is cross-checked against the
located-span path, with 600 collinear cases reconstructed from reversed origin
supports before querying. Focused cases exercise overlap cuts on both supports,
repeated point transfers, budget exhaustion, and maximum/subnormal coordinates.
The checks and first-generation fixed-width bounds are finite evidence, not a
formal proof or a qualification of arbitrary rational construction chains.

On Rust 1.99.1, formatting, Clippy with warnings denied, debug/release tests and
doctests, warnings-denied rustdoc, all eight fixture checks, and dirty-candidate
packaged-crate/downstream checks in both profiles passed locally. Hosted CI and
visual inspection of rendered documentation were not run for this increment.

### Original-support span pairs and point identity

An independent signed-`i128` rational oracle checks 2,400 collinear span pairs
and 3,840 transverse pairs, including reversed and vertical supports. The
collinear cases also cross-check raw-cutter results against an equivalent whole
second span and compare retained point coordinates across supports. Focused
cases cover unreduced equal points on different supports, exact contact at
maximum and minimum subnormal coordinate scales, and exhausted query budgets.
The finite tests and reviewed fixed-width bounds are evidence, not a proof.

Rust 1.99.1 formatting, Clippy with warnings denied, debug/release tests and
doctests, warnings-denied rustdoc, all eight fixture checks, and dirty-candidate
packaged crate and downstream compatibility checks passed locally. Hosted CI
and visual inspection of rendered documentation were not run.
