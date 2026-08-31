# Phase 35 expert review

Four-level EPT paging for relay measurement page GPA (`cursor/phase-35-ept-measurement-paging-0b4f`).

## Changes

| Component | Change |
|-----------|--------|
| EPT paging | `materialize_ept_paging()` builds 4-level walk from mapping records |
| `EptProgrammedTables` | Adds `paging_tables` for nested levels; synthetic child refs patched at install |
| `install_ept_tables` | Allocates nested pages, patches pointers, installs root |
| `append_ept_guest_mapping` | Triggers paging materialization after append |
| Gate D | Pre-loop measurement page EPT install; validates GPA mapped; reloads EPT pointer on all VMCS |

## Closes Phase 34 deferral

Phase 34 deferred full EPT root-table walk for high GPA (`0xFEB2_0000`). Phase 35 materializes a proper hierarchy so guests can access the hypervisor measurement page under live VMX.

## Verification

- `cargo test -p hv-ept`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
