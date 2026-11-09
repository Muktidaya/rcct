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

Linux and Windows jobs, and the hosted macOS jobs, have **not** run for this
candidate at the time this record was written. Runner availability and all
hosted results remain release gates. Before publication, run the full matrix on
the exact outgoing commit, review failures without silently loosening budgets,
repeat clean packaging, and review source/history for unintended disclosure.
Windows ARM64, macOS x86-64, other architectures, changed floating-point modes,
and broader Rust versions are not established by this record.

B-rep, Boolean operations, a broad geometry API, and the required Rust 1.99.1
baseline belong to the planned v0.2.0 scope. This release establishes only the
current primitive-level contracts and finite verification evidence. Formal
proofs, adaptive/exact geometric predicates, security audit and application-
specific safety qualification remain outside this evidence.
