# 0.2 release record and preparation checks

Version 0.2.0 was published from commit
`7bac8edb6a925b701ec5d1dfd04ebee249e8088f` on November 20, 2026.
The release added convex halfspace Booleans, exact indexed planar boundaries
and result reuse on the rectilinear and prismatic face domains.
[Source tag](https://github.com/Muktidaya/rcct/tree/v0.2.0) ·
[Published crate](https://crates.io/crates/rcct/0.2.0)

The 0.2.0 planar-solid milestone covers 19 acceptance cases (P01–P16 and
P18–P20). P17 exact close/reopen persistence, curved or trimmed surface
operations and arbitrary nonconvex B-rep Booleans are outside this release.
The [polyhedral contract](polyhedral-boolean-contract.md) describes the three
entry points and their arithmetic and resource limits.

Version 0.2.1 corrects the release and install documentation shipped in the
0.2.0 archive. Both versions specify Rust 1.100.0 as the minimum. The
manifest and lockfile identify the package version; the crate has no
third-party dependencies.

## Historical version references

The integration checkpoint `19d8f581522f0f9b35623f909f976e118eaa38ca`
and earlier verification records describe development revisions. The
`0.2.0-alpha.1` changelog and verification entries record their own Rust
1.99.1 toolchain. The 0.1.1 and 0.1.0 records document Rust 1.85.
`tests/api_compat.rs` and `tools/check_package.py` retain the comparison with
the public `v0.1.0` tag.

## Compiler and release gates

Version 0.2.0 raised the minimum from Rust 1.85 to 1.100.0 and pinned
the same compiler for project builds. During preparation, local qualification
passed on commit `fd65375947f5645729a0eee8ab3639de1863663d`
with Rust 1.100.0, including 155 tests in each profile, package/downstream
checks and an official manifest check. Its public [hosted run](https://github.com/Muktidaya/rcct/actions/runs/43565878750)
passed quality but failed four platform packaging steps on generated Python
bytecode. Those receipts do not qualify later commits, which require their own
validation. An older support claim requires a concrete
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

Before any further root integration, verify actual remote destination/ancestry
and clean canonical root, review the exact diff, and qualify the exact candidate.
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
