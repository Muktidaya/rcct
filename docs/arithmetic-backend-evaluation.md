# Exact arithmetic backend evaluation

This is a preliminary source and license comparison, not an adoption or
complete dependency audit. The current bounded code is original rcct Rust under
Apache-2.0, has no third-party dependency, and remains under the crate's
`forbid(unsafe_code)`. Its fixed-width proofs cover explicitly bounded raw and
first-generation inputs only. General rational growth needs checked limits and
cannot inherit those proofs.

| Candidate | Evidence and tradeoff | Disposition |
| --- | --- | --- |
| Existing fixed limbs | Original safe Rust, fixed work/storage, no dependency. The required width grows rapidly with construction depth. | Use only for documented bounded domains; no claim of arbitrary precision. |
| `num-bigint` 0.5.1 | [Pinned manifest](https://github.com/rust-num/num-bigint/blob/33c59ba44b7bdb09975b38a321b1b88c6a444005/Cargo.toml) declares MIT OR Apache-2.0 and `num-integer`/`num-traits`. Its [x86 division path](https://github.com/rust-num/num-bigint/blob/33c59ba44b7bdb09975b38a321b1b88c6a444005/src/biguint/division.rs) contains unsafe inline assembly. | Plausible dynamic backend, not adopted; transitive unsafe and resource accounting need explicit review. |
| `num-rational` 0.4.2 | [Published manifest](https://docs.rs/crate/num-rational/0.4.2/source/Cargo.toml.orig) declares MIT OR Apache-2.0 and defaults to `num-bigint` 0.4.x. | Rational option, not adopted; do not accidentally assume it uses 0.5.x or skip dependency review. |
| `bnum` 0.14.4 | [Versioned manifest](https://github.com/isaacholt100/bnum/blob/v0.14.4/Cargo.toml) declares MIT OR Apache-2.0, Rust 1.87 minimum and fixed-width types. Its [build script](https://github.com/isaacholt100/bnum/blob/v0.14.4/build.rs) probes the compiler and overflow setting. | Candidate for bounded arithmetic, not a solution to unbounded growth; source, build and unsafe review are incomplete. |
| `ibig` | The observed [upstream manifest](https://github.com/tczajka/ibig/blob/master/ibig/Cargo.toml) declares MIT OR Apache-2.0, Rust 1.95 and `ibig-core`/`smallvec`. This is a moving branch observation, not a pinned release audit. | Dynamic candidate for a later pinned artifact and transitive/unsafe review; not adopted. |

Pure Rust does not imply the absence of `unsafe` in transitive dependencies.
Choosing an Apache-compatible license alone does not establish complete
provenance or safety. An adopted backend needs a pinned dependency graph, actual
build/feature review, documented notices, platform review, a bounded-work policy
and an explicit decision on transitive unsafe code. No third-party code or OCCT
implementation was copied into this increment.
