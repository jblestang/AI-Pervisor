# Phase 40 expert review

Addresses Phase 39 expert review findings (`cursor/phase-40-invept-review-fixes-0b4f`).

## Fixes applied

| Review finding | Fix |
|----------------|-----|
| Encoded EPT pointer rejected by live validation | Allow EPTP control bits in low 12 bits; reject only reserved/misaligned low fields |
| INVEPT descriptor may be under-aligned | Use `#[repr(C, align(8))]` 128-bit descriptor for INVEPT memory operand |
| Reload batch accepted empty VMCS list | Fail closed when `vmcs_phys_list` is empty |
| Workspace clippy `indexing_slicing` in guest ABI | Use `try_into` for fixed-size byte array reads |
| Workspace clippy clean under `--all-features` | Fix indexing in EPT/guest-boot/resident paths; gate asm/test lint noise |

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p hv-ept`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
