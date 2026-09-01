# Phase 46 expert review

Addresses Phase 45 expert review findings (`cursor/phase-46-vmexit-mmio-relay-review-fixes-0b4f`).

## Fixes applied

| Review finding | Fix |
|----------------|-----|
| Host dispatch loop could VMRESUME with no handlers | Require non-empty `VmexitRelayDispatchConfig` when host dispatch is enabled |
| MMIO state write had no read-back check | Validate `E1000MmioState` after host page write |
| Weak MMIO event validation (`> 0`) | Require `>= GUEST_RELAY_BENCHMARK_FRAMES` IN TX doorbells when e1000 present |
| e1000 EPT plan could be incomplete | Fail if `e1000_by_vm` count mismatches layout `nic_e1000` devices |
| Read-only MMIO not verified after EPT reinstall | Re-check mapping record and leaf entry after `install_ept_tables` |
| Throughput path skipped MMIO cross-check | Validate execution seam MMIO events in relay throughput init |

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo test -p hv-ept`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
