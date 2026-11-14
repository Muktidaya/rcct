# Rust toolchain provenance

Project pin: `1.100.0`

The [Rust release announcement](https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/)
reports Rust 1.100.0 on 2026-11-12. The official stable manifest and checksum have not yet been verified for this candidate.
Run `python3 tools/release_toolchain.py --check` against the official Rust
stable distribution metadata and record the exact candidate's verification
before release; `--update` records manifest provenance when changing the pin.
