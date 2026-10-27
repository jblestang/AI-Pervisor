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
| `vstd` crate | `=0.0.0-2026-10-25-0158` (workspace dep) |

## Install (agent / lab)

```bash
# Example local unpack matching xtask fallbacks:
mkdir -p "$HOME/verus" && cd /tmp
curl -sL -o verus.zip \
  "https://github.com/verus-lang/verus/releases/download/release/0.2026.10.26.d17fbd0/verus-0.2026.10.26.d17fbd0-x86-linux.zip"
unzip -oq verus.zip -d "$HOME/verus"
export PATH="$HOME/verus/verus-x86-linux:$PATH"
# optional: VERUS_BIN, VERUS_HOME, CARGO_VERUS
```

`cargo xtask verify` prefers `cargo verus verify -p <crate>` when `cargo-verus` is resolvable via `CARGO_VERUS`, `VERUS_BIN`’s directory, `VERUS_HOME`, `PATH`, or `$HOME/verus/verus-x86-linux/`.

## Soft-skip vs hard-fail

| Condition | Behavior |
|-----------|----------|
| `cargo-verus` / Verus missing | **soft-skip** (exit 0) + message pointing here |
| Toolchain / edition mismatch reported by Verus | **soft-skip** (exit 0) |
| Proof error on an annotated crate | **hard-fail** (exit ≠ 0) |

## Annotated crates (Task 8)

| Crate | `[package.metadata.verus] verify` | Proofs |
|-------|-----------------------------------|--------|
| `ring` | `true` | `ring/src/proofs.rs` — pow2 / mask lemmas (`lemma_mask_bounded`, …) |
| `isolation` | `true` | `isolation/src/proofs.rs` — perms subset + range overlap/containment |

Exec datapath keeps `verus_stub` `requires!` / `ensures!` notes so freestanding/`cargo test` builds stay free of Verus cfg. Machine-checked content is the `verus!` modules above (ghost-erased under ordinary `cargo build`).

`pci-vf` / `guest-common` retain stub intent only (`verify = false`) until their exec APIs grow full `verus!` contracts.

## Commands

```bash
cargo xtask verify                 # preferred gate
cargo verus verify -p ring         # direct
cargo verus verify -p isolation
```
