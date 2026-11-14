# Rust toolchain provenance

Project pin: `1.100.0`

On the locally qualified `fd65375947f5645729a0eee8ab3639de1863663d`, the
release check read the official stable manifest over HTTPS and confirmed Rust
1.100.0, release date `2026-11-12`, and manifest SHA-256
`3051e859570f64a8c5acb9d09e87425db52c7d97287c67e8f6d57d7d34ef9258`.
Source: https://static.rust-lang.org/dist/channel-rust-stable.toml

The checksum comes from the same official HTTPS origin; it is consistency and
transport evidence, not an independent signature. Recheck official stable for
any later release preparation. This record does not qualify a later commit.
