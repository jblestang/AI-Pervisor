# Phase 33 expert review

Hypervisor-owned relay measurement counter page (`cursor/phase-33-hypervisor-measurement-page-0b4f`).

## Changes

| Component | Change |
|-----------|--------|
| ABI extension v2 | 40-byte `GuestBootInfoRelayMeasurement` adds `measurement_page_gpa` |
| Datapath planning | `RELAY_MEASUREMENT_PAGE_GUEST_PHYS` (`0xFEB2_0000`), `plan_relay_measurement_page_gpa()` |
| Resident install | `install_relay_measurement_page()` allocates hypervisor page, zero-inits header |
| EPT | `append_ept_guest_mapping()` for non-identity GPA→HPA mapping at runtime |
| Gate D | Out partition: install page, patch boot info GPA, append EPT mapping, record host phys |
| Guest firmware | Counter/TSC writes target hypervisor-owned page via published GPA |
| Measurement seam | Authoritative read from `measurement_page_host_phys`; boot-info EPT path is fallback |

## Trust model

Counters remain guest-writable at runtime (smoke tier). Phase 33 moves authoritative state to a hypervisor-allocated page with a dedicated EPT mapping and direct HPA reads post-execution. Read-only guest mapping and hypervisor-only increments remain deferred.

## Verification

- `cargo test -p hv-guest-abi -p hv-ept -p hv-guest-boot`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
