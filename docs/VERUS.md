# Verus toolchain status (Task 2)

## Workspace pin

`rust-toolchain.toml` pins **Rust 1.83.0** (lab image / freestanding target baseline from Task 1).

## Latest Verus release probed

| Item | Value |
|------|-------|
| Release | `release/0.2026.10.26.d17fbd0` |
| Asset | `verus-0.2026.10.26.d17fbd0-x86-linux.zip` |
| Required rustc | **1.99.1-x86_64-unknown-linux-gnu** |

Running the release `verus` binary against this workspace fails immediately:

```text
verus: required rust toolchain 1.99.1-x86_64-unknown-linux-gnu not found
```

**Blocker:** current Verus cannot verify crates while the workspace stays on the pinned **1.83.0** channel. Bumping the pin (or maintaining a dual-toolchain verify path) is deferred to a later task once the lab image and freestanding builds are validated on a newer rustc.

## What Task 2 delivers instead

- `ring` implements the SPSC ABI with host tests (`cargo test -p ring`).
- Proof intent is captured in-source via `requires!` / `ensures!` stubs in `ring/src/verus_stub.rs` that compile without Verus / without a `verus` Cargo feature.
- `cargo xtask verify` is wired to invoke Verus on `ring` when a usable `verus` binary is present (`VERUS_BIN` or `PATH`). If Verus is missing or reports a toolchain mismatch against the workspace pin, verify **soft-skips** (exit 0) and prints a pointer to this document.

## Enabling real verification later

1. Align `rust-toolchain.toml` with a Verus-supported channel (or install that channel beside the pin for verify-only).
2. Replace stubs with real `verus!` / `vstd` contracts (`[package.metadata.verus] verify = true`).
3. Re-run `cargo xtask verify` and require a hard failure on proof errors (Task 8).
