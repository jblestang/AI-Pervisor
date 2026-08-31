# Phase 36 expert review

Addresses Phase 35 expert review findings (`cursor/phase-36-ept-paging-review-fixes-0b4f`).

## Fixes applied

| Review finding | Fix |
|----------------|-----|
| `patch_ept_table_host_phys` silently skipped missing nested phys | Returns `Result`; fails on missing index or leftover synthetic pointers |
| No post-install host-physical validation | Gate D cross-checks `resolve_guest_phys_to_host` against installed page HPA |
| Overlapping guest mappings last-write-wins | `append_ept_guest_mapping` rejects overlapping ranges; materialize rejects conflicting leaf rewrites |
| Measurement page GPA overlapped out e1000 MMIO | Moved `RELAY_MEASUREMENT_PAGE_GUEST_PHYS` to `0xFEB4_0000` (after out BAR) |
| `table_bytes_mut` no-op on invalid index | Returns `Option`; mapping errors propagate |
| EPT pointer reload with empty `seam_inputs` | Reload guarded on non-empty VMCS batch |

## Verification

- `cargo test -p hv-ept`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
