# 0.2.0 integrated release preparation

This unreleased candidate combines the reviewed public integration checkpoint
`19d8f581522f0f9b35623f909f976e118eaa38ca` with release-preparation metadata.
The canonical public `root` was `c83f03a12fc7d6db323a1b5b94aa63308bbcff00`
at preparation; no root update or publication is implied. This candidate
contains the bounded planar, spatial and combinatorial topology capabilities
and limitations documented in the integrated checkpoint. It does not include
the separate private arithmetic/constructed-cut experiments, complete B-rep,
general 3D Booleans or a complete CAD kernel. No package, tag or release is
created by this preparation.

## Version-reference inventory

* `Cargo.toml` and `Cargo.lock` identify proposed `0.2.0`; the lock has no
  third-party dependencies. Proposed `rust-version = 1.100.0` is a consumer
  minimum change, distinct from the exact project build pin.
* The README uses a local path for the unpublished candidate. The earlier
  `0.2.0-alpha.1` changelog/verification passages, their 1.99.1 toolchain and
  exact receipts describe previous development candidates; they are historical,
  not support or qualification for this candidate.
* The 0.1.1 and 0.1.0 changelog and verification entries, Rust 1.85 references,
  hashes and CI receipts are historical. `tests/api_compat.rs`, `v0.1.0` Git tag
  and `tools/check_package.py` intentionally preserve the old public-surface
  comparison; generated consumer version 0.0.0 is not rcct's version.
* Fixed fixture strings, mathematical bounds and unrelated Python/action/tool
  versions are not package-version substitutions and remain unchanged.

## Compiler and release gates

This candidate proposes Rust 1.100.0 both as the declared minimum and the
initial exact project pin. Raising the consumer floor from published 0.1.1's
1.85 excludes Rust 1.85 through 1.99.x; a user's need for older support has not
been established. This consequential policy needs review and exact-candidate
validation. CI configures four platforms plus quality at the explicit pin; it
has not run for this candidate. An older support claim requires a concrete
need and a separately tested lane, not inference from historical results.

`tools/release_toolchain.py --check` checks official stable metadata; a
purposeful `--update` validates manifest/checksum from the official HTTPS
origin and emits a reviewable diff while changing only the project pin and
provenance file. It never publishes, tags, pushes, installs a toolchain,
changes a global default, or edits dependencies or the declared minimum. It
does not execute downloaded data. HTTPS and a checksum from that same origin
are not an independent signature. If the pin changes, separately review the
minimum, workflow coverage, exact-version docs and diff. A metadata/network
failure or malformed/mismatched checksum is not proof of the latest pin.

Before root integration, verify actual remote destination/ancestry and clean
canonical root, review this exact diff, and qualify the exact candidate.
Before any publication, complete separate release-scope, compatibility,
provenance and platform review; confirm official stable, all fixture checks,
formatting, warnings-denied lint/docs, debug/release, clean packaging and
downstream checks. A prior branch CI run is not evidence for this candidate.
No automatic release or publication is implied by a green build.

The update writes the pin and provenance sequentially; interruption can leave a
partial update. Inspect both files and reconcile them before any build or release.
A later pin refresh does not itself validate or raise the declared consumer floor.

The release pin tool and package checker require Python 3.11 or later for
standard-library TOML parsing; this does not change individual fixture requirements.
