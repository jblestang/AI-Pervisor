# Phase 37 expert review

Read-only guest EPT mapping and hypervisor-authoritative measurement publish (`cursor/phase-37-measurement-readonly-ept-0b4f`).

## Changes

| Component | Change |
|-----------|--------|
| EPT permissions | `guest_writable` on `EptProgrammedMapping`; `encode_ept_leaf_entry`, `append_ept_guest_read_only_mapping` |
| Gate D | Measurement page mapped read-only; validates EPT leaf lacks guest write |
| Hypervisor publish | `publish_relay_measurement_page_authoritative()` writes IPC-derived frames + guest TSC to host page |
| Measurement seam | Publishes authoritative counters before reading hypervisor-owned page |
| Guest firmware | Records counters/TSC in boot-info tail only (measurement page not guest-writable) |

## Trust model

Guests can no longer tamper with the hypervisor measurement page at runtime. Authoritative frame counts are derived from IPC delivered tail during hypervisor publish; TSC timing still originates from guest boot-info tail.

## Verification

- `cargo test -p hv-ept`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
