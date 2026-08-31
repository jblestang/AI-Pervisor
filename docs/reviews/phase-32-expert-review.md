# Phase 32 expert review

Addresses Phase 31 expert review findings (`cursor/phase-32-measurement-review-fixes-0b4f`).

## Fixes applied

| Review finding | Fix |
|----------------|-----|
| Counter incremented per loop, not per delivery | `run_out` returns `bool`; `record_relay_frame_completed` only on matched dequeue + egress |
| IPC read failures silently ignored | Propagate `read_ipc_delivered_frames_from_guest` errors; require non-zero IPC tail on `Executed` |
| EPT resolve checked start only | `resolve_guest_phys_range_to_host()` validates full read range within one mapping |
| TSC optional on live Executed | Gate D rejects zero `elapsed_tsc` when executed + frames meet threshold; live throughput requires TSC |
| IPC cross-check skipped at zero | `end_to_end_relay_frames` always `min(extension, ipc, expected)` |
| ABI v2 tail check used frames offset | Parser uses `guest_boot_info_relay_measurement_offset` for v2 size validation |

## Verification

- `cargo test -p hv-guest-abi -p hv-ept -p hv-guest-boot`
- `cargo test -p hv-datapath guest_relay`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
