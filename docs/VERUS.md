# Verus toolchain status

## Workspace pin

`rust-toolchain.toml` pins **Rust 1.99.1**, matching Verus release `0.2026.10.26` (`required rust toolchain 1.99.1-x86_64-unknown-linux-gnu`).

## Verus release alignment

| Item | Value |
|------|-------|
| Release | `release/0.2026.10.26.d17fbd0` |
| Asset | `verus-0.2026.10.26.d17fbd0-x86-linux.zip` |
| Required rustc | **1.99.1-x86_64-unknown-linux-gnu** |
| Workspace pin | **1.99.1** |

`cargo xtask verify` invokes Verus on annotated crates (`ring`, `isolation`) when `VERUS_BIN` or `verus` on `PATH` is set. Soft-skips (exit 0) when:

- the Verus binary is missing, or
- Verus reports a rustup/toolchain mismatch, or
- the crate still uses attribute stubs without `vstd` / `verus_builtin` (Task 8)

Other Verus failures hard-fail.

## Current proof status (`ring`, `isolation`)

- Host tests: `cargo test -p ring`, `cargo test -p isolation`.
- In-source proof intent uses `requires!` / `ensures!` stubs in each crate’s `src/verus_stub.rs` so the crates build without a Verus Cargo feature.
- `isolation` exposes `Region` / `MemoryMap::check_access` with overlap rejection for private WAN/LAN claims.
- Real `verus!` / `vstd` contracts and hard-fail CI on proof errors remain for Task 8 (`[package.metadata.verus] verify = true`).

## Enabling full verification

1. Install Verus release binaries; set `VERUS_BIN` or put `verus` on `PATH`.
2. Replace stubs with real `verus!` / `vstd` contracts; set `verify = true` in annotated crates’ `Cargo.toml`.
3. Re-run `cargo xtask verify` and require a hard failure on proof errors (Task 8).
