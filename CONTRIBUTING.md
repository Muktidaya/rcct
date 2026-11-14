# Contributing

The goal is an independent pure-Rust CAD kernel, using OCCT as a functional
reference without requiring fidelity to its implementation or architecture. Keep native OCCT
and C++ dependencies out of the implementation. Favor small reviewed increments
with useful behavior, tests, and explicit numerical contracts over empty module
stubs.

## Checks

Run formatting, Clippy with warnings denied, tests (including doctests), a
warnings-denied documentation build, deterministic fixture regeneration, and
`python3 tools/check_package.py` before a release. Validate all fixture
generators with `--check`. Run tests in both debug and
release profiles. Use the exact project toolchain in `rust-toolchain.toml`;
the proposed 0.2.0 minimum is Rust 1.100.0 and older consumer support is not
claimed for this candidate. Verify official stable metadata with
`python3 tools/release_toolchain.py --check`; a deliberate `--update` changes
only the project pin and provenance, so review the separate minimum-version
decision and diff. Do not change the machine-wide default for this project.

Document coordinate conventions, invalid inputs, tolerance decisions, overflow,
and degeneracy at the relevant API. Tests should include boundary and failure
cases. Approximate numerical assertions need an explicit justified tolerance.
See [the numerical contract](docs/numerics.md) and [verification record](docs/verification.md).
Fixture inputs are exact `f64` bit patterns. Regenerate fixtures with the checked-in
Python standard-library tool; do not hand-edit them or derive the reference by
calling rcct. A finite test corpus is evidence, not a universal error bound.

## Source documentation

Keep public contracts in rustdoc at the relevant type or operation. Use inline
comments for non-obvious implementation constraints, such as normalization's
validation order and overflow avoidance, rather than narrating each expression.
Examples should exercise the contract and identify when convenient exact inputs
hide general floating-point rounding. Run doctests and warnings-denied rustdoc
after a documentation pass, including on the supported minimum toolchain.

When numerical behavior changes, reconcile source docs with the README,
[contract](docs/numerics.md), fixture notes, and verification record. Record CI
results against an exact commit; a green earlier revision does not verify later
changes. Do not weaken a numerical budget simply to make a new platform pass.

## Provenance

Contributions are made under `Apache-2.0`. Keep implementations original or use
material whose license and provenance are compatible with this project. Do not
copy, translate, or transliterate LGPL-licensed OCCT source into this crate.
Use public specifications, mathematical references, and independently designed
algorithms; cite relevant references and distinguish those references from
implementation provenance.

Review source, tests, fixtures, and other assets before incorporation. Changing
language, names, or syntax does not make a source-derived implementation
independent. Do not claim clean-room certification or freedom from all patent
claims: neither has been established by this scaffold.

## Current architecture

`src/gp.rs` owns the elementary geometry foundation. Later geometry, topology,
modeling, and data-exchange layers are proposed directions, not existing APIs.
Avoid advertising parity until specific operations have differential or other
appropriate acceptance tests.

## Assurance evidence

Keep correctness claims local to the operation and evidence that supports them.
Record invariants, units, preconditions, floating-point failure modes, and the
tests or analysis that check them. Distinguish empirical testing from proof.
Differential agreement is useful evidence but does not establish correctness
when both implementations may share assumptions or defects.

Unsafe code is forbidden in this initial crate. Do not weaken that boundary,
introduce a native dependency, or imply safety qualification as an incidental
change. Supply-chain review, fuzzing, formal methods, independent audit, and
application-specific qualification are possible future work, not completed
properties of this scaffold.
