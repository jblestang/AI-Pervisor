# Phase 31 expert review

Addresses Phase 30 deferred measurement hardening items (`cursor/phase-31-measurement-hardening-0b4f`).

## Fixes applied

| Deferred item | Fix |
|---------------|-----|
| EPT-aware guest memory read | `hv-ept::resolve_guest_phys_to_host()`; hypervisor reads via EPT-resolved host copy in `hv-x86-cpu` |
| End-to-end frame counting | `InVmRelayMeasurement` cross-checks boot-info extension frames with out-partition IPC queue tail (`min` of extension, IPC, expected) |
| ABI v2 explicit measurement extension | 32-byte `GuestBootInfoRelayMeasurement` tail (`RLAY` magic, version, `frames_completed`, `tsc_start`, `tsc_end`); parse validates magic in `GuestBootInfoView` |
| TSC-based in-VM throughput timing | Out guest records RDTSC start/end; Gate D passes `elapsed_tsc` to `apply_live_guest_throughput_benchmark` (prefers TSC over mock nanos) |

## Key components

| Layer | Change |
|-------|--------|
| `hv-guest-abi` | `GuestBootInfoRelayMeasurement`, `parse_guest_boot_info_relay_measurement()`, `guest_relay_measurement_elapsed_tsc()` |
| `hv-ept` | `resolve_guest_phys_to_host()` (resolution only; reads in CPU seam) |
| `guest-common` | `relay.rs` extension init, TSC capture, out-partition frame counter |
| `hv-x86-cpu` | `GuestRelayMeasurementContext`, EPT-aware reads, `InVmRelayMeasurement` |
| `hv-datapath` | `DatapathBenchmarkConfig.tsc_hz`, `elapsed_nanos_from_tsc()`, `plan_out_ipc_consumer_guest_phys()` (VmId 2 for snapshot compatibility) |
| `hv-hypervisor-boot` | Gate D wires EPT tables + IPC consumer GPA + TSC into throughput path |

## Verification

- `cargo test -p hv-guest-abi -p hv-ept -p hv-guest-boot`
- `cargo test -p hv-datapath guest_relay`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
