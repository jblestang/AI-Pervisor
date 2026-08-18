# Phase 19 expert review

Multi-domain review of Gate D datapath malicious: compromised-guest attack simulation, IPC integrity scanner, pre-forward enforcement, and `datapath-malicious` orchestration (`cursor/phase-19-datapath-malicious-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Compromised-guest simulation | `CompromisedGuestAction`, `apply_compromised_guest_write` |
| IPC integrity scanner | `scan_ipc_queue_integrity`, `enforce_forward_integrity` |
| Pre-forward enforcement | `forward_synthetic_frame` integrity gate |
| Reference attack suite | `REFERENCE_COMPROMISED_SCENARIOS`, `run_reference_compromised_scenarios` |
| Gate D malicious orchestration | `GateDDatapathMaliciousResult`, `boot_*_gate_d_datapath_malicious*()` |
| UEFI + xtask | `datapath-malicious` feature chain, serial markers, coverage pass |

## Phase 18 deferrals closed

| Phase 18 item | Phase 19 disposition |
|---------------|---------------------|
| Malicious guest tests | **Partially closed** — host-simulated compromised-partition IPC + e1000 attacks |
| 200 Mbit/s performance benchmark | **Deferred** (Phase 20+) |
| Multi-partition VMLAUNCH | **Deferred** (Phase 20+) |
| Real guest ELF images | **Deferred** (Phase 20+) |
| Live guest IPC runtime | **Unchanged** — integrity enforced on host mock path only |
| DMAR MMIO | **Unchanged** |

## Attack scenarios covered

| Scenario | Detection path |
|----------|----------------|
| Forged slot metadata (valid=1, payload_len > slot_size) | `scan_ipc_queue_integrity` |
| Head/tail corruption (tail > head) | `scan_ipc_queue_integrity` |
| Cross-partition chan_a corruption | `scan_ipc_queue_integrity` + forward blocked |
| Stale slot replay after consumption | `scan_ipc_queue_integrity` |
| e1000 read-only register write | `handle_e1000_mmio_write` rejects at apply |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-malicious` | `hv-hypervisor-boot` | off | off | Gate D malicious orchestration atop `datapath-live` |
| `datapath-malicious` | `hv-hypervisor-efi` | off | opt-in | Malicious boot entry + IPC integrity markers |
| `datapath-malicious` | `hv-hypervisor-efi-bin` | off | opt-in | Builds on `datapath-live` |

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-datapath` — pass (includes `compromised_ipc`)
- `cargo test -p hv-hypervisor-boot --features datapath-malicious` — pass
- `cargo test -p hv-hypervisor-efi --features datapath-malicious` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — multi-pass includes `datapath-malicious`
- `cargo xtask build-boot-chain-live` — builds with `real-hw-execution,vmx-launch,datapath-foundation,datapath-live,datapath-malicious`

## Review status

Phase 19 closes the compromised-guest malicious test gap from Phase 18: host-simulated attack fixtures, IPC integrity scanning, pre-forward enforcement, and Gate D malicious orchestration with validate-only CPU seams. Performance benchmarking, multi-partition VMLAUNCH, and real guest ELFs remain deferred to Phase 20+.
