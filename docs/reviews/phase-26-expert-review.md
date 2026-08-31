# Phase 26 expert review

Multi-domain review of Gate D in-VM guest throughput benchmark: mock guest-runtime relay measurement with live VMX seam opt-in (`cursor/phase-26-guest-throughput-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Guest throughput benchmark | `run_mock_guest_throughput_benchmark`, `GuestThroughputBenchmarkResult`, disposition mapping |
| Guest throughput seam | `run_datapath_guest_throughput_cpu_seam`, `DatapathGuestThroughputCpuSeamOutcome` |
| Gate D guest-throughput | `GateDDatapathGuestThroughputResult`, guest-execution → benchmark → throughput seam orchestration |
| UEFI + xtask | `datapath-guest-throughput` feature chain, serial markers, coverage pass |

## Phase 25 deferrals closed

| Phase 25 item | Phase 26 disposition |
|---------------|---------------------|
| In-VM 200 Mbit/s measurement | **Closed (mock default)** — guest runtime relay benchmark with official metric; target enforced in Gate D init |
| Live VMX in-VM throughput | **Partial** — throughput seam delegates to guest VMLAUNCH path when live env ready; sustained guest-side relay loop deferred |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guest-throughput` | `hv-datapath` | always | n/a | Mock guest throughput benchmark + disposition helpers |
| `datapath-guest-throughput` | `hv-x86-cpu` | off | off | `run_datapath_guest_throughput_cpu_seam` |
| `datapath-guest-throughput` | `hv-hypervisor-boot` | off | off | Gate D guest-execution + in-VM benchmark + throughput seam |
| `datapath-guest-throughput` | `hv-hypervisor-efi` | off | opt-in | Guest-throughput boot entry + throughput markers |

## Serial markers

- `GATE_D_GUEST_THROUGHPUT_MARKER` — in-VM guest throughput orchestration succeeded
- `GATE_D_GUEST_THROUGHPUT_TARGET_MET_MARKER` — minimum run met 200 Mbit/s target
- Inherited markers from Phase 25 (guest execution, boot-info, source ELF, etc.)

## Verification

- `cargo xtask build-guests`
- `cargo test -p hv-datapath guest_throughput`
- `cargo test -p hv-x86-cpu --features datapath-guest-throughput`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-throughput`
- `cargo test -p hv-hypervisor-efi --features datapath-guest-throughput`
- `cargo clippy -p hv-hypervisor-boot -p hv-hypervisor-efi -p hv-x86-cpu -p hv-datapath --features datapath-guest-throughput -- -D warnings`

## Issues found and fixed

| Issue | Fix |
|-------|-----|
| IPC queue treated monotonic head as capacity | Ring-buffer occupancy check (`head - tail >= queue_slots`) and `% queue_slots` slot indexing in `hv-datapath` (aligned with guest-common) |
| Guest throughput benchmark reset queues per run | Reverted; sustained benchmark reuses one runtime — MID drain keeps producer unblocked |

## Review status

Phase 26 closes the in-VM throughput scaffolding gap from Phase 25: Gate D runs the guest runtime relay benchmark with the official 200 Mbit/s metric, validates the target in init, and wires a throughput CPU seam for live REAL_HW opt-in. Host/CI tests remain validate-only with mock timing; ring-0 firmware with live execution enabled may reach `GuestThroughputDisposition::Executed`. Sustained in-guest benchmark loops in freestanding guest source remain deferred.
